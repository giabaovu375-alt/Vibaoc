// ============================================================
// VIBAO RUNTIME (Rust/WASM) — lib.rs
// Crate root. Compiles to wasm32-unknown-unknown, replacing the old
// 17/18/19/20/21-runtime-*.ts with a pure Rust engine (except for DOM
// API calls, which must go through web-sys/wasm-bindgen since WASM has
// no direct DOM access).
// ============================================================

pub mod runtime;

pub use runtime::dom::VbRuntime;
pub use runtime::expr_eval::{eval, eval_tracked};
pub use runtime::state::{LoopFrame, SharedState, State, SubId};
pub use runtime::value::VbValue;
