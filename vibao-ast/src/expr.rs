// ============================================================
// VIBAO COMPILER (Rust) — ast/expr.rs
// EXPRESSIONS: Expr, LiteralValue, operators, and helper functions for
// quickly constructing nodes (used by the parser)
//
// Every type here derives Serialize/Deserialize (serde) — WHY: this
// crate is now SHARED between `vibaoc` (compiler, generates code) and
// `vibao-runtime` (WASM, runs in the browser). Codegen serializes an
// Expr to JSON at build time and embeds it in the JS output; when the
// page loads, WASM deserializes that exact JSON back into a real Expr
// so the Rust evaluator can run it directly — no JS string eval needed.
// This is the only bridge between the two crates (they never call each
// other's functions, only exchange JSON data).
// ============================================================

use serde::{Deserialize, Serialize};

use super::Pos;

// ════════════════════════════════════════════════════════════
// 11. EXPRESSIONS
// ════════════════════════════════════════════════════════════

/// Expr wraps recursive variants (Binary/Unary/MemberAccess) in
/// Box<Expr> for the same reason as Child above — an enum's size must
/// be fixed at compile time, and direct (non-pointer) recursion would
/// make it infinitely sized, so Rust requires boxing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Expr {
    Literal(LiteralValue, Pos),
    Variable(String, Pos), // does not include the leading $
    MemberAccess {
        object: Box<Expr>,
        property: String,
        pos: Pos,
    },
    Binary {
        op: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
        pos: Pos,
    },
    Unary {
        op: UnaryOp,
        operand: Box<Expr>,
        pos: Pos,
    },
    Call {
        callee: String,
        args: Vec<Expr>,
        pos: Pos,
    },
    ColorFunc {
        func: ColorFuncKind,
        color: Box<Expr>,
        amount: f64, // 0-100
        pos: Pos,
    },
    Array(Vec<Expr>, Pos),
    Object(Vec<(String, Expr)>, Pos),
    /// A string with variable interpolation: "Xin chao $ten" — split
    /// into its constituent parts.
    TemplateString(Vec<TemplatePart>, Pos),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LiteralValue {
    Str(String),
    /// (numeric value, original CSS unit if any — e.g. "50%" -> Num(50.0, Some("%".into())),
    /// "16" -> Num(16.0, None)). The unit is preserved from the lexer
    /// (NumberLit already carries the raw string) so codegen can correctly
    /// distinguish "50%" from "50" (which defaults to px) — this used to
    /// be a bug in the early Rust port, when the parser discarded the raw
    /// unit (_raw).
    Num(f64, Option<String>),
    Bool(bool),
    Color(String), // already resolved to hex, or kept as a CSS variable name
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TemplatePart {
    Text(String),
    Variable(String),
    Member(Vec<String>), // path: $obj.field.sub -> ["obj","field","sub"]
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    Neq,
    Gt,
    Gte,
    Lt,
    Lte,
    And,
    Or,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum UnaryOp {
    Not,
    Neg,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ColorFuncKind {
    TrongSuot,
    LamSang,
    LamToi,
}

// ════════════════════════════════════════════════════════════
// 15. HELPER CONSTRUCTORS — convenience functions for quickly building
// nodes (used by the parser)
// ════════════════════════════════════════════════════════════

impl Expr {
    pub fn literal_str(value: impl Into<String>, pos: Pos) -> Self {
        Expr::Literal(LiteralValue::Str(value.into()), pos)
    }

    /// Creates a numeric literal with no CSS unit (e.g. inside a plain
    /// arithmetic expression: $n - 1). Use literal_num_with_unit() when a
    /// unit needs to be preserved.
    pub fn literal_num(value: f64, pos: Pos) -> Self {
        Expr::Literal(LiteralValue::Num(value, None), pos)
    }

    /// Creates a numeric literal with its original CSS unit (e.g. "50%",
    /// "10px") — used by the parser when a NumberLit token's raw text
    /// carries a unit suffix.
    pub fn literal_num_with_unit(value: f64, unit: Option<String>, pos: Pos) -> Self {
        Expr::Literal(LiteralValue::Num(value, unit), pos)
    }

    pub fn literal_bool(value: bool, pos: Pos) -> Self {
        Expr::Literal(LiteralValue::Bool(value), pos)
    }

    pub fn pos(&self) -> Pos {
        match self {
            Expr::Literal(_, p) => *p,
            Expr::Variable(_, p) => *p,
            Expr::MemberAccess { pos, .. } => *pos,
            Expr::Binary { pos, .. } => *pos,
            Expr::Unary { pos, .. } => *pos,
            Expr::Call { pos, .. } => *pos,
            Expr::ColorFunc { pos, .. } => *pos,
            Expr::Array(_, p) => *p,
            Expr::Object(_, p) => *p,
            Expr::TemplateString(_, p) => *p,
        }
    }
}
