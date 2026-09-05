// ============================================================
// VIBAO COMPILER (Rust) — lexer/mod.rs
// Token types + Lexer: reads ViBao source code -> Vec<Token>
//
// This file was split from a single lexer.rs into several submodules
// grouped by responsibility, for readability/maintainability:
//   token.rs    — TokenKind + Token
//   tables.rs   — keyword / component / color tables
//   error.rs    — LexError
//   scan.rs     — the Lexer's detailed scan methods (an extra impl block)
//   helpers.rs  — module-level functions that don't need &self
//   tests.rs    — unit tests (only built under cfg(test))
//
// The locale <-> Tag (semantic identity) mapping table does NOT live
// here — see crate::locale (a sibling of lexer/parser/codegen in
// vibaoc/src/, not nested inside lexer — a locale is not "part of the
// lexer", it's an independent axis).
//
// REALLY WIRED UP (previously "will be parameterized later" — now
// done): keyword_map()/component_set() (tables.rs) NOW build from
// crate::locale::{vi,en}::{keyword_name_*,tag_name_*} instead of
// maintaining a separate hand-written table that could drift — the real
// lexer checks BOTH locale::vi AND locale::en AT THE SAME TIME for
// every keyword/tag, per the "ARCHITECTURAL DECISION... multi-locale
// model" (see ARCHITECTURE_PROPOSAL.md). A dev can type "box" or
// "khoi", "page" or "trang" — both work right at the lexer stage, not
// only at the resolve_tag() step after the lexer like before.
// ============================================================

mod error;
mod helpers;
mod scan;
mod tables;
mod token;
mod vietnamese;
mod vocabulary;

#[cfg(test)]
mod tests;

use std::collections::HashMap;

pub use error::LexError;
pub(crate) use tables::{color_map, resolve_color_name, color_func_name, resolve_color_func_name, is_known_action_or_function_name};
pub use token::{Token, TokenKind};

use helpers::{is_ident_start, single_char_bracket, single_char_operator};
use tables::{component_set, keyword_map};

// ════════════════════════════════════════════════════════════
// 4. LEXER
// ════════════════════════════════════════════════════════════

pub struct Lexer {
    /// The source is stored as a Vec<char>, not a raw &str/String —
    /// ViBao contains Vietnamese characters (multi-byte in UTF-8), so
    /// indexing a String directly by byte offset could split a
    /// multi-byte character and panic. Vec<char> gives O(1) random
    /// access matching the "characters" a user actually sees, mirroring
    /// how the old JS/TS version used src[i] (a JS string index is
    /// UTF-16 code units, which is similarly safe for most Vietnamese
    /// characters, so behavior stays consistent).
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

            // Comment
            if ch == '/' && self.peek(1) == '/' {
                self.skip_line_comment();
                continue;
            }
            if ch == '/' && self.peek(1) == '*' {
                self.skip_block_comment()?;
                continue;
            }

            // String
            if ch == '"' {
                let tok = self.read_string()?;
                self.tokens.push(tok);
                continue;
            }

            // A multi-word identifier with spaces, wrapped in single
            // quotes: 'mau chu' -> equivalent to the identifier
            // "mau_chu" (after normalize_vietnamese handles the spaces
            // + strips diacritics). Different from a "..." string
            // (StringLit, keeps its display content unchanged for the
            // user) — single quotes always produce an IDENTIFIER TOKEN,
            // going through the exact same normalization + keyword/
            // color/component lookup logic as an identifier typed
            // without spaces.
            //
            // CHANGED FEATURE (not a bug fix): this used to use
            // backticks for this — switched to single
            // quotes because on most mobile virtual keyboards
            // (iOS/Android), the backtick is buried in a secondary
            // symbol layer (you have to switch "123"->"#+=" to find
            // it), while a single quote sits RIGHT on the first symbol
            // layer — much easier to type when writing ViBao code on a
            // phone.
            if ch == '\'' {
                let tok = self.read_multi_word_identifier()?;
                self.tokens.push(tok);
                continue;
            }

            // Hex color
            if ch == '#' {
                let tok = self.read_hex_color()?;
                self.tokens.push(tok);
                continue;
            }

