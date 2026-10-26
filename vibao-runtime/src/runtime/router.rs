use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};

use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use web_sys::Event;

use super::dom;
use super::state::SharedState;
use super::value::VbValue;

#[derive(Clone)]
pub struct RouteEntry {
    pub pattern: String,
    pub param_names: Vec<String>,
}

pub struct RouteMatch {
    pub route: RouteEntry,
    pub params: BTreeMap<String, String>,
}

thread_local! {
    static ROUTES: RefCell<Vec<RouteEntry>> = RefCell::new(Vec::new());
    static CURRENT_ROUTE: RefCell<Option<String>> = RefCell::new(None);
    static CURRENT_PATH: RefCell<Option<String>> = RefCell::new(None);
}

fn register_route(pattern: &str) {
    let param_names = pattern
        .split('/')
        .filter(|seg| seg.starts_with(':'))
        .map(|seg| seg[1..].to_string())
        .collect();
    ROUTES.with(|r| {
        r.borrow_mut().push(RouteEntry {
            pattern: pattern.to_string(),
            param_names,
        });
    });
}

fn normalize_path(path: &str) -> String {
    let without_query = path.split('?').next().unwrap_or(path);
    let without_hash = without_query.split('#').next().unwrap_or(without_query);
    let trimmed = without_hash.trim_end_matches('/');
    if trimmed.is_empty() {
        "/".to_string()
    } else {
        trimmed.to_string()
    }
}

fn match_pattern(pattern: &str, path: &str) -> Option<BTreeMap<String, String>> {
    let normalized_pattern = normalize_path(pattern);
    let normalized_path = normalize_path(path);
    let pattern_segs: Vec<&str> = normalized_pattern.split('/').collect();
    let path_segs: Vec<&str> = normalized_path.split('/').collect();

    if pattern_segs.len() != path_segs.len() {
        return None;
    }

    let mut params = BTreeMap::new();
    for (p_seg, path_seg) in pattern_segs.iter().zip(path_segs.iter()) {
        if let Some(name) = p_seg.strip_prefix(':') {
            if path_seg.is_empty() {
                return None;
            }
            params.insert(name.to_string(), path_seg.to_string());
        } else if p_seg != path_seg {
            return None;
        }
    }

    Some(params)
}

fn match_route(path: &str) -> Option<RouteMatch> {
    ROUTES.with(|routes| {
        for route in routes.borrow().iter() {
            if let Some(params) = match_pattern(&route.pattern, path) {
                return Some(RouteMatch {
                    route: route.clone(),
                    params,
                });
            }
        }
        None
    })
}

async fn activate_page(shared: &SharedState, route_match: &RouteMatch) {
    let doc = match web_sys::window().and_then(|w| w.document()) {
        Some(d) => d,
        None => return,
    };

    if let Some(prev_pattern) = current_route() {
        if prev_pattern != route_match.route.pattern {
            dispatch_lifecycle_action(shared, &doc, &prev_pattern, "data-vb-on-huy").await;
        }
    }

    if let Ok(list) = doc.query_selector_all(".vb-page") {
        for i in 0..list.length() {
            if let Some(node) = list.item(i) {
                if let Ok(el) = node.dyn_into::<web_sys::HtmlElement>() {
                    let _ = el.style().set_property("display", "none");
                }
            }
        }
    }

    let escaped = route_match.route.pattern.replace('\\', "\\\\").replace('"', "\\\"");
    let selector = format!(".vb-page[data-route=\"{}\"]", escaped);

    let target = match doc.query_selector(&selector).ok().flatten() {
        Some(t) => t,
        None => {
            super::log::error(&format!(
                "[ViBao Router] No DOM found for route \"{}\"",
                route_match.route.pattern
            ));
            return;
        }
    };

    shared.borrow_mut().init_page_state(HashMap::new());

    {
        let mut state = shared.borrow_mut();
        for (key, value) in &route_match.params {
            state.set_state(key, VbValue::str(value.clone()));
        }
    }

    let initial_exprs = shared
        .borrow()
        .page_initial_states_for_route(&route_match.route.pattern);
    for (name, expr) in initial_exprs {
        let value = super::expr_eval::eval(shared, &expr);
        shared.borrow_mut().set_state(&name, value);
    }

    apply_page_metadata(&doc, shared, &route_match.route.pattern);

    CURRENT_ROUTE.with(|cur| {
        *cur.borrow_mut() = Some(route_match.route.pattern.clone());
    });

    dispatch_lifecycle_action(shared, &doc, &route_match.route.pattern, "data-vb-on-tai").await;

    dom::bind_subtree(shared, &target, None, None);

    if let Ok(html_el) = target.clone().dyn_into::<web_sys::HtmlElement>() {
        let _ = html_el.style().set_property("display", "block");
    }
}

