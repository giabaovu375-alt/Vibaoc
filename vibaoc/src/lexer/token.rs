use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Trang,
    UngDung,
    Theme,
    State,
    Nhap,
    Tu,

    Neu,
    KhongThi,
    NeuNhieu,
    TruongHop,
    MacDinh,
    VongLap,

    OnClick,
    OnHover,
    OnBlur,
    OnFocus,
    OnChange,
    OnSubmit,
    OnScroll,
    OnTai,
    OnHuy,

    Component(String),

    StringLit(String),
    NumberLit(f64, String),
    BoolLit(bool),
    ColorHex(String),
    ColorName(String),

Identifier(String),
    Variable(String),

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
    Bang,
    Percent,
    Equals,
    Colon,
    Comma,
    Dot,
    Semicolon,
    Arrow,

    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,

    At,
    Eof,
}

impl fmt::Display for TokenKind {
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
        let kind = TokenKind::Identifier("x".to_string());
        let r: &TokenKind = &kind;
        assert_eq!(format!("{}", r), "định danh 'x'");
    }
}
