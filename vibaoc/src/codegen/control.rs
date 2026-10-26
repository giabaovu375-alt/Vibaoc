use crate::codegen::css::{esc_attr, indent, indent2};
use crate::codegen::element::ElementCodegenHost;
use vibao_ast::{Child, IfNode, LoopKind, LoopNode, SwitchNode};

pub fn if_to_data_attr(expr_id: usize) -> String {
    format!("data-vb-if=\"{}\"", expr_id)
}

pub fn gen_if(node: &IfNode, host: &mut dyn ElementCodegenHost) -> String {
    let condition_id = crate::codegen::expr::register_expr(node.condition.clone());
    let consequent_html = host.gen_children(&node.consequent);
    let alternate_html = node
        .alternate
        .as_ref()
        .map(|c| host.gen_children(c))
        .unwrap_or_default();

    let if_id = host.next_id("if");
    let else_id = host.next_id("else");

    let if_block = format!(
        "<div id=\"{}\" {}>\n{}\n</div>",
        if_id,
        if_to_data_attr(condition_id),
        indent2(&consequent_html)
    );

    let else_block =
        if !alternate_html.is_empty() {
            format!(
            "<div id=\"{}\" data-vb-else data-vb-if-ref=\"{}\" style=\"display:none\">\n{}\n</div>",
            else_id, if_id, indent2(&alternate_html)
        )
        } else {
            String::new()
        };

    [if_block, else_block]
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn gen_switch(node: &SwitchNode, host: &mut dyn ElementCodegenHost) -> String {
    let subject_id = crate::codegen::expr::register_expr(node.subject.clone());
    let switch_id = host.next_id("switch");

    let mut html = format!(
        "<div id=\"{}\" data-vb-switch=\"{}\">\n",
        switch_id, subject_id
    );

    for case in &node.cases {
        let value_id = crate::codegen::expr::register_expr(case.value.clone());
        let body_html = host.gen_children(&case.body);
        let case_id = host.next_id("case");
        html.push_str(&format!(
            "  <div id=\"{}\" data-vb-case=\"{}\" style=\"display:none\">\n{}\n  </div>\n",
            case_id,
            value_id,
            indent(&body_html, 4)
        ));
    }

    if let Some(default_case) = &node.default_case {
        let default_html = host.gen_children(default_case);
        let default_id = host.next_id("default");
        html.push_str(&format!(
            "  <div id=\"{}\" data-vb-default style=\"display:none\">\n{}\n  </div>\n",
            default_id,
            indent(&default_html, 4)
        ));
    }

    html.push_str("</div>");
    html
}

pub struct CompiledLoop {
    pub iterable_expr_id: usize,
    pub item_var: String,
    pub index_var: Option<String>,
}

pub fn compile_loop_node(node: &LoopNode) -> CompiledLoop {
    match &node.kind {
        LoopKind::Each {
            iterable,
            item_var,
            index_var,
        } => CompiledLoop {
            iterable_expr_id: crate::codegen::expr::register_expr(iterable.clone()),
            item_var: item_var.replace('$', ""),
            index_var: index_var.as_ref().map(|v| v.replace('$', "")),
        },
        LoopKind::Range { from, to, var_name } => {
            let items: Vec<vibao_ast::Expr> = (*from..=*to)
                .map(|n| {
                    vibao_ast::Expr::literal_num(n as f64, vibao_ast::Pos { line: 0, column: 0 })
                })
                .collect();
            let array_expr = vibao_ast::Expr::Array(items, vibao_ast::Pos { line: 0, column: 0 });
            CompiledLoop {
                iterable_expr_id: crate::codegen::expr::register_expr(array_expr),
                item_var: var_name.clone(),
                index_var: None,
            }
        }
    }
}

pub fn loop_to_data_attr(compiled: &CompiledLoop, template_id: &str) -> String {
    #[derive(serde::Serialize)]
    struct LoopSpec<'a> {
        iterable_expr_id: usize,
        item_var: &'a str,
        index_var: Option<&'a str>,
        template_id: &'a str,
    }
    let spec = LoopSpec {
        iterable_expr_id: compiled.iterable_expr_id,
        item_var: &compiled.item_var,
        index_var: compiled.index_var.as_deref(),
        template_id,
    };
    let json = serde_json::to_string(&spec).unwrap_or_else(|_| "{}".to_string());
    format!("data-vb-loop=\"{}\"", esc_attr(&json))
}

