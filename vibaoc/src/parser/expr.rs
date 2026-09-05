// ============================================================
// VIBAO COMPILER (Rust) — parser/expr.rs
// Expression parsing: a Pratt parser for expressions (literals,
// variables, binary operators, function calls, color functions,
// arrays, objects, template strings). Equivalent to the expression
// section of 05-parser-core.ts + the entirety of 06-parser-expr.ts from
// the old TS version, combined here since Rust splits files by
// FUNCTIONAL GROUP (everything related to "expressions") rather than
// by "core vs extended" like the TS version did.
// ============================================================

use super::{ParseError, Parser};
use vibao_ast::{BinaryOp, Expr, LiteralValue, Pos, TemplatePart, UnaryOp};
use crate::lexer::TokenKind;

impl Parser {
    // ════════════════════════════════════════════════════════
    // ENTRY POINT — parses a full expression (including operators)
    // ════════════════════════════════════════════════════════

    /// Parses an expression using Pratt parsing (precedence climbing).
    /// min_prec is the minimum precedence threshold needed to keep
    /// "consuming" more operators — recursion unwinds once an operator
    /// with lower precedence than the threshold is encountered.
    pub(crate) fn parse_expr(&mut self, min_prec: u8) -> Result<Expr, ParseError> {
        let mut left = self.parse_primary()?;

        loop {
            let op = match self.current_binary_op() {
                Some(op) => op,
                None => break,
            };
            let prec = binary_precedence(op);
            if prec <= min_prec {
                break;
            }
            let pos = left.pos();
            self.advance(); // consume the operator token
            let right = self.parse_expr(prec)?;
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
                pos,
            };
        }

