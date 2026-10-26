use vibao_ast::{BinaryOp, ColorFuncKind, Expr, LiteralValue, TemplatePart, UnaryOp};

use super::state::{SharedState, State};
use super::value::VbValue;

pub fn eval_tracked(shared: &SharedState, expr: &Expr) -> VbValue {
    let mut state = shared.borrow_mut();
    eval_inner(&mut state, expr, true)
}

pub fn eval(shared: &SharedState, expr: &Expr) -> VbValue {
    let mut state = shared.borrow_mut();
    eval_inner(&mut state, expr, false)
}

pub fn eval_with_loop_frame(
    shared: &SharedState,
    expr: &Expr,
    loop_frame: Option<&super::state::LoopFrame>,
    component_id: Option<&str>,
) -> VbValue {
    let _guard = super::state::ScopeGuard::new(shared, loop_frame, component_id);

    let mut state = shared.borrow_mut();
    eval_inner(&mut state, expr, false)
}

pub fn eval_readonly(state: &State, expr: &Expr) -> VbValue {
    eval_readonly_with_frame(state, expr, None)
}

pub fn eval_readonly_with_frame(state: &State, expr: &Expr, extra_frame: Option<&super::state::LoopFrame>) -> VbValue {
    match expr {
        Expr::Literal(lit, _) => eval_literal(lit),
        Expr::Variable(name, _) => resolve_readonly_var(state, name, extra_frame),
        Expr::MemberAccess { object, property, .. } => {
            let obj = eval_readonly_with_frame(state, object, extra_frame);
            obj.get_field(property)
        }
        Expr::Binary { op, left, right, .. } => {
            let l = eval_readonly_with_frame(state, left, extra_frame);
            let r = eval_readonly_with_frame(state, right, extra_frame);
            eval_binary(*op, &l, &r)
        }
        Expr::Unary { op, operand, .. } => {
            let o = eval_readonly_with_frame(state, operand, extra_frame);
            match op {
                UnaryOp::Not => VbValue::Bool(!o.is_truthy()),
                UnaryOp::Neg => VbValue::Num(-o.to_num_or_zero()),
            }
        }
        Expr::Call { callee, args, .. } => {
            let arg_values: Vec<VbValue> = args.iter().map(|a| eval_readonly_with_frame(state, a, extra_frame)).collect();
            eval_call(callee, &arg_values)
        }
        Expr::ColorFunc { func, color, amount, .. } => {
            let color_val = eval_readonly_with_frame(state, color, extra_frame);
            eval_color_func(*func, &color_val, *amount)
        }
        Expr::Array(items, _) => {
            let values: Vec<VbValue> = items.iter().map(|e| eval_readonly_with_frame(state, e, extra_frame)).collect();
            VbValue::Array(values)
        }
        Expr::Object(fields, _) => {
            let entries: Vec<(String, VbValue)> = fields
                .iter()
                .map(|(k, v)| (k.clone(), eval_readonly_with_frame(state, v, extra_frame)))
                .collect();
            VbValue::object(entries)
        }
        Expr::TemplateString(parts, _) => eval_template_readonly(state, parts, extra_frame),
    }
}

fn resolve_readonly_var(state: &State, name: &str, extra_frame: Option<&super::state::LoopFrame>) -> VbValue {
    if let Some(frame) = extra_frame {
        if frame.item_var == name {
            return frame.item_value.clone();
        }
        if let Some(idx_var) = &frame.index_var {
            if idx_var == name {
                return frame.index_value.map(VbValue::Num).unwrap_or(VbValue::Null);
            }
        }
        let prefix = format!("{}.", frame.item_var);
        if let Some(sub_path) = name.strip_prefix(&prefix) {
            return frame.item_value.dig_path(sub_path);
        }
    }
    state.scope_resolve(name)
}

fn eval_template_readonly(state: &State, parts: &[TemplatePart], extra_frame: Option<&super::state::LoopFrame>) -> VbValue {
    let mut out = String::new();
    for part in parts {
        match part {
            TemplatePart::Text(text) => out.push_str(text),
            TemplatePart::Variable(name) => {
                out.push_str(&resolve_readonly_var(state, name, extra_frame).to_string());
            }
            TemplatePart::Member(path) => {
                out.push_str(&state.get_path(&path.join(".")).to_string());
            }
        }
    }
    VbValue::Str(out)
}