fn count_dom_producing_children(children: &[Child]) -> usize {
    children
        .iter()
        .filter(|c| {
            !matches!(
                c,
                Child::StateDecl(_) | Child::VarDecl(_) | Child::PageEvent(_)
            )
        })
        .count()
}

pub fn gen_loop(node: &LoopNode, host: &mut dyn ElementCodegenHost) -> String {
    let compiled = compile_loop_node(node);
    let loop_id = host.next_id("loop");
    let template_id = host.next_id("tpl");
    let body_html = host.gen_children(&node.body);

    let dom_child_count = count_dom_producing_children(&node.body);
    if dom_child_count > 1 {
        host.add_warning(format!(
            "vong_lap (dòng {}:{}): thân vòng lặp render ra {} phần tử top-level cho mỗi item (không bọc trong 1 container/box/stack). Vẫn hoạt động đúng, nhưng nên bọc trong 1 container để dễ áp style/layout và debug hơn.",
            node.pos.line, node.pos.column, dom_child_count,
        ));
    }

    for child in &node.body {
        if let Child::ComponentCall(call) = child {
            host.add_warning(format!(
                "vong_lap (dòng {}:{}): component \"{}\" (dòng {}:{}) được gọi TRỰC TIẾP làm phần tử top-level của thân vòng lặp, không có \"neu\"/element nào bao ngoài. Vẫn hoạt động đúng sau bản vá hiện tại, nhưng đây là dạng cấu trúc dễ vỡ nhất trong toàn bộ runtime - xem docs/LIMITATIONS.md. Gợi ý: bọc component trong 1 container (ví dụ `khoi(...) {{ {}(...) }}`) để tránh hoàn toàn dạng cấu trúc này.",
                node.pos.line, node.pos.column,
                call.name, call.pos.line, call.pos.column,
                call.name,
            ));
        }
    }

    let template = format!(
        "<template id=\"{}\">\n{}\n</template>",
        template_id,
        indent2(&body_html)
    );
    host.add_hoisted_template(template);

    let container = format!(
        "<div id=\"{}\" {}></div>",
        loop_id,
        loop_to_data_attr(&compiled, &template_id),
    );

    container
}

#[cfg(test)]
mod tests {
    use super::*;
    use vibao_ast::{BinaryOp, Expr, LiteralValue, Pos};

    fn p() -> Pos {
        Pos { line: 1, column: 1 }
    }

    #[test]
    fn test_if_to_data_attr_uses_numeric_expr_id() {
        let attr = if_to_data_attr(42);
        assert_eq!(attr, "data-vb-if=\"42\"");
    }

