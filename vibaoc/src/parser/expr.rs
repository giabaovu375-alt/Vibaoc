use super::{ParseError, Parser};
use crate::lexer::TokenKind;
use vibao_ast::{BinaryOp, Expr, LiteralValue, Pos, TemplatePart, UnaryOp};

impl Parser {
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
            self.advance();
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

    pub(crate) fn parse_value(&mut self) -> Result<Expr, ParseError> {
        self.parse_expr(0)
    }

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

    fn parse_primary(&mut self) -> Result<Expr, ParseError> {
        let pos = self.current_pos();

        if matches!(self.current().kind, TokenKind::Bang) {
            self.advance();
            let operand = self.parse_primary()?;
            return Ok(Expr::Unary {
                op: UnaryOp::Not,
                operand: Box::new(operand),
                pos,
            });
        }

        if matches!(self.current().kind, TokenKind::Minus) {
            self.advance();
            let operand = self.parse_primary()?;
            return Ok(Expr::Unary {
                op: UnaryOp::Neg,
                operand: Box::new(operand),
                pos,
            });
        }

        if self.check(&TokenKind::LParen) {
            self.advance();
            let inner = self.parse_expr(0)?;
            self.expect(&TokenKind::RParen)?;
            return Ok(inner);
        }

        if self.check(&TokenKind::LBracket) {
            return self.parse_array();
        }

        if self.check(&TokenKind::LBrace) {
            return self.parse_object();
        }

        if let TokenKind::Identifier(name) = &self.current().kind {
            let name = name.clone();
            if crate::lexer::resolve_color_func_name(&name).is_some() {
                return self.parse_color_func(&name, pos);
            }
        }

        let callee_name = match &self.current().kind {
            TokenKind::Identifier(n) => Some(n.clone()),
            TokenKind::Component(n) => Some(n.clone()),
            TokenKind::ColorName(n) => Some(n.clone()),
            _ => None,
        };
        if let Some(name) = callee_name {
            if self.check_at(1, &TokenKind::LParen) {
                self.advance();
                self.advance();
                let mut args = Vec::new();
                while !self.check(&TokenKind::RParen) && !self.check(&TokenKind::Eof) {
                    args.push(self.parse_expr(0)?);
                    self.skip_comma();
                }
                self.expect(&TokenKind::RParen)?;
                let callee = crate::locale::resolve_function_name(&name)
                    .map(|function| function.runtime_name().to_string())
                    .unwrap_or(name);
                return Ok(Expr::Call { callee, args, pos });
            }
        }

        if let TokenKind::Variable(_) = &self.current().kind {
            return self.parse_variable_expr();
        }

        self.parse_literal()
    }

    fn at_literal_index(&self) -> bool {
        self.check(&TokenKind::LBracket)
            && self.check_at(1, &TokenKind::NumberLit(0.0, String::new()))
            && self.check_at(2, &TokenKind::RBracket)
    }

