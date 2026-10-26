use vibao_ast::*;

fn p() -> Pos {
    Pos { line: 1, column: 1 }
}

fn roundtrip<T: serde::Serialize + serde::de::DeserializeOwned>(value: &T) -> T {
    let json = serde_json::to_string(value).expect("serialization must succeed");
    serde_json::from_str(&json).unwrap_or_else(|e| {
        panic!(
            "deserialization must succeed for the exact JSON the serializer produced.\nJSON: {}\nError: {}",
            json, e
        )
    })
}

#[test]
fn test_expr_literal_variants_roundtrip() {
    let exprs = vec![
        Expr::literal_num(42.0, p()),
        Expr::literal_num_with_unit(50.0, Some("%".to_string()), p()),
        Expr::literal_str("hello".to_string(), p()),
        Expr::Literal(LiteralValue::Bool(true), p()),
        Expr::Literal(LiteralValue::Color("#FF0000".to_string()), p()),
    ];
    for e in exprs {
        let back = roundtrip(&e);
        assert_eq!(
            format!("{:?}", e),
            format!("{:?}", back),
            "literal expr must round-trip identically: {:?}",
            e
        );
    }
}

#[test]
fn test_expr_variable_roundtrips() {
    let e = Expr::Variable("ten_bien".to_string(), p());
    let back = roundtrip(&e);
    match back {
        Expr::Variable(name, _) => assert_eq!(name, "ten_bien"),
        other => panic!("expected Variable, got {:?}", other),
    }
}

#[test]
fn test_expr_member_access_roundtrips() {
    let e = Expr::MemberAccess {
        object: Box::new(Expr::Variable("obj".to_string(), p())),
        property: "field".to_string(),
        pos: p(),
    };
    let back = roundtrip(&e);
    match back {
        Expr::MemberAccess { property, .. } => assert_eq!(property, "field"),
        other => panic!("expected MemberAccess, got {:?}", other),
    }
}

#[test]
fn test_expr_binary_and_unary_roundtrip() {
    let bin = Expr::Binary {
        op: BinaryOp::Add,
        left: Box::new(Expr::literal_num(1.0, p())),
        right: Box::new(Expr::literal_num(2.0, p())),
        pos: p(),
    };
    let back = roundtrip(&bin);
    match back {
        Expr::Binary { op, .. } => assert_eq!(op, BinaryOp::Add),
        other => panic!("expected Binary, got {:?}", other),
    }

    let un = Expr::Unary {
        op: UnaryOp::Not,
        operand: Box::new(Expr::Variable("x".to_string(), p())),
        pos: p(),
    };
    let back = roundtrip(&un);
    match back {
        Expr::Unary { op, .. } => assert_eq!(op, UnaryOp::Not),
        other => panic!("expected Unary, got {:?}", other),
    }
}

#[test]
fn test_every_binary_op_variant_roundtrips() {
    let ops = [
        BinaryOp::Add,
        BinaryOp::Sub,
        BinaryOp::Mul,
        BinaryOp::Div,
        BinaryOp::Mod,
        BinaryOp::Eq,
        BinaryOp::Neq,
        BinaryOp::Gt,
        BinaryOp::Gte,
        BinaryOp::Lt,
        BinaryOp::Lte,
        BinaryOp::And,
        BinaryOp::Or,
    ];
    for op in ops {
        let e = Expr::Binary {
            op,
            left: Box::new(Expr::literal_num(1.0, p())),
            right: Box::new(Expr::literal_num(2.0, p())),
            pos: p(),
        };
        let back = roundtrip(&e);
        match back {
            Expr::Binary { op: back_op, .. } => {
                assert_eq!(
                    back_op, op,
                    "BinaryOp::{:?} did not round-trip correctly",
                    op
                );
            }
            other => panic!("expected Binary, got {:?}", other),
        }
    }
}

#[test]
fn test_expr_call_roundtrips() {
    let e = Expr::Call {
        callee: "gia_tien".to_string(),
        args: vec![Expr::Variable("gia".to_string(), p())],
        pos: p(),
    };
    let back = roundtrip(&e);
    match back {
        Expr::Call { callee, args, .. } => {
            assert_eq!(callee, "gia_tien");
            assert_eq!(args.len(), 1);
        }
        other => panic!("expected Call, got {:?}", other),
    }
}

#[test]
fn test_expr_color_func_roundtrips() {
    let e = Expr::ColorFunc {
        func: ColorFuncKind::TrongSuot,
        color: Box::new(Expr::Literal(
            LiteralValue::Color("#000000".to_string()),
            p(),
        )),
        amount: 50.0,
        pos: p(),
    };
    let back = roundtrip(&e);
    match back {
        Expr::ColorFunc { func, amount, .. } => {
            assert_eq!(func, ColorFuncKind::TrongSuot);
            assert_eq!(amount, 50.0);
        }
        other => panic!("expected ColorFunc, got {:?}", other),
    }
}

#[test]
fn test_expr_array_and_object_roundtrip() {
    let arr = Expr::Array(
        vec![Expr::literal_num(1.0, p()), Expr::literal_num(2.0, p())],
        p(),
    );
    let back = roundtrip(&arr);
    match back {
        Expr::Array(items, _) => assert_eq!(items.len(), 2),
        other => panic!("expected Array, got {:?}", other),
    }

    let obj = Expr::Object(
        vec![(
            "ten".to_string(),
            Expr::literal_str("test".to_string(), p()),
        )],
        p(),
    );
    let back = roundtrip(&obj);
    match back {
        Expr::Object(fields, _) => {
            assert_eq!(fields.len(), 1);
            assert_eq!(fields[0].0, "ten");
        }
        other => panic!("expected Object, got {:?}", other),
    }
}

