// ============================================================
// VIBAO COMPILER (Rust) — codegen/component.rs
// Registers + generates HTML/JS for a custom component call (@the).
// Equivalent to ComponentRegistry from 09-parser-component.ts + the
// "7. COMPONENT CALL GENERATION" / genComponentDef() section of
// 11-codegen-core.ts.
//
// DESIGN NOTE: the original TS version used `globalRegistry`, a global
// module-level variable (a singleton, living for the process's entire
// lifetime). In Rust, that kind of global mutable state would need
// unsafe or a Mutex - unnecessary and not idiomatic here, since the
// compiler always runs its entire lifetime within a single generate()
// call. ComponentRegistry is instead a field owned by Codegen (see
// mod.rs), passed as a parameter instead of being static - the same
// behavior, but safer, and easier to test in isolation (each test
// creates its own registry, with no state leaking between tests the
// way a global variable could cause).

use vibao_ast::{ComponentDef, ComponentCall};
use crate::codegen::css::{esc_attr, indent2};
use crate::codegen::element::ElementCodegenHost;
use crate::codegen::expr::{register_expr, json_string};
use std::collections::HashMap;

/// The registry of @the component definitions - looked up by name
/// whenever a component call is encountered in the AST.
#[derive(Debug, Clone, Default)]
pub struct ComponentRegistry {
    defs: HashMap<String, ComponentDef>,
    /// Records warnings raised during registration/lookup (a
    /// duplicate-named component definition, calling an undefined
    /// component) - the old TS version called console.warn directly;
    /// here they're collected instead, so the caller (e.g.
    /// error-handler.rs/CLI) decides how to display them, instead of
    /// printing straight to stderr from inside the registry's plain
    /// logic.
    pub warnings: Vec<String>,
}

impl ComponentRegistry {
    pub fn new() -> Self {
        ComponentRegistry::default()
    }

    /// Registers an @the definition - records a warning (no panic) if
    /// the name already exists, then OVERWRITES it with the new
    /// definition, matching `this.defs.set(...)`'s unconditional
    /// behavior in the old TS version.
    pub fn register(&mut self, def: ComponentDef) {
        if self.defs.contains_key(&def.name) {
            self.warnings.push(format!("[ViBao] Component \"@the {}\" is being redefined", def.name));
        }
        self.defs.insert(def.name.clone(), def);
    }

    pub fn get(&self, name: &str) -> Option<&ComponentDef> {
        self.defs.get(name)
    }

#[allow(dead_code)]
    pub fn has(&self, name: &str) -> bool {
        self.defs.contains_key(name)
    }

}

/// Looks up `node.name` in the registry and returns a cloned, owned
/// `ComponentDef` (or an "unknown component" warning) - a separate step
/// from `gen_component_call_with_def()` below on purpose. See the note
/// on that function for why the lookup and the children-generation step
/// must NOT share one call frame that holds `registry` borrowed.
pub fn resolve_component_def(node: &ComponentCall, registry: &ComponentRegistry) -> Result<ComponentDef, String> {
    match registry.get(&node.name) {
        Some(d) => Ok(d.clone()),
        None => Err(format!("[ViBao] Component \"{}\" has not been defined with @the", node.name)),
    }
}

