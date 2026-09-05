// ============================================================
// VIBAO COMPILER (Rust) — lexer/scan.rs
// The Lexer's detailed scan methods: strings, numbers, identifiers,
// comments, 2-character operators, cursor movement...
// (an extra impl block for struct Lexer, declared in lexer/mod.rs)
// ============================================================

use super::error::LexError;
use super::helpers::{is_ident_char, unescape};
use super::token::{Token, TokenKind};
use super::Lexer;

impl Lexer {
    // ── Checks whether the previous token is a complete value ────────
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

    // ── 2-char operators ────────────────────────────────────────────
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

    // ── Comment ──────────────────────────────────────────────────────
    pub(crate) fn skip_line_comment(&mut self) {
        while !self.is_eof() && self.peek(0) != '\n' {
            self.advance();
        }
    }

    pub(crate) fn skip_block_comment(&mut self) -> Result<(), LexError> {
        let (start_line, start_col) = (self.line, self.column);
        self.advance(); // /
        self.advance(); // *
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

    // ── String "..." ─────────────────────────────────────────────────
    pub(crate) fn read_string(&mut self) -> Result<Token, LexError> {
        let (line, col) = (self.line, self.column);
        self.advance(); // skip the opening "
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

    /// Reads a multi-word identifier wrapped in single quotes:
    /// 'mau chu' — allows writing a prop/variable name with REAL SPACES
    /// between words, instead of requiring `_` like a normal identifier.
    /// The raw content (including spaces and Vietnamese diacritics) is
    /// collected exactly the way read_string() collects string content,
    /// THEN passed through the exact same classify_identifier() that
    /// read_identifier() uses — so 'mau chu' and 'mau_chu' (written
    /// without spaces) produce the SAME TokenKind, with no separate
    /// processing path that could drift out of sync.
    ///
    /// FUNCTION NAME (renamed from read_backtick_identifier):
    /// deliberately named NEUTRALLY, not tied to a specific character —
    /// the wrapping character has already changed once historically
    /// (backtick -> single quote, since a single quote is easier to
    /// type on a mobile virtual keyboard), so a neutral function name
    /// avoids needing another rename if something similar happens
    /// again.
    ///
    /// Does NOT support escaping (\') inside — a prop/variable name has
    /// no legitimate reason to contain a literal single-quote character,
    /// so this stays simple.
    pub(crate) fn read_multi_word_identifier(&mut self) -> Result<Token, LexError> {
        let (line, col) = (self.line, self.column);
        self.advance(); // skip the opening '
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
                message: "Quoted identifier cannot be empty (for example, 'text color')".to_string(),
                line,
                column: col,
            });
        }