async fn dispatch_lifecycle_action(shared: &SharedState, doc: &web_sys::Document, pattern: &str, attr_name: &str) {
    let escaped = pattern.replace('\\', "\\\\").replace('"', "\\\"");
    let selector = format!(".vb-page[data-route=\"{}\"]", escaped);

    let Some(el) = doc.query_selector(&selector).ok().flatten() else {
        return;
    };
    let Some(raw_id) = el.get_attribute(attr_name) else {
        return;
    };
    let Ok(action_id) = raw_id.trim().parse::<usize>() else {
        super::log::warn(&format!(
            "[ViBao Router] \"{}\" is not a valid action id: \"{}\"",
            attr_name, raw_id
        ));
        return;
    };
    let Some(actions) = super::action_registry::get(action_id) else {
        super::log::warn(&format!(
            "[ViBao Router] action id {} ({}) does not exist in the registry",
            action_id, attr_name
        ));
        return;
    };

    super::action::dispatch_all(shared, &actions, None, None).await;
}

pub fn navigate(shared: &SharedState, path: &str, replace: bool, from_popstate: bool) {
    let matched = match match_route(path) {
        Some(m) => m,
        None => {
            super::log::warn(&format!("[ViBao Router] No route found for \"{}\"", path));
            return;
        }
    };
    remember_path(path);

    if !from_popstate {
        if let Some(win) = web_sys::window() {
            if let Ok(history) = win.history() {
                let state = wasm_bindgen::JsValue::NULL;
                let result = if replace {
                    history.replace_state_with_url(&state, "", Some(path))
                } else {
                    history.push_state_with_url(&state, "", Some(path))
                };
                let _ = result;
            }
        }
    }

    let shared_clone = shared.clone();
    let matched_route = matched;
    wasm_bindgen_futures::spawn_local(async move {
        activate_page(&shared_clone, &matched_route).await;
    });

    if let Some(win) = web_sys::window() {
        win.scroll_to_with_x_and_y(0.0, 0.0);
    }
}

fn remember_path(path: &str) {
    CURRENT_PATH.with(|cur| *cur.borrow_mut() = Some(path.to_string()));
}

fn is_already_current(current: Option<&str>, new_path: &str) -> bool {
    current == Some(new_path)
}

pub fn current_route() -> Option<String> {
    CURRENT_ROUTE.with(|cur| cur.borrow().clone())
}

fn apply_page_metadata(doc: &web_sys::Document, shared: &SharedState, route: &str) {
    let (name, description) = match shared.borrow().page_metadata_for_route(route) {
        Some(meta) => meta,
        None => return,
    };

    if let Some(name) = name {
        doc.set_title(&name);
    }

    if let Some(description) = description {
        if let Some(existing) = doc.query_selector("meta[name=\"description\"]").ok().flatten() {
            let _ = existing.set_attribute("content", &description);
        } else if let Some(head) = doc.query_selector("head").ok().flatten() {
            if let Ok(meta) = doc.create_element("meta") {
                let _ = meta.set_attribute("name", "description");
                let _ = meta.set_attribute("content", &description);
                let _ = head.append_child(&meta);
            }
        }
    }
}

fn is_in_page_anchor(path: &str) -> bool {
    path.starts_with('#')
}

fn is_external_link(path: &str) -> bool {
    if path.starts_with("//") {
        return true;
    }
    let mut chars = path.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() => {}
        _ => return false,
    }
    for c in chars {
        if c == ':' {
            return true;
        }
        if !(c.is_ascii_alphanumeric() || c == '+' || c == '-' || c == '.') {
            return false;
        }
    }
    false
}

fn setup_link_interception(shared: &SharedState) {
    let doc = match web_sys::window().and_then(|w| w.document()) {
        Some(d) => d,
        None => return,
    };

    let shared_clone = shared.clone();
    let closure = Closure::<dyn FnMut(Event)>::new(move |evt: Event| {
        if let Ok(mouse_evt) = evt.clone().dyn_into::<web_sys::MouseEvent>() {
            if mouse_evt.ctrl_key() || mouse_evt.meta_key() || mouse_evt.shift_key() || mouse_evt.button() == 1 {
                return;
            }
        }

        let Some(target) = evt.target() else { return };
        let Ok(mut el) = target.dyn_into::<web_sys::Element>() else { return };

        loop {
            if el.tag_name().eq_ignore_ascii_case("a") {
                if let Some(path) = el.get_attribute("data-vb-link") {
                    if is_in_page_anchor(&path) || is_external_link(&path) || !path.starts_with('/') {
                        return;
                    }
                    evt.prevent_default();
                    navigate(&shared_clone, &path, false, false);
                    return;
                }
            }
            match el.parent_element() {
                Some(p) => el = p,
                None => break,
            }
        }
    });

    let target: web_sys::EventTarget = doc.into();
    let _ = target.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref());
    closure.forget();
}

