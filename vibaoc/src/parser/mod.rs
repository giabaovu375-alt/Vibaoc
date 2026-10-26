mod action;
mod app;
mod control;
mod element;
mod expr;

use crate::diagnostic::Locale;
use crate::lexer::{Token, TokenKind};
use std::fmt;
use vibao_ast::Pos;

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

pub struct Parser {
    pub(crate) tokens: Vec<Token>,
    pub(crate) pos: usize,
    pub(crate) locale: Locale,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser {
            tokens,
            pos: 0,
            locale: Locale::English,
        }
    }

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

    pub(crate) fn locale(&self) -> Locale {
        self.locale
    }

    pub(crate) fn parse_language_header(&mut self) -> Result<(), ParseError> {
        let is_lang = matches!(&self.current().kind, TokenKind::Identifier(name) if name == "lang");
        if !is_lang {
            return Ok(());
        }

        let lang_pos = self.current_pos();
        self.advance();
        self.consume(&TokenKind::Equals, "Expected '=' after 'lang'")?;
        let value = match self.advance().kind {
            TokenKind::StringLit(value) => value,
            other => {
                return Err(ParseError {
                    message: format!(
                        "Expected a language code string after 'lang =', received {}",
                        other
                    ),
                    line: lang_pos.line,
                    column: lang_pos.column,
                })
            }
        };
        self.consume(
            &TokenKind::Semicolon,
            "Expected ';' after the language declaration",
        )?;

        self.locale = Locale::from_code(&value).ok_or_else(|| ParseError {
            message: format!("Unsupported source language '{}'. English is always available; supported diagnostic languages: 'en', 'vi'", value),
            line: lang_pos.line,
            column: lang_pos.column,
        })?;
        Ok(())
    }
}

impl Parser {
    pub(crate) fn current(&self) -> &Token {
        self.tokens
            .get(self.pos)
            .unwrap_or_else(|| self.tokens.last().expect("empty token stream"))
    }

    pub(crate) fn peek(&self, offset: usize) -> &Token {
        self.tokens
            .get(self.pos + offset)
            .unwrap_or_else(|| self.tokens.last().expect("empty token stream"))
    }

    pub(crate) fn advance(&mut self) -> Token {
        let tok = self.current().clone();
        if !matches!(tok.kind, TokenKind::Eof) {
            self.pos += 1;
        }
        tok
    }

    pub(crate) fn check(&self, kind: &TokenKind) -> bool {
        std::mem::discriminant(&self.current().kind) == std::mem::discriminant(kind)
    }

    pub(crate) fn check_at(&self, offset: usize, kind: &TokenKind) -> bool {
        std::mem::discriminant(&self.peek(offset).kind) == std::mem::discriminant(kind)
    }

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

    pub(crate) fn check_ident(&self, name: &str) -> bool {
        matches!(&self.current().kind, TokenKind::Identifier(s) if s == name)
    }

    pub(crate) fn check_ident_or_color_name(&self, name: &str) -> bool {
        match &self.current().kind {
            TokenKind::Identifier(s) if s == name => true,
            TokenKind::ColorName(s) if s == name => true,
            _ => false,
        }
    }

    pub(crate) fn current_pos(&self) -> Pos {
        let t = self.current();
        Pos {
            line: t.line,
            column: t.column,
        }
    }

    pub(crate) fn error(&self, message: impl Into<String>) -> ParseError {
        let cur = self.current();
        ParseError {
            message: message.into(),
            line: cur.line,
            column: cur.column,
        }
    }

    pub(crate) fn skip_comma(&mut self) {
        if self.check(&TokenKind::Comma) {
            self.advance();
        }
    }

    pub(crate) fn is_at_end(&self) -> bool {
        matches!(self.current().kind, TokenKind::Eof)
    }

    pub(crate) fn match_token(&mut self, kind: &TokenKind) -> bool {
        if self.check(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

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

#[allow(dead_code)]
pub fn parse(tokens: Vec<Token>) -> Result<vibao_ast::Program, ParseError> {
    Parser::new(tokens).parse()
}

pub fn parse_with_locale(tokens: Vec<Token>) -> Result<(vibao_ast::Program, Locale), ParseError> {
    Parser::new(tokens).parse_with_locale()
}

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
        let tokens = tokenize(r#""hello""#).unwrap();
        let mut p = Parser::new(tokens);
        let err = p.expect(&TokenKind::LParen).unwrap_err();
        let msg = err.to_string();
        assert!(
            !msg.contains("StringLit("),
            "message still leaks Debug syntax: {}",
            msg
        );
        assert!(
            msg.contains("chuỗi \"hello\""),
            "message must describe the token naturally: {}",
            msg
        );
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
