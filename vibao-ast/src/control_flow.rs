use super::child::Child;
use super::expr::Expr;
use super::Pos;

#[derive(Debug, Clone)]
pub struct IfNode {
    pub condition: Expr,
    pub consequent: Vec<Child>,
    pub alternate: Option<Vec<Child>>,
    pub pos: Pos,
}

#[derive(Debug, Clone)]
pub struct SwitchNode {
    pub subject: Expr,
    pub cases: Vec<CaseNode>,
    pub default_case: Option<Vec<Child>>,
    pub pos: Pos,
}

#[derive(Debug, Clone)]
pub struct CaseNode {
    pub value: Expr,
    pub body: Vec<Child>,
    pub pos: Pos,
}

#[derive(Debug, Clone)]
pub struct LoopNode {
    pub kind: LoopKind,
    pub body: Vec<Child>,
    pub pos: Pos,
}

#[derive(Debug, Clone)]
pub enum LoopKind {
    Each {
        iterable: Expr,
        item_var: String,
        index_var: Option<String>,
    },
    Range {
        from: i64,
        to: i64,
        var_name: String,
    },
}
