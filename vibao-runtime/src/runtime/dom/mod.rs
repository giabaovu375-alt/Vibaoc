mod animation;
mod conditional;
mod events;
mod loop_render;
mod model;
mod subtree;
mod text_attr;
mod ui;

pub use ui::*;
pub(crate) use subtree::bind_subtree;

use std::sync::atomic::{AtomicU64, Ordering};
use wasm_bindgen::prelude::*;
use web_sys::{Document, Element, Window};
use crate::runtime::{expr_eval, expr_registry, state::{self, LoopFrame, SharedState}, value::VbValue};
use vibao_ast::Expr;

const ATTR_BIND_TEXT: &str = "data-vb-text";
const ATTR_BIND_ATTR_PREFIX: &str = "data-vb-attr-";
const ATTR_BIND_STYLE_PREFIX: &str = "data-vb-style-";
const ATTR_BIND_CLASS: &str = "data-vb-class";
const ATTR_IF: &str = "data-vb-if";
const ATTR_ELSE: &str = "data-vb-else";
const ATTR_LOOP: &str = "data-vb-loop";
const ATTR_LOOP_ITEM_MARKER: &str = "data-vb-loop-item";
const ATTR_SWITCH: &str = "data-vb-switch";
const ATTR_CASE: &str = "data-vb-case";
const ATTR_DEFAULT: &str = "data-vb-default";
const ATTR_EVENT_PREFIX: &str = "data-vb-on-";
const ATTR_MODEL: &str = "data-vb-model";
const ATTR_COMPONENT: &str = "data-vb-component";

static NEXT_COMPONENT_INSTANCE_ID: AtomicU64 = AtomicU64::new(1);

fn runtime_component_id(original: &str) -> String {
    let n = NEXT_COMPONENT_INSTANCE_ID.fetch_add(1, Ordering::Relaxed);
    format!("{}-r{}", original, n)
}
const ATTR_PROPS: &str = "data-vb-props";

fn window() -> Window {
    web_sys::window().expect("[ViBao] could not get `window` - running outside a browser?")
}

fn document() -> Document {
    window()
        .document()
        .expect("[ViBao] could not get `document`")
}

fn query_all(root: &Element, selector: &str) -> Vec<Element> {
    let mut out = Vec::new();
    if let Ok(true) = root.matches(selector) {
        out.push(root.clone());
    }
    if let Ok(list) = root.query_selector_all(selector) {
        for i in 0..list.length() {
            if let Some(node) = list.item(i) {
                if let Ok(el) = node.dyn_into::<Element>() {
                    out.push(el);
                }
            }
        }
    }
    out
}

fn is_inside_nested_component(el: &Element, root: &Element) -> bool {
    let mut current: Option<Element> = el.parent_element();
    while let Some(ancestor) = current {
        if ancestor.is_same_node(Some(root.as_ref())) {
            return false;
        }
        if ancestor.has_attribute(ATTR_COMPONENT) {
            return true;
        }
        current = ancestor.parent_element();
    }
    false
}

fn is_inside_switch_case(el: &Element, root: &Element) -> bool {
    let mut current: Option<Element> = el.parent_element();
    while let Some(ancestor) = current {
        if ancestor.is_same_node(Some(root.as_ref())) {
            return false;
        }
        if ancestor.has_attribute(ATTR_CASE) || ancestor.has_attribute(ATTR_DEFAULT) {
            return true;
        }
        current = ancestor.parent_element();
    }
    false
}

fn is_inside_loop(el: &Element, root: &Element) -> bool {
    if el.is_same_node(Some(root.as_ref())) {
        return false;
    }
    if el.has_attribute(ATTR_LOOP_ITEM_MARKER) {
        return true;
    }
    let mut current: Option<Element> = el.parent_element();
    while let Some(ancestor) = current {
        if ancestor.is_same_node(Some(root.as_ref())) {
            return false;
        }
        if ancestor.has_attribute(ATTR_LOOP_ITEM_MARKER) {
            return true;
        }
        current = ancestor.parent_element();
    }
    false
}

fn eval_expr_id_tracked(
    shared: &SharedState,
    id: usize,
    loop_frame: Option<&LoopFrame>,
    component_id: Option<&str>,
) -> VbValue {
    let _guard = state::ScopeGuard::new(shared, loop_frame, component_id);

    match expr_registry::get(id) {
        Some(expr) => expr_eval::eval_tracked(shared, &expr),
        None => {
            crate::runtime::log::warn(&format!("[ViBao] expr id {} does not exist in the registry", id));
            VbValue::Null
        }
    }
}

fn eval_expr_id(shared: &SharedState, id: usize) -> VbValue {
    match expr_registry::get(id) {
        Some(expr) => expr_eval::eval(shared, &expr),
        None => VbValue::Null,
    }
}

fn parse_expr_id(raw: &str, context: &str) -> Option<usize> {
    match raw.trim().parse::<usize>() {
        Ok(id) => Some(id),
        Err(_) => {
            crate::runtime::log::warn(&format!(
                "[ViBao] \"{}\" is not a valid expr id (context: {})",
                raw, context
            ));
            None
        }
    }
}

#[wasm_bindgen]
pub struct VbRuntime {
    shared: SharedState,
}

fn default_lang() -> String {
    "vi".to_string()
}

fn initialize_global_vars(shared: &SharedState, global_vars: &[(String, Expr)]) {
    for (name, expr) in global_vars {
        let value = crate::runtime::expr_eval::eval(shared, expr);
        shared.borrow_mut().set_var(name, value);
    }
}