    #[test]
    fn test_gen_if_registers_condition_expr_and_no_js_binding() {
        struct FakeHost {
            counter: u32,
            js_calls: Vec<String>,
        }
        impl FakeHost {
            fn new() -> Self {
                FakeHost {
                    counter: 0,
                    js_calls: vec![],
                }
            }
        }
        impl ElementCodegenHost for FakeHost {
            fn next_id(&mut self, tag: &str) -> String {
                self.counter += 1;
                format!("vb-{}-{}", tag, self.counter)
            }
            fn gen_children(&mut self, _children: &[Child]) -> String {
                String::new()
            }
            fn add_css(&mut self, _code: &str) {}
            fn add_media_query(&mut self, _code: &str) {}
            fn add_warning(&mut self, _msg: String) {}
            fn add_hoisted_template(&mut self, _html: String) {}
        }

        let node = IfNode {
            condition: Expr::Binary {
                op: BinaryOp::Gte,
                left: Box::new(Expr::Variable("dem".to_string(), p())),
                right: Box::new(Expr::Literal(LiteralValue::Num(10.0, None), p())),
                pos: p(),
            },
            consequent: vec![],
            alternate: None,
            pos: p(),
        };

        let mut host = FakeHost::new();
        let html = gen_if(&node, &mut host);

        assert!(
            host.js_calls.is_empty(),
            "gen_if must not generate any JS at all (the new architecture uses the expr registry, no JS binding) - actual: {:?}",
            host.js_calls
        );
        assert!(
            html.contains("data-vb-if=\""),
            "the HTML must contain data-vb-if: {}",
            html
        );
        let start = html.find("data-vb-if=\"").unwrap() + "data-vb-if=\"".len();
        let end = html[start..].find('"').unwrap() + start;
        let id_str = &html[start..end];
        assert!(
            id_str.parse::<usize>().is_ok(),
            "data-vb-if must contain an integer (an expr id), actual: \"{}\"",
            id_str
        );
    }

    #[test]
    fn test_gen_if_with_alternate_generates_else_div_as_sibling() {
        struct FakeHost {
            counter: u32,
        }
        impl FakeHost {
            fn new() -> Self {
                FakeHost { counter: 0 }
            }
        }
        impl ElementCodegenHost for FakeHost {
            fn next_id(&mut self, tag: &str) -> String {
                self.counter += 1;
                format!("vb-{}-{}", tag, self.counter)
            }
            fn gen_children(&mut self, children: &[Child]) -> String {
                if children.is_empty() {
                    String::new()
                } else {
                    "<span>alt</span>".to_string()
                }
            }
            fn add_css(&mut self, _code: &str) {}
            fn add_media_query(&mut self, _code: &str) {}
            fn add_warning(&mut self, _msg: String) {}
            fn add_hoisted_template(&mut self, _html: String) {}
        }

        let node = IfNode {
            condition: Expr::Variable("da_gui".to_string(), p()),
            consequent: vec![],
            alternate: Some(vec![Child::StateDecl(vibao_ast::StateDecl {
                name: "khong_dung".to_string(),
                value: Expr::literal_num(0.0, p()),
                pos: p(),
            })]),
            pos: p(),
        };
        let mut host = FakeHost::new();
        let html = gen_if(&node, &mut host);

        let if_pos = html.find("data-vb-if=\"").expect("must have an if div");
        let else_pos = html
            .find("data-vb-else")
            .expect("must have an else div when alternate is present");
        assert!(
            if_pos < else_pos,
            "the if div must appear BEFORE the else div in the HTML: {}",
            html
        );
        assert!(
            html.contains("style=\"display:none\""),
            "else must be hidden by default: {}",
            html
        );
    }

    #[test]
    fn test_compile_loop_each_strips_dollar_sign() {
        let node = LoopNode {
            kind: LoopKind::Each {
                iterable: Expr::Variable("ds".to_string(), p()),
                item_var: "item".to_string(),
                index_var: None,
            },
            body: vec![],
            pos: p(),
        };
        let compiled = compile_loop_node(&node);
        assert_eq!(compiled.item_var, "item");
        assert!(compiled.index_var.is_none());
    }

    #[test]
    fn test_compile_loop_range_desugars_with_i_as_item_var() {
        let node = LoopNode {
            kind: LoopKind::Range {
                from: 1,
                to: 5,
                var_name: "i".to_string(),
            },
            body: vec![],
            pos: p(),
        };
        let compiled = compile_loop_node(&node);
        assert_eq!(compiled.item_var, "i");
        assert!(compiled.index_var.is_none());
    }

    #[test]
    fn test_compile_loop_range_uses_custom_counter_variable_name() {
        let node = LoopNode {
            kind: LoopKind::Range {
                from: 1,
                to: 5,
                var_name: "dem".to_string(),
            },
            body: vec![],
            pos: p(),
        };
        let compiled = compile_loop_node(&node);
        assert_eq!(compiled.item_var, "dem");
    }