        Ok(left)
    }

    /// A convenience shortcut for parse_expr(0) — used everywhere "any
    /// value" is needed (props, args, ...) without caring about the
    /// precedence threshold.
    pub(crate) fn parse_value(&mut self) -> Result<Expr, ParseError> {
        self.parse_expr(0)
    }

    // ════════════════════════════════════════════════════════
    // BINARY OPERATOR DETECTION
    // ════════════════════════════════════════════════════════

    /// Recognizes whether the current token is a binary operator,
    /// returning None if not (stopping the Pratt parser loop). Checks
    /// against the explicit TokenKind — not based on a string value like
    /// a bug once encountered in the old TS version (getBinaryOp
    /// compared t.value instead of t.type).
    fn current_binary_op(&self) -> Option<BinaryOp> {
        match &self.current().kind {
            TokenKind::Plus => Some(BinaryOp::Add),
            TokenKind::Minus => Some(BinaryOp::Sub),
            TokenKind::Star => Some(BinaryOp::Mul),
            TokenKind::Slash => Some(BinaryOp::Div),
            TokenKind::Percent => Some(BinaryOp::Mod),
            TokenKind::Gt => Some(BinaryOp::Gt),
            TokenKind::Lt => Some(BinaryOp::Lt),
            TokenKind::Gte => Some(BinaryOp::Gte),
            TokenKind::Lte => Some(BinaryOp::Lte),
            TokenKind::EqEq => Some(BinaryOp::Eq),
            TokenKind::Neq => Some(BinaryOp::Neq),
            TokenKind::AndAnd => Some(BinaryOp::And),
            TokenKind::OrOr => Some(BinaryOp::Or),
            _ => None,
        }
    }

    // ════════════════════════════════════════════════════════
    // PRIMARY EXPRESSION
    // ════════════════════════════════════════════════════════

    fn parse_primary(&mut self) -> Result<Expr, ParseError> {
        let pos = self.current_pos();

        // Unary: !expr - logical negation (e.g. !$da_dang_nhap)
        if matches!(self.current().kind, TokenKind::Bang) {
            self.advance();
            let operand = self.parse_primary()?;
            return Ok(Expr::Unary {
                op: UnaryOp::Not,
                operand: Box::new(operand),
                pos,
            });
        }

        // Unary: -expr (not part of the current ViBao spec, but kept as
        // a placeholder for the future - a "-" in operand position has
        // already been folded directly into a NumberLit as a negative
        // sign by the lexer, so a separate Unary Neg here isn't needed
        // for the numeric case; this is only kept open for extension).
        if matches!(self.current().kind, TokenKind::Minus) {
            self.advance();
            let operand = self.parse_primary()?;
            return Ok(Expr::Unary {
                op: UnaryOp::Neg,
                operand: Box::new(operand),
                pos,
            });
        }

        // ( expr )
        if self.check(&TokenKind::LParen) {
            self.advance();
            let inner = self.parse_expr(0)?;
            self.expect(&TokenKind::RParen)?;
            return Ok(inner);
        }

        // Array [ ... ]
        if self.check(&TokenKind::LBracket) {
            return self.parse_array();
        }

        // Object { key: value, ... }
        // CONTEXT WARNING: '{' is also used for an action block
        // (on_click: { $n = $n - 1 }) and a children block (element {
        // ... }) elsewhere in ViBao's grammar - NOT every '{' should be
        // read as an Object literal. parse_primary() should only be
        // called in a context that's certain to be expecting an
        // EXPRESSION (e.g. the right side of '=', inside a function
        // call's args list). action.rs and element.rs MUST recognize and
        // consume the '{' of an action-block/children-block THEMSELVES
        // BEFORE calling parse_expr(), and must never let parse_expr
        // decide on its own what '{' means in those 2 contexts - failing
        // to do so would misinterpret it exactly like a bug once
        // encountered in the old TS version, where a construct's context
        // was misread.
        if self.check(&TokenKind::LBrace) {
            return self.parse_object();
        }

        // Color functions: trong_suot(...), lam_sang(...), lam_toi(...) -
        // the function name is checked against the shared table at
        // lexer::tables::resolve_color_func_name (UNIFIED SOURCE OF
        // TRUTH: these 3 names used to be independently hardcoded here
        // AND in codegen/mod.rs::color_func_name - 2 separate places that
        // could easily drift apart if only one side was updated when a
        // color function name changed or was added).
        if let TokenKind::Identifier(name) = &self.current().kind {
            let name = name.clone();
            if crate::lexer::resolve_color_func_name(&name).is_some() {
                return self.parse_color_func(&name, pos);
            }
        }

        // A function call inside an expression: gia_tien($x),
        // rut_gon($s, 50). A Component can also be called like a
        // function in some contexts (action: dieu_huong("/")), so this
        // checks both Identifier and Component followed by LParen.
        // ColorName is included too, for consistency with
        // expect_identifier_like() - guarding against a future builtin
        // function that happens to share a name with one of the 14 color
        // names (no such collision currently exists, but this follows
        // the general PRINCIPLE: any position that needs to read a NAME
        // should accept all 3 of these token kinds).
        let callee_name = match &self.current().kind {
            TokenKind::Identifier(n) => Some(n.clone()),
            TokenKind::Component(n) => Some(n.clone()),
            TokenKind::ColorName(n) => Some(n.clone()),
            _ => None,
        };
        if let Some(name) = callee_name {
            if self.check_at(1, &TokenKind::LParen) {
                self.advance(); // the function name
                self.advance(); // (
                let mut args = Vec::new();
                while !self.check(&TokenKind::RParen) && !self.check(&TokenKind::Eof) {
                    args.push(self.parse_expr(0)?);
                    self.skip_comma();
                }
                self.expect(&TokenKind::RParen)?;
                // FunctionName is a semantic identity; the locale
                // surface is normalized to its canonical runtime name
                // before the Expr is serialized into the registry. An
                // unknown function name is left as-is so the validator
                // can report an error.
                let callee = crate::locale::resolve_function_name(&name)
                    .map(|function| function.runtime_name().to_string())
                    .unwrap_or(name);
                return Ok(Expr::Call { callee, args, pos });
            }
        }

        // A variable $ten (+ member access $obj.field.sub)
        if let TokenKind::Variable(_) = &self.current().kind {
            return self.parse_variable_expr();
        }

        // A literal (string/number/bool/color) - including a bare identifier
        // used as a string value (e.g. huong:row, can:giua - see parse_literal).
        self.parse_literal()
    }

    // ── A variable + member access ($obj.field.sub) ──────────────────────
    fn parse_variable_expr(&mut self) -> Result<Expr, ParseError> {
        let pos = self.current_pos();
        let name = match self.advance().kind {
            TokenKind::Variable(n) => n,
            _ => unreachable!("already checked for Variable before calling this function"),
        };

        // Interpolated strings: if a variable's name were to contain a $
        // internally, that's handled differently, since a template
        // string is split elsewhere (ViBao parses a string like
        // "Xin chao $ten" separately, inside the lexer's read_string, as
        // one raw StringLit; parse_literal() below is what later splits
        // it into a TemplateString if needed - a standalone variable
        // here is always a plain $ten).
        let mut node = Expr::Variable(name, pos);

        while self.check(&TokenKind::Dot) {
            self.advance();
            let prop = match &self.current().kind {
                TokenKind::Identifier(s) => s.clone(),
                TokenKind::Component(s) => s.clone(), // e.g. $item.text if "text" collides with a component name
                // A REAL BUG THAT WAS FIXED (the same kind as "nhan"
                // colliding with a tag in parse_component_def): a field
                // name colliding with one of the 14 color names (e.g.
                // $item.do, $item.den) used to NOT parse - the lexer
                // emits ColorName before this code is even reached.
                TokenKind::ColorName(s) => s.clone(),
                other => {
                    return Err(self.error(format!(
                        "Expected a field name after '.', received {}",
                        other
                    )))
                }
            };
            self.advance();
            node = Expr::MemberAccess {
                object: Box::new(node),
                property: prop,
                pos,
            };
        }

        Ok(node)
    }

    // ── Literal: string/number/bool/color/bare-identifier ──────────
    pub(crate) fn parse_literal(&mut self) -> Result<Expr, ParseError> {
        let pos = self.current_pos();
        let tok = self.current().kind.clone();

        match tok {
            TokenKind::StringLit(s) => {
                self.advance();
                // If the string contains "$", split it into a
                // TemplateString so codegen can later bind the correct
                // dynamic variable - see parse_template_string.
                if s.contains('$') {
                    Ok(parse_template_string(&s, pos))
                } else {
                    Ok(Expr::Literal(LiteralValue::Str(s), pos))
                }
            }
            TokenKind::NumberLit(v, raw) => {
                self.advance();
                Ok(Expr::literal_num_with_unit(v, extract_unit_suffix(&raw), pos))
            }
            TokenKind::BoolLit(b) => {
                self.advance();
                Ok(Expr::Literal(LiteralValue::Bool(b), pos))
            }
            TokenKind::ColorHex(h) => {
                self.advance();
                Ok(Expr::Literal(LiteralValue::Color(h), pos))
            }
            TokenKind::ColorName(n) => {
                self.advance();
                // `resolve_color_name` returns `Option<String>` (changed
                // from a plain `String` so the validator layer can
                // distinguish "a valid color name" from "an unknown
                // name" ELSEWHERE - see validator.rs). HERE it is always
                // `Some`: the lexer only ever emits TokenKind::ColorName(n)
                // once it has already confirmed `n` exists in
                // color_map() (see is_prop_value_position in scan.rs) -
                // so a `None` at this point means an internal invariant
                // was violated (a real bug in the compiler itself), not
                // a ViBao user's syntax error.
                let hex = crate::lexer::resolve_color_name(&n)
                    .expect("invariant violated: the lexer should only ever emit ColorName for a name already present in color_map()");
                Ok(Expr::Literal(LiteralValue::Color(hex), pos))
            }
            // A bare identifier used as a string value - ViBao uses this
            // pattern extensively: huong:row, can:giua, fit:cover,
            // loai:email. This was a bug found and fixed in the old TS
            // version (this branch was missing entirely, crashing the
            // parser on every prop that used a bare keyword) - gotten
            // right from the start in the Rust port.
            TokenKind::Identifier(s) => {
                self.advance();
                Ok(Expr::Literal(LiteralValue::Str(s), pos))
            }
            TokenKind::Component(s) => {
                self.advance();
                Ok(Expr::Literal(LiteralValue::Str(s), pos))
            }
            other => Err(self.error(format!("Could not parse value: {}", other))),
        }
    }

    // ── Color functions: trong_suot(mau, amount), lam_sang(...), lam_toi(...) ──
    fn parse_color_func(&mut self, name: &str, pos: Pos) -> Result<Expr, ParseError> {
        // Checked against the shared table - see the note at the call
        // site (parse_primary) and at lexer::tables::color_func_map()
        // for why they're unified.
        let kind = crate::lexer::resolve_color_func_name(name)
            .unwrap_or_else(|| unreachable!("already checked this is a valid color function name before calling"));
        self.advance(); // the function name
        self.expect(&TokenKind::LParen)?;
        let color = self.parse_expr(0)?;
        self.expect(&TokenKind::Comma)?;
        let amount_tok = self.expect(&TokenKind::NumberLit(0.0, String::new()))?;
        let amount = match amount_tok.kind {
            TokenKind::NumberLit(v, _) => v,
            _ => unreachable!(),
        };
        self.expect(&TokenKind::RParen)?;
        Ok(Expr::ColorFunc {
            func: kind,
            color: Box::new(color),
            amount,
            pos,
        })
    }

    // ── Array [item1, item2, ...] ────────────────────────────────────
    fn parse_array(&mut self) -> Result<Expr, ParseError> {
        let pos = self.current_pos();
        self.expect(&TokenKind::LBracket)?;
        let mut items = Vec::new();
        while !self.check(&TokenKind::RBracket) && !self.check(&TokenKind::Eof) {
            items.push(self.parse_expr(0)?);
            self.skip_comma();
        }
        self.expect(&TokenKind::RBracket)?;
        Ok(Expr::Array(items, pos))
    }

    // ── Object { key: value, ... } ────────────────────────────────────
    fn parse_object(&mut self) -> Result<Expr, ParseError> {
        let pos = self.current_pos();
        self.expect(&TokenKind::LBrace)?;
        let mut fields = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.check(&TokenKind::Eof) {
            let key = self.expect_identifier_like()?;
            self.expect(&TokenKind::Colon)?;
            let value = self.parse_expr(0)?;
            fields.push((key, value));
            self.skip_comma();
        }
        self.expect(&TokenKind::RBrace)?;
        Ok(Expr::Object(fields, pos))
    }

    /// Reads a field/key name as an identifier - accepts Identifier,
    /// Component (e.g. a key name colliding with an existing component
    /// like "text"), AND ColorName.
    ///
    /// A REAL BUG THAT WAS FIXED (a deeper root cause than initially
    /// found): the lexer's `classify_identifier` (in scan.rs) has a
    /// FINAL fallback that applies REGARDLESS OF POSITION: any
    /// identifier matching one of the 14 valid color names (den, trang,
    /// do, xanh...) that ISN'T already a known keyword/component gets
    /// automatically lexed as `ColorName` - not only when it follows a
    /// ':' (a value position), as an old comment there assumed was the
    /// only case. The real-world impact turned out worse than initially
    /// thought: the prop-KEY `den` (e.g. `link(den: "/path")`) NEVER
    /// parsed, in ANY position within a prop list (even the very first
    /// position) - because the word "den" as a KEY was itself lexed as
    /// ColorName before expect_identifier_like() ever got to see it, and
    /// this function used to only accept Identifier/Component, not
    /// ColorName, so it always raised "Expected an identifier, received
    /// color name 'den'".
    ///
    /// Fixed at the actual root cause HERE (not just in the prop-value
    /// reading code): anywhere that needs to read a NAME (a prop's key,
    /// a field name, a param name...) must accept ColorName as a valid
    /// identifier and return its ORIGINAL NAME (not the already-resolved
    /// hex code) - as if it had never been misread as a color by the
    /// lexer. This is safe because the context "currently reading a
    /// NAME" (key/field/param) is never valid for an actual color value
    /// - if execution reaches this code, the ViBao author's intent must
    /// have been to use "den"/"trang"/... as an identifier, not as a
    /// color.
    pub(crate) fn expect_identifier_like(&mut self) -> Result<String, ParseError> {
        match &self.current().kind {
            TokenKind::Identifier(s) => {
                let s = s.clone();
                self.advance();
                Ok(s)
            }
            TokenKind::Component(s) => {
                let s = s.clone();
                self.advance();
                Ok(s)
            }
            TokenKind::ColorName(s) => {
                let s = s.clone();
                self.advance();
                Ok(s)
            }
            other => Err(self.error(format!(
                "Expected an identifier, received {}",
                other
            ))),
        }
    }
}

