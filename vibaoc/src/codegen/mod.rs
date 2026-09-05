// ============================================================
// VIBAO COMPILER (Rust) — codegen/mod.rs
// The assembly point for all of codegen: CodegenContext (state
// accumulated for CSS/JS/state throughout generation), Codegen (the
// main struct, implementing ElementCodegenHost so element.rs/control.rs
// can call back into genChildren recursively), and the generate()
// entry point. Equivalent to the header + sections "1/2/3/4/8" of
// 11-codegen-core.ts.
// ============================================================

pub mod action;
pub mod component;
pub mod control;
pub mod css;
pub mod element;
pub mod expr;
pub mod layout;
pub mod props;

use vibao_ast::{App, Child, ColorValue, Expr, Page, Program};
use crate::codegen::component::ComponentRegistry;
use crate::codegen::css::BASE_CSS;
use crate::codegen::element::{tag_to_class_name, ElementCodegenHost};
use std::collections::HashMap;

// ════════════════════════════════════════════════════════════
// OPTIONS
// ════════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
pub struct CodegenOptions {
    pub base_url: String,
}

impl Default for CodegenOptions {
    fn default() -> Self {
        CodegenOptions {
            base_url: "/".to_string(),
        }
    }
}

// ════════════════════════════════════════════════════════════
// CODEGEN CONTEXT — state accumulated during code generation
// ════════════════════════════════════════════════════════════

pub struct CodegenContext {
    pub options: CodegenOptions,
    id_counter: u32,
    css_blocks: Vec<String>,
    js_blocks: Vec<String>,
    /// A REAL BUG THAT WAS FIXED: unlike `js_blocks` (cleared by
    /// `reset_page()` after EVERY page), this buffer accumulates the
    /// mount JS (__vb.mountComponent(...)) for ALL pages, throughout the
    /// entire generate() process. `gen_app_js()` used to read
    /// `js_blocks` directly AFTER the `for page in &app.pages` loop, so
    /// only the LAST page's generated JS survived (every earlier page
    /// had already been cleared by `reset_page()` before `gen_app_js`
    /// was ever called). The consequence: components (@the) on every
    /// page except the last never got `mountComponent` called for them -
    /// their props were never hydrated at runtime, causing every `$bien`
    /// expression inside them to render as empty (a blank span), even
    /// though the static HTML was still correct. `add_js` now writes
    /// into BOTH buffers; `js_blocks` can still be used for page-local
    /// state/scope if needed, while `get_js()` (used by gen_app_js)
    /// reads from `all_js_blocks` - which `reset_page()` never touches.
    all_js_blocks: Vec<String>,
    media_queries: Vec<String>,
    /// Uses Vec<(String, Expr)> instead of a HashMap to preserve the
    /// declaration order of state - important when printing to JS
    /// (declaration order affects readability, and in some scripting
    /// languages could affect execution if a later variable depends on
    /// an earlier one - though JS hoisting makes this non-mandatory,
    /// preserving order is still a safer choice than a HashMap).
    state_vars: Vec<(String, Expr)>,
    // This field's data now reaches the runtime via `gen_app_js()`
    // below, which serializes it into the "globalVars" field of
    // optsJson (see BUG-19 in RUNTIME_BOUNDARY_BUGS.md for the full fix
    // history: a variable declared WITHOUT the `state` keyword -
    // "$ten = gia_tri" - used to be recorded here but never actually
    // reached the runtime, even though the runtime already had a
    // correct mechanism to receive it - only the wiring between the two
    // sides was missing).
    global_vars: Vec<(String, Expr)>,
    /// The `<template>` blocks "hoisted" out of a loop/switch - see
    /// add_hoisted_template in element.rs for the full reasoning.
    /// Belongs to a specific PAGE (only needs to exist while that page
    /// is active) - cleared in reset_page(), not preserved across pages
    /// like css_blocks (CSS is shared across all pages via a single
    /// style.css file).
    hoisted_templates: Vec<String>,
}

impl CodegenContext {
    pub fn new(options: CodegenOptions) -> Self {
        CodegenContext {
            options,
            id_counter: 0,
            css_blocks: Vec::new(),
            js_blocks: Vec::new(),
            all_js_blocks: Vec::new(),
            media_queries: Vec::new(),
            state_vars: Vec::new(),
            global_vars: Vec::new(),
            hoisted_templates: Vec::new(),
        }
    }

    pub fn next_id(&mut self, tag: &str) -> String {
        self.id_counter += 1;
        format!("vb-{}-{}", tag_to_class_name(tag), self.id_counter)
    }

    pub fn add_css(&mut self, block: &str) {
        if !block.trim().is_empty() {
            self.css_blocks.push(block.to_string());
        }
    }

    pub fn add_media_query(&mut self, mq: &str) {
        if !mq.trim().is_empty() {
            self.media_queries.push(mq.to_string());
        }
    }

    pub fn get_css(&self) -> String {
        self.css_blocks.iter().chain(self.media_queries.iter()).cloned().collect::<Vec<_>>().join("\n\n")
    }

#[allow(dead_code)]
    pub fn add_js(&mut self, block: &str) {
        if !block.trim().is_empty() {
            self.js_blocks.push(block.to_string());
            // Also writes into the app-wide accumulator buffer - NOT
            // cleared by reset_page(), see the note at the
            // all_js_blocks field declaration above.
            self.all_js_blocks.push(block.to_string());
        }
    }

    /// Returns the accumulated mount JS for the ENTIRE app (every
    /// page), used by gen_app_js() after the `for page in &app.pages`
    /// loop has finished - reads from `all_js_blocks` (never cleared by
    /// reset_page()), see the note at the field declaration.
    pub fn get_js(&self) -> String {
        self.all_js_blocks.join("\n\n")
    }

    /// Registers a `<template>` block to be printed at the top level of
    /// the page (see add_hoisted_template in element.rs - the full
    /// reasoning for why a template must NOT be embedded inline at the
    /// position it was generated).
    pub fn add_hoisted_template(&mut self, html: String) {
        if !html.trim().is_empty() {
            self.hoisted_templates.push(html);
        }
    }