fn setup_popstate_handler(shared: &SharedState) {
    let Some(win) = web_sys::window() else { return };
    let shared_clone = shared.clone();

    let closure = Closure::<dyn FnMut(Event)>::new(move |_evt: Event| {
        let path = web_sys::window()
            .and_then(|w| w.location().pathname().ok())
            .unwrap_or_else(|| "/".to_string());
        let current = CURRENT_PATH.with(|cur| cur.borrow().clone());
        if is_already_current(current.as_deref(), &path) {
            return;
        }
        navigate(&shared_clone, &path, false, true);
    });

    let target: web_sys::EventTarget = win.into();
    let _ = target.add_event_listener_with_callback("popstate", closure.as_ref().unchecked_ref());
    closure.forget();
}

pub fn boot_router(shared: &SharedState) {
    let doc = match web_sys::window().and_then(|w| w.document()) {
        Some(d) => d,
        None => return,
    };

    if let Ok(list) = doc.query_selector_all(".vb-page[data-route]") {
        for i in 0..list.length() {
            if let Some(node) = list.item(i) {
                if let Ok(el) = node.dyn_into::<web_sys::Element>() {
                    if let Some(route) = el.get_attribute("data-route") {
                        register_route(&route);
                    }
                }
            }
        }
    }

    setup_link_interception(shared);
    setup_popstate_handler(shared);

    let path = web_sys::window()
        .and_then(|w| w.location().pathname().ok())
        .unwrap_or_else(|| "/".to_string());

    match match_route(&path) {
        Some(matched) => {
            remember_path(&path);
            let shared_clone = shared.clone();
            wasm_bindgen_futures::spawn_local(async move {
                activate_page(&shared_clone, &matched).await;
            });
        }
        None => {
            let fallback = ROUTES.with(|r| r.borrow().first().cloned());
            if let Some(route) = fallback {
                super::log::warn(&format!(
                    "[ViBao Router] No route matched \"{}\", falling back to \"{}\".",
                    path, route.pattern
                ));
                navigate(shared, &route.pattern, true, false);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_already_current_ignores_anchor_only_popstate() {
        assert!(is_already_current(Some("/"), "/"));
        assert!(is_already_current(Some("/du-an/3"), "/du-an/3"));
        assert!(!is_already_current(Some("/"), "/du-an"));
        assert!(!is_already_current(Some("/du-an/3"), "/du-an/4"));
        assert!(!is_already_current(None, "/"));
    }

    #[test]
    fn test_is_external_link() {
        for v in ["https://github.com", "mailto:a@b.c", "tel:+84", "//cdn.x/y"] {
            assert!(is_external_link(v), "{}", v);
        }
        for v in ["/du-an", "#top", "du-an", ""] {
            assert!(!is_external_link(v), "{}", v);
        }
    }

    #[test]
    fn test_is_in_page_anchor() {
        assert!(is_in_page_anchor("#gioi-thieu"));
        assert!(is_in_page_anchor("#"));
        assert!(!is_in_page_anchor("/gioi-thieu"));
        assert!(!is_in_page_anchor("/san-pham/42"));
    }

    #[test]
    fn test_normalize_path_strips_query_and_hash() {
        assert_eq!(normalize_path("/gioi-thieu?x=1#section"), "/gioi-thieu");
    }

    #[test]
    fn test_normalize_path_strips_trailing_slash() {
        assert_eq!(normalize_path("/gioi-thieu/"), "/gioi-thieu");
        assert_eq!(normalize_path("/"), "/");
    }

    #[test]
    fn test_match_pattern_static_route() {
        let params = match_pattern("/gioi-thieu", "/gioi-thieu");
        assert!(params.is_some());
        assert!(params.unwrap().is_empty());
    }

    #[test]
    fn test_match_pattern_with_param() {
        let params = match_pattern("/san-pham/:id", "/san-pham/42").unwrap();
        assert_eq!(params.get("id"), Some(&"42".to_string()));
    }

    #[test]
    fn test_match_pattern_multi_param() {
        let params = match_pattern("/danh-muc/:cat/san-pham/:id", "/danh-muc/dien-tu/san-pham/7").unwrap();
        assert_eq!(params.get("cat"), Some(&"dien-tu".to_string()));
        assert_eq!(params.get("id"), Some(&"7".to_string()));
    }

    #[test]
    fn test_match_pattern_rejects_wrong_segment_count() {
        assert!(match_pattern("/san-pham/:id", "/san-pham/42/extra").is_none());
    }

    #[test]
    fn test_match_pattern_rejects_non_matching_static_segment() {
        assert!(match_pattern("/gioi-thieu", "/lien-he").is_none());
    }

    #[test]
    fn test_register_and_match_route() {
        ROUTES.with(|r| r.borrow_mut().clear());
        register_route("/san-pham/:id");
        let matched = match_route("/san-pham/99").expect("must match the registered route");
        assert_eq!(matched.route.pattern, "/san-pham/:id");
        assert_eq!(matched.params.get("id"), Some(&"99".to_string()));
    }

    #[test]
    fn test_match_route_no_match_returns_none() {
        ROUTES.with(|r| r.borrow_mut().clear());
        register_route("/gioi-thieu");
        assert!(match_route("/khong-ton-tai").is_none());
    }
}