// ════════════════════════════════════════════════════════════
// PRECEDENCE TABLE
// ════════════════════════════════════════════════════════════

fn binary_precedence(op: BinaryOp) -> u8 {
    match op {
        BinaryOp::Or => 1,
        BinaryOp::And => 2,
        BinaryOp::Eq
        | BinaryOp::Neq
        | BinaryOp::Gt
        | BinaryOp::Gte
        | BinaryOp::Lt
        | BinaryOp::Lte => 3,
        BinaryOp::Add | BinaryOp::Sub => 4,
        BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod => 5,
    }
}

// ════════════════════════════════════════════════════════════
// NUMBERS + CSS UNITS
// ════════════════════════════════════════════════════════════

/// Extracts the CSS unit suffix (px, %, vw, vh, em, rem) from the raw
/// number string returned by the lexer (e.g. "50%" -> Some("%"), "16"
/// -> None). The lexer (read_number) already matched these units and
/// appended them to the end of the number string - here we just read
/// back the trailing part that isn't a digit/dot/minus sign.
fn extract_unit_suffix(raw: &str) -> Option<String> {
    let unit_start = raw.find(|c: char| !c.is_ascii_digit() && c != '.' && c != '-')?;
    let unit = &raw[unit_start..];
    if unit.is_empty() {
        None
    } else {
        Some(unit.to_string())
    }
}

