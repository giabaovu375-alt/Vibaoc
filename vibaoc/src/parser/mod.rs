// ============================================================
// VIBAO COMPILER (Rust) — parser/mod.rs
// The Parser struct + its core helpers (advance, expect, check, error).
// The other files in this module (app.rs, expr.rs, element.rs,
// control.rs, action.rs) each `impl Parser { ... }` to add new methods
// to the SAME Parser struct defined here — Rust allows multiple impl
// blocks to live in multiple files as long as they're in the same
// crate, so there's no need to redefine the struct or use OOP-style
// inheritance.
// ============================================================

mod action;
mod app;
mod control;
mod element;
mod expr;

use vibao_ast::Pos;
use crate::diagnostic::Locale;
use crate::lexer::{Token, TokenKind};
use std::fmt;

// ════════════════════════════════════════════════════════════
// 1. PARSE ERROR
// ════════════════════════════════════════════════════════════

#[derive(Debug)]
pub struct ParseError {
    pub message: String,
    pub line: usize,
    pub column: usize,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "[parser] {} ({}:{})",
            self.message, self.line, self.column
        )
    }
}

impl std::error::Error for ParseError {}

// ════════════════════════════════════════════════════════════
// 2. PARSER STRUCT
// ════════════════════════════════════════════════════════════

pub struct Parser {
    /// The full token stream, already pre-filtered (no
    /// whitespace/comment tokens, since the lexer never emits those
    /// kinds — unlike the old TS version, which had to filter them
    /// separately at the parser stage since the TS lexer emitted both
    /// whitespace and comment tokens).
    pub(crate) tokens: Vec<Token>,
    pub(crate) pos: usize,
    pub(crate) locale: Locale,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens, pos: 0, locale: Locale::English }
    }

    // ── Public entry point ──────────────────────────────────────
    /// Parses the full token stream into a complete Program.
    /// The real implementation lives in app.rs (parse_app) — this
    /// function is just the one public entry point that code outside
    /// the parser module needs to know about.
#[allow(dead_code)]
    pub fn parse(mut self) -> Result<vibao_ast::Program, ParseError> {
        self.parse_language_header()?;
        let app = self.parse_app()?;
        self.expect(&TokenKind::Eof)?;
        Ok(vibao_ast::Program { app })
    }

    pub fn parse_with_locale(mut self) -> Result<(vibao_ast::Program, Locale), ParseError> {
        self.parse_language_header()?;
        let app = self.parse_app()?;
        self.expect(&TokenKind::Eof)?;
        Ok((vibao_ast::Program { app }, self.locale))
    }

    pub(crate) fn locale(&self) -> Locale { self.locale }

    pub(crate) fn parse_language_header(&mut self) -> Result<(), ParseError> {
        let is_lang = matches!(&self.current().kind, TokenKind::Identifier(name) if name == "lang");
        if !is_lang { return Ok(()); }

        let lang_pos = self.current_pos();
        self.advance();
        self.consume(&TokenKind::Equals, "Expected '=' after 'lang'")?;
        let value = match self.advance().kind {
            TokenKind::StringLit(value) => value,
            other => return Err(ParseError {
                message: format!("Expected a language code string after 'lang =', received {}", other),
                line: lang_pos.line,
                column: lang_pos.column,
            }),
        };
        self.consume(&TokenKind::Semicolon, "Expected ';' after the language declaration")?;

        self.locale = Locale::from_code(&value).ok_or_else(|| ParseError {
            message: format!("Unsupported source language '{}'. English is always available; supported diagnostic languages: 'en', 'vi'", value),
            line: lang_pos.line,
            column: lang_pos.column,
        })?;
        Ok(())
    }

}

// ════════════════════════════════════════════════════════════
// 3. CORE HELPERS — shared by every submodule in the parser module
// ════════════════════════════════════════════════════════════

impl Parser {
    /// The current token, without consuming it. If the token stream is
    /// exhausted, returns a "virtual" Eof at the last position — avoids
    /// having to handle Option<&Token> at every call site (simplifies
    /// the logic, at the cost of always having some token to compare
    /// against, even past the end of the array).
    pub(crate) fn current(&self) -> &Token {
        self.tokens
            .get(self.pos)
            .unwrap_or_else(|| self.tokens.last().expect("empty token stream"))
    }

    /// Looks ahead by an offset (without consuming), used for lookahead
    /// when 2 similar-looking syntax structures need to be
    /// distinguished (e.g. an IDENTIFIER followed by a COLON means
    /// key:value, otherwise it could be a literal or a function call).
    pub(crate) fn peek(&self, offset: usize) -> &Token {
        self.tokens
            .get(self.pos + offset)
            .unwrap_or_else(|| self.tokens.last().expect("empty token stream"))
    }

