// ============================================================
// VIBAO RUNTIME (Rust/WASM) — runtime/expr_registry.rs
// The RECEIVING side of the expr registry — the counterpart to the
// SENDING side in `vibaoc/src/codegen/expr.rs::take_expr_registry()`.
//
// At build time, codegen serializes every "dynamic" Expr to JSON,
// embedded into `__vb.boot({ exprRegistry: [...] })`. When the page
// loads, JS calls `__vb.boot(optsJson)` -> the function here
// deserializes that JSON into a real Vec<Expr>, stored in a
// thread_local so `evalExpr(id)` (called from JS whenever a binding
// needs to compute its value) can look up the correct Expr by index.
//
// Uses RefCell<Vec<Expr>> (not a Mutex) since WASM runs single-threaded
// within one browser tab - no multi-thread synchronization is needed.
// ============================================================

use std::cell::RefCell;

use vibao_ast::Expr;

thread_local! {
    static REGISTRY: RefCell<Vec<Expr>> = RefCell::new(Vec::new());
}

/// Loads the registry from JSON - called once during `__vb.boot(...)`.
/// If the JSON is malformed (shouldn't happen unless codegen/runtime
/// versions are out of sync), logs an error to the console and leaves
/// the registry empty instead of panicking - an error in the expr
/// registry shouldn't crash the whole app, only causing "dynamic"
/// bindings to return Null (safer than a blank-screen crash).
pub fn load_from_json(json: &str) {
    match serde_json::from_str::<Vec<Expr>>(json) {
        Ok(exprs) => {
            REGISTRY.with(|reg| {
                *reg.borrow_mut() = exprs;
            });
        }
        Err(err) => {
            crate::runtime::log::error(&format!(
                "[ViBao] Failed to parse exprRegistry JSON: {}",
                err
            ));
        }
    }
}

/// Looks up an Expr by id. Returns `None` if the id is invalid (out of
/// range) - the caller (evalExpr in dom.rs) should treat this as
/// VbValue::Null, not panic, since a bad id (due to a codegen bug or
/// the registry not yet loaded) shouldn't crash the whole page.
pub fn get(id: usize) -> Option<Expr> {
    REGISTRY.with(|reg| reg.borrow().get(id).cloned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use vibao_ast::Pos;

    #[test]
    fn test_load_and_get_roundtrip() {
        let e = Expr::literal_num(42.0, Pos { line: 1, column: 1 });
        let json = serde_json::to_string(&vec![e]).unwrap();
        load_from_json(&json);
        let fetched = get(0).expect("id 0 must exist after loading");
        match fetched {
            Expr::Literal(vibao_ast::LiteralValue::Num(n, _), _) => assert_eq!(n, 42.0),
            _ => panic!("wrong Expr type after roundtrip"),
        }
    }

    #[test]
    fn test_get_out_of_range_returns_none() {
        load_from_json("[]");
        assert!(get(999).is_none());
    }

    #[test]
    fn test_load_invalid_json_does_not_panic() {
        // Must not panic - only logs an error and leaves the registry as-is (empty).
        load_from_json("{ invalid json");
        assert!(get(0).is_none());
    }
}
