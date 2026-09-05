// ============================================================
// VIBAO COMPILER (Rust) — lexer/helpers.rs
// HELPER FUNCTIONS (module-level, don't need &self)
// ============================================================

use super::token::TokenKind;

// ════════════════════════════════════════════════════════════
// 5. HELPER FUNCTIONS (module-level, don't need &self)
// ════════════════════════════════════════════════════════════

pub(crate) fn is_ident_start(ch: char) -> bool {
    ch.is_alphabetic() || ch == '_'
}

pub(crate) fn is_ident_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

pub(crate) fn single_char_operator(ch: char) -> Option<TokenKind> {
    match ch {
        '+' => Some(TokenKind::Plus),
        '*' => Some(TokenKind::Star),
        '>' => Some(TokenKind::Gt),
        '<' => Some(TokenKind::Lt),
        _ => None,
    }
}

pub(crate) fn single_char_bracket(ch: char) -> Option<TokenKind> {
    match ch {
        '(' => Some(TokenKind::LParen),
        ')' => Some(TokenKind::RParen),
        '{' => Some(TokenKind::LBrace),
        '}' => Some(TokenKind::RBrace),
        '[' => Some(TokenKind::LBracket),
        ']' => Some(TokenKind::RBracket),
        ':' => Some(TokenKind::Colon),
        ',' => Some(TokenKind::Comma),
        '.' => Some(TokenKind::Dot),
        ';' => Some(TokenKind::Semicolon),
        '=' => Some(TokenKind::Equals),
        '/' => Some(TokenKind::Slash),
        '@' => Some(TokenKind::At),
        _ => None,
    }
}

pub(crate) fn unescape(ch: char) -> char {
    match ch {
        'n' => '\n',
        't' => '\t',
        'r' => '\r',
        other => other, // \" \\ \' and any other character pass through unchanged
    }
}
