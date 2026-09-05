// ============================================================
// VIBAO COMPILER (Rust) — ast/tests.rs
// UNIT TESTS
// ============================================================

use super::child::{get_prop, PropsMap};
use super::expr::{BinaryOp, Expr};
use super::Pos;

#[test]
fn test_expr_pos_extraction() {
    let pos = Pos { line: 3, column: 5 };
    let e = Expr::literal_num(42.0, pos);
    assert_eq!(e.pos(), pos);
}

#[test]
fn test_get_prop_lookup() {
    let pos = Pos { line: 1, column: 1 };
    let props: PropsMap = vec![
        ("mau".to_string(), Expr::literal_str("do", pos)),
        ("co".to_string(), Expr::literal_num(16.0, pos)),
    ];
    assert!(get_prop(&props, "mau").is_some());
    assert!(get_prop(&props, "khong_ton_tai").is_none());
}

#[test]
fn test_nested_expr_via_box() {
    let pos = Pos { line: 1, column: 1 };
    // $n - 1  -> Binary { Sub, Variable("n"), Literal(1) }
    let expr = Expr::Binary {
        op: BinaryOp::Sub,
        left: Box::new(Expr::Variable("n".to_string(), pos)),
        right: Box::new(Expr::literal_num(1.0, pos)),
        pos,
    };
    match expr {
        Expr::Binary { op, .. } => assert_eq!(op, BinaryOp::Sub),
        _ => panic!("wrong node type"),
    }
}