    #[test]
    fn test_loop_to_data_attr_each_produces_valid_json_with_registry_id() {
        let node = LoopNode {
            kind: LoopKind::Each {
                iterable: Expr::Variable("ds".to_string(), p()),
                item_var: "item".to_string(),
                index_var: Some("idx".to_string()),
            },
            body: vec![],
            pos: p(),
        };
        let compiled = compile_loop_node(&node);
        let attr = loop_to_data_attr(&compiled, "vb-tpl-1");
        assert!(
            attr.starts_with("data-vb-loop=\""),
            "must use the correct data-vb-loop attribute name: {}",
            attr
        );
        let json_start = attr.find('"').unwrap() + 1;
        let json_raw = &attr[json_start..attr.len() - 1];
        let json_unescaped = json_raw.replace("&quot;", "\"");
        assert!(json_unescaped.contains("\"item_var\":\"item\""));
        assert!(json_unescaped.contains("\"index_var\":\"idx\""));
        assert!(json_unescaped.contains("\"template_id\":\"vb-tpl-1\""));
        assert!(json_unescaped.contains("\"iterable_expr_id\":"));
    }

    #[test]
    fn test_range_loop_desugars_to_each_with_correct_array() {
        let node = LoopNode {
            kind: LoopKind::Range {
                from: 1,
                to: 3,
                var_name: "i".to_string(),
            },
            body: vec![],
            pos: p(),
        };
        let compiled = compile_loop_node(&node);
        assert_eq!(compiled.item_var, "i");
        assert!(compiled.index_var.is_none());

        let attr = loop_to_data_attr(&compiled, "vb-tpl-2");
        let json_unescaped = attr.replace("&quot;", "\"");
        assert!(
            json_unescaped.contains("\"iterable_expr_id\":"),
            "actual attr: {}",
            attr
        );
    }

    struct FakeHost {
        counter: u32,
        warnings: Vec<String>,
    }

    impl FakeHost {
        fn new() -> Self {
            FakeHost {
                counter: 0,
                warnings: vec![],
            }
        }
    }

    impl ElementCodegenHost for FakeHost {
        fn next_id(&mut self, tag: &str) -> String {
            self.counter += 1;
            format!("vb-{}-{}", tag, self.counter)
        }
        fn gen_children(&mut self, _children: &[vibao_ast::Child]) -> String {
            String::new()
        }
        fn add_css(&mut self, _code: &str) {}
        fn add_media_query(&mut self, _code: &str) {}
        fn add_warning(&mut self, msg: String) {
            self.warnings.push(msg);
        }
        fn add_hoisted_template(&mut self, _html: String) {}
    }

    fn dummy_element_child(tag: vibao_ast::Tag) -> Child {
        Child::Element(vibao_ast::Element {
            tag,
            content: None,
            identity: Default::default(),
            blocks: vec![],
            unknown_props: vec![],
            children: vec![],
            events: vec![],
            responsive: vec![],
            animation: Default::default(),
            pos: p(),
        })
    }

    #[test]
    fn test_count_dom_producing_children_ignores_state_and_var_decl() {
        let children = vec![
            Child::StateDecl(vibao_ast::StateDecl {
                name: "x".to_string(),
                value: Expr::literal_num(0.0, p()),
                pos: p(),
            }),
            dummy_element_child(vibao_ast::Tag::Text),
        ];
        assert_eq!(count_dom_producing_children(&children), 1);
    }

    #[test]
    fn test_count_dom_producing_children_counts_multiple_elements() {
        let children = vec![
            dummy_element_child(vibao_ast::Tag::Text),
            dummy_element_child(vibao_ast::Tag::DuongKe),
        ];
        assert_eq!(count_dom_producing_children(&children), 2);
    }

