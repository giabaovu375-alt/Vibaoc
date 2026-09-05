// ============================================================
// VIBAO COMPILER (Rust) — codegen/action.rs
// Compiles an Action (a statement inside an event block: nhan_click,
// on_tai, ...) into an action id registered in the registry, to be
// embedded in the HTML as "data-vb-on-<event>=\"<id>\"". The WASM
// runtime (vibao-runtime::action) reads this registry and executes it
// itself in pure Rust - NO JS is generated at build time.
//
// HISTORY: this file USED TO have a set of compile_*() functions that
// generated plain JS (compile_action/compile_assign/compile_function_call/
// compile_api_call/compile_if_action/compile_event_handler/
// compile_page_load/compile_hover_animation/compile_scroll_animation)
// - the ORIGINAL approach from when the runtime was still JS
// (__vb.toast, __api.call...). That entire block WAS REMOVED (along
// with its 13 corresponding tests) once the runtime fully switched to
// Rust/WASM, leaving those JS-emitting functions with no real call
// path left anywhere in the build pipeline - they only still existed
// to avoid breaking old tests, i.e. they were genuinely dead code. The
// registry
// (register_action_body/take_action_registry/compile_event_handler_registry/
// compile_page_load_registry) is the ONLY remaining path.
// ============================================================

use std::cell::RefCell;

use vibao_ast::{Action, EventName, PageEvent, PageEventName};

thread_local! {
    /// The accumulator holding every "event handler body" (Vec<Action>)
    /// registered during the current build pass. The index in the Vec
    /// IS the action id used in "data-vb-on-click='<id>'" - a complete
    /// mirror of EXPR_REGISTRY in expr.rs.
    static ACTION_REGISTRY: RefCell<Vec<Vec<Action>>> = RefCell::new(Vec::new());
}

/// Registers a sequence of Actions (one event handler's body) into the
/// registry, returning the id to embed in the HTML as
/// "data-vb-on-<event>=\"<id>\"".
pub fn register_action_body(actions: Vec<Action>) -> usize {
    ACTION_REGISTRY.with(|reg| {
        let mut reg = reg.borrow_mut();
        reg.push(actions);
        reg.len() - 1
    })
}

/// Retrieves the entire accumulated registry AND clears it (resetting
/// for the next build pass) - called at the end of `gen_app_js`, at the
/// same time as `take_expr_registry()`.
pub fn take_action_registry() -> Vec<Vec<Action>> {
    ACTION_REGISTRY.with(|reg| std::mem::take(&mut *reg.borrow_mut()))
}

/// Maps a ViBao event name (the EventName enum) to the real DOM event
/// name. Matches EVENT_DOM_MAP in the old TS version.
fn event_to_dom(name: &EventName) -> &'static str {
    match name {
        EventName::OnClick => "click",
        EventName::OnHover => "mouseenter",
        EventName::OnBlur => "blur",
        EventName::OnFocus => "focus",
        EventName::OnChange => "change",
        EventName::OnSubmit => "submit",
        EventName::OnScroll => "scroll",
    }
}

// ════════════════════════════════════════════════════════════
// EVENT HANDLER — REGISTRY (the CORRECT path for the current pipeline)
// ════════════════════════════════════════════════════════════

/// Compiles an EventNode into a single HTML attribute
/// `data-vb-on-<dom-event>="<actionId>"`, generating NO JS.
///
/// Returns `(attribute_name, value)` instead of a complete HTML string,
/// so the caller (element.rs) can decide how to insert it into the
/// attribute string it's currently building (preserving element.rs's
/// existing style, without imposing a format string here).
pub fn compile_event_handler_registry(event: &vibao_ast::EventNode) -> (String, String) {
    let dom_event = event_to_dom(&event.name);
    let attr_name = format!("data-vb-on-{}", dom_event);
    let action_id = register_action_body(event.body.clone());
    (attr_name, action_id.to_string())
}

// ════════════════════════════════════════════════════════════
// PAGE LOAD / UNLOAD — REGISTRY (the CORRECT path for the current pipeline)
// ════════════════════════════════════════════════════════════

/// Compiles the list of PageEvents (on_tai/on_huy) into 2 action ids
/// (registered into the same registry as on_click/on_hover/...),
/// generating NO JS. Returns (id_on_tai, id_on_huy) as number strings -
/// None if the page doesn't declare the corresponding event (avoids
/// registering a useless empty action into the registry).
///
/// The runtime (router.rs::activate_page) reads the 2 attributes
/// "data-vb-on-tai"/"data-vb-on-huy" embedded on the `.vb-page` div
/// itself (see codegen/mod.rs::gen_page) to know which action to
/// dispatch when the page is activated/left.
pub fn compile_page_load_registry(events: &[PageEvent]) -> (Option<String>, Option<String>) {
    let on_tai_body: Vec<Action> = events
        .iter()
        .filter(|e| e.name == PageEventName::OnTai)
        .flat_map(|e| e.body.iter().cloned())
        .collect();
    let on_huy_body: Vec<Action> = events
        .iter()
        .filter(|e| e.name == PageEventName::OnHuy)
        .flat_map(|e| e.body.iter().cloned())
        .collect();

    let id_on_tai = if on_tai_body.is_empty() {
        None
    } else {
        Some(register_action_body(on_tai_body).to_string())
    };
    let id_on_huy = if on_huy_body.is_empty() {
        None
    } else {
        Some(register_action_body(on_huy_body).to_string())
    };

    (id_on_tai, id_on_huy)
}

// ════════════════════════════════════════════════════════════
// UNIT TESTS
// ════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use vibao_ast::{Expr, Pos};

    fn p() -> Pos {
        Pos { line: 1, column: 1 }
    }

    #[test]
    fn test_register_action_body_assigns_sequential_ids() {
        take_action_registry(); // clean this thread's registry first
        let a1 = vec![Action::Assign {
            target: "dem".to_string(),
            value: Expr::literal_num(1.0, p()),
            pos: p(),
        }];
        let a2 = vec![Action::Assign {
            target: "dem".to_string(),
            value: Expr::literal_num(2.0, p()),
            pos: p(),
        }];
        let id1 = register_action_body(a1);
        let id2 = register_action_body(a2);
        assert_eq!(id2, id1 + 1);
    }

    #[test]
    fn test_take_action_registry_drains_and_resets() {
        take_action_registry();
        register_action_body(vec![]);
        register_action_body(vec![]);
        let drained = take_action_registry();
        assert_eq!(drained.len(), 2);
        let empty = take_action_registry();
        assert_eq!(empty.len(), 0);
    }

    #[test]
    fn test_compile_event_handler_registry_emits_attribute_not_js() {
        take_action_registry();
        let event = vibao_ast::EventNode {
            name: EventName::OnClick,
            body: vec![Action::FunctionCall {
                name: "thong_bao".to_string(),
                args: vec![Expr::literal_str("Xin chào".to_string(), p())],
                opts: vec![],
                assign_to: None,
                pos: p(),
            }],
            pos: p(),
        };
        let (attr_name, action_id) = compile_event_handler_registry(&event);
        assert_eq!(attr_name, "data-vb-on-click");
        // action_id must parse as a number (not JS code)
        assert!(action_id.parse::<usize>().is_ok());

        // The registry must actually contain the action just registered.
        let registry = take_action_registry();
        let id: usize = action_id.parse().unwrap();
        assert_eq!(registry[id].len(), 1);
    }
}
