pub mod action;
pub mod component;
pub mod control;
pub mod css;
pub mod element;
pub mod expr;
pub mod global_style;
pub mod layout;
pub mod props;
pub mod theme;

use crate::codegen::component::ComponentRegistry;
use crate::codegen::css::{esc_attr, BASE_CSS};
use crate::codegen::element::{tag_to_class_name, ElementCodegenHost};
use std::collections::HashMap;
use vibao_ast::{App, Child, ColorValue, Expr, Page, Program};

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

pub struct CodegenContext {
    pub options: CodegenOptions,
    id_counter: u32,
    css_blocks: Vec<String>,
    js_blocks: Vec<String>,
    all_js_blocks: Vec<String>,
    media_queries: Vec<String>,
    state_vars: Vec<(String, Expr)>,
    global_vars: Vec<(String, Expr)>,
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
        self.css_blocks
            .iter()
            .chain(self.media_queries.iter())
            .cloned()
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    #[allow(dead_code)]
    pub fn add_js(&mut self, block: &str) {
        if !block.trim().is_empty() {
            self.js_blocks.push(block.to_string());
            self.all_js_blocks.push(block.to_string());
        }
    }

    pub fn get_js(&self) -> String {
        self.all_js_blocks.join("\n\n")
    }

    pub fn add_hoisted_template(&mut self, html: String) {
        if !html.trim().is_empty() {
            self.hoisted_templates.push(html);
        }
    }

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

    pub fn reset_page(&mut self) {
        self.js_blocks.clear();
        self.state_vars.clear();
        self.hoisted_templates.clear();
    }
}

pub struct CodegenOutput {
    pub html: String,
    pub css: String,
    pub js: String,
    pub pages: HashMap<String, String>,
    pub warnings: Vec<String>,
    pub default_title: Option<String>,
    pub default_description: Option<String>,
    pub stylesheets: Vec<String>,
    pub theme_names: Vec<String>,
}

pub struct Codegen {
    pub ctx: CodegenContext,
    registry: ComponentRegistry,
}

impl Codegen {
    pub fn new(options: CodegenOptions) -> Self {
        Codegen {
            ctx: CodegenContext::new(options),
            registry: ComponentRegistry::new(),
        }
    }

    pub fn generate(&mut self, program: &Program) -> CodegenOutput {
        let app = &program.app;

        for def in &app.components {
            self.registry.register(def.clone());
        }
        self.ctx.add_css(BASE_CSS);
        theme::activate(&app.themes);
        self.ctx.add_css(&theme::generate_css(&app.themes));
        let global_style = global_style::build(&app.global_style);
        self.ctx.add_css(&global_style.css);
        self.registry.warnings.extend(global_style.warnings);

        let mut page_initial_state_vars: Vec<(String, Vec<(String, Expr)>)> = Vec::new();
        let mut page_metadata: Vec<(String, Option<String>, Option<String>)> = Vec::new();

        let mut pages: HashMap<String, String> = HashMap::new();
        for page in &app.pages {
            self.ctx.reset_page();
            let html = self.gen_page(page);
            page_initial_state_vars.push((
                page.route.clone(),
                self.ctx.get_state_vars().iter().cloned().collect(),
            ));
            page_metadata.push((
                page.route.clone(),
                page.name.clone(),
                page.description.clone(),
            ));
            pages.insert(page.route.clone(), html);
        }

        let html = pages
            .get("/")
            .cloned()
            .unwrap_or_else(|| pages.values().next().cloned().unwrap_or_default());
        let js = self.gen_app_js(app, &page_initial_state_vars, &page_metadata);
        let warnings = self.registry.warnings.clone();
        let default_page = app
            .pages
            .iter()
            .find(|p| p.route == "/")
            .or_else(|| app.pages.first());
        let default_title = default_page.and_then(|p| p.name.clone());
        let default_description = default_page.and_then(|p| p.description.clone());

        CodegenOutput {
            html,
            css: self.ctx.get_css(),
            js,
            pages,
            warnings,
            default_title,
            default_description,
            stylesheets: global_style.stylesheets,
            theme_names: app.themes.iter().map(|t| t.name.clone()).collect(),
        }
    }

