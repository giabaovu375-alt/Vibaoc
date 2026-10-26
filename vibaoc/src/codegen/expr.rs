use std::cell::RefCell;

use vibao_ast::{Expr, LiteralValue};

thread_local! {
    static EXPR_REGISTRY: RefCell<Vec<Expr>> = RefCell::new(Vec::new());
}

pub fn register_expr(expr: Expr) -> usize {
    EXPR_REGISTRY.with(|reg| {
        let mut reg = reg.borrow_mut();
        reg.push(expr);
        reg.len() - 1
    })
}

pub fn take_expr_registry() -> Vec<Expr> {
    EXPR_REGISTRY.with(|reg| std::mem::take(&mut *reg.borrow_mut()))
}

pub(crate) fn format_number(n: f64) -> String {
    if n.fract() == 0.0 && n.abs() < 1e15 {
        format!("{}", n as i64)
    } else {
        n.to_string()
    }
}

pub fn json_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            other => out.push(other),
        }
    }
    out.push('"');
    out
}

#[derive(Debug, Clone)]
pub enum ResolvedValue {
    Static(String),
    Size(String),
    Color(String),
    Dynamic,
}

impl ResolvedValue {
    pub fn as_css(&self) -> String {
        match self {
            ResolvedValue::Static(s) => s.clone(),
            ResolvedValue::Size(s) => s.clone(),
            ResolvedValue::Color(s) => s.clone(),
            ResolvedValue::Dynamic => String::new(),
        }
    }

    pub fn is_dynamic(&self) -> bool {
        matches!(self, ResolvedValue::Dynamic)
    }
}

pub fn resolve_value(expr: &Expr) -> ResolvedValue {
    match expr {
        Expr::Literal(LiteralValue::Color(hex), _) => ResolvedValue::Color(hex.clone()),
        Expr::Literal(LiteralValue::Num(n, Some(unit)), _) => {
            ResolvedValue::Size(format!("{}{}", format_number(*n), unit))
        }
        Expr::Literal(LiteralValue::Num(n, None), _) => {
            ResolvedValue::Size(format!("{}px", format_number(*n)))
        }
        Expr::Literal(LiteralValue::Str(s), _) => ResolvedValue::Static(s.clone()),
        Expr::Literal(LiteralValue::Bool(b), _) => ResolvedValue::Static(b.to_string()),
        Expr::Variable(name, _) if super::theme::is_theme_variable(name) => {
            ResolvedValue::Static(format!("var({})", super::theme::css_var_name(name)))
        }
        _ => ResolvedValue::Dynamic,
    }
}

pub fn get_static_value(expr: &Expr) -> String {
    match expr {
        Expr::Literal(LiteralValue::Str(s), _) => s.clone(),
        Expr::Literal(LiteralValue::Num(n, Some(unit)), _) => format!("{}{}", format_number(*n), unit),
        Expr::Literal(LiteralValue::Num(n, None), _) => format_number(*n),
        Expr::Literal(LiteralValue::Bool(b), _) => b.to_string(),
        Expr::Literal(LiteralValue::Color(c), _) => c.clone(),
        Expr::Variable(name, _) if super::theme::is_theme_variable(name) => {
            format!("var({})", super::theme::css_var_name(name))
        }
        _ => "__dynamic__".to_string(),
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
    fn test_number_formatting_no_trailing_zero() {
        assert_eq!(format_number(5.0), "5");
        assert_eq!(format_number(5.5), "5.5");
    }

    #[test]
    fn test_json_string_escapes_quotes() {
        assert_eq!(json_string("a\"b"), "\"a\\\"b\"");
    }

    #[test]
    fn test_resolve_value_color_literal() {
        let e = Expr::Literal(LiteralValue::Color("#FFFFFF".to_string()), p());
        match resolve_value(&e) {
            ResolvedValue::Color(c) => assert_eq!(c, "#FFFFFF"),
            _ => panic!("must be Color"),
        }
    }

    #[test]
    fn test_resolve_value_number_gets_px() {
        let e = Expr::literal_num(16.0, p());
        match resolve_value(&e) {
            ResolvedValue::Size(s) => assert_eq!(s, "16px"),
            _ => panic!("must be Size"),
        }
    }

    #[test]
    fn test_resolve_value_variable_is_dynamic() {
        let e = Expr::Variable("n".to_string(), p());
        assert!(resolve_value(&e).is_dynamic());
    }

    #[test]
    fn test_get_static_value_dynamic_sentinel() {
        let e = Expr::Variable("n".to_string(), p());
        assert_eq!(get_static_value(&e), "__dynamic__");
    }

    fn kich_hoat_theme_voi_bien(ten_bien: &str) {
        let theme = vibao_ast::Theme {
            name: "sang".to_string(),
            variables: vec![vibao_ast::VarDecl {
                name: ten_bien.to_string(),
                value: Expr::Literal(LiteralValue::Color("#FFFFFF".to_string()), p()),
                pos: p(),
            }],
            pos: p(),
        };
        super::super::theme::activate(&[theme]);
    }

    #[test]
    fn bien_theme_duoc_phan_giai_tinh_thanh_var_css() {
        kich_hoat_theme_voi_bien("nen");
        let e = Expr::Variable("nen".to_string(), p());
        let resolved = resolve_value(&e);
        assert!(!resolved.is_dynamic());
        assert_eq!(resolved.as_css(), "var(--nen)");
        assert_eq!(get_static_value(&e), "var(--nen)");
    }

    #[test]
    fn bien_state_khac_ten_van_la_dong_khi_co_theme() {
        kich_hoat_theme_voi_bien("nen");
        let e = Expr::Variable("dem".to_string(), p());
        assert!(resolve_value(&e).is_dynamic());
        assert_eq!(get_static_value(&e), "__dynamic__");
    }

    #[test]
    fn truy_cap_thuoc_tinh_cua_ten_trung_bien_theme_van_la_dong() {
        kich_hoat_theme_voi_bien("nen");
        let e = Expr::Variable("nen.con".to_string(), p());
        assert!(resolve_value(&e).is_dynamic());
    }

    #[test]
    fn khong_co_theme_thi_moi_bien_deu_dong() {
        super::super::theme::activate(&[]);
        let e = Expr::Variable("nen".to_string(), p());
        assert!(resolve_value(&e).is_dynamic());
    }

    #[test]
    fn test_registry_assigns_sequential_ids() {
        let e1 = Expr::literal_num(1.0, p());
        let e2 = Expr::literal_num(2.0, p());
        let id1 = register_expr(e1);
        let id2 = register_expr(e2);
        assert_eq!(id2, id1 + 1);
    }

    #[test]
    fn test_take_expr_registry_drains_and_resets() {
        take_expr_registry();

        register_expr(Expr::literal_num(1.0, p()));
        register_expr(Expr::literal_num(2.0, p()));

        let drained = take_expr_registry();
        assert_eq!(drained.len(), 2);

        let empty = take_expr_registry();
        assert_eq!(empty.len(), 0);
    }
}