#[wasm_bindgen]
impl VbRuntime {
    #[wasm_bindgen(constructor)]
    pub fn new(opts_json: &str) -> VbRuntime {
        console_error_panic_hook::set_once();

        #[derive(serde::Deserialize)]
        struct BootOpts {
            #[serde(rename = "exprRegistry")]
            expr_registry: Vec<Expr>,
            #[serde(rename = "actionRegistry", default)]
            action_registry: Vec<Vec<vibao_ast::Action>>,
            #[serde(rename = "baseURL")]
            base_url: String,
            #[serde(rename = "lang", default = "default_lang")]
            lang: String,
            #[serde(rename = "pageInitialState", default)]
            page_initial_state: Vec<(String, Vec<(String, Expr)>)>,
            #[serde(rename = "globalVars", default)]
            global_vars: Vec<(String, Expr)>,
            #[serde(rename = "pageMetadata", default)]
            page_metadata: Vec<(String, Option<String>, Option<String>)>,
        }

        let opts: BootOpts = match serde_json::from_str(opts_json) {
            Ok(o) => o,
            Err(err) => {
                crate::runtime::log::error(&format!("[ViBao] boot() received invalid JSON: {}", err));
                BootOpts {
                    expr_registry: Vec::new(),
                    action_registry: Vec::new(),
                    base_url: String::new(),
                    lang: default_lang(),
                    page_initial_state: Vec::new(),
                    global_vars: Vec::new(),
                    page_metadata: Vec::new(),
                }
            }
        };

        expr_registry::load_from_json(
            &serde_json::to_string(&opts.expr_registry).unwrap_or_else(|_| "[]".to_string()),
        );
        crate::runtime::action_registry::load_from_json(
            &serde_json::to_string(&opts.action_registry).unwrap_or_else(|_| "[]".to_string()),
        );

        let shared = state::new_shared_state();
        shared.borrow_mut().set_base_url(opts.base_url);
        shared.borrow_mut().set_lang(opts.lang);

        initialize_global_vars(&shared, &opts.global_vars);

        shared.borrow_mut().set_page_initial_states(opts.page_initial_state);
        shared.borrow_mut().set_page_metadata(opts.page_metadata);
        crate::runtime::router::boot_router(&shared);

        VbRuntime { shared }
    }

    #[wasm_bindgen(js_name = evalExpr)]
    pub fn eval_expr(&self, id: usize) -> JsValue {
        let v = eval_expr_id(&self.shared, id);
        v.to_js_value()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn runtime_component_ids_are_unique() {
        let a = super::runtime_component_id("vb-card-1");
        let b = super::runtime_component_id("vb-card-1");
        assert_ne!(a, b);
        assert!(a.starts_with("vb-card-1-r"));
        assert!(b.starts_with("vb-card-1-r"));
    }

    #[test]
    fn test_event_needs_prevent_default() {
        assert!(event_needs_prevent_default("submit"));
        assert!(!event_needs_prevent_default("click"));
        assert!(!event_needs_prevent_default("change"));
        assert!(!event_needs_prevent_default("input"));
    }

    use super::*;
    use super::animation::hover_animation_class_name;
    use super::events::event_needs_prevent_default;
    use super::text_attr::css_style_value_string;

    #[test]
    fn global_vars_can_reference_earlier_global_vars() {
        let shared = state::new_shared_state();
        let globals = vec![
            ("phi".to_string(), Expr::literal_num(10.0, vibao_ast::Pos { line: 1, column: 1 })),
            (
                "tong".to_string(),
                Expr::Binary {
                    op: vibao_ast::BinaryOp::Add,
                    left: Box::new(Expr::Variable("phi".to_string(), vibao_ast::Pos { line: 2, column: 1 })),
                    right: Box::new(Expr::literal_num(5.0, vibao_ast::Pos { line: 2, column: 8 })),
                    pos: vibao_ast::Pos { line: 2, column: 1 },
                },
            ),
        ];
        super::initialize_global_vars(&shared, &globals);
        assert_eq!(shared.borrow().get_var("tong"), VbValue::Num(15.0));
    }

    #[test]
    fn test_hover_animation_uses_hover_css_prefix() {
        assert_eq!(hover_animation_class_name("phong_to"), "vb-hover-phong_to");
        assert_eq!(hover_animation_class_name("lam_sang"), "vb-hover-lam_sang");
    }

    #[test]
    fn test_css_length_prop_number_gets_px_suffix() {
        assert_eq!(css_style_value_string("width", &VbValue::Num(120.0)), "120px");
        assert_eq!(css_style_value_string("padding", &VbValue::Num(16.0)), "16px");
        assert_eq!(css_style_value_string("border-radius", &VbValue::Num(8.0)), "8px");
        assert_eq!(css_style_value_string("font-size", &VbValue::Num(24.0)), "24px");
    }

    #[test]
    fn test_css_length_prop_trims_trailing_zero() {
        assert_eq!(css_style_value_string("width", &VbValue::Num(120.0)), "120px");
    }

    #[test]
    fn test_non_length_prop_not_affected() {
        assert_eq!(css_style_value_string("z-index", &VbValue::Num(5.0)), "5");
        assert_eq!(css_style_value_string("line-height", &VbValue::Num(1.5)), "1.5");
    }

    #[test]
    fn test_string_value_not_double_suffixed() {
        assert_eq!(css_style_value_string("width", &VbValue::Str("50%".to_string())), "50%");
        assert_eq!(css_style_value_string("width", &VbValue::Str("calc(100% - 20px)".to_string())), "calc(100% - 20px)");
    }

    #[test]
    fn test_color_prop_string_unaffected() {
        assert_eq!(
            css_style_value_string("background-color", &VbValue::Str("#3182CE".to_string())),
            "#3182CE"
        );
    }
}
