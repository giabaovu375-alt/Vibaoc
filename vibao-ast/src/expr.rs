use serde::{Deserialize, Serialize};

use super::Pos;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Expr {
    Literal(LiteralValue, Pos),
    Variable(String, Pos),
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
        amount: f64,
        pos: Pos,
    },
    Array(Vec<Expr>, Pos),
    Object(Vec<(String, Expr)>, Pos),
    TemplateString(Vec<TemplatePart>, Pos),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LiteralValue {
    Str(String),
    Num(f64, Option<String>),
    Bool(bool),
    Color(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TemplatePart {
    Text(String),
    Variable(String),
    Member(Vec<String>),
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

impl Expr {
    pub fn literal_str(value: impl Into<String>, pos: Pos) -> Self {
        Expr::Literal(LiteralValue::Str(value.into()), pos)
    }

    pub fn literal_num(value: f64, pos: Pos) -> Self {
        Expr::Literal(LiteralValue::Num(value, None), pos)
    }

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
