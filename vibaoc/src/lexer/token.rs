// ============================================================
// VIBAO COMPILER (Rust) — lexer/token.rs
// TokenKind + Token: defines the token kinds and the concrete token struct
// ============================================================

use std::fmt;

// ════════════════════════════════════════════════════════════
// 1. TOKEN TYPE
// ════════════════════════════════════════════════════════════

/// Every token kind in ViBao. Uses an enum instead of a string type like
/// the old TS version — Rust's match will catch a compile error if a
/// branch is ever missed (exhaustiveness checking is built into the
/// language; no need to hand-write it like in TS).
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Keywords - declarations
    Trang,
    UngDung,
    Theme,
    State,
    Nhap, // `nhap X tu "..."` (import X from "...")
    Tu,   // 'tu' (from) in 'nhap X tu "..."'

    // Keywords - control flow
    Neu,
    KhongThi,
    NeuNhieu,
    TruongHop,
    MacDinh,
    VongLap,

    // Keywords - events
    OnClick,
    OnHover,
    OnBlur,
    OnFocus,
    OnChange,
    OnSubmit,
    OnScroll,
    OnTai,
    OnHuy,

    // Component (text, box, flex, button, ...)
    Component(String),

    // Literals
    StringLit(String),
    NumberLit(f64, String), // (value, original string including unit if any — e.g. "50%")
    BoolLit(bool),
    ColorHex(String),
    ColorName(String),

Identifier(String),
    Variable(String), // $ten_bien (does not include the leading $)

    // Operators
    Plus,
    Minus,
    Star,
    Slash,
    Gt,
    Lt,
    Gte,
    Lte,
    EqEq,
    Neq,
    AndAnd,
    OrOr,
    Bang,    // ! (logical negation, its own token - distinct from Neq "!=")
    Percent, // % used as the modulo operator, distinct from the CSS unit suffix "50%"
    Equals,
    Colon,
    Comma,
    Dot,
    Semicolon,
    Arrow, // -> (also accepts the unicode arrow character)

    // Brackets
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,

    // Special
    At, // @ (standalone token, not @the - used for @hieu_ung, @di_dong...)
    Eof,
}

impl fmt::Display for TokenKind {
    /// Displays a token in a HUMAN-READABLE form, used in parse error
    /// messages - very different from `{:?}` (Debug), which prints the
    /// raw Rust enum syntax (e.g. `Identifier("mua_nen")`) and would
    /// look like an internal debugging error to someone writing ViBao
    /// code. Every error site in parser/*.rs should use `{}` (Display,
    /// this impl) instead of `{:?}`.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TokenKind::Trang => write!(f, "từ khóa 'trang'"),
            TokenKind::UngDung => write!(f, "từ khóa 'ung_dung'"),
            TokenKind::Theme => write!(f, "từ khóa 'theme'"),
            TokenKind::State => write!(f, "từ khóa 'state'"),
            TokenKind::Nhap => write!(f, "từ khóa 'nhap'"),
            TokenKind::Tu => write!(f, "từ khóa 'tu'"),

            TokenKind::Neu => write!(f, "từ khóa 'neu'"),
            TokenKind::KhongThi => write!(f, "từ khóa 'khong_thi'"),
            TokenKind::NeuNhieu => write!(f, "từ khóa 'neu_nhieu'"),
            TokenKind::TruongHop => write!(f, "từ khóa 'truong_hop'"),
            TokenKind::MacDinh => write!(f, "từ khóa 'mac_dinh'"),
            TokenKind::VongLap => write!(f, "từ khóa 'vong_lap'"),

            TokenKind::OnClick => write!(f, "từ khóa 'on_click'"),
            TokenKind::OnHover => write!(f, "từ khóa 'on_hover'"),
            TokenKind::OnBlur => write!(f, "từ khóa 'on_blur'"),
            TokenKind::OnFocus => write!(f, "từ khóa 'on_focus'"),
            TokenKind::OnChange => write!(f, "từ khóa 'on_change'"),
            TokenKind::OnSubmit => write!(f, "từ khóa 'on_submit'"),
            TokenKind::OnScroll => write!(f, "từ khóa 'on_scroll'"),
            TokenKind::OnTai => write!(f, "từ khóa 'on_tai'"),
            TokenKind::OnHuy => write!(f, "từ khóa 'on_huy'"),

