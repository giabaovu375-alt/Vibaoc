use super::subtree::{bind_component, bind_subtree};
use super::{
    document, eval_expr_id_tracked, query_all, ATTR_COMPONENT, ATTR_LOOP, ATTR_LOOP_ITEM_MARKER,
};
use crate::runtime::state::{self, LoopFrame, SharedState};
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use web_sys::Element;

pub(super) struct LoopBindingState {
    rendered_node_count: usize,
}

pub(super) fn unregister_component_props_in_subtree(shared: &SharedState, root: &web_sys::Node) {
    let mut ids = Vec::new();
    if let Some(el) = root.dyn_ref::<Element>() {
        if el.has_attribute(ATTR_COMPONENT) {
            if let Some(id) = el.get_attribute("id") {
                ids.push(id);
            }
        }
        for child in query_all(el, &format!("[{}]", ATTR_COMPONENT)) {
            if let Some(id) = child.get_attribute("id") {
                ids.push(id);
            }
        }
    }
    if !ids.is_empty() {
        let mut state = shared.borrow_mut();
        for id in ids {
            state.unregister_props(&id);
        }
    }
}

pub(super) fn bind_loop(
    shared: &SharedState,
    anchor: &Element,
    outer_loop_frame: Option<&LoopFrame>,
    component_id: Option<&str>,
) {
    let raw = match anchor.get_attribute(ATTR_LOOP) {
        Some(r) => r,
        None => return,
    };

    #[derive(serde::Deserialize)]
    struct LoopSpec {
        iterable_expr_id: usize,
        item_var: String,
        index_var: Option<String>,
        template_id: String,
    }

    let spec: LoopSpec = match serde_json::from_str(&raw) {
        Ok(s) => s,
        Err(err) => {
            crate::runtime::log::error(&format!(
                "[ViBao] invalid data-vb-loop JSON: {} ({})",
                raw, err
            ));
            return;
        }
    };

    let doc = document();
    let template_el = match doc.get_element_by_id(&spec.template_id) {
        Some(t) => t,
        None => {
            crate::runtime::log::error(&format!(
                "[ViBao] <template id=\"{}\"> not found for the loop",
                spec.template_id
            ));
            return;
        }
    };
    let template: web_sys::HtmlTemplateElement = match template_el.dyn_into() {
        Ok(t) => t,
        Err(_) => return,
    };

    let parent = match anchor.parent_node() {
        Some(p) => p,
        None => return,
    };

    let binding_state = Rc::new(std::cell::RefCell::new(LoopBindingState {
        rendered_node_count: 0,
    }));
    let anchor = anchor.clone();
    let outer_loop_frame = outer_loop_frame.cloned();
    let component_id = component_id.map(|s| s.to_string());

    state::subscribe(
        shared,
        Box::new(move |sh: &SharedState| {
            let iterable = eval_expr_id_tracked(
                sh,
                spec.iterable_expr_id,
                outer_loop_frame.as_ref(),
                component_id.as_deref(),
            );
            let items = iterable.as_array().cloned().unwrap_or_default();

            let bs = binding_state.borrow_mut();
            for _ in 0..bs.rendered_node_count {
                if let Some(next) = anchor.next_sibling() {
                    unregister_component_props_in_subtree(sh, &next);
                    let _ = parent.remove_child(&next);
                }
            }
            let mut total_nodes_this_render: usize = 0;
            drop(bs);

            let mut insert_after: web_sys::Node = anchor.clone().into();
            for (index, item) in items.iter().enumerate() {
                let fragment = template
                    .content()
                    .clone_node_with_deep(true)
                    .unwrap_or_else(|_| doc.create_document_fragment().into());

                let frame = LoopFrame {
                    item_var: spec.item_var.clone(),
                    item_value: item.clone(),
                    index_var: spec.index_var.clone(),
                    index_value: Some(index as f64),
                };

                if let Ok(frag_el) = fragment.clone().dyn_into::<web_sys::DocumentFragment>() {
                    bind_fragment_with_loop_frame(sh, &frag_el, &frame, component_id.as_deref());
                    let marked_children = frag_el.children();
                    for i in 0..marked_children.length() {
                        if let Some(child) = marked_children.item(i) {
                            let _ = child.set_attribute(ATTR_LOOP_ITEM_MARKER, "1");
                        }
                    }
                }

                total_nodes_this_render += fragment.child_nodes().length() as usize;

                let last_node_of_fragment = fragment.last_child();

                if let Some(new_sibling) = insert_after.next_sibling() {
                    let _ = parent.insert_before(&fragment, Some(&new_sibling));
                } else {
                    let _ = parent.append_child(&fragment);
                }

                if let Some(last) = last_node_of_fragment {
                    insert_after = last;
                }
            }

            binding_state.borrow_mut().rendered_node_count = total_nodes_this_render;
        }),
    );
}

pub(super) fn bind_fragment_with_loop_frame(
    shared: &SharedState,
    fragment: &web_sys::DocumentFragment,
    frame: &LoopFrame,
    component_id: Option<&str>,
) {
    let children = fragment.children();
    for i in 0..children.length() {
        if let Some(el) = children.item(i) {
            if el.has_attribute(ATTR_COMPONENT) {
                bind_component(shared, &el, Some(frame));
            } else {
                bind_subtree(shared, &el, Some(frame), component_id);
            }
        }
    }
}