    /// Gets every hoisted template for the CURRENT page (cleared by
    /// reset_page() when moving to another page) - used when assembling
    /// the page's final HTML, printed as top-level siblings at the end
    /// of `.vb-page`, not nested inside any other template/container.
    pub fn get_hoisted_templates(&self) -> String {
        self.hoisted_templates.join("\n")
    }

    pub fn add_state_var(&mut self, name: &str, val: &Expr) {
        self.state_vars.push((name.to_string(), val.clone()));
    }

    pub fn add_global_var(&mut self, name: &str, val: &Expr) {
        self.global_vars.push((name.to_string(), val.clone()));
    }

    pub fn get_state_vars(&self) -> &[(String, Expr)] {
        &self.state_vars
    }

    pub fn get_global_vars(&self) -> &[(String, Expr)] {
        &self.global_vars
    }

    /// Resets each page's own state (JS blocks, state vars) before
    /// starting to generate a new page - global CSS/JS blocks (BASE_CSS,
    /// global vars) are NOT reset.
    ///
    /// IMPORTANT: `id_counter` is deliberately NOT reset here (already
    /// fixed - `self.id_counter = 0` used to be here; this was a REAL
    /// BUG found through a test build of a multi-page app: since the
    /// current build architecture is a REAL SPA (every page merged into
    /// a single index.html, see main.rs::cmd_build), resetting
    /// id_counter to 0 per page caused EVERY page to generate exactly
    /// the same ids (e.g. "vb-box-1" appearing on both the "/" page and
    /// the "/gioi-thieu" page within the SAME DOM). The consequence:
    /// every document.getElementById()/querySelector("#...") only ever
    /// matched the element from the FIRST page generated, silently
    /// "breaking" the binding/style of every subsequent page (no error
    /// raised, just running incorrectly). id_counter now counts
    /// CONTINUOUSLY throughout the entire app, guaranteeing globally
    /// unique ids - a hard HTML requirement once multiple "pages"
    /// coexist within one real DOM.
    pub fn reset_page(&mut self) {
        self.js_blocks.clear();
        self.state_vars.clear();
        self.hoisted_templates.clear();
    }
}

// ════════════════════════════════════════════════════════════
// OUTPUT — the final result of generate()
// ════════════════════════════════════════════════════════════

pub struct CodegenOutput {
    /// The HTML for the "/" page (or the first page if there's no "/").
    pub html: String,
    pub css: String,
    pub js: String,
    /// The HTML for EACH page, by route - used for a multi-page build.
    pub pages: HashMap<String, String>,
    /// Warnings accumulated during generation (a duplicate-named
    /// component, calling an undefined component...) - the caller
    /// (CLI/bundler) decides how to display them. Unlike the old TS
    /// version (calling console.warn directly), see the note in
    /// component.rs.
    pub warnings: Vec<String>,
}

// ════════════════════════════════════════════════════════════
// CODEGEN — the main struct
// ════════════════════════════════════════════════════════════

pub struct Codegen {
    pub ctx: CodegenContext,
    registry: ComponentRegistry,
}

impl Codegen {
    pub fn new(options: CodegenOptions) -> Self {
        Codegen { ctx: CodegenContext::new(options), registry: ComponentRegistry::new() }
    }

    /// The main entry point - compiles a full Program into HTML/CSS/JS.
    /// Equivalent to Codegen.generate().
    pub fn generate(&mut self, program: &Program) -> CodegenOutput {
        let app = &program.app;

        for def in &app.components {
            self.registry.register(def.clone());
        }
        // app.variables (variables declared at the `ung_dung` level, not
        // `state`) go directly into page_initial_state_json below (using
        // the `app.variables` parameter directly, not through
        // ctx.global_vars) - add_global_var() is no longer needed here
        // (it used to be, DUPLICATING the path above, and was removed -
        // see the BUG-19 fix, RUNTIME_BOUNDARY_BUGS.md). ctx.global_vars
        // now ONLY receives data from Child::VarDecl (a variable "bien
        // $ten = ..." nested INSIDE a page/element) - this path is now
        // WIRED into its own "globalVars" JSON field, see gen_app_js().
        self.ctx.add_css(BASE_CSS);

        // A REAL BUG THAT WAS FIXED (found through an actual test build:
        // a state declared with "state $x = ..." inside `trang(...)`
        // displayed WRONG, and every time it accumulated, its value got
        // CONCATENATED AS A STRING instead of added numerically, e.g.
        // clicking "+4" repeatedly produced 4 -> 44 -> 444 instead of 8
        // -> 12 -> 16): `gen_app_js` (below) used to only read
        // `app.variables` (state declared at the `ung_dung` level) to
        // build "pageInitialState" embedded in optsJson - COMPLETELY
        // MISSING `page.states` (state declared at the `trang` level,
        // much more common in practice since most state only makes
        // sense within one specific page). `gen_page()` STILL called
        // `self.ctx.add_state_var(...)` for each page-level state
        // (correctly storing it in `ctx.state_vars`), but NOBODY read
        // `ctx.get_state_vars()` back before the next page's
        // `reset_page()` call wiped it out - the data was lost
        // completely, leaving no trace to diagnose by reading the code
        // alone (only surfacing when ACTUALLY RUN and observing the
        // runtime value). The chain of consequences: that state never
        // existed in the VbRuntime state map -> read back as Null ->
        // `eval_add(Null, 4)` (see expr_eval.rs) fell into the "not
        // both operands are Num" branch -> STRING CONCATENATION instead
        // of numeric addition (this by itself isn't a bug - it's
        // correct JS-like "undefined + 4" semantics, but this SURFACE
        // SYMPTOM easily looked like a bug in the Add operator, while
        // the REAL bug was that the state was never initialized in the
        // first place).
        //
        // The fix: accumulate `ctx.get_state_vars()` for EACH PAGE into
        // `page_initial_state_vars` RIGHT AFTER gen_page(page), BEFORE
        // the next loop iteration calls reset_page() (clearing ctx's
        // state_vars). Each page's state is placed into pageInitialState
        // because the current build architecture is a REAL SPA (every
        // page coexists in one DOM / one VbRuntime initialization - see
        // the note on reset_page() about not resetting id_counter for
        // the same reason) - a page's state needs the correct initial
        // value EVEN WHILE that page isn't active yet, since its binding
        // already exists in the DOM, waiting for the router to activate
        // it.
        //
        // A LIMITATION WORTH NOTING (not a new bug, but an inherent
        // consequence of the existing "1 global state namespace for the
        // whole SPA" design - not expanding the scope of this fix to
        // solve it here): if 2 DIFFERENT PAGES accidentally declare the
        // SAME state name (e.g. both "/" and "/gioi-thieu" have
        // "state $dem = ..."), pageInitialState will only keep the
        // value from whichever was declared LAST (whichever page comes
        // later in the .vbao file overwrites the earlier one, since
        // they share the same key in the global state map) - those 2
        // pages will actually SHARE one runtime value, not be
        // independent the way a .vbao author might assume. To make
        // state truly independent per page, use unique variable names
        // (e.g. $dem_trang_chu vs $dem_gioi_thieu).
        let mut page_initial_state_vars: Vec<(String, Vec<(String, Expr)>)> = Vec::new();

        let mut pages: HashMap<String, String> = HashMap::new();
        for page in &app.pages {
            self.ctx.reset_page();
            let html = self.gen_page(page);
            page_initial_state_vars.push((page.route.clone(), self.ctx.get_state_vars().iter().cloned().collect()));
            pages.insert(page.route.clone(), html);
        }

        let html = pages.get("/").cloned().unwrap_or_else(|| pages.values().next().cloned().unwrap_or_default());
        let js = self.gen_app_js(app, &page_initial_state_vars);
        let warnings = self.registry.warnings.clone();

        CodegenOutput { html, css: self.ctx.get_css(), js, pages, warnings }
    }