    /// Consumes the current token, returns it, then advances the cursor
    /// by 1. Never advances past Eof — calling advance() repeatedly once
    /// already at Eof will always return that same Eof token, with no
    /// panic and no out-of-bounds read.
    pub(crate) fn advance(&mut self) -> Token {
        let tok = self.current().clone();
        if !matches!(tok.kind, TokenKind::Eof) {
            self.pos += 1;
        }
        tok
    }

    /// Compares the current token's kind — uses the discriminant instead
    /// of a direct == comparison so the data inside a variant doesn't
    /// matter (e.g. only needs to know "is this a StringLit", not what
    /// specific string is inside it, for this check).
    pub(crate) fn check(&self, kind: &TokenKind) -> bool {
        std::mem::discriminant(&self.current().kind) == std::mem::discriminant(kind)
    }

    /// Same as check() but checks at an offset position instead of the current one.
    pub(crate) fn check_at(&self, offset: usize, kind: &TokenKind) -> bool {
        std::mem::discriminant(&self.peek(offset).kind) == std::mem::discriminant(kind)
    }

    /// Requires the current token to have exactly the given kind,
    /// consuming it if so, raising a clear error otherwise. This is the
    /// familiar "expect" found in every recursive-descent parser —
    /// equivalent to this.expect() in the old TS version.
    pub(crate) fn expect(&mut self, kind: &TokenKind) -> Result<Token, ParseError> {
        if self.check(kind) {
            Ok(self.advance())
        } else {
            let cur = self.current();
            Err(ParseError {
                message: format!("Expected {}, received {}", kind, cur.kind),
                line: cur.line,
                column: cur.column,
            })
        }
    }

    /// Checks whether the current token is an Identifier with exactly
    /// the given string value — used for contextual "soft keywords"
    /// (e.g. "trong" in the vong_lap syntax) that have no dedicated
    /// TokenKind, unlike a hard keyword like Trang/Neu (which already
    /// has its own enum variant in the lexer, checked via `check`).
    pub(crate) fn check_ident(&self, name: &str) -> bool {
        matches!(&self.current().kind, TokenKind::Identifier(s) if s == name)
    }

    /// Same as `check_ident`, but ALSO accepts the case where the lexer
    /// classified this word as `TokenKind::ColorName` instead of
    /// `Identifier` — this happens when a word is BOTH a contextual
    /// keyword AND a valid color name (for example "den" = "to" in the
    /// `vong_lap ... tu N den M` syntax, but "den" is also the name of
    /// the color black in color_map()).
    ///
    /// BUG ALREADY FIXED: `read_identifier()`/`classify_identifier()`
    /// has a FINAL fallback branch that checks color_map()
    /// UNCONDITIONALLY of position (unlike the earlier, higher-priority
    /// branch which only applies right after a ':') — so "den" in
    /// "1 den 3" (which never follows a ':') still got classified as
    /// ColorName("den") instead of Identifier("den"), making
    /// `check_ident("den")` (which only matches Identifier)
    /// unexpectedly return false, producing a confusing parse error
    /// ("Expected a number... received color name 'den'").
    ///
    /// DELIBERATELY did NOT fix this by changing `check_ident` itself
    /// (used in many other places), to avoid silently loosening every
    /// check_ident() call across the entire codebase — this separate
    /// helper is only applied at the specific spots CONFIRMED to be at
    /// risk of a color-name collision.
    pub(crate) fn check_ident_or_color_name(&self, name: &str) -> bool {
        match &self.current().kind {
            TokenKind::Identifier(s) if s == name => true,
            TokenKind::ColorName(s) if s == name => true,
            _ => false,
        }
    }

    /// The (line, column) position of the current token — used to
    /// attach a Pos to a newly created node, matching "currentPos()" in
    /// the old TS version.
    pub(crate) fn current_pos(&self) -> Pos {
        let t = self.current();
        Pos {
            line: t.line,
            column: t.column,
        }
    }

    /// Builds a parse error at the current token's position — a short
    /// convenience helper instead of manually constructing
    /// ParseError { ... } everywhere an error needs to be raised.
    pub(crate) fn error(&self, message: impl Into<String>) -> ParseError {
        let cur = self.current();
        ParseError {
            message: message.into(),
            line: cur.line,
            column: cur.column,
        }
    }

    /// Consumes the current token if it's a Comma — used at the end of
    /// every list-parsing loop (args, props, children...) to accept an
    /// optional separator comma (no trailing comma required).
    pub(crate) fn skip_comma(&mut self) {
        if self.check(&TokenKind::Comma) {
            self.advance();
        }
    }

    // ────────────────────────────────────────────────────────────
    // EXTRA HELPER FUNCTIONS FOR COMPATIBILITY WITH ACTION/APP/ELEMENT
    // ────────────────────────────────────────────────────────────

    /// Checks whether the token stream is exhausted
    pub(crate) fn is_at_end(&self) -> bool {
        matches!(self.current().kind, TokenKind::Eof)
    }