fn eval_inner(state: &mut State, expr: &Expr, tracked: bool) -> VbValue {
    match expr {
        Expr::Literal(lit, _) => eval_literal(lit),

        Expr::Variable(name, _) => {
            if tracked {
                state.scope_resolve_tracked(name)
            } else {
                state.scope_resolve(name)
            }
        }

        Expr::MemberAccess { object, property, .. } => {
            let obj = eval_inner(state, object, tracked);
            obj.get_field(property)
        }

        Expr::Binary { op, left, right, .. } => {
            let l = eval_inner(state, left, tracked);
            let r = eval_inner(state, right, tracked);
            eval_binary(*op, &l, &r)
        }

        Expr::Unary { op, operand, .. } => {
            let o = eval_inner(state, operand, tracked);
            match op {
                UnaryOp::Not => VbValue::Bool(!o.is_truthy()),
                UnaryOp::Neg => VbValue::Num(-o.to_num_or_zero()),
            }
        }

        Expr::Call { callee, args, .. } => {
            let arg_values: Vec<VbValue> =
                args.iter().map(|a| eval_inner(state, a, tracked)).collect();
            eval_call(callee, &arg_values)
        }

        Expr::ColorFunc { func, color, amount, .. } => {
            let color_val = eval_inner(state, color, tracked);
            eval_color_func(*func, &color_val, *amount)
        }

        Expr::Array(items, _) => {
            let values: Vec<VbValue> =
                items.iter().map(|e| eval_inner(state, e, tracked)).collect();
            VbValue::Array(values)
        }

        Expr::Object(fields, _) => {
            let entries: Vec<(String, VbValue)> = fields
                .iter()
                .map(|(k, v)| (k.clone(), eval_inner(state, v, tracked)))
                .collect();
            VbValue::object(entries)
        }

        Expr::TemplateString(parts, _) => eval_template(state, parts, tracked),
    }
}

fn eval_literal(lit: &LiteralValue) -> VbValue {
    match lit {
        LiteralValue::Num(n, _unit) => VbValue::Num(*n),
        LiteralValue::Str(s) => VbValue::Str(s.clone()),
        LiteralValue::Bool(b) => VbValue::Bool(*b),
        LiteralValue::Color(c) => VbValue::Str(c.clone()),
    }
}

fn eval_binary(op: BinaryOp, l: &VbValue, r: &VbValue) -> VbValue {
    match op {
        BinaryOp::Add => eval_add(l, r),
        BinaryOp::Sub => VbValue::Num(l.to_num_or_zero() - r.to_num_or_zero()),
        BinaryOp::Mul => VbValue::Num(l.to_num_or_zero() * r.to_num_or_zero()),
        BinaryOp::Div => VbValue::Num(l.to_num_or_zero() / r.to_num_or_zero()),
        BinaryOp::Mod => VbValue::Num(l.to_num_or_zero() % r.to_num_or_zero()),
        BinaryOp::Eq => VbValue::Bool(l.strict_eq(r)),
        BinaryOp::Neq => VbValue::Bool(!l.strict_eq(r)),
        BinaryOp::Gt => VbValue::Bool(l.partial_cmp_loose(r) == std::cmp::Ordering::Greater),
        BinaryOp::Gte => VbValue::Bool(
            !l.is_nan_like(r)
                && matches!(l.partial_cmp_loose(r), std::cmp::Ordering::Greater | std::cmp::Ordering::Equal),
        ),
        BinaryOp::Lt => VbValue::Bool(l.partial_cmp_loose(r) == std::cmp::Ordering::Less),
        BinaryOp::Lte => VbValue::Bool(
            !l.is_nan_like(r)
                && matches!(l.partial_cmp_loose(r), std::cmp::Ordering::Less | std::cmp::Ordering::Equal),
        ),
        BinaryOp::And => VbValue::Bool(l.is_truthy() && r.is_truthy()),
        BinaryOp::Or => VbValue::Bool(l.is_truthy() || r.is_truthy()),
    }
}

