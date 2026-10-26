use wasm_bindgen::prelude::*;
use web_sys::{Element, HtmlElement};
use crate::runtime::{state::{self, LoopFrame, SharedState}, value::VbValue};
use super::{ATTR_BIND_ATTR_PREFIX, ATTR_BIND_CLASS, ATTR_BIND_STYLE_PREFIX, ATTR_BIND_TEXT, eval_expr_id_tracked, parse_expr_id};

pub(super) fn bind_text(shared: &SharedState, el: &Element, loop_frame: Option<&LoopFrame>, component_id: Option<&str>) {
    let raw = match el.get_attribute(ATTR_BIND_TEXT) {
        Some(r) => r,
        None => return,
    };
    let Some(id) = parse_expr_id(&raw, "data-vb-text") else {
        return;
    };

    let el = el.clone();
    let loop_frame = loop_frame.cloned();
    let component_id = component_id.map(|s| s.to_string());
    state::subscribe(
        shared,
        Box::new(move |sh: &SharedState| {
            let v = eval_expr_id_tracked(sh, id, loop_frame.as_ref(), component_id.as_deref());
            el.set_text_content(Some(&v.to_string()));
        }),
    );
}

pub(super) fn bind_all_attrs(shared: &SharedState, el: &Element, loop_frame: Option<&LoopFrame>, component_id: Option<&str>) {
    let attr_names = list_attribute_names_with_prefix(el, ATTR_BIND_ATTR_PREFIX);
    for full_name in attr_names {
        let target_attr = &full_name[ATTR_BIND_ATTR_PREFIX.len()..];
        let raw = match el.get_attribute(&full_name) {
            Some(r) => r,
            None => continue,
        };
        let id = match parse_expr_id(&raw, &full_name) {
            Some(id) => id,
            None => continue,
        };

        let el = el.clone();
        let target_attr = target_attr.to_string();
        let loop_frame = loop_frame.cloned();
        let component_id = component_id.map(|s| s.to_string());
        state::subscribe(
            shared,
            Box::new(move |sh: &SharedState| {
                let v = eval_expr_id_tracked(sh, id, loop_frame.as_ref(), component_id.as_deref());
                if v.is_null() {
                    let _ = el.remove_attribute(&target_attr);
                } else {
                    let _ = el.set_attribute(&target_attr, &v.to_string());
                }
            }),
        );
    }
}

pub(super) const CSS_LENGTH_PROPS: [&str; 13] = [
    "width", "height", "padding", "border-radius", "margin", "gap",
    "row-gap", "column-gap", "min-width", "max-width", "min-height",
    "max-height", "font-size",
];

pub(super) fn css_style_value_string(css_prop: &str, v: &VbValue) -> String {
    match v.as_num() {
        Some(_) if CSS_LENGTH_PROPS.contains(&css_prop) => {
            format!("{}px", v)
        }
        _ => v.to_string(),
    }
}

pub(super) fn bind_all_styles(shared: &SharedState, el: &Element, loop_frame: Option<&LoopFrame>, component_id: Option<&str>) {
    let attr_names = list_attribute_names_with_prefix(el, ATTR_BIND_STYLE_PREFIX);
    for full_name in attr_names {
        let css_prop = &full_name[ATTR_BIND_STYLE_PREFIX.len()..];
        let raw = match el.get_attribute(&full_name) {
            Some(r) => r,
            None => continue,
        };
        let id = match parse_expr_id(&raw, &full_name) {
            Some(id) => id,
            None => continue,
        };

        let html_el: HtmlElement = match el.clone().dyn_into() {
            Ok(h) => h,
            Err(_) => continue,
        };
        let css_prop = css_prop.to_string();
        let loop_frame = loop_frame.cloned();
        let component_id = component_id.map(|s| s.to_string());
        state::subscribe(
            shared,
            Box::new(move |sh: &SharedState| {
                let v = eval_expr_id_tracked(sh, id, loop_frame.as_ref(), component_id.as_deref());
                let style = html_el.style();
                if v.is_null() {
                    let _ = style.remove_property(&css_prop);
                } else {
                    let value_str = css_style_value_string(&css_prop, &v);
                    let _ = style.set_property(&css_prop, &value_str);
                }
            }),
        );
    }
}

pub(super) fn bind_class(shared: &SharedState, el: &Element, loop_frame: Option<&LoopFrame>, component_id: Option<&str>) {
    let raw = match el.get_attribute(ATTR_BIND_CLASS) {
        Some(r) => r,
        None => return,
    };

    let pairs: Vec<(usize, String)> = raw
        .split(',')
        .filter_map(|entry| {
            let mut parts = entry.splitn(2, ':');
            let id_str = parts.next()?;
            let class_name = parts.next()?;
            let id = parse_expr_id(id_str, ATTR_BIND_CLASS)?;
            Some((id, class_name.to_string()))
        })
        .collect();

    if pairs.is_empty() {
        return;
    }

    let el = el.clone();
    let loop_frame = loop_frame.cloned();
    let component_id = component_id.map(|s| s.to_string());
    state::subscribe(
        shared,
        Box::new(move |sh: &SharedState| {
            let class_list = el.class_list();
            for (id, class_name) in &pairs {
                let v = eval_expr_id_tracked(sh, *id, loop_frame.as_ref(), component_id.as_deref());
                let _ = class_list.toggle_with_force(class_name, v.is_truthy());
            }
        }),
    );
}

pub(super) fn list_attribute_names_with_prefix(el: &Element, prefix: &str) -> Vec<String> {
    let attrs = el.attributes();
    let mut out = Vec::new();
    for i in 0..attrs.length() {
        if let Some(attr) = attrs.item(i) {
            let name = attr.name();
            if name.starts_with(prefix) {
                out.push(name);
            }
        }
    }
    out
}