// ════════════════════════════════════════════════════════════
// TEMPLATE STRING PARSING (a module-level function, doesn't need &self)
// ════════════════════════════════════════════════════════════

/// Splits a raw string like "Xin chao $ten, ban $tuoi.nam tuoi" into an
/// Expr::TemplateString with interleaved text/variable/member parts.
/// Called from parse_literal() when a string is found to contain a "$".
fn parse_template_string(raw: &str, pos: Pos) -> Expr {
    let chars: Vec<char> = raw.chars().collect();
    let mut parts = Vec::new();
    let mut i = 0;
    let mut text_buf = String::new();

    while i < chars.len() {
        if chars[i] == '$' {
            // A dollar sign only starts interpolation when followed by a
            // valid variable-name start. This keeps literal currency and
            // standalone dollar signs from disappearing silently.
            if i + 1 >= chars.len()
                || !(chars[i + 1].is_ascii_alphabetic() || chars[i + 1] == '_')
            {
                text_buf.push('$');
                i += 1;
                continue;
            }

            if !text_buf.is_empty() {
                parts.push(TemplatePart::Text(std::mem::take(&mut text_buf)));
            }
            i += 1;
            let mut name = String::new();
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                name.push(chars[i]);
                i += 1;
            }

            // $obj.field.sub - collected into a single path
            let mut path = vec![name];
            while i < chars.len() && chars[i] == '.' {
                i += 1;
                let mut sub = String::new();
                while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                    sub.push(chars[i]);
                    i += 1;
                }
                if !sub.is_empty() {
                    path.push(sub);
                }
            }

            if path.len() > 1 {
                parts.push(TemplatePart::Member(path));
            } else {
                parts.push(TemplatePart::Variable(path.into_iter().next().unwrap_or_default()));
            }
        } else {
            text_buf.push(chars[i]);
            i += 1;
        }
    }

    if !text_buf.is_empty() {
        parts.push(TemplatePart::Text(text_buf));
    }

    Expr::TemplateString(parts, pos)
}