    fn parse_variable_expr(&mut self) -> Result<Expr, ParseError> {
        let pos = self.current_pos();
        let name = match self.advance().kind {
            TokenKind::Variable(n) => n,
            _ => unreachable!("already checked for Variable before calling this function"),
        };

        let mut node = Expr::Variable(name, pos);

        while self.check(&TokenKind::Dot) || self.at_literal_index() {
            if self.check(&TokenKind::LBracket) {
                self.advance();
                let index = match &self.current().kind {
                    TokenKind::NumberLit(v, _) => *v as usize,
                    _ => 0,
                };
                self.advance();
                self.expect(&TokenKind::RBracket)?;
                node = Expr::MemberAccess {
                    object: Box::new(node),
                    property: index.to_string(),
                    pos,
                };
                continue;
            }
            self.advance();
            let prop = match &self.current().kind {
                TokenKind::Identifier(s) => s.clone(),
                TokenKind::Component(s) => s.clone(),
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

    pub(crate) fn parse_literal(&mut self) -> Result<Expr, ParseError> {
        let pos = self.current_pos();
        let tok = self.current().kind.clone();

        match tok {
            TokenKind::StringLit(s) => {
                self.advance();
                if s.contains('$') {
                    Ok(parse_template_string(&s, pos))
                } else {
                    Ok(Expr::Literal(LiteralValue::Str(s), pos))
                }
            }
            TokenKind::NumberLit(v, raw) => {
                self.advance();
                Ok(Expr::literal_num_with_unit(
                    v,
                    extract_unit_suffix(&raw),
                    pos,
                ))
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
                let hex = crate::lexer::resolve_color_name(&n)
                    .expect("invariant violated: the lexer should only ever emit ColorName for a name already present in color_map()");
                Ok(Expr::Literal(LiteralValue::Color(hex), pos))
            }
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

    fn parse_color_func(&mut self, name: &str, pos: Pos) -> Result<Expr, ParseError> {
        let kind = crate::lexer::resolve_color_func_name(name).unwrap_or_else(|| {
            unreachable!("already checked this is a valid color function name before calling")
        });
        self.advance();
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
            other => Err(self.error(format!("Expected an identifier, received {}", other))),
        }
    }
}

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

fn extract_unit_suffix(raw: &str) -> Option<String> {
    let unit_start = raw.find(|c: char| !c.is_ascii_digit() && c != '.' && c != '-')?;
    let unit = &raw[unit_start..];
    if unit.is_empty() {
        None
    } else {
        Some(unit.to_string())
    }
}

fn parse_template_string(raw: &str, pos: Pos) -> Expr {
    let chars: Vec<char> = raw.chars().collect();
    let mut parts = Vec::new();
    let mut i = 0;
    let mut text_buf = String::new();

    while i < chars.len() {
        if chars[i] == '$' {
            if i + 1 >= chars.len() || !(chars[i + 1].is_alphabetic() || chars[i + 1] == '_') {
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

            let mut path = vec![crate::lexer::strip_diacritics_keep_case(&name)];
            while i < chars.len() && chars[i] == '.' {
                i += 1;
                let mut sub = String::new();
                while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                    sub.push(chars[i]);
                    i += 1;
                }
                if !sub.is_empty() {
                    path.push(crate::lexer::strip_diacritics_keep_case(&sub));
                }
            }

            if path.len() > 1 {
                parts.push(TemplatePart::Member(path));
            } else {
                parts.push(TemplatePart::Variable(
                    path.into_iter().next().unwrap_or_default(),
                ));
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
        let expr = parse_expr_from_source("1 + 2 * 3");
        match expr {
            Expr::Binary {
                op: BinaryOp::Add,
                right,
                ..
            } => match *right {
                Expr::Binary {
                    op: BinaryOp::Mul, ..
                } => {}
                _ => panic!("wrong precedence: * must be in the right branch of +"),
            },
            _ => panic!("result is not Binary Add"),
        }
    }

    #[test]
    fn test_variable_minus_number() {
        let expr = parse_expr_from_source("$n - 1");
        match expr {
            Expr::Binary {
                op: BinaryOp::Sub,
                left,
                right,
                ..
            } => {
                assert!(matches!(*left, Expr::Variable(ref s, _) if s == "n"));
                assert!(matches!(*right, Expr::Literal(LiteralValue::Num(v, _), _) if v == 1.0));
            }
            _ => panic!("must parse as subtraction"),
        }
    }

    #[test]
    fn test_bare_identifier_as_string_literal() {
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
    fn test_literal_index_becomes_numeric_member_access() {
        let expr = parse_expr_from_source("$ph.choices[0].message");
        match expr {
            Expr::MemberAccess {
                object, property, ..
            } => {
                assert_eq!(property, "message");
                match *object {
                    Expr::MemberAccess {
                        object, property, ..
                    } => {
                        assert_eq!(property, "0");
                        assert!(
                            matches!(*object, Expr::MemberAccess { ref property, .. } if property == "choices")
                        );
                    }
                    _ => panic!("chỉ số [0] phải thành MemberAccess \"0\""),
                }
            }
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
    fn test_template_string_variable_with_diacritics_is_normalized() {
        let expr = parse_expr_from_source(r#""Xin chào $tên, $đường_dẫn.mô_tả""#);
        match expr {
            Expr::TemplateString(parts, _) => {
                assert_eq!(parts.len(), 4);
                assert!(matches!(parts[1], TemplatePart::Variable(ref s) if s == "ten"));
                assert!(
                    matches!(parts[3], TemplatePart::Member(ref p) if p == &["duong_dan", "mo_ta"])
                );
            }
            _ => panic!("phải tách thành TemplateString"),
        }
    }

    #[test]
    fn test_logical_and_or_precedence() {
        let expr = parse_expr_from_source("$a > 1 && $b > 2 || $c");
        match expr {
            Expr::Binary {
                op: BinaryOp::Or, ..
            } => {}
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
            Expr::Unary {
                op: UnaryOp::Not,
                operand,
                ..
            } => {
                assert!(matches!(*operand, Expr::Variable(ref s, _) if s == "da_dang_nhap"));
            }
            _ => panic!("must parse as Unary Not"),
        }
    }

    #[test]
    fn test_modulo_operator() {
        let expr = parse_expr_from_source("$n % 2");
        match expr {
            Expr::Binary {
                op: BinaryOp::Mod,
                left,
                right,
                ..
            } => {
                assert!(matches!(*left, Expr::Variable(ref s, _) if s == "n"));
                assert!(matches!(*right, Expr::Literal(LiteralValue::Num(v, _), _) if v == 2.0));
            }
            _ => panic!("must parse as modulo (Mod)"),
        }
    }

    #[test]
    fn test_number_literal_keeps_css_unit() {
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
        let expr = parse_expr_from_source("1 + 2 % 3");
        match expr {
            Expr::Binary {
                op: BinaryOp::Add,
                right,
                ..
            } => match *right {
                Expr::Binary {
                    op: BinaryOp::Mod, ..
                } => {}
                _ => panic!("% must be in the right branch of +"),
            },
            _ => panic!("result is not Binary Add"),
        }
    }

    #[test]
    fn test_expect_identifier_like_accepts_color_name_token() {
        let tokens = tokenize("den").unwrap();
        let mut p = Parser::new(tokens);
        let name = p.expect_identifier_like().unwrap();
        assert_eq!(name, "den");
    }

    #[test]
    fn test_object_literal_with_color_named_key_parses() {
        let expr = parse_expr_from_source(r#"{ mau: "x", den: "y" }"#);
        match expr {
            Expr::Object(fields, _) => {
                assert!(
                    fields.iter().any(|(k, _)| k == "den"),
                    "missing field 'den': {:?}",
                    fields
                );
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
