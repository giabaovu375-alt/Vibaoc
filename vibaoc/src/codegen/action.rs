use std::cell::RefCell;

use vibao_ast::{Action, EventName, PageEvent, PageEventName};

thread_local! {
    static ACTION_REGISTRY: RefCell<Vec<Vec<Action>>> = RefCell::new(Vec::new());
}

pub fn register_action_body(actions: Vec<Action>) -> usize {
    ACTION_REGISTRY.with(|reg| {
        let mut reg = reg.borrow_mut();
        reg.push(actions);
        reg.len() - 1
    })
}

pub fn take_action_registry() -> Vec<Vec<Action>> {
    ACTION_REGISTRY.with(|reg| std::mem::take(&mut *reg.borrow_mut()))
}

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

pub fn compile_event_handler_registry(event: &vibao_ast::EventNode) -> (String, String) {
    let dom_event = event_to_dom(&event.name);
    let attr_name = format!("data-vb-on-{}", dom_event);
    let action_id = register_action_body(event.body.clone());
    (attr_name, action_id.to_string())
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use vibao_ast::{Expr, Pos};

    fn p() -> Pos {
        Pos { line: 1, column: 1 }
    }

    #[test]
    fn test_register_action_body_assigns_sequential_ids() {
        take_action_registry();
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
        assert!(action_id.parse::<usize>().is_ok());

        let registry = take_action_registry();
        let id: usize = action_id.parse().unwrap();
        assert_eq!(registry[id].len(), 1);
    }
}