            TokenKind::Component(name) => write!(f, "thành phần '{}'", name),

            TokenKind::StringLit(s) => write!(f, "chuỗi \"{}\"", s),
            TokenKind::NumberLit(_, raw) => write!(f, "số {}", raw),
            TokenKind::BoolLit(b) => write!(f, "giá trị luận lý {}", b),
            TokenKind::ColorHex(h) => write!(f, "mã màu {}", h),
            TokenKind::ColorName(n) => write!(f, "tên màu '{}'", n),

            TokenKind::Identifier(s) => write!(f, "định danh '{}'", s),
            TokenKind::Variable(s) => write!(f, "biến '${}'", s),

            TokenKind::Plus => write!(f, "'+'"),
            TokenKind::Minus => write!(f, "'-'"),
            TokenKind::Star => write!(f, "'*'"),
            TokenKind::Slash => write!(f, "'/'"),
            TokenKind::Gt => write!(f, "'>'"),
            TokenKind::Lt => write!(f, "'<'"),
            TokenKind::Gte => write!(f, "'>='"),
            TokenKind::Lte => write!(f, "'<='"),
            TokenKind::EqEq => write!(f, "'=='"),
            TokenKind::Neq => write!(f, "'!='"),
            TokenKind::AndAnd => write!(f, "'&&'"),
            TokenKind::OrOr => write!(f, "'||'"),
            TokenKind::Bang => write!(f, "'!'"),
            TokenKind::Percent => write!(f, "'%'"),
            TokenKind::Equals => write!(f, "'='"),
            TokenKind::Colon => write!(f, "':'"),
            TokenKind::Comma => write!(f, "','"),
            TokenKind::Dot => write!(f, "'.'"),
            TokenKind::Semicolon => write!(f, "';'"),
            TokenKind::Arrow => write!(f, "'->'"),

            TokenKind::LParen => write!(f, "'('"),
            TokenKind::RParen => write!(f, "')'"),
            TokenKind::LBrace => write!(f, "'{{'"),
            TokenKind::RBrace => write!(f, "'}}'"),
            TokenKind::LBracket => write!(f, "'['"),
            TokenKind::RBracket => write!(f, "']'"),

            TokenKind::At => write!(f, "'@'"),
            TokenKind::Eof => write!(f, "kết thúc file"),
        }
    }
}

/// A single concrete token in the source, with its position for
/// precise error reporting.
#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub line: usize,
    pub column: usize,
}

impl Token {
    pub(crate) fn new(kind: TokenKind, line: usize, column: usize) -> Self {
        Token { kind, line, column }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A dev-experience regression test: parse error messages must
    /// display a token in HUMAN-READABLE form (e.g. "định danh 'abc'"),
    /// not Rust's raw Debug syntax (e.g. 'Identifier("abc")') - if the
    /// Display impl ever accidentally gets simplified back to
    /// `write!(f, "{:?}", self)`, this test will fail immediately.
    #[test]
    fn test_display_identifier_is_human_readable() {
        let s = format!("{}", TokenKind::Identifier("abc".to_string()));
        assert_eq!(s, "định danh 'abc'");
        assert!(!s.contains("Identifier("), "Display must not leak the raw enum Debug syntax");
    }

    #[test]
    fn test_display_string_lit_is_human_readable() {
        let s = format!("{}", TokenKind::StringLit("Xin chào".to_string()));
        assert_eq!(s, "chuỗi \"Xin chào\"");
    }

    #[test]
    fn test_display_punctuation_uses_symbol_not_variant_name() {
        assert_eq!(format!("{}", TokenKind::RParen), "')'");
        assert_eq!(format!("{}", TokenKind::Colon), "':'");
        assert_eq!(format!("{}", TokenKind::LBrace), "'{'");
    }

    #[test]
    fn test_display_works_through_reference() {
        // Many places in the parser match on `&self.current().kind`, so
        // Display needs to work correctly through a reference (Rust
        // auto-derefs since &T: Display whenever T: Display).
        let kind = TokenKind::Identifier("x".to_string());
        let r: &TokenKind = &kind;
        assert_eq!(format!("{}", r), "định danh 'x'");
    }
}