    // ════════════════════════════════════════════════════════
    // PAGE GENERATION
    // ════════════════════════════════════════════════════════

    fn gen_page(&mut self, page: &Page) -> String {
        for s in &page.states {
            self.ctx.add_state_var(&s.name, &s.value);
        }
        // on_tai/on_huy now go through the action registry (pure Rust),
        // NO LONGER generating JS via compile_page_load() (the old
        // architecture) - the 2 action ids (if present) are embedded
        // directly on the .vb-page div itself as
        // data-vb-on-tai/data-vb-on-huy; the runtime's router.rs reads
        // them itself and dispatches at the correct moment when the
        // page is activated/left.
        let (id_on_tai, id_on_huy) = action::compile_page_load_registry(&page.events);
        let lifecycle_attrs = [
            id_on_tai.map(|id| format!("data-vb-on-tai=\"{}\"", id)),
            id_on_huy.map(|id| format!("data-vb-on-huy=\"{}\"", id)),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" ");

        let children_html = page
            .children
            .iter()
            .map(|c| self.gen_child(c))
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("\n");

        // BUG ALREADY FIXED: every <template> generated by a NESTED
        // loop/switch inside children_html has already been "hoisted"
        // out of its original position via add_hoisted_template() (see
        // the full reasoning in element.rs) - collected here, appended
        // AFTER children_html, as a top-level SIBLING inside .vb-page,
        // never nested inside any other template/container no matter
        // how deeply nested it was in the original AST tree.
        let hoisted_templates = self.ctx.get_hoisted_templates();
        let body_content = if hoisted_templates.trim().is_empty() {
            children_html
        } else {
            format!("{}\n{}", children_html, hoisted_templates)
        };

        let bg_style = page
            .mau_nen
            .as_ref()
            .map(|cv| format!("style=\"background-color:{}\"", resolve_page_bg_color(cv)))
            .unwrap_or_default();

        format!(
            "<div class=\"vb-page\" data-route=\"{}\" {} {}>\n{}\n</div>",
            page.route,
            bg_style,
            lifecycle_attrs,
            css::indent2(&body_content)
        )
    }

    // ════════════════════════════════════════════════════════
    // CHILD DISPATCH
    // ════════════════════════════════════════════════════════

    /// Generates the HTML for a single Child - dispatching by variant.
    /// Equivalent to genChild(). StateDecl/VarDecl/PageEvent generate
    /// no HTML (they only have a side effect on ctx), returning "" so
    /// the caller filters them out.
    pub fn gen_child(&mut self, child: &Child) -> String {
        match child {
            Child::StateDecl(s) => {
                self.ctx.add_state_var(&s.name, &s.value);
                String::new()
            }
            Child::VarDecl(v) => {
                self.ctx.add_global_var(&v.name, &v.value);
                String::new()
            }
            // Child::PageEvent is NEVER actually created by the parser -
            // the parser always pushes on_tai/on_huy directly into
            // `Page::events` (its own field, see
            // parser/app.rs::parse_page), never into `children`. This
            // branch only exists because Child is an enum broad enough
            // to theoretically hold a PageEvent (the old design, before
            // Page had its own `events` field). This branch used to call
            // action::compile_page_load() (the old JS approach, now
            // REMOVED entirely - see action.rs) - since no path from a
            // real .vibao file can ever reach this branch, removing that
            // call doesn't change the compiler's observable behavior at
            // all, it just cleans up a reference to a function that no
            // longer exists.
            Child::PageEvent(_) => String::new(),
            Child::If(node) => control::gen_if(node, self),
            Child::Switch(node) => control::gen_switch(node, self),
            Child::Loop(node) => control::gen_loop(node, self),
            Child::Element(el) => element::gen_element(el, self),
            Child::ComponentCall(call) => {
                // BORROW CHECKER NOTE + FIXED BUG (nested component
                // calls, e.g. a component calling ANOTHER component from
                // inside a neu/khong_thi branch - see the detailed note
                // on component::gen_component_call_with_def()): calling
                // component::gen_component_call(call, &self.registry, self)
                // directly isn't possible - Rust would reject it since
                // it would borrow self.registry immutably AND self
                // mutably at the same time (because the host: &mut dyn
                // ElementCodegenHost parameter needs &mut self).
                //
                // The OLD workaround temporarily `std::mem::take`-ed the
                // registry out of self, then called a single function
                // that BOTH looked up the definition AND generated its
                // children - meaning self.registry stayed EMPTY for the
                // full duration of children-generation. That's fine for
                // a component whose body has no further component calls,
                // but breaks the instant the body itself calls another
                // component (a nested lookup would find nothing, since
                // the registry was still taken out by the OUTER call).
                //
                // The fix: split the "take registry, look up, restore
                // registry" step from the "generate children" step -
                // resolve_component_def() only needs registry for a
                // quick clone-and-return, so self.registry can be
                // restored to a real, usable state IMMEDIATELY after
                // (and well before) host.gen_children() ever recurses
                // back into self.gen_child() for the component's body.
                // A nested component call inside that body now finds the
                // registry fully populated again, no matter how many
                // levels of nesting are involved.
                let registry = std::mem::take(&mut self.registry);
                let resolved = component::resolve_component_def(call, &registry);
                self.registry = registry;
                match resolved {
                    Ok(def) => component::gen_component_call_with_def(call, &def, self),
                    Err(warning) => {
                        self.registry.warnings.push(warning);
                        format!("<!-- unknown component: {} -->", call.name)
                    }
                }
            }
        }
    }

