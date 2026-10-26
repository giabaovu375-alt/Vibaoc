use super::error::LexError;
use super::helpers::{is_ident_char, unescape};
use super::token::{Token, TokenKind};
use super::Lexer;

impl Lexer {
    pub(crate) fn prev_token_is_value(&self) -> bool {
        match self.tokens.last() {
            None => false,
            Some(t) => matches!(
                t.kind,
                TokenKind::NumberLit(_, _)
                    | TokenKind::Variable(_)
                    | TokenKind::Identifier(_)
                    | TokenKind::Component(_)
                    | TokenKind::ColorName(_)
                    | TokenKind::ColorHex(_)
                    | TokenKind::BoolLit(_)
                    | TokenKind::StringLit(_)
                    | TokenKind::RParen
                    | TokenKind::RBracket
            ),
        }
    }

    pub(crate) fn try_two_char_op(&mut self) -> Option<Token> {
        let a = self.peek(0);
        let b = self.peek(1);
        let kind = match (a, b) {
            ('=', '=') => Some(TokenKind::EqEq),
            ('!', '=') => Some(TokenKind::Neq),
            ('>', '=') => Some(TokenKind::Gte),
            ('<', '=') => Some(TokenKind::Lte),
            ('&', '&') => Some(TokenKind::AndAnd),
            ('|', '|') => Some(TokenKind::OrOr),
            _ => None,
        };
        kind.map(|k| {
            let (line, col) = (self.line, self.column);
            self.advance();
            self.advance();
            Token::new(k, line, col)
        })
    }

    pub(crate) fn skip_line_comment(&mut self) {
        while !self.is_eof() && self.peek(0) != '\n' {
            self.advance();
        }
    }

    pub(crate) fn skip_block_comment(&mut self) -> Result<(), LexError> {
        let (start_line, start_col) = (self.line, self.column);
        self.advance();
        self.advance();
        loop {
            if self.is_eof() {
                return Err(LexError {
                    message: "Unterminated block comment (missing */)".to_string(),
                    line: start_line,
                    column: start_col,
                });
            }
            if self.peek(0) == '*' && self.peek(1) == '/' {
                self.advance();
                self.advance();
                return Ok(());
            }
            self.advance();
        }
    }

    pub(crate) fn read_string(&mut self) -> Result<Token, LexError> {
        let (line, col) = (self.line, self.column);
        self.advance();
        let mut value = String::new();

        loop {
            if self.is_eof() {
                return Err(LexError {
                    message: "Unterminated string literal (missing closing \")".to_string(),
                    line,
                    column: col,
                });
            }
            let ch = self.peek(0);
            if ch == '"' {
                self.advance();
                break;
            }
            if ch == '\\' {
                self.advance();
                let escaped = self.peek(0);
                self.advance();
                value.push(unescape(escaped));
                continue;
            }
            value.push(ch);
            self.advance();
        }

        Ok(Token::new(TokenKind::StringLit(value), line, col))
    }

    pub(crate) fn read_multi_word_identifier(&mut self) -> Result<Token, LexError> {
        let (line, col) = (self.line, self.column);
        self.advance();
        let mut raw = String::new();

        loop {
            if self.is_eof() {
                return Err(LexError {
                    message: "Unterminated quoted identifier (missing closing ')".to_string(),
                    line,
                    column: col,
                });
            }
            let ch = self.peek(0);
            if ch == '\'' {
                self.advance();
                break;
            }
            raw.push(ch);
            self.advance();
        }

        if raw.trim().is_empty() {
            return Err(LexError {
                message: "Quoted identifier cannot be empty (for example, 'text color')"
                    .to_string(),
                line,
                column: col,
            });
        }

        let normalized = crate::lexer::vietnamese::normalize_vietnamese(raw.trim());
        Ok(self.classify_identifier(normalized.clone(), normalized, line, col))
    }

    pub(crate) fn read_hex_color(&mut self) -> Result<Token, LexError> {
        let (line, col) = (self.line, self.column);
        self.advance();
        let mut hex = String::from("#");
        while !self.is_eof() && self.peek(0).is_ascii_hexdigit() {
            hex.push(self.peek(0));
            self.advance();
        }
        if ![4, 5, 7, 9].contains(&hex.len()) {
            return Err(LexError {
                message: format!("Invalid hex color: {} — expected #RGB or #RRGGBB", hex),
                line,
                column: col,
            });
        }
        Ok(Token::new(TokenKind::ColorHex(hex), line, col))
    }

