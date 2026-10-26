use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::*;
use web_sys::{Document, Element, HtmlElement};
use crate::runtime::{state::{SharedState}};
use super::document;

pub fn toast(message: &str, kieu: &str, duration_ms: i32) {
    let doc = document();
    let container = match ensure_toast_container(&doc) {
        Some(c) => c,
        None => return,
    };

    let el = match doc.create_element("div") {
        Ok(e) => e,
        Err(_) => return,
    };
    let _ = el.set_class_name(&format!("vb-toast vb-toast-{}", kieu));
    el.set_text_content(Some(message));
    let _ = container.append_child(&el);

    let el_for_timeout = el.clone();
    let closure = Closure::once(move || {
        let _ = el_for_timeout
            .parent_node()
            .map(|parent| parent.remove_child(&el_for_timeout));
    });
    if let Some(win) = web_sys::window() {
        let _ = win.set_timeout_with_callback_and_timeout_and_arguments_0(
            closure.as_ref().unchecked_ref(),
            duration_ms.max(0),
        );
    }
    closure.forget();
}

pub(super) fn ensure_toast_container(doc: &Document) -> Option<Element> {
    if let Some(existing) = doc.query_selector(".vb-toast-container").ok().flatten() {
        return Some(existing);
    }
    let el = doc.create_element("div").ok()?;
    let _ = el.set_class_name("vb-toast-container");
    doc.body()?.append_child(&el).ok()?;
    Some(el)
}

pub fn alert(message: &str) {
    if let Some(win) = web_sys::window() {
        let _ = win.alert_with_message(message);
    }
}

pub fn navigate(shared: &SharedState, path: &str) {
    crate::runtime::router::navigate(shared, path, false, false);
}

pub fn open_tab(path: &str) {
    let win = match web_sys::window() {
        Some(w) => w,
        None => return,
    };
    let url = if path.starts_with("http://") || path.starts_with("https://") {
        path.to_string()
    } else {
        let origin = win.location().origin().unwrap_or_default();
        format!("{}{}", origin, path)
    };
    let _ = win.open_with_url_and_target_and_features(&url, "_blank", "noopener");
}

pub fn open_modal(id: &str) {
    let doc = document();
    let el_id = format!("vb-modal-{}", id);
    let Some(overlay) = doc.get_element_by_id(&el_id) else {
        crate::runtime::log::warn(&format!("[ViBao] Modal \"{}\" not found", id));
        return;
    };
    if let Ok(html_el) = overlay.dyn_into::<HtmlElement>() {
        let _ = html_el.style().set_property("display", "flex");
        let _ = html_el.focus();
    }
    if let Some(body) = doc.body() {
        let _ = body.style().set_property("overflow", "hidden");
    }
}

pub fn close_modal(id: &str) {
    let doc = document();
    let el_id = format!("vb-modal-{}", id);
    let Some(overlay) = doc.get_element_by_id(&el_id) else {
        return;
    };
    if let Ok(html_el) = overlay.dyn_into::<HtmlElement>() {
        let _ = html_el.style().set_property("display", "none");
    }
    let any_open = doc
        .query_selector_all("[id^='vb-modal-']")
        .ok()
        .map(|list| {
            (0..list.length()).any(|i| {
                list.item(i)
                    .and_then(|n| n.dyn_into::<HtmlElement>().ok())
                    .map(|el| el.style().get_property_value("display").unwrap_or_default() != "none")
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false);

    if !any_open {
        if let Some(body) = doc.body() {
            let _ = body.style().set_property("overflow", "");
        }
    }
}

pub fn scroll_to(target: &str) {
    let id = target.trim_start_matches('#');
    let doc = document();
    let Some(el) = doc.get_element_by_id(id) else {
        crate::runtime::log::warn(&format!("[ViBao] cuon_den(): element #{} not found", id));
        return;
    };
    el.scroll_into_view();
}

pub fn scroll_top() {
    if let Some(win) = web_sys::window() {
        win.scroll_to_with_x_and_y(0.0, 0.0);
    }
}

pub fn set_theme(requested: &str) {
    let Some(root) = document().document_element() else {
        return;
    };
    let list = root.get_attribute("data-vb-themes").unwrap_or_default();
    let names: Vec<&str> = list.split(',').filter(|n| !n.is_empty()).collect();
    if names.is_empty() {
        crate::runtime::log::warn("[ViBao] dat_chu_de(): trang không khai báo theme nào");
        return;
    }

    let next = if requested.is_empty() {
        let current = current_theme(&root, &names);
        let index = names.iter().position(|n| *n == current).unwrap_or(0);
        names[(index + 1) % names.len()].to_string()
    } else if names.contains(&requested) {
        requested.to_string()
    } else {
        crate::runtime::log::warn(&format!("[ViBao] dat_chu_de(): theme \"{}\" không tồn tại", requested));
        return;
    };

    let _ = root.set_attribute("data-vb-theme", &next);
    if let Some(storage) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
        let _ = storage.set_item("vb-theme", &next);
    }
}

fn current_theme(root: &Element, names: &[&str]) -> String {
    if let Some(theme) = root.get_attribute("data-vb-theme") {
        return theme;
    }
    web_sys::window()
        .and_then(|w| w.get_computed_style(root).ok().flatten())
        .and_then(|style| style.get_property_value("--vb-theme").ok())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| names[0].to_string())
}

pub fn copy_text(_text: &str) {
    crate::runtime::log::warn(
        "[ViBao] sao_chep() is not yet supported - the Clipboard API needs the build flag \
         --cfg=web_sys_unstable_apis, see the note in dom.rs::copy_text.",
    );
}
