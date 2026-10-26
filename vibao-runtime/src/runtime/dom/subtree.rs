use std::collections::HashMap;
use web_sys::{Element};
use crate::runtime::{expr_eval, expr_registry, state::{self, LoopFrame, SharedState}, value::VbValue};
use super::{ATTR_BIND_CLASS, ATTR_BIND_TEXT, ATTR_COMPONENT, ATTR_IF, ATTR_LOOP, ATTR_PROPS, ATTR_SWITCH, is_inside_loop, is_inside_nested_component, is_inside_switch_case, query_all, runtime_component_id};
use super::animation::bind_animations;
use super::conditional::{bind_if, bind_switch};
use super::events::bind_events;
use super::loop_render::bind_loop;
use super::model::bind_model;
use super::text_attr::{bind_all_attrs, bind_all_styles, bind_class, bind_text};

pub(super) fn bind_component(shared: &SharedState, el: &Element, loop_frame: Option<&LoopFrame>) {
    let Some(original_id) = el.get_attribute("id") else {
        crate::runtime::log::error("[ViBao] component is missing an id attribute - skipping bind");
        return;
    };

    let id = if loop_frame.is_some() {
        let runtime_id = runtime_component_id(&original_id);
        let _ = el.set_attribute("id", &runtime_id);
        runtime_id
    } else {
        original_id
    };
    let props_raw = el.get_attribute(ATTR_PROPS).unwrap_or_else(|| "{}".to_string());

    let prop_expr_ids: HashMap<String, usize> = match serde_json::from_str(&props_raw) {
        Ok(m) => m,
        Err(err) => {
            crate::runtime::log::error(&format!(
                "[ViBao] invalid data-vb-props JSON for component #{}: {} ({})",
                id, props_raw, err
            ));
            return;
        }
    };

    let captured_loop_frame = loop_frame.cloned();

    let mut getters: HashMap<String, Box<dyn Fn(&state::State) -> VbValue>> = HashMap::new();
    for (prop_name, expr_id) in prop_expr_ids {
        let getter_loop_frame = captured_loop_frame.clone();
        getters.insert(
            prop_name.clone(),
            Box::new(move |state: &state::State| match expr_registry::get(expr_id) {
                Some(expr) => expr_eval::eval_readonly_with_frame(state, &expr, getter_loop_frame.as_ref()),
                None => {
                    crate::runtime::log::warn(&format!(
                        "[ViBao] expr id {} (component prop) does not exist in the registry",
                        expr_id
                    ));
                    VbValue::Null
                }
            }),
        );
    }

    shared.borrow_mut().register_props(&id, getters);
    shared.borrow_mut().push_component_scope(&id);
    bind_subtree(shared, el, loop_frame, Some(&id));
    shared.borrow_mut().pop_component_scope(&id);
}

pub(crate) fn bind_subtree(shared: &SharedState, root: &Element, loop_frame: Option<&LoopFrame>, component_id: Option<&str>) {
    for el in query_all(root, &format!("[{}]", ATTR_BIND_TEXT)) {
        if is_inside_nested_component(&el, root) || is_inside_switch_case(&el, root) || is_inside_loop(&el, root) {
            continue;
        }
        bind_text(shared, &el, loop_frame, component_id);
    }
    for el in query_all(root, &format!("[{}]", ATTR_IF)) {
        if is_inside_nested_component(&el, root) || is_inside_switch_case(&el, root) || is_inside_loop(&el, root) {
            continue;
        }
        bind_if(shared, &el, loop_frame, component_id);
    }
    for el in query_all(root, &format!("[{}]", ATTR_LOOP)) {
        if is_inside_nested_component(&el, root) || is_inside_switch_case(&el, root) || is_inside_loop(&el, root) {
            continue;
        }
        bind_loop(shared, &el, loop_frame, component_id);
    }
    for el in query_all(root, &format!("[{}]", ATTR_SWITCH)) {
        if is_inside_nested_component(&el, root) {
            continue;
        }
        if is_inside_loop(&el, root) {
            continue;
        }
        bind_switch(shared, &el, loop_frame, component_id);
    }
    for el in query_all(root, &format!("[{}]", ATTR_BIND_CLASS)) {
        if is_inside_nested_component(&el, root) || is_inside_switch_case(&el, root) || is_inside_loop(&el, root) {
            continue;
        }
        bind_class(shared, &el, loop_frame, component_id);
    }
    for el in query_all(root, "*") {
        if is_inside_nested_component(&el, root) || is_inside_switch_case(&el, root) || is_inside_loop(&el, root) {
            continue;
        }
        bind_all_attrs(shared, &el, loop_frame, component_id);
        bind_all_styles(shared, &el, loop_frame, component_id);
        bind_events(shared, &el, loop_frame, component_id);
        bind_model(shared, &el, loop_frame, component_id);
        bind_animations(&el);
    }

    for el in query_all(root, &format!("[{}]", ATTR_COMPONENT)) {
        let is_root_itself = el.is_same_node(Some(root.as_ref()));
        if is_root_itself
            || is_inside_nested_component(&el, root)
            || is_inside_switch_case(&el, root)
            || is_inside_loop(&el, root)
        {
            continue;
        }
        bind_component(shared, &el, loop_frame);
    }
}