    pub(crate) fn read_variable(&mut self) -> Token {
        let (line, col) = (self.line, self.column);
        self.advance();
        let mut name = String::new();
        while !self.is_eof() && is_ident_char(self.peek(0)) {
            name.push(self.peek(0));
            self.advance();
        }
        let name = crate::lexer::vietnamese::strip_diacritics_keep_case(&name);
        Token::new(TokenKind::Variable(name), line, col)
    }

    pub(crate) fn read_number(&mut self) -> Token {
        let (line, col) = (self.line, self.column);
        let mut raw = String::new();

        if self.peek(0) == '-' {
            raw.push('-');
            self.advance();
        }
        while !self.is_eof() && self.peek(0).is_ascii_digit() {
            raw.push(self.peek(0));
            self.advance();
        }
        if self.peek(0) == '.' && self.peek(1).is_ascii_digit() {
            raw.push('.');
            self.advance();
            while !self.is_eof() && self.peek(0).is_ascii_digit() {
                raw.push(self.peek(0));
                self.advance();
            }
        }

        for unit in ["px", "vw", "vh", "em", "rem", "%"] {
            if self.match_str_at_cursor(unit) {
                raw.push_str(unit);
                for _ in 0..unit.chars().count() {
                    self.advance();
                }
                break;
            }
        }

        let numeric_part: String = raw
            .chars()
            .take_while(|c| c.is_ascii_digit() || *c == '.' || *c == '-')
            .collect();
        let value: f64 = numeric_part.parse().unwrap_or(0.0);

        Token::new(TokenKind::NumberLit(value, raw), line, col)
    }

    pub(crate) fn match_str_at_cursor(&self, s: &str) -> bool {
        let chars: Vec<char> = s.chars().collect();
        for (i, c) in chars.iter().enumerate() {
            if self.peek(i as isize) != *c {
                return false;
            }
        }
        let after = self.peek(chars.len() as isize);
        !is_ident_char(after)
    }

    pub(crate) fn read_identifier(&mut self) -> Token {
        let (line, col) = (self.line, self.column);
        let mut name = String::new();
        while !self.is_eof() && is_ident_char(self.peek(0)) {
            name.push(self.peek(0));
            self.advance();
        }

        let raw = crate::lexer::vietnamese::strip_diacritics_keep_case(&name);
        let lookup = crate::lexer::vietnamese::normalize_vietnamese(&name);

        self.classify_identifier(raw, lookup, line, col)
    }

    pub(crate) fn classify_identifier(
        &mut self,
        raw: String,
        lookup: String,
        line: usize,
        col: usize,
    ) -> Token {
        if lookup == "true" {
            return Token::new(TokenKind::BoolLit(true), line, col);
        }
        if lookup == "false" {
            return Token::new(TokenKind::BoolLit(false), line, col);
        }

        let is_prop_value_position =
            matches!(self.tokens.last().map(|t| &t.kind), Some(TokenKind::Colon));

        if is_prop_value_position && self.colors.contains_key(lookup.as_str()) {
            return Token::new(TokenKind::ColorName(raw), line, col);
        }
        if let Some(kw) = self.keywords.get(lookup.as_str()) {
            return Token::new(kw.clone(), line, col);
        }
        if self.components.iter().any(|c| *c == lookup.as_str()) {
            return Token::new(TokenKind::Component(raw), line, col);
        }
        if self.colors.contains_key(lookup.as_str()) {
            return Token::new(TokenKind::ColorName(raw), line, col);
        }

        let name = raw;

        Token::new(TokenKind::Identifier(name), line, col)
    }

    pub(crate) fn advance(&mut self) -> char {
        let ch = self.chars[self.pos];
        self.pos += 1;
        if ch == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        ch
    }

    pub(crate) fn peek(&self, offset: isize) -> char {
        let idx = self.pos as isize + offset;
        if idx < 0 || idx as usize >= self.chars.len() {
            '\0'
        } else {
            self.chars[idx as usize]
        }
    }

    pub(crate) fn is_eof(&self) -> bool {
        self.pos >= self.chars.len()
    }

    pub(crate) fn skip_whitespace(&mut self) {
        while !self.is_eof() {
            let ch = self.peek(0);
            if ch == ' ' || ch == '\t' || ch == '\r' || ch == '\n' {
                self.advance();
            } else {
                break;
            }
        }
    }
}
