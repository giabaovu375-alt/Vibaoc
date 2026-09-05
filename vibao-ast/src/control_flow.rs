// ============================================================
// VIBAO COMPILER (Rust) — ast/control_flow.rs
// CONTROL FLOW: if / switch / loop
// ============================================================

use super::child::Child;
use super::expr::Expr;
use super::Pos;

// ════════════════════════════════════════════════════════════
// 8. CONTROL FLOW
// ════════════════════════════════════════════════════════════


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
        /// Counter variable name (without the leading "$") — e.g. "i" in
        /// "vong_lap $i tu 1 den 3", or "vong_lap tu 1 den 3" (no explicit
        /// name given, defaults to "i" — decided by the PARSER at parse
        /// time, not by a default value here).
        ///
        /// Bug already fixed: this field used to not exist — the parser
        /// correctly parsed a custom counter name (e.g. "$dem" in
        /// "vong_lap $dem tu 1 den 3") but IMMEDIATELY discarded it
        /// (`let _ = first_var;`), and codegen hardcoded "i" for EVERY
        /// Range loop regardless of what name the dev chose. Result:
        /// `vong_lap $dem tu 1 den 3 { text($dem) }` generated a loop that
        /// ran the correct NUMBER of times, but $dem ALWAYS resolved to
        /// empty/error (because the runtime actually set a variable named
        /// "i", not "dem") — no build error, only a runtime bug, hard to
        /// diagnose without reading codegen closely.
        var_name: String,
    },
}