#[test]
fn test_expr_template_string_roundtrips() {
    let e = Expr::TemplateString(
        vec![
            TemplatePart::Text("Xin chao ".to_string()),
            TemplatePart::Variable("ten".to_string()),
            TemplatePart::Member(vec!["item".to_string(), "tuoi".to_string()]),
        ],
        p(),
    );
    let back = roundtrip(&e);
    match back {
        Expr::TemplateString(parts, _) => assert_eq!(parts.len(), 3),
        other => panic!("expected TemplateString, got {:?}", other),
    }
}

#[test]
fn test_nested_expr_roundtrips() {
    let e = Expr::Binary {
        op: BinaryOp::Add,
        left: Box::new(Expr::Variable("a".to_string(), p())),
        right: Box::new(Expr::Call {
            callee: "gia_tien".to_string(),
            args: vec![Expr::Variable("b".to_string(), p())],
            pos: p(),
        }),
        pos: p(),
    };
    let back = roundtrip(&e);
    match back {
        Expr::Binary { op, right, .. } => {
            assert_eq!(op, BinaryOp::Add);
            assert!(matches!(*right, Expr::Call { .. }));
        }
        other => panic!("expected Binary, got {:?}", other),
    }
}

#[test]
fn test_action_assign_roundtrips() {
    let a = Action::Assign {
        target: "dem".to_string(),
        value: Expr::literal_num(0.0, p()),
        pos: p(),
    };
    let back = roundtrip(&a);
    match back {
        Action::Assign { target, .. } => assert_eq!(target, "dem"),
        other => panic!("expected Assign, got {:?}", other),
    }
}

#[test]
fn test_action_function_call_roundtrips() {
    let a = Action::FunctionCall {
        name: "thong_bao".to_string(),
        args: vec![Expr::literal_str("Xin chao".to_string(), p())],
        opts: vec![],
        assign_to: None,
        pos: p(),
    };
    let back = roundtrip(&a);
    match back {
        Action::FunctionCall { name, args, .. } => {
            assert_eq!(name, "thong_bao");
            assert_eq!(args.len(), 1);
        }
        other => panic!("expected FunctionCall, got {:?}", other),
    }
}

#[test]
fn test_action_if_action_roundtrips() {
    let a = Action::IfAction {
        condition: Expr::Variable("dang_nhap".to_string(), p()),
        consequent: vec![Action::Assign {
            target: "x".to_string(),
            value: Expr::literal_num(1.0, p()),
            pos: p(),
        }],
        alternate: None,
        pos: p(),
    };
    let back = roundtrip(&a);
    match back {
        Action::IfAction { consequent, .. } => assert_eq!(consequent.len(), 1),
        other => panic!("expected IfAction, got {:?}", other),
    }
}

#[test]
fn test_action_api_call_with_nested_callbacks_roundtrips() {
    let a = Action::ApiCall {
        method: "GET".to_string(),
        endpoint: Expr::literal_str("/health".to_string(), p()),
        data: None,
        headers: Some(Expr::Object(
            vec![(
                "Authorization".to_string(),
                Expr::literal_str("Bearer k".to_string(), p()),
            )],
            p(),
        )),
        background: true,
        on_success: Some(vec![Action::Assign {
            target: "status".to_string(),
            value: Expr::literal_str("ok".to_string(), p()),
            pos: p(),
        }]),
        on_failure: Some(vec![Action::FunctionCall {
            name: "thong_bao".to_string(),
            args: vec![],
            opts: vec![],
            assign_to: None,
            pos: p(),
        }]),
        assign_to: None,
        pos: p(),
    };
    let back = roundtrip(&a);
    match back {
        Action::ApiCall {
            headers,
            background,
            on_success,
            on_failure,
            ..
        } => {
            assert!(headers.is_some(), "headers must survive round-trip");
            assert!(background, "background must survive round-trip");
            assert_eq!(
                on_success.map(|v| v.len()),
                Some(1),
                "nested on_success actions must survive round-trip"
            );
            assert_eq!(
                on_failure.map(|v| v.len()),
                Some(1),
                "nested on_failure actions must survive round-trip"
            );
        }
        other => panic!("expected ApiCall, got {:?}", other),
    }
}

#[test]
fn test_action_sequence_roundtrips_as_registered() {
    let actions = vec![
        Action::Assign {
            target: "dem".to_string(),
            value: Expr::Binary {
                op: BinaryOp::Add,
                left: Box::new(Expr::Variable("dem".to_string(), p())),
                right: Box::new(Expr::literal_num(1.0, p())),
                pos: p(),
            },
            pos: p(),
        },
        Action::FunctionCall {
            name: "thong_bao".to_string(),
            args: vec![Expr::literal_str("Da cap nhat".to_string(), p())],
            opts: vec![],
            assign_to: None,
            pos: p(),
        },
    ];
    let json = serde_json::to_string(&actions).expect("serialization must succeed");
    let back: Vec<Action> = serde_json::from_str(&json).expect("deserialization must succeed");
    assert_eq!(
        back.len(),
        2,
        "the full action sequence must survive round-trip intact"
    );
}
