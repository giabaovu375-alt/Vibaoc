// ============================================================
// VIBAO RUNTIME (Rust/WASM) — runtime/router.rs
// A port of 20-runtime-router.ts to pure Rust. This is a REAL SPA
// router - every ViBao page is built together into a single index.html
// (see vibaoc/src/main.rs::cmd_build), with each page being a
// `<div class="vb-page" data-route="...">`. The router only
// SHOWS/HIDES the correct div for the current URL, never reloads the
// page, never fetches HTML over the network - every page already
// exists in the DOM from the initial load.
//
// DIFFERENCES from the original JS version:
//   - No __guards/registerGuard - ViBao currently has no guard
//     declaration syntax in the language (no "guard(...)" in ast.rs),
//     so an auth-gate is left for a later round once the language has
//     matching syntax.
//   - A route pattern (":id") uses manual segment matching instead of
//     compiling to a real Regex - avoiding a `regex` dependency just
//     for one simple task (matching by "/"), sufficient for most use
//     cases.
// ============================================================

use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};

use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use web_sys::Event;

use super::dom;
use super::state::SharedState;
use super::value::VbValue;

// ════════════════════════════════════════════════════════════
// 1. ROUTE REGISTRY & MATCHING
// ════════════════════════════════════════════════════════════

/// A registered route - equivalent to an element of the `__routes`
/// array in the JS version.
#[derive(Clone)]
pub struct RouteEntry {
    /// The original pattern, e.g. "/san-pham/:id" - used as the key
    /// for looking up `[data-route="..."]` in the DOM.
    pub pattern: String,
    /// Param names in the order they appear, e.g. ["id"] for the
    /// pattern "/san-pham/:id".
    pub param_names: Vec<String>,
}

/// The result of matching a route: the route itself + the real param
/// values taken from the URL, e.g. {"id": "42"}.
pub struct RouteMatch {
    pub route: RouteEntry,
    pub params: BTreeMap<String, String>,
}

thread_local! {
    static ROUTES: RefCell<Vec<RouteEntry>> = RefCell::new(Vec::new());
    static CURRENT_ROUTE: RefCell<Option<String>> = RefCell::new(None);
}

/// Registers a route into the registry - called by `boot_router()` for
/// every page found in the DOM (`[data-route]`) at startup.
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

/// Normalizes a path: stripping the query string, hash, and trailing
/// slash (except the root "/") - equivalent to the start of
/// `__matchRoute` in the JS version.
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

