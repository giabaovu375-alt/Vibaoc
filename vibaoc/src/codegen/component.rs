use crate::codegen::css::{esc_attr, indent2};
use crate::codegen::element::ElementCodegenHost;
use crate::codegen::expr::{json_string, register_expr};
use crate::codegen::expr::{resolve_value, ResolvedValue};
use std::collections::HashMap;
use vibao_ast::{ComponentCall, ComponentDef};

#[derive(Debug, Clone, Default)]
pub struct ComponentRegistry {
    defs: HashMap<String, ComponentDef>,
    pub warnings: Vec<String>,
}

impl ComponentRegistry {
    pub fn new() -> Self {
        ComponentRegistry::default()
    }

    pub fn register(&mut self, def: ComponentDef) {
        if self.defs.contains_key(&def.name) {
            self.warnings.push(format!(
                "[ViBao] Component \"@the {}\" is being redefined",
                def.name
            ));
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

pub fn resolve_component_def(
    node: &ComponentCall,
    registry: &ComponentRegistry,
) -> Result<ComponentDef, String> {
    match registry.get(&node.name) {
        Some(d) => Ok(d.clone()),
        None => Err(format!(
            "[ViBao] Component \"{}\" has not been defined with @the",
            node.name
        )),
    }
}

pub fn gen_component_call_with_def(
    node: &ComponentCall,
    def: &ComponentDef,
    host: &mut dyn ElementCodegenHost,
) -> String {
    let auto_id = host.next_id(&node.name);
    let id = match &node.identity.id {
        Some(expr) => match resolve_value(expr) {
            ResolvedValue::Static(s) => esc_attr(&s),
            _ => auto_id,
        },
        None => auto_id,
    };
    let children_html = host.gen_children(&def.children);

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

#[cfg(test)]
mod tests {
    use super::*;
    use vibao_ast::{Child, DataType, Expr, ParamDef, Pos};

    fn p() -> Pos {
        Pos { line: 1, column: 1 }
    }

    fn gen_html(
        node: &ComponentCall,
        registry: &ComponentRegistry,
        host: &mut dyn ElementCodegenHost,
    ) -> String {
        let def = resolve_component_def(node, registry).expect("component phải được đăng ký");
        gen_component_call_with_def(node, &def, host)
    }

    struct FakeHost {
        counter: u32,
        js: Vec<String>,
    }

    impl FakeHost {
        fn new() -> Self {
            FakeHost {
                counter: 0,
                js: vec![],
            }
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
        let def = ComponentDef {
            name: "The_Card".to_string(),
            params: vec![],
            children: vec![],
            pos: p(),
        };
        registry.register(def);
        assert!(registry.has("The_Card"));
        assert!(registry.get("The_Card").is_some());
    }

    #[test]
    fn test_register_duplicate_warns_but_overwrites() {
        let mut registry = ComponentRegistry::new();
        registry.register(ComponentDef {
            name: "X".to_string(),
            params: vec![],
            children: vec![],
            pos: p(),
        });
        registry.register(ComponentDef {
            name: "X".to_string(),
            params: vec![ParamDef {
                name: "a".to_string(),
                data_type: DataType::Chuoi,
                default_value: None,
                pos: p(),
            }],
            children: vec![],
            pos: p(),
        });
        assert_eq!(registry.warnings.len(), 1);
        assert_eq!(registry.get("X").unwrap().params.len(), 1);
    }

    #[test]
    fn test_resolve_component_def_unknown_returns_error() {
        let registry = ComponentRegistry::new();
        let call = ComponentCall {
            name: "KhongTonTai".to_string(),
            props: vec![],
            identity: Default::default(),
            children: vec![],
            pos: p(),
        };
        let error = resolve_component_def(&call, &registry).unwrap_err();
        assert!(error.contains("KhongTonTai"), "{}", error);
    }

    #[test]
    fn test_gen_component_call_known_generates_props_expr_id_no_dead_js() {
        crate::codegen::expr::take_expr_registry();
        let mut registry = ComponentRegistry::new();
        registry.register(ComponentDef {
            name: "The_Card".to_string(),
            params: vec![],
            children: vec![],
            pos: p(),
        });
        let mut host = FakeHost::new();
        let call = ComponentCall {
            name: "The_Card".to_string(),
            props: vec![("tieu_de".to_string(), Expr::literal_str("Xin chào", p()))],
            identity: Default::default(),
            children: vec![],
            pos: p(),
        };
        let html = gen_html(&call, &registry, &mut host);
        assert!(html.contains("data-vb-component=\"The_Card\""));
        assert!(
            host.js.is_empty(),
            "gen_component_call_with_def must not generate any JS, got: {:?}",
            host.js
        );
        assert!(
            html.contains("data-vb-props=\"{&quot;tieu_de&quot;:0}\""),
            "data-vb-props must contain {{\"tieu_de\": <numeric exprId>}}, actual HTML:\n{}",
            html
        );
        let registry_snapshot = crate::codegen::expr::take_expr_registry();
        assert_eq!(registry_snapshot.len(), 1);
        assert!(
            matches!(&registry_snapshot[0], Expr::Literal(vibao_ast::LiteralValue::Str(s), _) if s == "Xin chào")
        );
    }

    #[test]
    fn test_component_call_id_prop_overrides_auto_generated_id() {
        crate::codegen::expr::take_expr_registry();
        let mut registry = ComponentRegistry::new();
        registry.register(ComponentDef {
            name: "The_Card".to_string(),
            params: vec![],
            children: vec![],
            pos: p(),
        });
        let mut host = FakeHost::new();
        let call = ComponentCall {
            name: "The_Card".to_string(),
            props: vec![],
            identity: vibao_ast::IdentityProps {
                id: Some(Expr::literal_str("gioi-thieu", p())),
                class_binding: None,
                html_tag: None,
            },
            children: vec![],
            pos: p(),
        };
        let html = gen_html(&call, &registry, &mut host);
        assert!(html.contains("id=\"gioi-thieu\""));
        assert_eq!(
            html.matches("id=\"").count(),
            1,
            "không được có 2 id trùng nhau"
        );
        assert!(
            !html.contains("&quot;id&quot;"),
            "id không được xuất hiện trong data-vb-props: {}",
            html
        );
    }

    #[test]
    fn test_component_call_id_prop_is_attribute_escaped() {
        crate::codegen::expr::take_expr_registry();
        let mut registry = ComponentRegistry::new();
        registry.register(ComponentDef {
            name: "The_Card".to_string(),
            params: vec![],
            children: vec![],
            pos: p(),
        });
        let mut host = FakeHost::new();
        let call = ComponentCall {
            name: "The_Card".to_string(),
            props: vec![],
            identity: vibao_ast::IdentityProps {
                id: Some(Expr::literal_str("a\"b&c", p())),
                class_binding: None,
                html_tag: None,
            },
            children: vec![],
            pos: p(),
        };
        let html = gen_html(&call, &registry, &mut host);
        assert!(html.contains("id=\"a&quot;b&amp;c\""), "{}", html);
    }

    #[test]
    fn test_component_call_without_id_prop_keeps_auto_generated_id() {
        crate::codegen::expr::take_expr_registry();
        let mut registry = ComponentRegistry::new();
        registry.register(ComponentDef {
            name: "The_Card".to_string(),
            params: vec![],
            children: vec![],
            pos: p(),
        });
        let mut host = FakeHost::new();
        let call = ComponentCall {
            name: "The_Card".to_string(),
            props: vec![],
            identity: Default::default(),
            children: vec![],
            pos: p(),
        };
        let html = gen_html(&call, &registry, &mut host);
        assert!(html.contains("id=\"vb-The_Card-"));
    }

    #[test]
    fn test_component_param_named_id_never_treated_as_directive() {
        crate::codegen::expr::take_expr_registry();
        let mut registry = ComponentRegistry::new();
        registry.register(ComponentDef {
            name: "The_SanPham".to_string(),
            params: vec![vibao_ast::ParamDef {
                name: "id".to_string(),
                data_type: vibao_ast::DataType::So,
                default_value: None,
                pos: p(),
            }],
            children: vec![],
            pos: p(),
        });
        let mut host = FakeHost::new();
        let call = ComponentCall {
            name: "The_SanPham".to_string(),
            props: vec![("id".to_string(), Expr::literal_str("gia-tri-tinh", p()))],
            identity: Default::default(),
            children: vec![],
            pos: p(),
        };
        let html = gen_html(&call, &registry, &mut host);
        assert!(
            html.contains("&quot;id&quot;"),
            "id (param thật, không phải identity) phải luôn đi vào data-vb-props: {}",
            html
        );
        assert!(html.contains("id=\"vb-The_SanPham-"), "phải giữ auto-generated id vì identity.id không được set (chỉ props[\"id\"] có giá trị)");
    }
}