// ════════════════════════════════════════════════════════════
// UNIT TESTS
// ════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::tokenize;
    use vibao_ast::ColorFuncKind;

    fn parse_expr_from_source(src: &str) -> Expr {
        let tokens = tokenize(src).unwrap();
        let mut p = Parser::new(tokens);
        p.parse_expr(0).unwrap()
    }

    #[test]
    fn test_simple_arithmetic_precedence() {
        // 1 + 2 * 3  ->  must group as 1 + (2 * 3), not (1+2)*3
        let expr = parse_expr_from_source("1 + 2 * 3");
        match expr {
            Expr::Binary { op: BinaryOp::Add, right, .. } => match *right {
                Expr::Binary { op: BinaryOp::Mul, .. } => {} // correct
                _ => panic!("wrong precedence: * must be in the right branch of +"),
            },
            _ => panic!("result is not Binary Add"),
        }
    }

    #[test]
    fn test_variable_minus_number() {
        // $n - 1 - the exact bug once encountered in the old TS/JS
        // version (a "-" standing apart with spaces)
        let expr = parse_expr_from_source("$n - 1");
        match expr {
            Expr::Binary { op: BinaryOp::Sub, left, right, .. } => {
                assert!(matches!(*left, Expr::Variable(ref s, _) if s == "n"));
                assert!(matches!(*right, Expr::Literal(LiteralValue::Num(v, _), _) if v == 1.0));
            }
            _ => panic!("must parse as subtraction"),
        }
    }

    #[test]
    fn test_bare_identifier_as_string_literal() {
        // "row" standing alone (e.g. the value of huong:row) must parse
        // as a string literal, not an error - a bug once encountered in
        // the old TS version.
        let expr = parse_expr_from_source("row");
        assert!(matches!(expr, Expr::Literal(LiteralValue::Str(ref s), _) if s == "row"));
    }

    #[test]
    fn test_member_access_chain() {
        let expr = parse_expr_from_source("$obj.field.sub");
        match expr {
            Expr::MemberAccess { property, .. } => assert_eq!(property, "sub"),
            _ => panic!("must be MemberAccess"),
        }
    }

    #[test]
    fn test_function_call_in_expr() {
        let expr = parse_expr_from_source("gia_tien($gia)");
        match expr {
            Expr::Call { callee, args, .. } => {
                assert_eq!(callee, "gia_tien");
                assert_eq!(args.len(), 1);
            }
            _ => panic!("must be Call"),
        }
    }

    #[test]
    fn test_template_dollar_literal_with_currency() {
        let expr = parse_expr_from_source(r#""Gia: $50""#);
        match expr {
            Expr::TemplateString(parts, _) => {
                assert_eq!(parts.len(), 1);
                assert!(matches!(parts[0], TemplatePart::Text(ref s) if s == "Gia: $50"));
            }
            _ => panic!("a string containing $50 must remain as text"),
        }
    }

    #[test]
    fn test_template_dollar_literal_when_not_followed_by_identifier_start() {
        let expr = parse_expr_from_source(r#""Gia tien: $ va $5""#);
        match expr {
            Expr::TemplateString(parts, _) => {
                assert_eq!(parts.len(), 1);
                assert!(matches!(parts[0], TemplatePart::Text(ref s) if s == "Gia tien: $ va $5"));
            }
            _ => panic!("a $ that doesn't start interpolation must be preserved"),
        }
    }

    #[test]
    fn test_template_string_extraction() {
        let expr = parse_expr_from_source(r#""Xin chào $ten""#);
        match expr {
            Expr::TemplateString(parts, _) => {
                assert_eq!(parts.len(), 2);
                assert!(matches!(parts[0], TemplatePart::Text(ref s) if s == "Xin chào "));
                assert!(matches!(parts[1], TemplatePart::Variable(ref s) if s == "ten"));
            }
            _ => panic!("must split into a TemplateString"),
        }
    }

    #[test]
    fn test_logical_and_or_precedence() {
        // $a > 1 && $b > 2 || $c   ->   (($a>1) && ($b>2)) || $c
        let expr = parse_expr_from_source("$a > 1 && $b > 2 || $c");
        match expr {
            Expr::Binary { op: BinaryOp::Or, .. } => {} // correct: Or at the outermost level (lowest precedence)
            _ => panic!("the lowest-precedence operator (||) must be at the root of the tree"),
        }
    }

    #[test]
    fn test_color_function() {
        let expr = parse_expr_from_source("trong_suot(den, 50)");
        match expr {
            Expr::ColorFunc { func, amount, .. } => {
                assert_eq!(func, ColorFuncKind::TrongSuot);
                assert_eq!(amount, 50.0);
            }
            _ => panic!("must be ColorFunc"),
        }
    }

    #[test]
    fn test_logical_not_unary() {
        let expr = parse_expr_from_source("!$da_dang_nhap");
        match expr {
            Expr::Unary { op: UnaryOp::Not, operand, .. } => {
                assert!(matches!(*operand, Expr::Variable(ref s, _) if s == "da_dang_nhap"));
            }
            _ => panic!("must parse as Unary Not"),
        }
    }

    #[test]
    fn test_modulo_operator() {
        let expr = parse_expr_from_source("$n % 2");
        match expr {
            Expr::Binary { op: BinaryOp::Mod, left, right, .. } => {
                assert!(matches!(*left, Expr::Variable(ref s, _) if s == "n"));
                assert!(matches!(*right, Expr::Literal(LiteralValue::Num(v, _), _) if v == 2.0));
            }
            _ => panic!("must parse as modulo (Mod)"),
        }
    }

    #[test]
    fn test_number_literal_keeps_css_unit() {
        // "50%" must keep its "%" unit in the AST, not just at the lexer
        // level - codegen (props.rs/layout.rs) needs to know this is a %
        // rather than the default px. This was a bug found by
        // cross-checking the old ast.rs (Num(f64) had nowhere to store a
        // unit) against the lexer (NumberLit(f64, String) carried the raw
        // string).
        let expr = parse_expr_from_source("50%");
        match expr {
            Expr::Literal(LiteralValue::Num(v, unit), _) => {
                assert_eq!(v, 50.0);
                assert_eq!(unit, Some("%".to_string()));
            }
            _ => panic!("must be a Literal Num with unit %"),
        }
    }

    #[test]
    fn test_number_literal_without_unit_is_none() {
        let expr = parse_expr_from_source("16");
        match expr {
            Expr::Literal(LiteralValue::Num(v, unit), _) => {
                assert_eq!(v, 16.0);
                assert_eq!(unit, None);
            }
            _ => panic!("must be a Literal Num with no unit"),
        }
    }

    #[test]
    fn test_modulo_precedence_with_addition() {
        // 1 + 2 % 3  ->  1 + (2 % 3), since % has the same precedence as * / (higher than +)
        let expr = parse_expr_from_source("1 + 2 % 3");
        match expr {
            Expr::Binary { op: BinaryOp::Add, right, .. } => match *right {
                Expr::Binary { op: BinaryOp::Mod, .. } => {}
                _ => panic!("% must be in the right branch of +"),
            },
            _ => panic!("result is not Binary Add"),
        }
    }

    /// A regression test DIRECTLY at the actual root cause of the bug
    /// (not only through the higher-level tests in element.rs):
    /// `expect_identifier_like()` must accept a `ColorName` token as a
    /// valid name, in ANY position that needs to read a NAME
    /// (key/field/param) - not only when it happens to follow a ':'.
    /// Before the fix, this function panicked with "Expected an
    /// identifier name, received color name 'den'" whenever one of the
    /// 14 color names (den, trang, do, xanh...) appeared in a KEY
    /// position - discovered through an actual test build:
    /// `link(den: "/path")` never parsed, because `den` (the prop-key
    /// itself) was always lexed as ColorName by the final fallback in
    /// classify_identifier (scan.rs), which applies regardless of
    /// position, not only after a ':' like the is_prop_value_position
    /// branch above it.
    #[test]
    fn test_expect_identifier_like_accepts_color_name_token() {
        let tokens = tokenize("den").unwrap();
        let mut p = Parser::new(tokens);
        let name = p.expect_identifier_like().unwrap();
        assert_eq!(name, "den");
    }

    /// Same root cause, reproducing the exact real-world scenario that
    /// caused the bug: an object literal with a KEY matching a color
    /// name, in the 2nd position (after a ','), exactly like how
    /// `link("Trang chu", den: "...")` places the key `den` in the 2nd
    /// position of the prop list.
    #[test]
    fn test_object_literal_with_color_named_key_parses() {
        let expr = parse_expr_from_source(r#"{ mau: "x", den: "y" }"#);
        match expr {
            Expr::Object(fields, _) => {
                assert!(fields.iter().any(|(k, _)| k == "den"), "missing field 'den': {:?}", fields);
            }
            _ => panic!("result must be an Object"),
        }
    }

    #[test]
    fn test_english_builtin_function_is_normalized_to_canonical_runtime_name() {
        let expr = parse_expr_from_source("format_price(1000)");
        match expr {
            Expr::Call { callee, .. } => assert_eq!(callee, "gia_tien"),
            other => panic!("expected Expr::Call, got {:?}", other),
        }
    }

}