/// Matches `path` against a `pattern` (which may contain a ":name"
/// segment) segment by segment, split by "/". Returns a param map if
/// it matches.
fn match_pattern(pattern: &str, path: &str) -> Option<BTreeMap<String, String>> {
    // Keeps the normalized String in its own variable BEFORE splitting
    // - calling .split() directly on normalize_path(...)'s result (a
    // temporary value) would make the resulting Vec<&str> borrow from a
    // String dropped at the end of that statement, creating a dangling
    // reference (error E0716). Kept as a separate variable so the
    // String lives long enough until pattern_segs/path_segs are no
    // longer used.
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
            // A param segment - accepts any non-empty value, basic
            // percent-encoding decoding via js_sys if needed (skipped
            // for simplicity, most route params contain no special
            // characters).
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

/// Finds the route matching `path`, prioritized by registration order
/// (a route registered earlier takes priority if 2 patterns both match
/// the same path - rare with typical route design).
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

// ════════════════════════════════════════════════════════════
// 2. PAGE ACTIVATION / TEARDOWN
// ════════════════════════════════════════════════════════════

/// Hides every `.vb-page`, shows exactly the one matching
/// `route_match`, injects route params into state (so `$id` in
/// "trang(\"/san-pham/:id\")" resolves correctly), then re-binds that
/// subtree.
///
/// NOTE: calls `bind_subtree` AGAIN on every activation (even when
/// returning to a previously activated page) - this REGISTERS A NEW
/// SUBSCRIBER every time, matching the original JS version (its comment
/// says "safe since __subscribe creates a new binding itself"). This is
/// a MINOR memory leak if a user navigates back and forth through a
/// route many times - acceptable for a typical web app use case (not an
/// app running for hours without a refresh), but noted here clearly in
/// case optimization (unsubscribing the old binding before rebinding)
/// is needed later.
// BUG ALREADY FIXED: this function used to be SYNCHRONOUS - the
// execution order was (1) show the new page's div (display: ""), (2)
// bind_subtree() (each binding runs itself ONCE immediately during
// subscribe(), see state.rs::subscribe), THEN ONLY AFTER THAT
// (3) dispatch on_tai via spawn_local (fire-and-forget, async). With the
// very common "initialize state in on_tai" pattern (e.g.
// `on_tai { $dem = 0 }` instead of a static `state $dem = 0` declaration
// - needed when the initial value depends on logic/a route param/goi_api),
// this meant the page's FIRST RENDER always read state as Null (not yet
// assigned) - a REAL FOUC (not theoretical): if on_tai called goi_api()
// (a real network await), the window of "showing wrong content" could
// last hundreds of ms before flush() (run at the end of on_tai) made
// binding re-run with the correct value.
//
// The fix: activate_page is now an `async fn`, dispatching on_tai
// BEFORE bind_subtree/showing the new page - mirroring how
// VbRuntime::new() already received pageInitialState per route before
// boot_router() (see dom.rs). The new page stays display:none until
// on_tai finishes, avoiding a flash of content with empty state.
async fn activate_page(shared: &SharedState, route_match: &RouteMatch) {
    let doc = match web_sys::window().and_then(|w| w.document()) {
        Some(d) => d,
        None => return,
    };

    // Before hiding the OLD page (if any), reads its data-vb-on-huy and
    // dispatches it - this is exactly the right moment to run on_huy
    // (LEAVING that page), before its DOM gets display:none.
    if let Some(prev_pattern) = current_route() {
        if prev_pattern != route_match.route.pattern {
            dispatch_lifecycle_action(shared, &doc, &prev_pattern, "data-vb-on-huy").await;
        }
    }

    // Hides every page.
    if let Ok(list) = doc.query_selector_all(".vb-page") {
        for i in 0..list.length() {
            if let Some(node) = list.item(i) {
                if let Ok(el) = node.dyn_into::<web_sys::HtmlElement>() {
                    let _ = el.style().set_property("display", "none");
                }
            }
        }
    }

    // Finds this route's exact div via a CSS attribute selector.
    // Escapes '"' in the pattern (in case the route contains special
    // characters) to avoid breaking the selector's syntax.
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

    // Each route has its own reactive state. Clears the previous
    // page's subscribers/state first, then injects route params before
    // evaluating the new page's initial state. This order lets
    // `state $x = $id` use a route param.
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

    CURRENT_ROUTE.with(|cur| {
        *cur.borrow_mut() = Some(route_match.route.pattern.clone());
    });

    // Dispatches on_tai BEFORE bind_subtree/showing the page - every
    // "$x = ..." inside on_tai now finishes set_state()-ing before any
    // of this page's bindings run their first render.
    dispatch_lifecycle_action(shared, &doc, &route_match.route.pattern, "data-vb-on-tai").await;

    // Re-binds the entire subtree of the page just activated - runs
    // AFTER on_tai, so every binding's first render already sees the
    // correct state.
    dom::bind_subtree(shared, &target, None, None);

    // Only shows the page after it's been bound with the correct state
    // - avoiding a flash of wrong (Null/empty) content that then
    // "jumps" to the real value.
    //
    // BUG FIX (white screen on navigating to ANY non-"/" route):
    // this used to set the inline style to "" (empty string) to
    // "show" the page. Per the CSSStyleDeclaration spec, setting a
    // property to "" REMOVES that inline declaration entirely rather
    // than giving it a value - it does NOT force the element visible.
    // Once the inline override is gone, the element falls back to the
    // page's own stylesheet rule
    // (`.vb-page[data-route]:not([data-route="/"]) { display: none; }`,
    // see codegen/css.rs), which still says "none" for every route
    // other than "/". So every navigation to a non-"/" route left the
    // freshly-activated page's div permanently `display: none`,
    // invisibly overridden by the stylesheet the instant the inline
    // style was cleared - matching exactly the reported symptom (URL
    // updates correctly via pushState, no panic, no error logged,
    // content silently never appears). The "/" route never showed
    // this bug because the stylesheet rule already excludes it.
    // Fix: explicitly set "block" (matching how a bare `<div>`, which
    // is what a .vb-page compiles to, is displayed by default) instead
    // of clearing the property.
    if let Ok(html_el) = target.clone().dyn_into::<web_sys::HtmlElement>() {
        let _ = html_el.style().set_property("display", "block");
    }
}

/// Finds `.vb-page[data-route="<pattern>"]`, reads the `attr_name`
/// attribute (either "data-vb-on-tai" or "data-vb-on-huy"), and if it
/// has a valid action id, looks it up in the action registry and
/// dispatches it (async, fire-and-forget via spawn_local -
/// activate_page/navigate are synchronous functions, unable to .await
/// directly).
// BUG ALREADY FIXED: this function used to be synchronous, spawning its
// action via spawn_local() internally (fire-and-forget) - meaning the
// caller (activate_page) could NOT wait for on_tai to finish before
// bind_subtree/showing the page, regardless of where the caller placed
// this call in the code. It now returns a Future directly so
// activate_page can .await it - spawn_local (if a detached run is ever
// needed) is now the outer caller's job (navigate/boot_router), not
// this lower-level function's.
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

// ════════════════════════════════════════════════════════════
// 3. NAVIGATE
// ════════════════════════════════════════════════════════════

/// Navigates the SPA to `path` - updates the URL via the History API
/// (no reload), activating the correct page. Equivalent to
/// `__vbRouterNavigate`.
///
/// `replace`: uses `history.replaceState` instead of `pushState` - used
/// when falling back to the default route at boot (shouldn't create
/// another history entry for the app "auto-correcting a wrong URL"
/// itself).
/// `from_popstate`: if true, does NOT update the URL anymore (the URL
/// is already correct - this navigation came from the user clicking
/// Back/Forward).
pub fn navigate(shared: &SharedState, path: &str, replace: bool, from_popstate: bool) {
    let matched = match match_route(path) {
        Some(m) => m,
        None => {
            super::log::warn(&format!("[ViBao Router] No route found for \"{}\"", path));
            return;
        }
    };

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

    // activate_page is now async (running on_tai before bind/showing
    // the page - see the comment on activate_page) - navigate() still
    // needs a synchronous signature since it's called from a
    // click/popstate event closure, so spawn_local() happens at this
    // outer calling layer, not inside activate_page.
    let shared_clone = shared.clone();
    let matched_route = matched;
    wasm_bindgen_futures::spawn_local(async move {
        activate_page(&shared_clone, &matched_route).await;
    });

    if let Some(win) = web_sys::window() {
        win.scroll_to_with_x_and_y(0.0, 0.0);
    }
}

/// Returns the currently active route pattern, if any.
pub fn current_route() -> Option<String> {
    CURRENT_ROUTE.with(|cur| cur.borrow().clone())
}

// ════════════════════════════════════════════════════════════
// 4. LINK INTERCEPTION & POPSTATE
// ════════════════════════════════════════════════════════════

/// Intercepts a click on `<a data-vb-link="/path">` to navigate via
/// the router instead of letting the browser reload the whole page.
/// Uses event delegation on document (a single listener) - working
/// automatically even for links dynamically generated by a loop later,
/// with no need to re-bind on every render.
fn setup_link_interception(shared: &SharedState) {
    let doc = match web_sys::window().and_then(|w| w.document()) {
        Some(d) => d,
        None => return,
    };

    let shared_clone = shared.clone();
    let closure = Closure::<dyn FnMut(Event)>::new(move |evt: Event| {
        // Lets Ctrl/Cmd/middle-click open in a new tab normally -
        // only intervenes for a plain left click.
        if let Ok(mouse_evt) = evt.clone().dyn_into::<web_sys::MouseEvent>() {
            if mouse_evt.ctrl_key() || mouse_evt.meta_key() || mouse_evt.shift_key() || mouse_evt.button() == 1 {
                return;
            }
        }

        let Some(target) = evt.target() else { return };
        let Ok(mut el) = target.dyn_into::<web_sys::Element>() else { return };

        // Finds the nearest <a data-vb-link> ancestor from the target
        // upward (manual bubbling via the parent chain) - so clicking
        // an icon/span inside an <a> still works correctly.
        loop {
            if el.tag_name().eq_ignore_ascii_case("a") {
                if let Some(path) = el.get_attribute("data-vb-link") {
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

/// Handles the browser's Back/Forward buttons.
fn setup_popstate_handler(shared: &SharedState) {
    let Some(win) = web_sys::window() else { return };
    let shared_clone = shared.clone();

    let closure = Closure::<dyn FnMut(Event)>::new(move |_evt: Event| {
        let path = web_sys::window()
            .and_then(|w| w.location().pathname().ok())
            .unwrap_or_else(|| "/".to_string());
        navigate(&shared_clone, &path, false, true);
    });

    let target: web_sys::EventTarget = win.into();
    let _ = target.add_event_listener_with_callback("popstate", closure.as_ref().unchecked_ref());
    closure.forget();
}

// ════════════════════════════════════════════════════════════
// 5. BOOT
// ════════════════════════════════════════════════════════════

/// Starts the router - scans every `.vb-page[data-route]` already in
/// the DOM to register routes, attaches link interception + popstate,
/// then activates the correct page per the current URL. Called once
/// from `dom.rs::VbRuntime::new()` AFTER bind_subtree() has run for the
/// whole `<body>` - since boot_router() relies on `.vb-page` elements
/// already existing in the DOM.
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
            // activate_page is now async (running on_tai before
            // bind/showing the page for the first time - see the
            // comment on activate_page). boot_router() still keeps a
            // synchronous signature since it's called directly from
            // VbRuntime::new() (a wasm-bindgen constructor, which can't
            // be async), so spawn_local() happens here.
            let shared_clone = shared.clone();
            wasm_bindgen_futures::spawn_local(async move {
                activate_page(&shared_clone, &matched).await;
            });
        }
        None => {
            // No route matched - falls back to the first registered
            // route (usually "/") instead of leaving the page
            // completely blank.
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
        ROUTES.with(|r| r.borrow_mut().clear()); // cleans this thread's registry
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