    #[test]
    fn test_gen_loop_warns_when_body_has_multiple_top_level_elements() {
        let node = LoopNode {
            kind: LoopKind::Each {
                iterable: Expr::Variable("ds".to_string(), p()),
                item_var: "item".to_string(),
                index_var: None,
            },
            body: vec![
                dummy_element_child(vibao_ast::Tag::Text),
                dummy_element_child(vibao_ast::Tag::DuongKe),
            ],
            pos: p(),
        };
        let mut host = FakeHost::new();
        gen_loop(&node, &mut host);
        assert_eq!(
            host.warnings.len(),
            1,
            "must produce exactly 1 warning when the loop body has 2 top-level elements"
        );
        assert!(
            host.warnings[0].contains("2 phần tử top-level"),
            "warning content: {}",
            host.warnings[0]
        );
    }

    #[test]
    fn test_gen_loop_no_warning_when_body_has_single_top_level_element() {
        let node = LoopNode {
            kind: LoopKind::Each {
                iterable: Expr::Variable("ds".to_string(), p()),
                item_var: "item".to_string(),
                index_var: None,
            },
            body: vec![dummy_element_child(vibao_ast::Tag::Khoi)],
            pos: p(),
        };
        let mut host = FakeHost::new();
        gen_loop(&node, &mut host);
        assert!(
            host.warnings.is_empty(),
            "should not warn when the loop body has only 1 top-level element (the common, correct case): {:?}",
            host.warnings
        );
    }

    fn dummy_component_call(name: &str) -> Child {
        Child::ComponentCall(vibao_ast::ComponentCall {
            name: name.to_string(),
            props: vec![],
            identity: Default::default(),
            children: vec![],
            pos: p(),
        })
    }

    #[test]
    fn test_gen_loop_warns_when_component_called_directly_with_no_wrapper() {
        let node = LoopNode {
            kind: LoopKind::Each {
                iterable: Expr::Variable("bai_viet".to_string(), p()),
                item_var: "bv".to_string(),
                index_var: None,
            },
            body: vec![dummy_component_call("TheBaiViet")],
            pos: p(),
        };
        let mut host = FakeHost::new();
        gen_loop(&node, &mut host);
        assert_eq!(
            host.warnings.len(),
            1,
            "must produce exactly 1 warning when a component is the loop's sole top-level child: {:?}",
            host.warnings
        );
        assert!(
            host.warnings[0].contains("TheBaiViet"),
            "warning should name the component: {}",
            host.warnings[0]
        );
        assert!(
            host.warnings[0].contains("TRỰC TIẾP"),
            "warning content: {}",
            host.warnings[0]
        );
    }

    #[test]
    fn test_gen_loop_no_warning_when_component_is_wrapped_in_neu() {
        let if_node = IfNode {
            condition: Expr::Variable("bv.da_xuat_ban".to_string(), p()),
            consequent: vec![dummy_component_call("TheBaiViet")],
            alternate: None,
            pos: p(),
        };
        let node = LoopNode {
            kind: LoopKind::Each {
                iterable: Expr::Variable("bai_viet".to_string(), p()),
                item_var: "bv".to_string(),
                index_var: None,
            },
            body: vec![Child::If(Box::new(if_node))],
            pos: p(),
        };
        let mut host = FakeHost::new();
        gen_loop(&node, &mut host);
        assert!(
            host.warnings.is_empty(),
            "should not warn when the component sits inside a neu/If wrapper, not directly: {:?}",
            host.warnings
        );
    }

    #[test]
    fn test_gen_loop_warns_once_per_direct_component_call_not_per_loop() {
        let node = LoopNode {
            kind: LoopKind::Each {
                iterable: Expr::Variable("ds".to_string(), p()),
                item_var: "x".to_string(),
                index_var: None,
            },
            body: vec![dummy_component_call("TheA"), dummy_component_call("TheB")],
            pos: p(),
        };
        let mut host = FakeHost::new();
        gen_loop(&node, &mut host);
        assert_eq!(
            host.warnings.len(),
            3,
            "actual warnings: {:?}",
            host.warnings
        );
        assert!(host.warnings.iter().any(|w| w.contains("TheA")));
        assert!(host.warnings.iter().any(|w| w.contains("TheB")));
    }
}