/// Generates the mount HTML for a component call, given its ALREADY
/// RESOLVED (owned, cloned) definition.
///
/// FIXED (root cause of nested-component-call bugs, e.g. a component
/// calling ANOTHER component inside a neu/khong_thi branch — see
/// docs/VIBAO_SPEC.md section 9): this function used to take
/// `registry: &ComponentRegistry` directly, look up `node.name` in it
/// (`&ComponentDef`, a live borrow), and hold that borrow across the
/// `host.gen_children(&def.children)` call below - all within ONE
/// call frame. The caller in mod.rs (`Codegen::gen_child`) has to work
/// around Rust's borrow checker by temporarily `std::mem::take`-ing
/// `self.registry` out of `self` before calling in (see the note at
/// that call site) - which meant `self.registry` was EMPTY for the
/// ENTIRE duration of the old function, including while
/// `host.gen_children` recursed back into `self.gen_child()` for the
/// component's body. If that body contains ANOTHER component call
/// (exactly this fixture's TheThanhVien -> TheNhan case), that nested
/// call did the exact same `std::mem::take(&mut self.registry)` trick
/// again - but registry was ALREADY empty at that point (still held out
/// by the OUTER call, not yet restored) - so the nested lookup ALWAYS
/// reported "not been defined with @the", even though it plainly was.
/// This ONLY manifested for a call nested inside another component's
/// body (an if/switch/loop branch, or a plain nested Element), never
/// for a top-level call directly inside a page - matching exactly what
/// the failing test observed (TheThanhVien mounted fine, TheNhan - only
/// ever called FROM INSIDE TheThanhVien's body - never appeared at
/// all, regardless of which if/else branch was taken).
///
/// The fix: split the old single function into two steps
/// (resolve_component_def() above + this function), so the caller in
/// mod.rs can restore `self.registry` in between - BEFORE generating
/// children, not after. By the time a nested component call is reached
/// during `host.gen_children`, the registry is already back in place
/// and resolves normally, however many levels deep the nesting goes.
pub fn gen_component_call_with_def(
    node: &ComponentCall,
    def: &ComponentDef,
    host: &mut dyn ElementCodegenHost,
) -> String {
    let id = host.next_id(&node.name);
    let children_html = host.gen_children(&def.children);

    // FIXED (a real bug found through an actual test build and
    // cross-checking against the runtime): this line used to call
    // `host.add_js("__vb.mountComponent(...)")` - but NO `window.__vb`
    // object was ever created anywhere (the WASM runtime binds
    // everything itself through bind_subtree(), scanning data-vb-*
    // attributes, and never exposes a separate __vb.* JS API) - so this
    // call ALWAYS crashed the instant the script ran: "ReferenceError:
    // __vb is not defined". This was the actual root cause of @the
    // components never getting their props hydrated in a real browser
    // (not just the earlier-fixed reset_page() bug - that bug only
    // affected HOW MANY TIMES this call was generated, while the call
    // itself was always broken regardless).
    //
    // The fix: generate no JS at all - instead REGISTER each prop value
    // into the expr_registry (exactly like how data-vb-text/data-vb-if
    // register their expressions), embedding {key: exprId} into
    // data-vb-props. The runtime (a new `bind_component` in
    // vibao-runtime/dom.rs) scans for data-vb-component itself during
    // bind_subtree(), calls register_props(id, ...) itself with a
    // getter closure for each key (reading via eval_expr_id(exprId)),
    // then binds the child subtree RIGHT WITHIN that component's scope
    // (push/pop component_scope) - no separate JS "mount call" needed
    // at all, consistent with how if/loop/switch already work.
    let props_json = build_props_json(&node.props);

    let html = format!(
        "<div id=\"{}\" data-vb-component=\"{}\" data-vb-props=\"{}\">\n{}\n</div>",
        id,
        node.name,
        esc_attr(&props_json),
        indent2(&children_html)
    );

    html
}

/// Convenience wrapper combining resolve_component_def() +
/// gen_component_call_with_def() into the old single-call shape
/// (returns `(html, warning)`) - kept for callers (and existing unit
/// tests below) that generate a single, non-recursive component call
/// against an already-complete, borrowable registry. NOT used by the
/// real compiler pipeline anymore (see mod.rs::gen_child's
/// Child::ComponentCall arm) specifically because holding `registry`
/// borrowed across `gen_component_call_with_def`'s call into
/// `host.gen_children()` is exactly the pattern that breaks nested
/// component calls when `host` is a type (like the real `Codegen`) that
/// itself owns the registry and needs to reclaim it during that same
/// recursive call - see the long note on gen_component_call_with_def()
/// above for the full explanation. Safe to use here because `FakeHost`
/// in these tests doesn't own a ComponentRegistry at all, so there's no
/// re-entrant borrow conflict to worry about.
pub fn gen_component_call(
    node: &ComponentCall,
    registry: &ComponentRegistry,
    host: &mut dyn ElementCodegenHost,
) -> (String, Option<String>) {
    match resolve_component_def(node, registry) {
        Ok(def) => (gen_component_call_with_def(node, &def, host), None),
        Err(warning) => (format!("<!-- unknown component: {} -->", node.name), Some(warning)),
    }
}