    fn gen_children_internal(&mut self, children: &[Child]) -> String {
        children.iter().map(|c| self.gen_child(c)).filter(|s| !s.is_empty()).collect::<Vec<_>>().join("\n")
    }

    // ════════════════════════════════════════════════════════
    // APP JS GENERATION
    // ════════════════════════════════════════════════════════

    fn gen_app_js(&self, _app: &App, page_initial_state_vars: &[(String, Vec<(String, Expr)>)]) -> String {
        // REMOVED (a real, serious bug - found through careful code
        // reading, not just a test build): this used to have code
        // generating lines like `const <name> = <expr_to_js_default(value)>;`
        // for EVERY global variable (app.variables) - with the
        // stated intent that it was "ONLY for display/documentation
        // when reading app.js by eye, with NO connection to VbRuntime
        // (WASM)". But `expr_to_js_default()` (and the entirety of
        // expr_to_js()/map_call_fn()/map_color_fn() that it calls into)
        // belonged to the OLD JS architecture - generating calls to
        // `__fmt.giaTien(...)`/`__fmt.rutGon(...)`/
        // `__color.trongSuot(...)`/... - these JS objects do NOT EXIST
        // in the current runtime (the WASM runtime computes these
        // functions itself in pure Rust, see
        // vibao-runtime::expr_eval). If ANY global variable (e.g.
        // `bien $x = gia_tien(1000)`) used one of these functions, the
        // generated `const` line would throw
        // `ReferenceError: __fmt is not defined` on the VERY FIRST LINE
        // of the IIFE - stopping execution BEFORE VbRuntime could even
        // be initialized (the code further below, in `bootstrap_js`) -
        // meaning the ENTIRE app rendered a blank screen, not just that
        // one variable.
        //
        // This code was completely UNNECESSARY functionally: the real
        // value of every global variable (app.variables) was already
        // correctly folded into "pageInitialState" (the JSON embedded in
        // optsJson below) - WASM reads that field during boot() and
        // calls the real set_state(), which IS the actual source of
        // truth (see the note in generate()/gen_page() about the fix
        // related to pageInitialState). So this
        // display-only "Global vars" block was removed entirely instead
        // of trying to patch it safely - it added no functionality the
        // build didn't already have, only extra risk.

        // REMOVED: this used to have `component_defs`, generated via
        // `component::gen_component_def()` - each @the component
        // generated a line
        // `__vb.defineComponent('Ten', function(__props) {...})`.
        // Removed entirely for two reasons: (1) `__vb` doesn't exist in
        // the runtime - every such call always crashed the instant the
        // script ran; (2) with the NEW component design (props are now
        // registered through expr_registry, read by bind_component() in
        // vibao-runtime/dom.rs - see
        // component.rs::gen_component_call), there's no longer any
        // concept of "defining a component render function in JS" at
        // all - the HTML for every component call is already generated
        // statically, in full, at build time. No "component declaration"
        // step is needed at runtime whatsoever.

        // Retrieves every Expr registered via register_expr() (called
        // directly, scattered across
        // props.rs/element.rs/control.rs/component.rs) throughout this
        // build pass, serializing it to JSON to embed into __vb.boot().
        // WASM reads this array during startup, deserializes it back
        // into a real Vec<Expr>, and the Rust evaluator (vibao-runtime)
        // computes values using the exact index used in the "data-vb-*"
        // attributes (e.g. data-vb-text, data-vb-if, data-vb-bind-*) in
        // the HTML output. The registry ALWAYS has data on most real
        // pages (any prop/if/loop/text using a dynamic expression calls
        // register_expr) - it is NOT empty as an old note once
        // described (that note was talking about
        // `expr_to_js_registry()`, a helper function that was removed
        // since it never had a caller - completely different from
        // `register_expr()` here, which is called directly and very
        // extensively).
        let expr_registry = expr::take_expr_registry();
        let expr_registry_json = serde_json::to_string(&expr_registry)
            .unwrap_or_else(|_| "[]".to_string());

        // A mirror of expr_registry - each element is a Vec<Action> (one
        // event handler's body), registered via
        // action::compile_event_handler_registry() (called from
        // element.rs when generating HTML for an element with an
        // event). WASM (action_registry.rs) reads this array during
        // boot, looking things up by the id embedded in
        // "data-vb-on-click='<id>'" in the HTML.
        let action_registry = action::take_action_registry();
        let action_registry_json = serde_json::to_string(&action_registry)
            .unwrap_or_else(|_| "[]".to_string());

        // BUG ALREADY FIXED: "state $x = <value>" used to only generate
        // a single JS line "const x = <value>;" (see vars_js below) - a
        // local JS variable, with NO connection whatsoever to
        // VbRuntime. WASM always booted with COMPLETELY EMPTY state,
        // losing EVERY declared initial value entirely (reading back
        // VbValue::Null instead of the real declared value). This is
        // now serialized as (variable name, Expr) into JSON, embedded
        // into optsJson under the "pageInitialState" field -
        // vibao-runtime reads this field during boot() and eval()s each
        // Expr then calls the real set_state() (see the corresponding
        // fix in vibao-runtime/src/runtime/dom.rs). Reactive state is
        // stored BY ROUTE so the router can correctly reset a page's
        // state when the SPA navigates. State from every page is no
        // longer merged into a single global namespace.
        let page_initial_state_json = serde_json::to_string(&page_initial_state_vars)
            .unwrap_or_else(|_| "[]".to_string());

        // BUG-19 ALREADY FIXED (see RUNTIME_BOUNDARY_BUGS.md):
        // a variable declared WITHOUT the `state` keyword (the syntax
        // "$ten = gia_tri", Child::VarDecl - unlike StateDecl above) was
        // correctly recorded by ctx.global_vars but PREVIOUSLY had no
        // path to serialize and send it to the runtime - even though
        // the runtime ALREADY HAD the correct mechanism to receive it
        // (State::init_global_vars(), a `vars` field kept separate from
        // `state` - not reactive, doesn't trigger a re-render, matching
        // the "local constant" meaning of the non-`state` syntax). This
        // is now serialized separately, embedded into optsJson under
        // the "globalVars" field - vibao-runtime reads this field during
        // boot() and evals + calls init_global_vars() itself (see the
        // corresponding fix in vibao-runtime/src/runtime/dom.rs).
        //
        // A SEPARATE "globalVars" field was DELIBERATELY kept apart
        // instead of merging it into "pageInitialState" - "pageInitialState"
        // and "globalVars" carry DIFFERENT RUNTIME MEANINGS (reactive vs.
        // non-reactive) - merging them would force the runtime to
        // re-guess that classification (losing information), exactly
        // the kind of mistake that once happened with
        // PropsMap/TokenKind::Component (a raw string, forcing meaning
        // to be guessed from the name) - here there are already 2 clear
        // AST types (StateDecl/VarDecl), so that distinction is kept
        // intact throughout the pipeline instead of being flattened
        // away.
        let global_vars_json = serde_json::to_string(self.ctx.get_global_vars())
            .unwrap_or_else(|_| "[]".to_string());

        // ── WASM BOOTSTRAP ──────────────────────────────────────────
        // `wasm-bindgen` generates a separate JS "glue" file (e.g.
        // "vibao_runtime.js") during a build using `wasm-bindgen-cli`,
        // NOT while `vibaoc` is running - that file exports a default
        // initialization function (the default export, called `init`)
        // to load its accompanying ".wasm" file, and exports the
        // `VbRuntime` class (already declared with `#[wasm_bindgen]` in
        // dom.rs).
        //
        // IMPORTANT: `output.js` (this function's result) is embedded
        // into HTML as an ordinary `<script>...</script>` (a classic
        // script), not `<script type="module">` - main.rs/the CLI only
        // prints plain JS, and doesn't decide the wrapping script tag
        // itself. So static `import ... from ...` syntax is NOT used
        // here (only valid inside a module script) - a DYNAMIC
        // `import()` is used instead (a FUNCTION returning a Promise),
        // syntax that's valid in both a classic script and a module
        // script.
        //
        // The path assumes the standard output layout of
        // `wasm-pack build`:
        //   pkg/vibao_runtime.js
        //   pkg/vibao_runtime_bg.wasm
        // If the real project places these files elsewhere, change this
        // WASM_JS_PATH constant (nothing else in codegen needs
        // changing).
        const WASM_JS_PATH: &str = "./pkg/vibao_runtime.js";

        let bootstrap_js = format!(
            r#"
// ── WASM Bootstrap (dynamic import — works inside a classic script) ──
(async function __vbBoot() {{
  try {{
    const wasmModule = await import("{wasm_path}");
    await wasmModule.default(); // loads the accompanying .wasm file, waiting for WASM to be ready
    const optsJson = JSON.stringify({{
      baseURL: "{base_url}",
      exprRegistry: {expr_registry_json},
      actionRegistry: {action_registry_json},
      pageInitialState: {page_initial_state_json},
      globalVars: {global_vars_json}
    }});
    window.__vbRuntime = new wasmModule.VbRuntime(optsJson); // keeps a reference for debugging from the console
  }} catch (err) {{
    console.error("[ViBao] Failed to start the runtime:", err);
  }}
}})();"#,
            wasm_path = WASM_JS_PATH,
            base_url = self.ctx.options.base_url,
            expr_registry_json = expr_registry_json,
            action_registry_json = action_registry_json,
            page_initial_state_json = page_initial_state_json,
            global_vars_json = global_vars_json,
        );

        format!(
            "// ViBao Generated JS — DO NOT EDIT\n(function() {{\n'use strict';\n\n// ── App init ──\n{}\n}})();\n{}",
            self.ctx.get_js(),
            bootstrap_js,
        )
    }
}

