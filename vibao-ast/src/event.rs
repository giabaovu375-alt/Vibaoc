// ============================================================
// VIBAO COMPILER (Rust) — ast/event.rs
// EVENTS & ACTIONS (inside an event handler)
//
// Action derives Serialize/Deserialize just like Expr — WHY: following
// the same "pure Rust, no JS eval" architecture chosen for expressions,
// actions (thong_bao/goi_api/gan_bien...) triggered by e.g. a button
// click also need to be EXECUTED by Rust/WASM, rather than generating
// JS and eval-ing it. Codegen serializes Vec<Action> to JSON through an
// "action registry" (mirroring the expr registry); the runtime
// deserializes it back and dispatches it itself (see vibao-runtime::action).
// ============================================================

use serde::{Deserialize, Serialize};

use super::child::PropsMap;
use super::expr::Expr;
use super::Pos;

// ════════════════════════════════════════════════════════════
// 9. EVENTS
// ════════════════════════════════════════════════════════════

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

// ════════════════════════════════════════════════════════════
// 10. ACTIONS (inside an event handler)
// ════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Action {
    FunctionCall {
        name: String,
        args: Vec<Expr>,
        opts: PropsMap, // named options, e.g. thong_bao(msg, kieu:thanh_cong)
        assign_to: Option<String>,
        pos: Pos,
    },
    Assign {
        target: String, // variable name, without the leading $
        value: Expr,
        pos: Pos,
    },
    ApiCall {
        method: String,
        endpoint: Expr,
        data: Option<Expr>,
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
