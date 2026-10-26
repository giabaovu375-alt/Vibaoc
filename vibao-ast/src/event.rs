use serde::{Deserialize, Serialize};

use super::child::PropsMap;
use super::expr::Expr;
use super::Pos;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventNode {
    pub name: EventName,
    pub body: Vec<Action>,
    pub pos: Pos,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EventName {
    OnClick,
    OnHover,
    OnBlur,
    OnFocus,
    OnChange,
    OnSubmit,
    OnScroll,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Action {
    FunctionCall {
        name: String,
        args: Vec<Expr>,
        opts: PropsMap,
        assign_to: Option<String>,
        pos: Pos,
    },
    Assign {
        target: String,
        value: Expr,
        pos: Pos,
    },
    ApiCall {
        method: String,
        endpoint: Expr,
        data: Option<Expr>,
        #[serde(default)]
        headers: Option<Expr>,
        #[serde(default)]
        background: bool,
        assign_to: Option<String>,
        on_success: Option<Vec<Action>>,
        on_failure: Option<Vec<Action>>,
        pos: Pos,
    },
    IfAction {
        condition: Expr,
        consequent: Vec<Action>,
        alternate: Option<Vec<Action>>,
        pos: Pos,
    },
}