    /// Checks whether the current token matches, consuming it immediately if so
    pub(crate) fn match_token(&mut self, kind: &TokenKind) -> bool {
        if self.check(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    /// Same as expect but accepts a custom error message
    pub(crate) fn consume(&mut self, kind: &TokenKind, message: &str) -> Result<Token, ParseError> {
        if self.check(kind) {
            Ok(self.advance())
        } else {
            let cur = self.current();
            Err(ParseError {
                message: format!("{} (Received: {})", message, cur.kind),
                line: cur.line,
                column: cur.column,
            })
        }
    }
}

// ════════════════════════════════════════════════════════════
// 4. MODULE-LEVEL ENTRY POINT — used by main.rs
// ════════════════════════════════════════════════════════════

/// Parses a token stream into a Program. Equivalent to the `parse()`
/// function exported from 05-parser-core.ts in the old version — the
/// one entry point that code outside this module (main.rs, and later
/// codegen.rs) needs to call.
#[allow(dead_code)]
pub fn parse(tokens: Vec<Token>) -> Result<vibao_ast::Program, ParseError> {
    Parser::new(tokens).parse()
}

pub fn parse_with_locale(tokens: Vec<Token>) -> Result<(vibao_ast::Program, Locale), ParseError> {
    Parser::new(tokens).parse_with_locale()
}

// ════════════════════════════════════════════════════════════
// 5. UNIT TESTS
// ════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::tokenize;

    #[test]
    fn test_advance_stops_at_eof() {
        let tokens = tokenize("").unwrap();
        let mut p = Parser::new(tokens);
        for _ in 0..5 {
            let t = p.advance();
            assert_eq!(t.kind, TokenKind::Eof);
        }
    }

    #[test]
    fn test_check_and_expect() {
        let tokens = tokenize(r#""hello""#).unwrap();
        let mut p = Parser::new(tokens);
        assert!(p.check(&TokenKind::StringLit(String::new())));
        let tok = p.expect(&TokenKind::StringLit(String::new())).unwrap();
        assert!(matches!(tok.kind, TokenKind::StringLit(ref s) if s == "hello"));
    }

    #[test]
    fn test_expect_wrong_kind_errors() {
        let tokens = tokenize(r#""hello""#).unwrap();
        let mut p = Parser::new(tokens);
        let result = p.expect(&TokenKind::LParen);
        assert!(result.is_err());
    }

    #[test]
    fn test_expect_error_message_is_human_readable_not_debug_syntax() {
        // A dev-experience improvement: an error message must read
        // naturally (e.g. "string \"hello\"") instead of Rust's raw Debug
        // syntax (e.g. 'StringLit("hello")') - using {:?} here used to
        // make a syntax error look like an internal crash, confusing for
        // someone unfamiliar with Rust.
        let tokens = tokenize(r#""hello""#).unwrap();
        let mut p = Parser::new(tokens);
        let err = p.expect(&TokenKind::LParen).unwrap_err();
        let msg = err.to_string();
        assert!(!msg.contains("StringLit("), "message still leaks Debug syntax: {}", msg);
        assert!(msg.contains("chuỗi \"hello\""), "message must describe the token naturally: {}", msg);
    }

    #[test]
    fn test_check_ident() {
        let tokens = tokenize("state").unwrap();
        let p = Parser::new(tokens);
        let tokens2 = tokenize("layout_custom_name").unwrap();
        let p2 = Parser::new(tokens2);
        assert!(p2.check_ident("layout_custom_name"));
        let _ = p; 
    }
}

#[cfg(test)]
mod language_header_tests {
    use super::*;
    use crate::lexer::tokenize;

    #[test]
    fn missing_language_defaults_to_english() {
        let tokens = tokenize("ung_dung(\"x\") {}").unwrap();
        let (_, locale) = parse_with_locale(tokens).unwrap();
        assert_eq!(locale, Locale::English);
    }

    #[test]
    fn vietnamese_language_header_is_consumed() {
        let tokens = tokenize("lang = \"vi\";\nung_dung(\"x\") {}").unwrap();
        let (_, locale) = parse_with_locale(tokens).unwrap();
        assert_eq!(locale, Locale::Vietnamese);
    }

    #[test]
    fn english_language_header_is_optional_but_valid() {
        let tokens = tokenize("lang = \"en\";\nung_dung(\"x\") {}").unwrap();
        let (_, locale) = parse_with_locale(tokens).unwrap();
        assert_eq!(locale, Locale::English);
    }

    #[test]
    fn unsupported_language_is_rejected() {
        let tokens = tokenize("lang = \"ja\";\nung_dung(\"x\") {}").unwrap();
        let error = parse_with_locale(tokens).unwrap_err();
        assert!(error.message.contains("Unsupported source language"));
    }
}