            // Variable $ten
            if ch == '$' {
                let tok = self.read_variable();
                self.tokens.push(tok);
                continue;
            }

            // Arrow -> (the unicode arrow character)
            if ch == '→' {
                let (line, col) = (self.line, self.column);
                self.advance();
                self.tokens.push(Token::new(TokenKind::Arrow, line, col));
                continue;
            }
            // Arrow ASCII ->
            if ch == '-' && self.peek(1) == '>' {
                let (line, col) = (self.line, self.column);
                self.advance();
                self.advance();
                self.tokens.push(Token::new(TokenKind::Arrow, line, col));
                continue;
            }

            // 2-character operators — MUST be checked before single-char
            // operators to avoid accidentally splitting them into 2
            // separate tokens (e.g. "==" splitting into Equals + Equals).
            if let Some(tok) = self.try_two_char_op() {
                self.tokens.push(tok);
                continue;
            }

            // A standalone "-": subtraction operator, or a number's
            // negative sign? Decided by the preceding token — if the
            // previous token was already a complete value (number/
            // variable/identifier/closing bracket/string), "-" here is
            // always subtraction, regardless of whether a digit follows
            // it directly or not (e.g. both "$n-1" and "$n - 1" are
            // subtraction). This was a bug encountered and fixed twice
            // in the old TS/JS versions (the mini-compiler and the full
            // TS compiler) — gotten right from the start in the Rust
            // port.
            if ch == '-' {
                let prev_is_value = self.prev_token_is_value();
                if prev_is_value {
                    let (line, col) = (self.line, self.column);
                    self.advance();
                    self.tokens.push(Token::new(TokenKind::Minus, line, col));
                    continue;
                }
                // Not preceded by an operand position -> could be a
                // negative sign; falls through to the Number branch
                // below.
            }

            // Number (including a negative sign attached to a digit, and
            // a unit suffix like px/%/...)
            if ch.is_ascii_digit() || (ch == '-' && self.peek(1).is_ascii_digit()) {
                let tok = self.read_number();
                self.tokens.push(tok);
                continue;
            }

            // The remaining single-character operators: + * > <
            if let Some(kind) = single_char_operator(ch) {
                let (line, col) = (self.line, self.column);
                self.advance();
                self.tokens.push(Token::new(kind, line, col));
                continue;
            }

            // A standalone '!' (logical negation): "!=" was already
            // caught by try_two_char_op() above, so if execution reaches
            // here it must be a lone '!'.
            if ch == '!' {
                let (line, col) = (self.line, self.column);
                self.advance();
                self.tokens.push(Token::new(TokenKind::Bang, line, col));
                continue;
            }

            // A standalone '%' (modulo operator): only reaches this
            // branch when it does NOT directly follow a digit, since
            // that case was already consumed by read_number() as a CSS
            // unit suffix (see read_number; the Number branch above
            // calls read_number when ch.is_ascii_digit()). Examples:
            //   "50%"                -> NumberLit(50, "50%")   — CSS unit
            //   "$n % 2" or "$n%2"   -> Variable, Percent, NumberLit — modulo
            if ch == '%' {
                let (line, col) = (self.line, self.column);
                self.advance();
                self.tokens.push(Token::new(TokenKind::Percent, line, col));
                continue;
            }

            // Identifier / keyword / component / color
            if is_ident_start(ch) {
                let tok = self.read_identifier();
                self.tokens.push(tok);
                continue;
            }

            // Brackets & remaining single characters
            if let Some(kind) = single_char_bracket(ch) {
                let (line, col) = (self.line, self.column);
                self.advance();
                self.tokens.push(Token::new(kind, line, col));
                continue;
            }

            // An unrecognized character — raise a clear error instead of
            // silently skipping it (this exact "silently skip" pattern
            // was the source of some of the hardest-to-debug bugs in the
            // old TS/JS versions — a shifted token stream with no trace
            // to follow).
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

// ════════════════════════════════════════════════════════════
// 6. PUBLIC ENTRY POINT
// ════════════════════════════════════════════════════════════

/// Tokenizes a ViBao source string. Used by main.rs and parser.rs.
pub fn tokenize(source: &str) -> Result<Vec<Token>, LexError> {
    Lexer::new(source).tokenize()
}