        // A multi-word identifier ('mau chu'...) is always used for a
        // prop/variable name — never a dev-chosen PascalCase component
        // name that needs its casing preserved (unlike the
        // read_identifier() case above) — so `raw` passed into
        // classify_identifier() here is already the NORMALIZED form,
        // preserving the old behavior (writing 'Mau Chu' or 'mau chu'
        // both produce the same token).
        let normalized = crate::lexer::vietnamese::normalize_vietnamese(raw.trim());
        Ok(self.classify_identifier(normalized.clone(), normalized, line, col))
    }

    // ── Color #hex ──────────────────────────────────────────────────────
    pub(crate) fn read_hex_color(&mut self) -> Result<Token, LexError> {
        let (line, col) = (self.line, self.column);
        self.advance(); // #
        let mut hex = String::from("#");
        while !self.is_eof() && self.peek(0).is_ascii_hexdigit() {
            hex.push(self.peek(0));
            self.advance();
        }
        if ![4, 5, 7, 9].contains(&hex.len()) {
            return Err(LexError {
                message: format!(
                    "Invalid hex color: {} — expected #RGB or #RRGGBB",
                    hex
                ),
                line,
                column: col,
            });
        }
        Ok(Token::new(TokenKind::ColorHex(hex), line, col))
    }

    // ── Variable $ten_bien ────────────────────────────────────────────────
    pub(crate) fn read_variable(&mut self) -> Token {
        let (line, col) = (self.line, self.column);
        self.advance(); // $
        let mut name = String::new();
        while !self.is_eof() && is_ident_char(self.peek(0)) {
            name.push(self.peek(0));
            self.advance();
        }
        Token::new(TokenKind::Variable(name), line, col)
    }

    // ── Number (integer/decimal/CSS unit) ─────────────────────────────
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

        // A CSS unit directly following: px, %, vw, vh, em, rem
        for unit in ["px", "vw", "vh", "em", "rem", "%"] {
            if self.match_str_at_cursor(unit) {
                raw.push_str(unit);
                for _ in 0..unit.chars().count() {
                    self.advance();
                }
                break;
            }
        }

        // The plain numeric value (unit stripped) for later use in codegen.
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
        // Ensures this doesn't false-match inside a longer identifier,
        // e.g. "pxx" should not be read as the unit "px" plus a leftover
        // "x" that causes a downstream error.
        let after = self.peek(chars.len() as isize);
        !is_ident_char(after)
    }

    // ── Identifier / keyword / component / color ───────────────────────
    pub(crate) fn read_identifier(&mut self) -> Token {
        let (line, col) = (self.line, self.column);
        let mut name = String::new();
        while !self.is_eof() && is_ident_char(self.peek(0)) {
            name.push(self.peek(0));
            self.advance();
        }

        // Normalizes Vietnamese WITH diacritics into a diacritics-free
        // form RIGHT HERE, before any lookup table (keywords/colors/
        // components) is consulted — lets a dev write "mau" (with
        // diacritics) and get identical behavior to "mau" (without) with
        // NO changes needed to any of those tables. See
        // lexer/vietnamese.rs for details + the design rationale. This
        // only applies here (the identifier branch) — scan_string is a
        // completely separate path, so "Xin chao" inside
        // text("Xin chao") never goes through this function and keeps
        // its diacritics untouched.
        //
        // `raw`: diacritics stripped but casing PRESERVED — used as the
        // actual token value when the result is
        // Identifier/Component/ColorName, so a dev-chosen identifier
        // name (e.g. "@the TheBao(...)"/"@the Do(...)") never loses its
        // casing.
        // `lookup`: diacritics stripped AND lowercased — used ONLY to
        // check the keyword/color/component tables (never stored in the
        // token), ensuring consistent matching no matter how the dev
        // capitalized or accented what they typed.
        let raw = crate::lexer::vietnamese::strip_diacritics_keep_case(&name);
        let lookup = crate::lexer::vietnamese::normalize_vietnamese(&name);

        self.classify_identifier(raw, lookup, line, col)
    }

    /// Classifies an identifier into the correct token kind — `raw` is
    /// the ORIGINAL string (casing preserved, may still have diacritics
    /// if called from read_multi_word_identifier), `lookup` is the
    /// NORMALIZED form (lowercase, no diacritics, snake_case) used
    /// specifically for checking the keywords/colors/components tables.
    /// The returned token ALWAYS carries the `raw` value (except when it
    /// matches a keyword with its own TokenKind, where the inner data
    /// doesn't matter) — ensuring a dev-chosen name's casing is never
    /// lost, while table matching stays consistent no matter how the
    /// dev capitalized or accented what they typed.
    pub(crate) fn classify_identifier(&mut self, raw: String, lookup: String, line: usize, col: usize) -> Token {
        if lookup == "true" {
            return Token::new(TokenKind::BoolLit(true), line, col);
        }
        if lookup == "false" {
            return Token::new(TokenKind::BoolLit(false), line, col);
        }

        // "trang" (and, in principle, any other word with a name
        // collision) is both the "page" declaration keyword AND the
        // Vietnamese name for the color white — only treated as a color
        // name when the immediately preceding token is a ':' (i.e. in a
        // prop's value position, e.g. "color:trang"). This was a bug
        // found and fixed twice in the old TS/JS versions; gotten right
        // from the start here.
        let is_prop_value_position = matches!(
            self.tokens.last().map(|t| &t.kind),
            Some(TokenKind::Colon)
        );

        if is_prop_value_position && self.colors.contains_key(lookup.as_str()) {
            return Token::new(TokenKind::ColorName(raw), line, col);
        }
        if let Some(kw) = self.keywords.get(lookup.as_str()) {
            return Token::new(kw.clone(), line, col);
        }
        // .iter().any(|c| *c == name) instead of
        // .contains(&name.as_str()) — this is clearer and avoids
        // reasoning about &&str / &str / String coercion through several
        // layers of nested references (a common source of hard-to-read
        // type errors, especially without a compiler on hand to verify
        // it while writing).
        if self.components.iter().any(|c| *c == lookup.as_str()) {
            return Token::new(TokenKind::Component(raw), line, col);
        }
        if self.colors.contains_key(lookup.as_str()) {
            return Token::new(TokenKind::ColorName(raw), line, col);
        }

        let name = raw;

        Token::new(TokenKind::Identifier(name), line, col)
    }

    // ── Cursor movement ─────────────────────────────────────────────
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
