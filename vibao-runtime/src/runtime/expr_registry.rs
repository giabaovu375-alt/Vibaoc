use std::cell::RefCell;

use vibao_ast::Expr;

thread_local! {
    static REGISTRY: RefCell<Vec<Expr>> = RefCell::new(Vec::new());
}

pub fn load_from_json(json: &str) {
    match serde_json::from_str::<Vec<Expr>>(json) {
        Ok(exprs) => {
            REGISTRY.with(|reg| {
                *reg.borrow_mut() = exprs;
            });
        }
        Err(err) => {
            crate::runtime::log::error(&format!(
                "[ViBao] Failed to parse exprRegistry JSON: {}",
                err
            ));
        }
    }
}

pub fn get(id: usize) -> Option<Expr> {
    REGISTRY.with(|reg| reg.borrow().get(id).cloned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use vibao_ast::Pos;

    #[test]
    fn test_load_and_get_roundtrip() {
        let e = Expr::literal_num(42.0, Pos { line: 1, column: 1 });
        let json = serde_json::to_string(&vec![e]).unwrap();
        load_from_json(&json);
        let fetched = get(0).expect("id 0 must exist after loading");
        match fetched {
            Expr::Literal(vibao_ast::LiteralValue::Num(n, _), _) => assert_eq!(n, 42.0),
            _ => panic!("wrong Expr type after roundtrip"),
        }
    }

    #[test]
    fn test_get_out_of_range_returns_none() {
        load_from_json("[]");
        assert!(get(999).is_none());
    }

    #[test]
    fn test_load_invalid_json_does_not_panic() {
        load_from_json("{ invalid json");
        assert!(get(0).is_none());
    }
}