// ════════════════════════════════════════════════════════════
// ElementCodegenHost — lets element.rs/control.rs/component.rs call
// back into Codegen::gen_child() recursively, without creating an
// import cycle between the submodules and mod.rs.
// ════════════════════════════════════════════════════════════

impl ElementCodegenHost for Codegen {
    fn next_id(&mut self, tag: &str) -> String {
        self.ctx.next_id(tag)
    }

    fn gen_children(&mut self, children: &[Child]) -> String {
        self.gen_children_internal(children)
    }

    fn add_css(&mut self, code: &str) {
        self.ctx.add_css(code);
    }

    fn add_media_query(&mut self, code: &str) {
        self.ctx.add_media_query(code);
    }

    fn add_warning(&mut self, msg: String) {
        self.registry.warnings.push(msg);
    }

    fn add_hoisted_template(&mut self, html: String) {
        self.ctx.add_hoisted_template(html);
    }
}

// ════════════════════════════════════════════════════════════
// PAGE BACKGROUND COLOR RESOLUTION
// ════════════════════════════════════════════════════════════

/// Resolves a ColorValue (used specifically for page-level mau_nen)
/// into a CSS string. Equivalent to resolvePageBgColor().
///
/// UNIFIED SOURCE OF TRUTH: this file used to have its own
/// `COLOR_NAME_MAP`/`resolve_color_name`, separate from
/// `lexer::tables::color_map()`/`resolve_color_name` - 2 tables holding
/// duplicate data, and the version here also carried a silent fallback
/// bug (an unknown color name got passed through as an invalid CSS
/// value, silently ignored by the browser, with nobody the wiser). Now
/// uses `crate::lexer::resolve_color_name` DIRECTLY (the single source
/// of truth, already in lexer/tables.rs) - the duplicate table was
/// removed entirely.
pub fn resolve_page_bg_color(cv: &ColorValue) -> String {
    match cv {
        ColorValue::Hex(hex) => hex.clone(),
        ColorValue::Name(name) => {
            // INVARIANT: `ColorValue::Name` is only ever created by the
            // parser from a `TokenKind::ColorName` (see
            // parser/expr.rs::parse_literal) - and the lexer ONLY emits
            // ColorName for a name that actually exists in color_map()
            // (see scan.rs::classify_identifier). So `name` here is
            // ALWAYS valid; a `None` would mean an internal invariant
            // was violated (a real bug in the compiler), not a ViBao
            // user's syntax error - the same invariant-handling approach
            // used in parser/expr.rs::parse_literal, for the exact same
            // reason.
            crate::lexer::resolve_color_name(name).unwrap_or_else(|| {
                panic!(
                    "invariant violated: ColorValue::Name(\"{}\") is not in color_map() - \
                     the lexer should never emit ColorName for an invalid name",
                    name
                )
            })
        }
        ColorValue::Variable(name) => format!("var(--{})", name.replace('_', "-")),
        ColorValue::Func { func, args } => {
            let args_str = args
                .iter()
                .map(|a| expr::get_static_value(a))
                .collect::<Vec<_>>()
                .join(", ");
            format!("{}({})", crate::lexer::color_func_name(*func), args_str)
        }
    }
}