fn eval_add(l: &VbValue, r: &VbValue) -> VbValue {
    if let (Some(a), Some(b)) = (coerce_add_number(l), coerce_add_number(r)) {
        return VbValue::Num(a + b);
    }
    VbValue::Str(format!("{}{}", l, r))
}

fn coerce_add_number(v: &VbValue) -> Option<f64> {
    match v {
        VbValue::Num(n) => Some(*n),
        VbValue::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
        VbValue::Null => Some(0.0),
        VbValue::Str(s) => s.trim().parse::<f64>().ok(),
        VbValue::Array(_) | VbValue::Object(_) => None,
    }
}

fn eval_call(callee: &str, args: &[VbValue]) -> VbValue {
    match callee {
        "lam_tron" => VbValue::Num(args.first().map(|v| v.to_num_or_zero()).unwrap_or(0.0).round()),
        "phan_tram" => {
            let n = args.first().map(|v| v.to_num_or_zero()).unwrap_or(0.0);
            VbValue::Str(format!("{}%", format_number_trim(n)))
        }
        "hoa_chu" => {
            let s = args.first().and_then(|v| v.as_str()).unwrap_or("").to_string();
            VbValue::Str(s.to_uppercase())
        }
        "rut_gon" => {
            let s = args.first().and_then(|v| v.as_str()).unwrap_or("");
            let max_len = args.get(1).map(|v| v.to_num_or_zero() as usize).unwrap_or(20);
            if s.chars().count() > max_len {
                let truncated: String = s.chars().take(max_len).collect();
                VbValue::Str(format!("{}...", truncated))
            } else {
                VbValue::Str(s.to_string())
            }
        }
        "gia_tien" => {
            let n = args.first().map(|v| v.to_num_or_zero()).unwrap_or(0.0);
            VbValue::Str(format!("{}đ", group_thousands(n as i64)))
        }
        "ngay" => {
            let s = args.first().and_then(|v| v.as_str()).unwrap_or("");
            format_iso_date(s)
        }
        _ => VbValue::Null,
    }
}

fn format_number_trim(n: f64) -> String {
    if n.fract() == 0.0 && n.is_finite() {
        format!("{}", n as i64)
    } else {
        n.to_string()
    }
}

fn group_thousands(n: i64) -> String {
    let neg = n < 0;
    let s = n.unsigned_abs().to_string();
    let mut out = String::new();
    for (i, ch) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            out.push('.');
        }
        out.push(ch);
    }
    let grouped: String = out.chars().rev().collect();
    if neg {
        format!("-{}", grouped)
    } else {
        grouped
    }
}

fn format_iso_date(s: &str) -> VbValue {
    let date_part = s.split('T').next().unwrap_or(s);
    let parts: Vec<&str> = date_part.split('-').collect();
    if parts.len() == 3 {
        VbValue::Str(format!("{}/{}/{}", parts[2], parts[1], parts[0]))
    } else {
        VbValue::Str(s.to_string())
    }
}

fn eval_color_func(func: ColorFuncKind, color: &VbValue, amount: f64) -> VbValue {
    let hex = match color.as_str() {
        Some(s) => s,
        None => return color.clone(),
    };
    let (r, g, b) = match parse_hex_rgb(hex) {
        Some(rgb) => rgb,
        None => return color.clone(),
    };
    let amount = amount.clamp(0.0, 100.0);

    match func {
        ColorFuncKind::TrongSuot => {
            let alpha = 1.0 - amount / 100.0;
            VbValue::Str(format!("rgba({}, {}, {}, {})", r, g, b, alpha))
        }
        ColorFuncKind::LamSang => {
            let f = amount / 100.0;
            let nr = lerp_to(r, 255, f);
            let ng = lerp_to(g, 255, f);
            let nb = lerp_to(b, 255, f);
            VbValue::Str(format_hex(nr, ng, nb))
        }
        ColorFuncKind::LamToi => {
            let f = amount / 100.0;
            let nr = lerp_to(r, 0, f);
            let ng = lerp_to(g, 0, f);
            let nb = lerp_to(b, 0, f);
            VbValue::Str(format_hex(nr, ng, nb))
        }
    }
}

