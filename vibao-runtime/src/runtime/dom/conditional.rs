use super::subtree::bind_subtree;
use super::{
    eval_expr_id_tracked, parse_expr_id, ATTR_CASE, ATTR_DEFAULT, ATTR_ELSE, ATTR_IF, ATTR_SWITCH,
};
use crate::runtime::state::{self, LoopFrame, SharedState};
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use web_sys::{Element, HtmlElement};

pub(super) fn bind_if(
    shared: &SharedState,
    el: &Element,
    loop_frame: Option<&LoopFrame>,
    component_id: Option<&str>,
) {
    let raw = match el.get_attribute(ATTR_IF) {
        Some(r) => r,
        None => return,
    };
    let id = match parse_expr_id(&raw, ATTR_IF) {
        Some(id) => id,
        None => return,
    };

    let html_el: HtmlElement = match el.clone().dyn_into() {
        Ok(h) => h,
        Err(_) => return,
    };

    let else_html_el: Option<HtmlElement> = el
        .next_element_sibling()
        .filter(|sib| sib.has_attribute(ATTR_ELSE))
        .and_then(|sib| sib.dyn_into::<HtmlElement>().ok());

    let original_display = html_el
        .style()
        .get_property_value("display")
        .unwrap_or_default();
    let original_display = if original_display == "none" {
        String::new()
    } else {
        original_display
    };

    let loop_frame = loop_frame.cloned();
    let component_id = component_id.map(|s| s.to_string());
    state::subscribe(
        shared,
        Box::new(move |sh: &SharedState| {
            let v = eval_expr_id_tracked(sh, id, loop_frame.as_ref(), component_id.as_deref());
            let style = html_el.style();
            if v.is_truthy() {
                let _ = style.set_property("display", &original_display);
                if let Some(else_el) = &else_html_el {
                    let _ = else_el.style().set_property("display", "none");
                }
            } else {
                let _ = style.set_property("display", "none");
                if let Some(else_el) = &else_html_el {
                    let _ = else_el.style().set_property("display", "");
                }
            }
        }),
    );
}

pub(super) fn bind_switch(
    shared: &SharedState,
    switch_el: &Element,
    loop_frame: Option<&LoopFrame>,
    component_id: Option<&str>,
) {
    let raw = match switch_el.get_attribute(ATTR_SWITCH) {
        Some(r) => r,
        None => return,
    };
    let Some(subject_id) = parse_expr_id(&raw, ATTR_SWITCH) else {
        return;
    };

    let children = switch_el.children();
    let mut branches: Vec<(Element, Option<usize>)> = Vec::new();
    for i in 0..children.length() {
        let Some(child) = children.item(i) else {
            continue;
        };
        if child.has_attribute(ATTR_DEFAULT) {
            branches.push((child, None));
        } else if let Some(case_raw) = child.get_attribute(ATTR_CASE) {
            if let Some(case_id) = parse_expr_id(&case_raw, ATTR_CASE) {
                branches.push((child, Some(case_id)));
            }
        }
    }

    if branches.is_empty() {
        return;
    }

    let loop_frame = loop_frame.cloned();
    let component_id = component_id.map(|s| s.to_string());
    let bound_branches: Rc<std::cell::RefCell<std::collections::HashSet<usize>>> =
        Rc::new(std::cell::RefCell::new(std::collections::HashSet::new()));
    state::subscribe(
        shared,
        Box::new(move |sh: &SharedState| {
            let subject_val =
                eval_expr_id_tracked(sh, subject_id, loop_frame.as_ref(), component_id.as_deref());

            let mut matched_idx: Option<usize> = None;
            for (idx, (_, case_id)) in branches.iter().enumerate() {
                if let Some(cid) = case_id {
                    let case_val = eval_expr_id_tracked(
                        sh,
                        *cid,
                        loop_frame.as_ref(),
                        component_id.as_deref(),
                    );
                    if matched_idx.is_none() && subject_val.strict_eq(&case_val) {
                        matched_idx = Some(idx);
                    }
                }
            }

            let show_idx =
                matched_idx.or_else(|| branches.iter().position(|(_, case_id)| case_id.is_none()));

            for (idx, (el, _)) in branches.iter().enumerate() {
                if let Ok(html_el) = el.clone().dyn_into::<HtmlElement>() {
                    let display = if Some(idx) == show_idx { "" } else { "none" };
                    let _ = html_el.style().set_property("display", display);
                }
            }

            if let Some(idx) = show_idx {
                let already_bound = bound_branches.borrow().contains(&idx);
                if !already_bound {
                    bound_branches.borrow_mut().insert(idx);
                    let (branch_el, _) = &branches[idx];
                    bind_subtree(sh, branch_el, loop_frame.as_ref(), component_id.as_deref());
                }
            }
        }),
    );
}