// ════════════════════════════════════════════════════════════
// UNIT TESTS
// ════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use vibao_ast::Pos;

    fn p() -> Pos {
        Pos { line: 1, column: 1 }
    }

    #[test]
    fn test_resolve_page_bg_color_name_known() {
        // Uses lexer::resolve_color_name directly (the single source of
        // truth, now unified) - replacing 2 old tests
        // (test_resolve_color_name_known/
        // test_resolve_color_name_unknown_passthrough) since codegen's
        // own separate COLOR_NAME_MAP was removed. The test for the
        // PASSTHROUGH behavior of an unknown name was removed along with
        // it - that behavior was exactly the BUG that got fixed
        // (invariant panic instead of a silent passthrough), see the
        // panic test below.
        assert_eq!(
            resolve_page_bg_color(&ColorValue::Name("do".to_string())),
            "#E53E3E"
        );
    }

    #[test]
    #[should_panic(expected = "invariant violated")]
    fn test_resolve_page_bg_color_name_unknown_panics() {
        // BUG ALREADY FIXED: a color name NOT in color_map() (e.g.
        // accidentally calling this function directly with an arbitrary
        // string, bypassing the normal lexer/parser path) used to be
        // silently PASSED THROUGH as invalid CSS (e.g.
        // "background-color:mau_la_theo") - silently ignored by the
        // browser, with nobody the wiser. It now panics clearly right at
        // the point the invariant was violated, instead of letting the
        // error drift all the way to the CSS output.
        resolve_page_bg_color(&ColorValue::Name("mau_la_theo".to_string()));
    }

    #[test]
    fn test_resolve_page_bg_color_hex() {
        assert_eq!(resolve_page_bg_color(&ColorValue::Hex("#123456".to_string())), "#123456");
    }

    #[test]
    fn test_resolve_page_bg_color_variable() {
        assert_eq!(
            resolve_page_bg_color(&ColorValue::Variable("mau_chinh".to_string())),
            "var(--mau-chinh)"
        );
    }

    #[test]
    fn test_context_next_id_increments() {
        let mut ctx = CodegenContext::new(CodegenOptions::default());
        assert_eq!(ctx.next_id("khoi"), "vb-box-1");
        assert_eq!(ctx.next_id("khoi"), "vb-box-2");
    }

    #[test]
    fn test_context_reset_page_keeps_id_counter_and_accumulated_js_global() {
        // FIXED (2 bugs within the same test):
        //
        // 1. id_counter: this test used to assert id_counter RESET to 1
        //    after reset_page() - that was exactly the BUG behavior that
        //    caused duplicate ids across pages in a real multi-page SPA
        //    app. id_counter must keep counting continuously across
        //    multiple pages, never resetting - guaranteeing globally
        //    unique ids.
        //
        // 2. get_js(): this test used to assert get_js() == "" right
        //    after reset_page() - that test was confirming (and
        //    inadvertently locking in) the exact BUG that made
        //    gen_app_js() (called AFTER the loop in generate() runs
        //    across every page) only ever see the LAST page's mount JS,
        //    since each page's reset_page() call cleared js_blocks
        //    without anything accumulating it elsewhere. The real-world
        //    consequence: every @the component on every page except the
        //    last never got __vb.mountComponent(...) called for it ->
        //    their props (e.g. $tieu_de, $mo_ta) never hydrated,
        //    rendering empty on every page except the last.
        //
        //    get_js() now reads from a separate `all_js_blocks` buffer,
        //    which reset_page() does NOT clear - the mount JS for EVERY
        //    page is preserved and correctly merged into the final
        //    app.js.
        let mut ctx = CodegenContext::new(CodegenOptions::default());
        ctx.next_id("khoi"); // "vb-box-1", simulating generating page 1
        ctx.add_js("mount_trang_1();");
        ctx.add_global_var("g", &Expr::literal_num(1.0, p()));
        ctx.reset_page(); // switching to generate page 2
        ctx.add_js("mount_trang_2();");

        assert_eq!(ctx.next_id("khoi"), "vb-box-2"); // NOT reset - keeps counting
        // JS from BOTH pages must still be present in get_js() - page 1 isn't lost.
        assert!(ctx.get_js().contains("mount_trang_1();"));
        assert!(ctx.get_js().contains("mount_trang_2();"));
        assert_eq!(ctx.get_global_vars().len(), 1); // global vars are still present
    }

    #[test]
    fn test_multi_page_ids_never_collide() {
        // A direct regression test for the fixed bug: simulates
        // generating ids for 2 consecutive "pages" (calling reset_page()
        // in between, just like the real generate() does for each page
        // in app.pages), confirming no id ever collides - this is
        // exactly the required condition for
        // document.getElementById() in a multi-page SPA to never
        // accidentally match another page's element.
        let mut ctx = CodegenContext::new(CodegenOptions::default());
        let mut all_ids = std::collections::HashSet::new();

        for _page in 0..3 {
            for _el in 0..5 {
                let id = ctx.next_id("khoi");
                assert!(all_ids.insert(id.clone()), "duplicate id: {}", id);
            }
            ctx.reset_page();
        }
        assert_eq!(all_ids.len(), 15); // 3 pages * 5 elements, none colliding
    }

    #[test]
    fn test_generate_empty_program_has_base_css() {
        let program = Program {
            app: App {
                name: "test".to_string(),
                imports: vec![],
                variables: vec![],
                themes: vec![],
                components: vec![],
                pages: vec![],
                pos: p(),
            },
        };
        let mut codegen = Codegen::new(CodegenOptions::default());
        let out = codegen.generate(&program);
        assert!(out.css.contains("ViBao Base CSS"));
    }

    #[test]
    fn test_generate_mounts_component_calls_on_every_page_not_just_last() {
        // An end-to-end regression test for the original real bug
        // (reported via a test build): a multi-page app where EVERY page
        // calls the same @the component - BEFORE THIS FIX (the old
        // JS-mount architecture, see the history in component.rs), the
        // bug was that the final app.js ONLY contained
        // __vb.mountComponent(...) for the LAST page in app.pages
        // (because gen_app_js() read js_blocks AFTER the generate()
        // loop, while each page's reset_page() call cleared js_blocks as
        // soon as moving to the next page).
        //
        // TEST UPDATED (not a new bug - found through an actual test
        // build by a user: `cargo test` reported a failure because this
        // test had become STALE relative to the current architecture):
        // since the "components never actually worked" fix (see
        // component.rs::gen_component_call), the concept of
        // "__vb.mountComponent(...) via JS" was REMOVED ENTIRELY from
        // the design - a component now renders directly to static HTML
        // `<div data-vb-component="...">` RIGHT WITHIN each page's HTML,
        // with no separate JS mount call at all anymore. So the old
        // assertion (counting `"__vb.mountComponent"` in out.js) no
        // longer means anything - out.js will NEVER contain that string
        // again, whether or not the original bug (missing mount on some
        // pages) were to reappear. This test now checks the CORRECT
        // invariant the NEW mechanism needs to guarantee: each page in
        // `out.pages` (HashMap<route, that page's own html>) must ITSELF
        // contain its own correct `data-vb-component="TieuDe"` - not
        // depending on page order or lost to a side effect between page
        // generation passes.
        use vibao_ast::{ComponentDef, ComponentCall, PropsMap};

        let comp_def = ComponentDef {
            name: "TieuDe".to_string(),
            params: vec![],
            children: vec![],
            pos: p(),
        };

        let make_page = |route: &str| Page {
            route: route.to_string(),
            name: None,
            mau_nen: None,
            states: vec![],
            events: vec![],
            children: vec![Child::ComponentCall(ComponentCall {
                name: "TieuDe".to_string(),
                props: PropsMap::new(),
                children: vec![],
                pos: p(),
            })],
            pos: p(),
        };

        let routes = ["/", "/trang-2", "/trang-3"];
        let program = Program {
            app: App {
                name: "test".to_string(),
                imports: vec![],
                variables: vec![],
                themes: vec![],
                components: vec![comp_def],
                pages: routes.iter().map(|r| make_page(r)).collect(),
                pos: p(),
            },
        };

        let mut codegen = Codegen::new(CodegenOptions::default());
        let out = codegen.generate(&program);

        // Each route must have its own HTML containing
        // data-vb-component="TieuDe" - this is the real invariant that
        // must hold in the current architecture (not counting a JS mount
        // call, which no longer exists).
        for route in routes {
            let page_html = out.pages.get(route).unwrap_or_else(|| {
                panic!("out.pages is missing HTML for route \"{}\". Available routes: {:?}", route, out.pages.keys().collect::<Vec<_>>())
            });
            assert!(
                page_html.contains("data-vb-component=\"TieuDe\""),
                "route \"{}\" must contain the mounted TieuDe component, actual HTML:\n{}",
                route, page_html
            );
        }
    }

    #[test]
    fn test_generate_includes_page_level_state_in_page_initial_state() {
        // An END-TO-END regression test for a real bug (found through a
        // build and test run in a real browser, NOT visible from
        // reading the code alone or from the 225 previously-passing
        // tests - exactly why this case needed its own dedicated test):
        // "state $x = <value>" declared INSIDE `trang(...)` (unlike
        // "state $x" at the `ung_dung` level) used to be completely
        // omitted from "pageInitialState" embedded in the bootstrap JS
        // (see the full explanation in generate()/gen_app_js() -
        // ctx.state_vars was correctly populated by gen_page(), but
        // nobody read it back before the next page's reset_page() call
        // wiped it out).
        //
        // The real-world symptom observed: that state always read back
        // as Null at runtime (never actually set_state()'d) - clicking a
        // button to accumulate a value repeatedly (e.g.
        // "$ban_kinh = $ban_kinh + 4") produced STRING CONCATENATION
        // (4 -> 44 -> 444) instead of numeric addition (8 -> 12 -> 16),
        // since eval_add() treats Null as not-a-Num and falls into the
        // string-concatenation branch (see
        // vibao-runtime/src/runtime/expr_eval.rs::eval_add - that
        // function itself is CORRECT per JS semantics; the real bug was
        // that the state was never initialized in the first place, not
        // the Add operator).
        let page = Page {
            route: "/".to_string(),
            name: None,
            mau_nen: None,
            states: vec![vibao_ast::StateDecl {
                name: "ban_kinh".to_string(),
                value: Expr::literal_num(8.0, p()),
                pos: p(),
            }],
            events: vec![],
            children: vec![],
            pos: p(),
        };

        let program = Program {
            app: App {
                name: "test".to_string(),
                imports: vec![],
                variables: vec![],
                themes: vec![],
                components: vec![],
                pages: vec![page],
                pos: p(),
            },
        };

        let mut codegen = Codegen::new(CodegenOptions::default());
        let out = codegen.generate(&program);

        // Extracts the pageInitialState section from app.js (the
        // bootstrap script) - finds the "pageInitialState:" marker then
        // parses the JSON right after it, instead of comparing a raw
        // string (fragile if the JSON's whitespace formatting ever
        // changes).
        let marker = "pageInitialState: ";
        let start = out.js.find(marker).unwrap_or_else(|| {
            panic!("app.js must contain \"pageInitialState: \", actual:\n{}", out.js)
        }) + marker.len();
        // pageInitialState is NO LONGER the last field in optsJson
        // (since "globalVars" was added right after it, see the BUG-19
        // fix) - cuts up to the ",\n" marker (the end of the value,
        // before the next field) instead of up to "});" like before.
        // The real last field now is "globalVars", which has its own
        // dedicated test below.
        let end = out.js[start..].find(",\n      globalVars").unwrap_or_else(|| {
            panic!("could not find a valid end point for pageInitialState in app.js:\n{}", out.js)
        }) + start;
        let page_initial_state_slice = out.js[start..end].trim();

        assert!(
            page_initial_state_slice.contains("\"ban_kinh\""),
            "pageInitialState must contain the page-level state \"ban_kinh\", actual: {}",
            page_initial_state_slice
        );
    }

    #[test]
    fn test_generate_keeps_page_initial_state_separate_by_route() {
        let page_a = Page {
            route: "/".to_string(),
            name: None,
            mau_nen: None,
            states: vec![vibao_ast::StateDecl {
                name: "counter_home".to_string(),
                value: Expr::literal_num(1.0, p()),
                pos: p(),
            }],
            events: vec![],
            children: vec![],
            pos: p(),
        };
        let page_b = Page {
            route: "/about".to_string(),
            name: None,
            mau_nen: None,
            states: vec![vibao_ast::StateDecl {
                name: "counter_about".to_string(),
                value: Expr::literal_num(2.0, p()),
                pos: p(),
            }],
            events: vec![],
            children: vec![],
            pos: p(),
        };

        let program = Program {
            app: App {
                name: "test".to_string(),
                imports: vec![],
                variables: vec![],
                themes: vec![],
                components: vec![],
                pages: vec![page_a, page_b],
                pos: p(),
            },
        };

        let mut codegen = Codegen::new(CodegenOptions::default());
        let out = codegen.generate(&program);

        let start = out.js.find("pageInitialState: ").unwrap() + "pageInitialState: ".len();
        let end = out.js[start..].find(",\n      globalVars").unwrap() + start;
        let page_state = &out.js[start..end];

        assert!(page_state.contains("/"));
        assert!(page_state.contains("/about"));
        assert!(page_state.contains("counter_home"));
        assert!(page_state.contains("counter_about"));
    }

    #[test]
    fn test_global_vars_from_nested_child_vardecl_reach_app_js() {
        // BUG-19 ALREADY FIXED (see RUNTIME_BOUNDARY_BUGS.md):
        // "$ten = gia_tri" (WITHOUT the `state` keyword) declared nested
        // inside a page - Child::VarDecl - used to be correctly recorded
        // by ctx.add_global_var() but was NEVER serialized into app.js.
        // The runtime already had init_global_vars()/a `vars` field
        // ready to receive exactly this kind of data (unlike `state`,
        // non-reactive), but nothing ever sent data to it. This test
        // confirms the "globalVars" field now appears in app.js and
        // contains the correctly declared variable.
        let page = Page {
            route: "/".to_string(),
            name: None,
            mau_nen: None,
            states: vec![],
            events: vec![],
            children: vec![Child::VarDecl(vibao_ast::VarDecl {
                name: "gia_co_dinh".to_string(),
                value: Expr::literal_num(1000.0, p()),
                pos: p(),
            })],
            pos: p(),
        };

        let program = Program {
            app: App {
                name: "test".to_string(),
                imports: vec![],
                variables: vec![vibao_ast::VarDecl {
                    name: "app_state".to_string(),
                    value: Expr::literal_num(7.0, p()),
                    pos: p(),
                }],
                themes: vec![],
                components: vec![],
                pages: vec![page],
                pos: p(),
            },
        };

        let mut codegen = Codegen::new(CodegenOptions::default());
        let out = codegen.generate(&program);

        let marker = "globalVars: ";
        let start = out.js.find(marker).unwrap_or_else(|| {
            panic!("app.js must contain \"globalVars: \", actual:\n{}", out.js)
        }) + marker.len();
        // globalVars IS the last field in optsJson - cuts up to
        // "\n    });" globalVars is currently the last field in optsJson.
        let end = out.js[start..].find("\n    });").unwrap_or_else(|| {
            panic!("could not find a valid end point for globalVars in app.js:\n{}", out.js)
        }) + start;
        let global_vars_slice = out.js[start..end].trim();

        assert!(
            global_vars_slice.contains("\"gia_co_dinh\""),
            "globalVars must contain the variable \"gia_co_dinh\" declared nested inside the page (Child::VarDecl), actual: {}",
            global_vars_slice
        );
        assert!(
            !global_vars_slice.contains("\"app_state\""),
            "app.variables must not be duplicated into globalVars: {}",
            global_vars_slice
        );
    }
}