    fn gen_page(&mut self, page: &Page) -> String {
        for s in &page.states {
            self.ctx.add_state_var(&s.name, &s.value);
        }
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
            esc_attr(&page.route),
            bg_style,
            lifecycle_attrs,
            css::indent2(&body_content)
        )
    }

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
            Child::PageEvent(_) => String::new(),
            Child::If(node) => control::gen_if(node, self),
            Child::Switch(node) => control::gen_switch(node, self),
            Child::Loop(node) => control::gen_loop(node, self),
            Child::Element(el) => element::gen_element(el, self),
            Child::ComponentCall(call) => {
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
        children
            .iter()
            .map(|c| self.gen_child(c))
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn gen_app_js(
        &self,
        app: &App,
        page_initial_state_vars: &[(String, Vec<(String, Expr)>)],
        page_metadata: &[(String, Option<String>, Option<String>)],
    ) -> String {
        let expr_registry = expr::take_expr_registry();
        let expr_registry_json =
            serde_json::to_string(&expr_registry).unwrap_or_else(|_| "[]".to_string());

        let action_registry = action::take_action_registry();
        let action_registry_json =
            serde_json::to_string(&action_registry).unwrap_or_else(|_| "[]".to_string());

        let page_initial_state_json =
            serde_json::to_string(&page_initial_state_vars).unwrap_or_else(|_| "[]".to_string());

        let page_metadata_json =
            serde_json::to_string(page_metadata).unwrap_or_else(|_| "[]".to_string());

        let global_vars_json =
            serde_json::to_string(self.ctx.get_global_vars()).unwrap_or_else(|_| "[]".to_string());

        const WASM_JS_PATH: &str = "./pkg/vibao_runtime.js";

        let bootstrap_js = format!(
            r#"
(async function __vbBoot() {{
  try {{
    const wasmModule = await import("{wasm_path}");
    await wasmModule.default();
    const optsJson = JSON.stringify({{
      baseURL: "{base_url}",
      lang: "{lang}",
      exprRegistry: {expr_registry_json},
      actionRegistry: {action_registry_json},
      pageInitialState: {page_initial_state_json},
      pageMetadata: {page_metadata_json},
      globalVars: {global_vars_json}
    }});
    window.__vbRuntime = new wasmModule.VbRuntime(optsJson);
  }} catch (err) {{
    console.error("[ViBao] Failed to start the runtime:", err);
  }}
}})();"#,
            wasm_path = WASM_JS_PATH,
            base_url = self.ctx.options.base_url,
            lang = app.lang,
            expr_registry_json = expr_registry_json,
            action_registry_json = action_registry_json,
            page_initial_state_json = page_initial_state_json,
            page_metadata_json = page_metadata_json,
            global_vars_json = global_vars_json,
        );

        format!(
            "(function() {{\n'use strict';\n\n{}\n}})();\n{}",
            self.ctx.get_js(),
            bootstrap_js,
        )
    }
}

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