fn parse_hex_rgb(hex: &str) -> Option<(u8, u8, u8)> {
    let h = hex.strip_prefix('#').unwrap_or(hex);
    if h.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&h[0..2], 16).ok()?;
    let g = u8::from_str_radix(&h[2..4], 16).ok()?;
    let b = u8::from_str_radix(&h[4..6], 16).ok()?;
    Some((r, g, b))
}

fn lerp_to(from: u8, to: u8, f: f64) -> u8 {
    let from = from as f64;
    let to = to as f64;
    (from + (to - from) * f).round().clamp(0.0, 255.0) as u8
}

fn format_hex(r: u8, g: u8, b: u8) -> String {
    format!("#{:02X}{:02X}{:02X}", r, g, b)
}

fn eval_template(state: &mut State, parts: &[TemplatePart], tracked: bool) -> VbValue {
    let mut out = String::new();
    for part in parts {
        match part {
            TemplatePart::Text(text) => out.push_str(text),
            TemplatePart::Variable(name) => {
                let v = if tracked {
                    state.scope_resolve_tracked(name)
                } else {
                    state.scope_resolve(name)
                };
                out.push_str(&v.to_string());
            }
            TemplatePart::Member(path) => {
                let full_path = path.join(".");
                let v = if tracked {
                    let mut parts_iter = path.iter();
                    let root_name = match parts_iter.next() {
                        Some(p) => p,
                        None => continue,
                    };
                    let mut cur = state.scope_resolve_tracked(root_name);
                    for p in parts_iter {
                        if cur.is_null() {
                            break;
                        }
                        cur = cur.get_field(p);
                    }
                    cur
                } else {
                    state.get_path(&full_path)
                };
                out.push_str(&v.to_string());
            }
        }
    }
    VbValue::Str(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use vibao_ast::Pos;

    fn p() -> Pos {
        Pos { line: 1, column: 1 }
    }

    fn setup() -> SharedState {
        super::super::state::new_shared_state()
    }

    #[test]
    fn test_eval_literal_number() {
        let shared = setup();
        let e = Expr::literal_num(42.0, p());
        assert_eq!(eval(&shared, &e).as_num(), Some(42.0));
    }

    #[test]
    fn test_eval_variable_reads_global_state() {
        let shared = setup();
        shared.borrow_mut().set_state("dem", VbValue::num(5.0));
        let e = Expr::Variable("dem".to_string(), p());
        assert_eq!(eval(&shared, &e).as_num(), Some(5.0));
    }

    #[test]
    fn test_eval_binary_add_numbers() {
        let shared = setup();
        let e = Expr::Binary {
            op: BinaryOp::Add,
            left: Box::new(Expr::literal_num(1.0, p())),
            right: Box::new(Expr::literal_num(2.0, p())),
            pos: p(),
        };
        assert_eq!(eval(&shared, &e).as_num(), Some(3.0));
    }

    #[test]
    fn test_eval_binary_add_string_concat() {
        let shared = setup();
        let e = Expr::Binary {
            op: BinaryOp::Add,
            left: Box::new(Expr::literal_str("a", p())),
            right: Box::new(Expr::literal_str("b", p())),
            pos: p(),
        };
        assert_eq!(eval(&shared, &e).as_str(), Some("ab"));
    }

    #[test]
    fn test_eval_binary_add_numeric_string_as_number() {
        let shared = setup();
        let e = Expr::Binary {
            op: BinaryOp::Add,
            left: Box::new(Expr::literal_str("4", p())),
            right: Box::new(Expr::literal_num(1.0, p())),
            pos: p(),
        };
        assert_eq!(eval(&shared, &e).as_num(), Some(5.0));
    }

    #[test]
    fn test_button_only_undeclared_counter_adds_numerically() {
        let shared = setup();
        let value_expr = Expr::Binary {
            op: BinaryOp::Add,
            left: Box::new(Expr::Variable("count".to_string(), p())),
            right: Box::new(Expr::literal_num(1.0, p())),
            pos: p(),
        };

        for expected in [1.0, 2.0, 3.0] {
            let v = eval(&shared, &value_expr);
            shared.borrow_mut().set_state("count", v);
            assert_eq!(shared.borrow().peek_state("count"), VbValue::Num(expected));
        }
    }

    #[test]
    fn test_eval_binary_add_non_numeric_string_still_concats() {
        let shared = setup();
        let e = Expr::Binary {
            op: BinaryOp::Add,
            left: Box::new(Expr::literal_str("count: ", p())),
            right: Box::new(Expr::literal_num(1.0, p())),
            pos: p(),
        };
        assert_eq!(eval(&shared, &e).as_str(), Some("count: 1"));
    }

    #[test]
    fn test_eval_binary_eq_strict() {
        let shared = setup();
        let e = Expr::Binary {
            op: BinaryOp::Eq,
            left: Box::new(Expr::literal_num(1.0, p())),
            right: Box::new(Expr::literal_num(1.0, p())),
            pos: p(),
        };
        assert_eq!(eval(&shared, &e), VbValue::Bool(true));
    }

    #[test]
    fn test_eval_unary_not() {
        let shared = setup();
        let e = Expr::Unary {
            op: UnaryOp::Not,
            operand: Box::new(Expr::literal_bool(false, p())),
            pos: p(),
        };
        assert_eq!(eval(&shared, &e), VbValue::Bool(true));
    }

    #[test]
    fn test_eval_member_access_rong_do_dai() {
        let shared = setup();
        shared.borrow_mut().set_state(
            "ds",
            VbValue::Array(vec![VbValue::num(1.0), VbValue::num(2.0)]),
        );
        let base = Expr::Variable("ds".to_string(), p());
        let e = Expr::MemberAccess {
            object: Box::new(base),
            property: "do_dai".to_string(),
            pos: p(),
        };
        assert_eq!(eval(&shared, &e).as_num(), Some(2.0));
    }

    #[test]
    fn test_eval_call_lam_tron() {
        let shared = setup();
        let e = Expr::Call {
            callee: "lam_tron".to_string(),
            args: vec![Expr::literal_num(3.7, p())],
            pos: p(),
        };
        assert_eq!(eval(&shared, &e).as_num(), Some(4.0));
    }

    #[test]
    fn test_eval_call_gia_tien_groups_thousands() {
        let shared = setup();
        let e = Expr::Call {
            callee: "gia_tien".to_string(),
            args: vec![Expr::literal_num(1234567.0, p())],
            pos: p(),
        };
        assert_eq!(eval(&shared, &e).as_str(), Some("1.234.567đ"));
    }

    #[test]
    fn test_eval_color_func_lam_sang() {
        let shared = setup();
        let e = Expr::ColorFunc {
            func: ColorFuncKind::LamSang,
            color: Box::new(Expr::Literal(LiteralValue::Color("#000000".to_string()), p())),
            amount: 100.0,
            pos: p(),
        };
        assert_eq!(eval(&shared, &e).as_str(), Some("#FFFFFF"));
    }

    #[test]
    fn test_eval_color_func_trong_suot() {
        let shared = setup();
        let e = Expr::ColorFunc {
            func: ColorFuncKind::TrongSuot,
            color: Box::new(Expr::Literal(LiteralValue::Color("#FF0000".to_string()), p())),
            amount: 50.0,
            pos: p(),
        };
        assert_eq!(eval(&shared, &e).as_str(), Some("rgba(255, 0, 0, 0.5)"));
    }

    #[test]
    fn test_eval_template_string() {
        let shared = setup();
        shared.borrow_mut().set_state("ten", VbValue::str("An"));
        let parts = vec![
            TemplatePart::Text("Xin chào ".to_string()),
            TemplatePart::Variable("ten".to_string()),
        ];
        let e = Expr::TemplateString(parts, p());
        assert_eq!(eval(&shared, &e).as_str(), Some("Xin chào An"));
    }

    #[test]
    fn test_eval_tracked_registers_dependency() {
        use super::super::state::{flush, subscribe};

        let shared = setup();
        shared.borrow_mut().set_state("n", VbValue::num(1.0));

        let seen = std::rc::Rc::new(std::cell::RefCell::new(0.0));
        let seen_clone = seen.clone();
        let e = Expr::Variable("n".to_string(), p());

        subscribe(
            &shared,
            Box::new(move |sh: &SharedState| {
                let v = eval_tracked(sh, &e);
                *seen_clone.borrow_mut() = v.as_num().unwrap_or(0.0);
            }),
        );
        assert_eq!(*seen.borrow(), 1.0);

        shared.borrow_mut().set_state("n", VbValue::num(2.0));
        flush(&shared);
        assert_eq!(*seen.borrow(), 2.0);
    }

    #[test]
    fn test_gte_lte_invalid_numeric_string_are_false() {
        let shared = setup();
        let left = Expr::literal_str("abc", p());
        let right = Expr::literal_num(10.0, p());
        for op in [BinaryOp::Gte, BinaryOp::Lte] {
            let e = Expr::Binary {
                op,
                left: Box::new(left.clone()),
                right: Box::new(right.clone()),
                pos: p(),
            };
            assert_eq!(eval(&shared, &e), VbValue::Bool(false));
        }
    }

    #[test]
    fn test_gte_lte_numeric_string_still_coerce() {
        let shared = setup();
        let left = Expr::literal_str("5", p());
        let right = Expr::literal_num(10.0, p());
        let lte = Expr::Binary {
            op: BinaryOp::Lte,
            left: Box::new(left.clone()),
            right: Box::new(right.clone()),
            pos: p(),
        };
        let gte = Expr::Binary {
            op: BinaryOp::Gte,
            left: Box::new(left),
            right: Box::new(right),
            pos: p(),
        };
        assert_eq!(eval(&shared, &lte), VbValue::Bool(true));
        assert_eq!(eval(&shared, &gte), VbValue::Bool(false));
    }

    #[test]
    fn test_eval_loop_scope_priority_over_global() {
        use super::super::state::LoopFrame;

        let shared = setup();
        shared.borrow_mut().set_state("item", VbValue::str("global"));
        shared.borrow_mut().push_loop_scope(LoopFrame {
            item_var: "item".to_string(),
            item_value: VbValue::str("local"),
            index_var: None,
            index_value: None,
        });

        let e = Expr::Variable("item".to_string(), p());
        assert_eq!(eval_tracked(&shared, &e).as_str(), Some("local"));
    }

    #[test]
    fn test_eval_readonly_with_frame_resolves_field_from_captured_frame() {
        use super::super::state::LoopFrame;

        let shared = setup();
        let state_ref = shared.borrow();

        let frame = LoopFrame {
            item_var: "bv".to_string(),
            item_value: VbValue::object(vec![
                ("tieu_de".to_string(), VbValue::str("Học ViBao trong 1 tuần")),
                ("luot_thich".to_string(), VbValue::num(24.0)),
            ]),
            index_var: None,
            index_value: None,
        };

        let e = Expr::MemberAccess {
            object: Box::new(Expr::Variable("bv".to_string(), p())),
            property: "tieu_de".to_string(),
            pos: p(),
        };

        assert_eq!(eval_readonly(&state_ref, &e), VbValue::Null);

        assert_eq!(
            eval_readonly_with_frame(&state_ref, &e, Some(&frame)).as_str(),
            Some("Học ViBao trong 1 tuần")
        );
    }

    #[test]
    fn test_eval_readonly_with_frame_falls_back_to_global_state_when_no_match() {
        let shared = setup();
        shared.borrow_mut().set_state("ten", VbValue::str("Toàn cục"));
        let state_ref = shared.borrow();

        use super::super::state::LoopFrame;
        let frame = LoopFrame {
            item_var: "bv".to_string(),
            item_value: VbValue::str("khong lien quan"),
            index_var: None,
            index_value: None,
        };
        let e = Expr::Variable("ten".to_string(), p());
        assert_eq!(
            eval_readonly_with_frame(&state_ref, &e, Some(&frame)).as_str(),
            Some("Toàn cục")
        );
    }

    #[test]
    fn test_eval_readonly_with_frame_none_matches_plain_eval_readonly() {
        let shared = setup();
        shared.borrow_mut().set_state("dem", VbValue::num(7.0));
        let state_ref = shared.borrow();
        let e = Expr::Variable("dem".to_string(), p());
        assert_eq!(
            eval_readonly_with_frame(&state_ref, &e, None),
            eval_readonly(&state_ref, &e)
        );
    }
}
