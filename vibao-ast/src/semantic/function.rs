// ============================================================
// VIBAO — vibao-ast/src/semantic/function.rs
// FunctionName — SEMANTIC IDENTITY for ViBao's builtin PURE EXPRESSION
// functions (used inside `Expr::Call` — e.g. `$gia = gia_tien(1000)`).
//
// Differs from ActionName in one important way: these are PURE
// functions (return a value, NO side effects, don't need
// `SharedState`) — handled by
// `vibao-runtime/src/runtime/expr_eval.rs::eval_call()`, COMPLETELY
// SEPARATE from `action.rs::dispatch_function_call()` (different file
// and different mechanism: `eval_call` is synchronous and receives
// already-evaluated `&[VbValue]`; `dispatch_function_call` is
// asynchronous (`async fn`) and has side effects through
// `SharedState`).
//
// SCOPE FOR THIS ROUND (discussed with the user — decided to SPLIT INTO
// 3 LAYERS to avoid "scope creep": turning "wire FunctionName into the
// semantic layer" into an unplanned sub-project of "build an expression
// semantic validator"):
//   Layer 1 (DONE — this file + the locale layer): vocabulary + locale
//     mapping, exactly the same level of work done for ActionName
//     BEFORE it was wired into the validator.
//   Layer 2 (DONE — see AUDIT.md "UPDATE — FunctionName Layer 1+2"):
//     a full inventory of every place `Expr` appears in the AST — NOT a
//     traversal implementation, just a recorded list so the next round
//     doesn't have to re-survey the AST from scratch.
//   Layer 3 (DONE): a real recursive `check_expr()` in validator.rs now
//     checks Expr::Call everywhere an Expr is visited. The parser also
//     normalizes a resolved FunctionName to its canonical runtime name
//     so the evaluator doesn't depend on the locale surface.
//
// SOURCE OF TRUTH: matches 1:1 with `expr_eval.rs::eval_call()` — 6
// names, with NO warning logged for an unrecognized name (unlike
// Action's `dispatch_function_call` — it silently returns
// `VbValue::Null`, no logging at all).
// ============================================================

use serde::{Deserialize, Serialize};

/// The FULL list of 6 ViBao builtin expression functions — matches 1:1
/// with `expr_eval.rs::eval_call()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FunctionName {
    /// Source: "gia_tien" (price) — formats a number as VND currency
    /// (e.g. "1.000d").
    FormatPrice,
    /// Source: "ngay" (date) — formats an ISO date string as dd/mm/yyyy.
    FormatDate,
    /// Source: "rut_gon" (shorten) — truncates a string, appending "..."
    /// if it exceeds the given length.
    Truncate,
    /// Source: "hoa_chu" (uppercase) — uppercases an entire string.
    Uppercase,
    /// Source: "phan_tram" (percent) — formats a number as a percentage
    /// string.
    FormatPercent,
    /// Source: "lam_tron" (round) — rounds a number.
    Round,
}


impl FunctionName {
    /// The canonical name the current runtime evaluator uses.
    pub const fn runtime_name(self) -> &'static str {
        match self {
            Self::FormatPrice => "gia_tien",
            Self::FormatDate => "ngay",
            Self::Truncate => "rut_gon",
            Self::Uppercase => "hoa_chu",
            Self::FormatPercent => "phan_tram",
            Self::Round => "lam_tron",
        }
    }
}