/// Builds the JSON object string {key: exprId, ...} from a PropsMap -
/// each value is a NUMBER (an index into expr_registry), not a JS
/// string like the old design. The runtime deserializes this JSON
/// itself during bind_component(), then looks up expr_registry via
/// eval_expr_id(exprId) to get the real value whenever the relevant
/// state changes - consistent with data-vb-text/data-vb-if/
/// data-vb-attr-* (all of which use a "numeric exprId" rather than a JS
/// string).
fn build_props_json(props: &vibao_ast::PropsMap) -> String {
    let entries = props
        .iter()
        .map(|(k, v)| {
            let expr_id = register_expr(v.clone());
            format!("{}:{}", json_string(k), expr_id)
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("{{{}}}", entries)
}

// REMOVED: `gen_component_def()` used to be here, generating
// `__vb.defineComponent('Ten', function(__props) {...})`. Removed
// entirely (not just stopped calling) since this function's ONLY reason
// to exist was generating a JS string calling `__vb.*` - an API that
// doesn't exist in the runtime and would always crash. Keeping this
// function around with no caller would mislead a future reader (making
// it look like it's still a working part of the system). See
// `gen_component_call()` above and `bind_component()`
// (vibao-runtime/src/runtime/dom.rs) for how a component is actually
// "defined" in the new design: it isn't - every component call already
// has its full HTML rendered at build time, and the runtime only needs
// to register props (via expr_registry) and bind the child subtree.

// ════════════════════════════════════════════════════════════
// UNIT TESTS
// ════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use vibao_ast::{Child, Expr, ParamDef, DataType, Pos};

    fn p() -> Pos {
        Pos { line: 1, column: 1 }
    }

    struct FakeHost {
        counter: u32,
        js: Vec<String>,
    }

    impl FakeHost {
        fn new() -> Self {
            FakeHost { counter: 0, js: vec![] }
        }
    }

    impl ElementCodegenHost for FakeHost {
        fn next_id(&mut self, tag: &str) -> String {
            self.counter += 1;
            format!("vb-{}-{}", tag, self.counter)
        }
        fn gen_children(&mut self, children: &[Child]) -> String {
            children
                .iter()
                .map(|c| match c {
                    Child::Element(el) => crate::codegen::element::gen_element(el, self),
                    _ => String::new(),
                })
                .collect::<Vec<_>>()
                .join("\n")
        }
        fn add_css(&mut self, _code: &str) {}
        fn add_media_query(&mut self, _code: &str) {}
    }

    #[test]
    fn test_register_and_get() {
        let mut registry = ComponentRegistry::new();
        let def = ComponentDef { name: "The_Card".to_string(), params: vec![], children: vec![], pos: p() };
        registry.register(def);
        assert!(registry.has("The_Card"));
        assert!(registry.get("The_Card").is_some());
    }

    #[test]
    fn test_register_duplicate_warns_but_overwrites() {
        let mut registry = ComponentRegistry::new();
        registry.register(ComponentDef { name: "X".to_string(), params: vec![], children: vec![], pos: p() });
        registry.register(ComponentDef {
            name: "X".to_string(),
            params: vec![ParamDef { name: "a".to_string(), data_type: DataType::Chuoi, default_value: None, pos: p() }],
            children: vec![],
            pos: p(),
        });
        assert_eq!(registry.warnings.len(), 1);
        assert_eq!(registry.get("X").unwrap().params.len(), 1);
    }

    #[test]
    fn test_gen_component_call_unknown_returns_warning() {
        let registry = ComponentRegistry::new();
        let mut host = FakeHost::new();
        let call = ComponentCall { name: "KhongTonTai".to_string(), props: vec![], children: vec![], pos: p() };
        let (html, warning) = gen_component_call(&call, &registry, &mut host);
        assert!(html.contains("unknown component: KhongTonTai"));
        assert!(warning.is_some());
    }

    #[test]
    fn test_gen_component_call_known_generates_props_expr_id_no_dead_js() {
        // FIXED: this test used to assert `host.js.len() == 1` and
        // `host.js[0].contains("__vb.mountComponent")` - meaning it
        // confirmed (and inadvertently locked in) the exact BUG
        // behavior: generating a JS call to an API that doesn't exist in
        // the runtime (`window.__vb`), always crashing with
        // "ReferenceError: __vb is not defined" the instant the script
        // ran. gen_component_call now generates NO JS at all (host.js
        // must be empty) - props are registered directly into
        // expr_registry via register_expr(), embedding the exprId
        // (a number) into data-vb-props as JSON {key: exprId}. The
        // runtime (bind_component(), see
        // vibao-runtime/src/runtime/dom.rs) reads this JSON itself while
        // binding.
        crate::codegen::expr::take_expr_registry(); // clean the global registry before the test, avoiding leftover ids from another test that ran earlier (thread_local is shared within a single cargo test run)
        let mut registry = ComponentRegistry::new();
        registry.register(ComponentDef { name: "The_Card".to_string(), params: vec![], children: vec![], pos: p() });
        let mut host = FakeHost::new();
        let call = ComponentCall {
            name: "The_Card".to_string(),
            props: vec![("tieu_de".to_string(), Expr::literal_str("Xin chào", p()))],
            children: vec![],
            pos: p(),
        };
        let (html, warning) = gen_component_call(&call, &registry, &mut host);
        assert!(warning.is_none());
        assert!(html.contains("data-vb-component=\"The_Card\""));
        // NO JS is generated anymore - this is exactly the fix.
        assert!(host.js.is_empty(), "gen_component_call must not generate any JS, got: {:?}", host.js);
        // data-vb-props must contain the JSON {"tieu_de": <number>}
        // (with double-quotes HTML-escaped to &quot;) - the exprId is a
        // NUMBER, not a JS string like the old design.
        assert!(
            html.contains("data-vb-props=\"{&quot;tieu_de&quot;:0}\""),
            "data-vb-props must contain {{\"tieu_de\": <numeric exprId>}}, actual HTML:\n{}",
            html
        );
        // Confirms the correct Expr was registered into the registry at that id.
        let registry_snapshot = crate::codegen::expr::take_expr_registry();
        assert_eq!(registry_snapshot.len(), 1);
        assert!(matches!(&registry_snapshot[0], Expr::Literal(vibao_ast::LiteralValue::Str(s), _) if s == "Xin chào"));
    }

    // REMOVED: `test_gen_component_def_shape` used to be here, testing
    // the `gen_component_def()` function - that function was removed
    // entirely (see the note at its old location in this file) since it
    // only ever generated a call to the `__vb.defineComponent(...)` API,
    // which doesn't exist in the runtime.
}
