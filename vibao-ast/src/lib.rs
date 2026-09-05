// ============================================================
// VIBAO — crate `vibao-ast`
// Defines the AST node types, SHARED between the two crates in the
// workspace:
//   - `vibaoc`         (compiler: lexer → parser → codegen)
//   - `vibao-runtime`  (WASM: runs in the browser, evaluates Expr
//                       directly in Rust instead of eval-ing JS strings)
//
// This used to live inside `vibaoc` alone (as `ast.rs`/`ast/mod.rs`).
// It was split into its own crate so `vibao-runtime` can
// `use vibao_ast::Expr` instead of redefining it (avoiding two Expr
// definitions drifting apart over time). Every type here derives
// Serialize/Deserialize (serde) because this crate IS the "data
// contract" between the two crates: codegen serializes an Expr to
// JSON at build time, and the runtime deserializes that exact JSON
// at run time.
//
// This file was split from a single ast.rs into several submodules
// grouped by concept, for readability/maintainability:
//   program.rs      — Program, App, Page, PageEvent
//   decl.rs         — VarDecl, StateDecl, Theme, ComponentDef, ParamDef, DataType
//   child.rs        — Child, Element, ComponentCall, PropsMap, get_prop
//   control_flow.rs — IfNode, SwitchNode, CaseNode, LoopNode, LoopKind
//   event.rs        — EventNode, EventName, Action
//   expr.rs         — Expr, LiteralValue, TemplatePart, operators, helper constructors
//   style.rs        — ColorValue, AnimationProps, LapValue, Breakpoint, ResponsiveNode
//   semantic/       — Tag, PropKey (+ ActionName/FunctionName):
//                      semantic identity FOR CONCEPTS WITH A FINITE
//                      VOCABULARY, kept separate from how they are
//                      SPELLED in any one locale (see semantic/mod.rs)
//   tests.rs        — unit tests (only built under cfg(test))
//
// Every type here is re-exported directly from `vibao_ast::`, so call
// sites like `ast::Page`, `ast::Expr`, `ast::get_prop(...)` in
// `vibaoc`'s parser/codegen only need `use crate::ast::` changed to
// `use vibao_ast::` — no other usage changes required.
// ============================================================

use serde::{Deserialize, Serialize};

pub mod child;
pub mod control_flow;
pub mod decl;
pub mod event;
pub mod expr;
pub mod program;
pub mod semantic;
pub mod style;

#[cfg(test)]
mod tests;

pub use child::{get_prop, Child, ComponentCall, Element, PropsMap};
pub use control_flow::{CaseNode, IfNode, LoopKind, LoopNode, SwitchNode};
pub use decl::{ComponentDef, DataType, ParamDef, StateDecl, Theme, VarDecl};
pub use event::{Action, EventName, EventNode};
pub use expr::{BinaryOp, ColorFuncKind, Expr, LiteralValue, TemplatePart, UnaryOp};
pub use program::{App, ImportDecl, Page, PageEvent, PageEventName, Program};
pub use semantic::{prop_spec, tag_spec, ActionName, FunctionName, PropKey, PropSpec, Tag, TagKind, TagSpec};
pub use style::{AnimationProps, Breakpoint, ColorValue, LapValue, ResponsiveNode};

// ════════════════════════════════════════════════════════════
// 1. SOURCE POSITION (used for error reporting)
// ════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Pos {
    pub line: usize,
    pub column: usize,
}
