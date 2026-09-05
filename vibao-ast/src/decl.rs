// ============================================================
// VIBAO COMPILER (Rust) — ast/decl.rs
// VARIABLES & STATE & THEME & @THE (COMPONENT DEFINITION)
// ============================================================

use super::child::Child;
use super::expr::Expr;
use super::Pos;

// ════════════════════════════════════════════════════════════
// 4. VARIABLES & STATE & THEME
// ════════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
pub struct VarDecl {
    pub name: String, // does not include the leading $
    pub value: Expr,
    pub pos: Pos,
}

#[derive(Debug, Clone)]
pub struct StateDecl {
    pub name: String,
    pub value: Expr,
    pub pos: Pos,
}

#[derive(Debug, Clone)]
pub struct Theme {
    pub name: String,
    pub variables: Vec<VarDecl>,
    pub pos: Pos,
}

// ════════════════════════════════════════════════════════════
// 5. @THE — COMPONENT DEFINITION
// ════════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
pub struct ComponentDef {
    pub name: String,
    pub params: Vec<ParamDef>,
    pub children: Vec<Child>,
    pub pos: Pos,
}

#[derive(Debug, Clone)]
pub struct ParamDef {
    pub name: String,
    pub data_type: DataType,
    pub default_value: Option<Expr>,
    pub pos: Pos,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DataType {
    Chuoi,
    So,
    Mau,
    Bool,
    Mang,
    DoiTuong,
    HanhDong,
    Any,
}