pub fn resolve_page_bg_color(cv: &ColorValue) -> String {
    match cv {
        ColorValue::Hex(hex) => hex.clone(),
        ColorValue::Name(name) => crate::lexer::resolve_color_name(name).unwrap_or_else(|| {
            panic!(
                "invariant violated: ColorValue::Name(\"{}\") is not in color_map() - \
                     the lexer should never emit ColorName for an invalid name",
                name
            )
        }),
        ColorValue::Variable(name) => format!("var({})", theme::css_var_name(name)),
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

#[cfg(test)]
mod tests {
    use super::*;
    use vibao_ast::Pos;

    fn p() -> Pos {
        Pos { line: 1, column: 1 }
    }

    #[test]
    fn test_resolve_page_bg_color_name_known() {
        assert_eq!(
            resolve_page_bg_color(&ColorValue::Name("do".to_string())),
            "#E53E3E"
        );
    }

    #[test]
    #[should_panic(expected = "invariant violated")]
    fn test_resolve_page_bg_color_name_unknown_panics() {
        resolve_page_bg_color(&ColorValue::Name("mau_la_theo".to_string()));
    }

    #[test]
    fn test_resolve_page_bg_color_hex() {
        assert_eq!(
            resolve_page_bg_color(&ColorValue::Hex("#123456".to_string())),
            "#123456"
        );
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
        let mut ctx = CodegenContext::new(CodegenOptions::default());
        ctx.next_id("khoi");
        ctx.add_js("mount_trang_1();");
        ctx.add_global_var("g", &Expr::literal_num(1.0, p()));
        ctx.reset_page();
        ctx.add_js("mount_trang_2();");

        assert_eq!(ctx.next_id("khoi"), "vb-box-2");
        assert!(ctx.get_js().contains("mount_trang_1();"));
        assert!(ctx.get_js().contains("mount_trang_2();"));
        assert_eq!(ctx.get_global_vars().len(), 1);
    }

    #[test]
    fn test_multi_page_ids_never_collide() {
        let mut ctx = CodegenContext::new(CodegenOptions::default());
        let mut all_ids = std::collections::HashSet::new();

        for _page in 0..3 {
            for _el in 0..5 {
                let id = ctx.next_id("khoi");
                assert!(all_ids.insert(id.clone()), "duplicate id: {}", id);
            }
            ctx.reset_page();
        }
        assert_eq!(all_ids.len(), 15);
    }

    #[test]
    fn test_generate_empty_program_has_base_css() {
        let program = Program {
            app: App {
                name: "test".to_string(),
                lang: "vi".to_string(),
                imports: vec![],
                variables: vec![],
                themes: vec![],
                global_style: vec![],
                components: vec![],
                pages: vec![],
                pos: p(),
            },
        };
        let mut codegen = Codegen::new(CodegenOptions::default());
        let out = codegen.generate(&program);
        assert!(out.css.contains("box-sizing: border-box"));
    }

    #[test]
    fn test_generate_mounts_component_calls_on_every_page_not_just_last() {
        use vibao_ast::{ComponentCall, ComponentDef, PropsMap};

        let comp_def = ComponentDef {
            name: "TieuDe".to_string(),
            params: vec![],
            children: vec![],
            pos: p(),
        };

        let make_page = |route: &str| Page {
            route: route.to_string(),
            name: None,
            description: None,
            mau_nen: None,
            states: vec![],
            events: vec![],
            children: vec![Child::ComponentCall(ComponentCall {
                name: "TieuDe".to_string(),
                props: PropsMap::new(),
                identity: Default::default(),
                children: vec![],
                pos: p(),
            })],
            pos: p(),
        };

        let routes = ["/", "/trang-2", "/trang-3"];
        let program = Program {
            app: App {
                name: "test".to_string(),
                lang: "vi".to_string(),
                imports: vec![],
                variables: vec![],
                themes: vec![],
                global_style: vec![],
                components: vec![comp_def],
                pages: routes.iter().map(|r| make_page(r)).collect(),
                pos: p(),
            },
        };

        let mut codegen = Codegen::new(CodegenOptions::default());
        let out = codegen.generate(&program);

        for route in routes {
            let page_html = out.pages.get(route).unwrap_or_else(|| {
                panic!(
                    "out.pages is missing HTML for route \"{}\". Available routes: {:?}",
                    route,
                    out.pages.keys().collect::<Vec<_>>()
                )
            });
            assert!(
                page_html.contains("data-vb-component=\"TieuDe\""),
                "route \"{}\" must contain the mounted TieuDe component, actual HTML:\n{}",
                route,
                page_html
            );
        }
    }

    #[test]
    fn test_generate_includes_page_level_state_in_page_initial_state() {
        let page = Page {
            route: "/".to_string(),
            name: None,
            description: None,
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
                lang: "vi".to_string(),
                imports: vec![],
                variables: vec![],
                themes: vec![],
                global_style: vec![],
                components: vec![],
                pages: vec![page],
                pos: p(),
            },
        };

        let mut codegen = Codegen::new(CodegenOptions::default());
        let out = codegen.generate(&program);

        let marker = "pageInitialState: ";
        let start = out.js.find(marker).unwrap_or_else(|| {
            panic!(
                "app.js must contain \"pageInitialState: \", actual:\n{}",
                out.js
            )
        }) + marker.len();
        let end = out.js[start..]
            .find(",\n      globalVars")
            .unwrap_or_else(|| {
                panic!(
                    "could not find a valid end point for pageInitialState in app.js:\n{}",
                    out.js
                )
            })
            + start;
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
            description: None,
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
            description: None,
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
                lang: "vi".to_string(),
                imports: vec![],
                variables: vec![],
                themes: vec![],
                global_style: vec![],
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
        let page = Page {
            route: "/".to_string(),
            name: None,
            description: None,
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
                lang: "vi".to_string(),
                imports: vec![],
                variables: vec![vibao_ast::VarDecl {
                    name: "app_state".to_string(),
                    value: Expr::literal_num(7.0, p()),
                    pos: p(),
                }],
                themes: vec![],
                global_style: vec![],
                components: vec![],
                pages: vec![page],
                pos: p(),
            },
        };

        let mut codegen = Codegen::new(CodegenOptions::default());
        let out = codegen.generate(&program);

        let marker = "globalVars: ";
        let start =
            out.js.find(marker).unwrap_or_else(|| {
                panic!("app.js must contain \"globalVars: \", actual:\n{}", out.js)
            }) + marker.len();
        let end = out.js[start..].find("\n    });").unwrap_or_else(|| {
            panic!(
                "could not find a valid end point for globalVars in app.js:\n{}",
                out.js
            )
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
