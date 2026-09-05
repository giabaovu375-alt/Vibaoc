// ============================================================
// VIBAO — vibao-ast/src/semantic/mod.rs
// Semantic identity layer — the ONE place that defines "meaning" for
// concepts with a finite vocabulary in ViBao (Tag, PropKey, ActionName,
// FunctionName).
//
// Placed in `vibao-ast` (not in `vibaoc` or `vibao-runtime` separately)
// because this is the ONLY crate both the compiler and the runtime
// depend on — meaning that if an enum defined here gains or loses a
// variant, Rust FORCES both sides to update (a non-exhaustive match is
// a compile error), rather than relying on a convention someone has to
// remember to keep in sync.
// ============================================================

pub mod action;
pub mod function;
pub mod prop;
pub mod registry;
pub mod tag;

pub use action::ActionName;
pub use function::FunctionName;
pub use prop::PropKey;
pub use registry::{prop_spec, tag_spec, PropSpec, TagKind, TagSpec};
pub use tag::Tag;
