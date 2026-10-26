mod error;
mod helpers;
mod scan;
mod tables;
mod token;
mod vietnamese;

#[cfg(test)]
mod tests;

use std::collections::HashMap;

pub use error::LexError;
pub(crate) use tables::{
    color_func_name, color_map, is_known_action_or_function_name, resolve_color_func_name,
    resolve_color_name,
};
pub use token::{Token, TokenKind};
pub(crate) use vietnamese::strip_diacritics_keep_case;

use helpers::{is_ident_start, single_char_bracket, single_char_operator};
use tables::{component_set, keyword_map};

pub struct Lexer {
    chars: Vec<char>,
    pos: usize,
    line: usize,
    column: usize,
    tokens: Vec<Token>,
    keywords: HashMap<&'static str, TokenKind>,
    components: Vec<&'static str>,
    colors: HashMap<&'static str, &'static str>,
}

impl Lexer {
    pub fn new(source: &str) -> Self {
        Lexer {
            chars: source.chars().collect(),
            pos: 0,
            line: 1,
            column: 1,
            tokens: Vec::new(),
            keywords: keyword_map(),
            components: component_set(),
            colors: color_map(),
        }
    }

    pub fn tokenize(mut self) -> Result<Vec<Token>, LexError> {
        while !self.is_eof() {
            self.skip_whitespace();
            if self.is_eof() {
                break;
            }

            let ch = self.peek(0);

            if ch == '/' && self.peek(1) == '/' {
                self.skip_line_comment();
                continue;
            }
            if ch == '/' && self.peek(1) == '*' {
                self.skip_block_comment()?;
                continue;
            }

            if ch == '"' {
                let tok = self.read_string()?;
                self.tokens.push(tok);
                continue;
            }

            if ch == '\'' {
                let tok = self.read_multi_word_identifier()?;
                self.tokens.push(tok);
                continue;
            }

            if ch == '#' {
                let tok = self.read_hex_color()?;
                self.tokens.push(tok);
                continue;
            }

            if ch == '$' {
                let tok = self.read_variable();
                self.tokens.push(tok);
                continue;
            }

            if ch == '→' {
                let (line, col) = (self.line, self.column);
                self.advance();
                self.tokens.push(Token::new(TokenKind::Arrow, line, col));
                continue;
            }
            if ch == '-' && self.peek(1) == '>' {
                let (line, col) = (self.line, self.column);
                self.advance();
                self.advance();
                self.tokens.push(Token::new(TokenKind::Arrow, line, col));
                continue;
            }

            if let Some(tok) = self.try_two_char_op() {
                self.tokens.push(tok);
                continue;
            }

            if ch == '-' {
                let prev_is_value = self.prev_token_is_value();
                if prev_is_value {
                    let (line, col) = (self.line, self.column);
                    self.advance();
                    self.tokens.push(Token::new(TokenKind::Minus, line, col));
                    continue;
                }
            }

            if ch.is_ascii_digit() || (ch == '-' && self.peek(1).is_ascii_digit()) {
                let tok = self.read_number();
                self.tokens.push(tok);
                continue;
            }

            if let Some(kind) = single_char_operator(ch) {
                let (line, col) = (self.line, self.column);
                self.advance();
                self.tokens.push(Token::new(kind, line, col));
                continue;
            }

            if ch == '!' {
                let (line, col) = (self.line, self.column);
                self.advance();
                self.tokens.push(Token::new(TokenKind::Bang, line, col));
                continue;
            }

            if ch == '%' {
                let (line, col) = (self.line, self.column);
                self.advance();
                self.tokens.push(Token::new(TokenKind::Percent, line, col));
                continue;
            }

            if is_ident_start(ch) {
                let tok = self.read_identifier();
                self.tokens.push(tok);
                continue;
            }

            if let Some(kind) = single_char_bracket(ch) {
                let (line, col) = (self.line, self.column);
                self.advance();
                self.tokens.push(Token::new(kind, line, col));
                continue;
            }

            return Err(LexError {
                message: format!("Unrecognized character: '{}'", ch),
                line: self.line,
                column: self.column,
            });
        }

        let (line, col) = (self.line, self.column);
        self.tokens.push(Token::new(TokenKind::Eof, line, col));
        Ok(self.tokens)
    }
}

pub fn tokenize(source: &str) -> Result<Vec<Token>, LexError> {
    Lexer::new(source).tokenize()
}
