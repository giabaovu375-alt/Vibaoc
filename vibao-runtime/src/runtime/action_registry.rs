// ============================================================
// VIBAO RUNTIME (Rust/WASM) — runtime/action_registry.rs
// A counterpart to expr_registry.rs, but for `Action` instead of `Expr`.
//
// Codegen serializes every Vec<Action> (each event handler's body, e.g.
// on_click/on_change/...) to JSON, embedded into __vb.boot(). At
// runtime, dom.rs (bind_events) only embeds an "action id" into the
// data-vb-on-click="<actionId>" attribute - when a button is clicked,
// the registry is looked up by that id, retrieving the real
// Vec<Action>, handed to action::dispatch() to execute.
// ============================================================

use std::cell::RefCell;

use vibao_ast::Action;

thread_local! {
    static REGISTRY: RefCell<Vec<Vec<Action>>> = RefCell::new(Vec::new());
}

pub fn load_from_json(json: &str) {
    match serde_json::from_str::<Vec<Vec<Action>>>(json) {
        Ok(actions) => {
            REGISTRY.with(|reg| {
                *reg.borrow_mut() = actions;
            });
        }
        Err(err) => {
            crate::runtime::log::error(&format!(
                "[ViBao] Failed to parse actionRegistry JSON: {}",
                err
            ));
        }
    }
}

/// Looks up a sequence of Actions (an event handler's body) by id.
pub fn get(id: usize) -> Option<Vec<Action>> {
    REGISTRY.with(|reg| reg.borrow().get(id).cloned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use vibao_ast::Pos;

    fn p() -> Pos {
        Pos { line: 1, column: 1 }
    }

    #[test]
    fn test_load_and_get_roundtrip() {
        let actions = vec![Action::Assign {
            target: "dem".to_string(),
            value: vibao_ast::Expr::literal_num(1.0, p()),
            pos: p(),
        }];
        let json = serde_json::to_string(&vec![actions]).unwrap();
        load_from_json(&json);
        let fetched = get(0).expect("id 0 must exist");
        assert_eq!(fetched.len(), 1);
    }

    #[test]
    fn test_get_out_of_range_returns_none() {
        load_from_json("[]");
        assert!(get(999).is_none());
    }
}
