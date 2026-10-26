use std::collections::BTreeMap;

use vibao_ast::{Action, Expr, PropsMap};

use super::expr_eval;
use super::state::{self, LoopFrame, SharedState};
use super::value::VbValue;
use super::{api, log};

pub async fn dispatch_all(
    shared: &SharedState,
    actions: &[Action],
    loop_frame: Option<&LoopFrame>,
    component_id: Option<&str>,
) {
    for action in actions {
        dispatch_one(shared, action, loop_frame, component_id).await;
    }
}

fn dispatch_one<'a>(
    shared: &'a SharedState,
    action: &'a Action,
    loop_frame: Option<&'a LoopFrame>,
    component_id: Option<&'a str>,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + 'a>> {
    Box::pin(async move {
        match action {
            Action::FunctionCall { name, args, .. }
                if matches!(
                    name.as_str(),
                    "them_vao_mang" | "xoa_theo_id" | "cap_nhat_theo_id"
                ) =>
            {
                dispatch_array_mutation(shared, name, args, loop_frame, component_id);
            }

            Action::FunctionCall {
                name,
                args,
                opts,
                assign_to,
                ..
            } => {
                let arg_values: Vec<VbValue> = args
                    .iter()
                    .map(|e| expr_eval::eval_with_loop_frame(shared, e, loop_frame, component_id))
                    .collect();
                let opt_values = eval_opts(shared, opts, loop_frame, component_id);

                let result = dispatch_function_call(shared, name, &arg_values, &opt_values).await;

                if let Some(var_name) = assign_to {
                    shared.borrow_mut().set_state(var_name, result);
                }
            }

            Action::Assign { target, value, .. } => {
                let v = expr_eval::eval_with_loop_frame(shared, value, loop_frame, component_id);
                shared.borrow_mut().set_state(target, v);
            }

            Action::ApiCall {
                background: true, ..
            } => {
                let shared = shared.clone();
                let mut detached = action.clone();
                if let Action::ApiCall { background, .. } = &mut detached {
                    *background = false;
                }
                let loop_frame = loop_frame.cloned();
                let component_id = component_id.map(|id| id.to_string());
                wasm_bindgen_futures::spawn_local(async move {
                    dispatch_one(
                        &shared,
                        &detached,
                        loop_frame.as_ref(),
                        component_id.as_deref(),
                    )
                    .await;
                });
            }

            Action::ApiCall {
                method,
                endpoint,
                data,
                headers,
                assign_to,
                on_success,
                on_failure,
                ..
            } => {
                let endpoint_val =
                    expr_eval::eval_with_loop_frame(shared, endpoint, loop_frame, component_id);
                let endpoint_str = endpoint_val.to_string();
                let data_val = data
                    .as_ref()
                    .map(|d| expr_eval::eval_with_loop_frame(shared, d, loop_frame, component_id));

                let headers_val = headers
                    .as_ref()
                    .map(|h| expr_eval::eval_with_loop_frame(shared, h, loop_frame, component_id));

                let base_url = state::get_base_url(shared);
                let result = api::call(
                    &base_url,
                    method,
                    &endpoint_str,
                    data_val.as_ref(),
                    headers_val.as_ref(),
                )
                .await;

                if let Some(var_name) = assign_to {
                    shared.borrow_mut().set_state(var_name, result.data.clone());
                }

                if result.ok {
                    if let Some(actions) = on_success {
                        dispatch_all(shared, actions, loop_frame, component_id).await;
                    }
                } else {
                    log::warn(&format!(
                        "[ViBao] goi_api failed: {}",
                        result.error.as_deref().unwrap_or("unknown error")
                    ));
                    if let Some(actions) = on_failure {
                        dispatch_all(shared, actions, loop_frame, component_id).await;
                    }
                }
            }

            Action::IfAction {
                condition,
                consequent,
                alternate,
                ..
            } => {
                let cond_val =
                    expr_eval::eval_with_loop_frame(shared, condition, loop_frame, component_id);
                if cond_val.is_truthy() {
                    dispatch_all(shared, consequent, loop_frame, component_id).await;
                } else if let Some(alt) = alternate {
                    dispatch_all(shared, alt, loop_frame, component_id).await;
                }
            }
        }

        state::flush(shared);
    })
}

fn dispatch_array_mutation(
    shared: &SharedState,
    name: &str,
    args: &[Expr],
    loop_frame: Option<&LoopFrame>,
    component_id: Option<&str>,
) {
    let Some(Expr::Variable(array_name, _)) = args.first() else {
        log::warn(&format!(
            "[ViBao] {}(...): the first argument must be a bare state variable (e.g. $tasks), not a more complex expression - skipping this action.",
            name
        ));
        return;
    };

    let mut state = shared.borrow_mut();
    let current = state.get_state(array_name);
    let Some(mut items) = current.as_array().cloned() else {
        log::warn(&format!(
            "[ViBao] {}(\"{}\", ...): the variable \"${}\" is not currently an array (value: {:?}) - skipping this action.",
            name, array_name, array_name, current
        ));
        return;
    };
    drop(state);

    match name {
        "them_vao_mang" => {
            let Some(value_expr) = args.get(1) else {
                log::warn("[ViBao] them_vao_mang(ten_mang, gia_tri): missing the 2nd argument (the value to add).");
                return;
            };
            let value = expr_eval::eval_with_loop_frame(shared, value_expr, loop_frame, component_id);
            items.push(value);
        }
        "xoa_theo_id" => {
            let Some((field_name, id_value)) = eval_id_field_args(shared, name, args, loop_frame, component_id) else {
                return;
            };
            let Some(pos) = items.iter().position(|item| item.get_field(&field_name).strict_eq(&id_value)) else {
                log::warn(&format!(
                    "[ViBao] xoa_theo_id(\"{}\", \"{}\", {:?}): no matching element found - skipping (the element may have already been removed by another action).",
                    array_name, field_name, id_value
                ));
                return;
            };
            items.remove(pos);
        }
        "cap_nhat_theo_id" => {
            let Some((field_name, id_value)) = eval_id_field_args(shared, name, args, loop_frame, component_id) else {
                return;
            };
            let Some(value_expr) = args.get(3) else {
                log::warn("[ViBao] cap_nhat_theo_id(ten_mang, ten_field_id, gia_tri_id, gia_tri_moi): missing the 4th argument (the new value).");
                return;
            };
            let Some(pos) = items.iter().position(|item| item.get_field(&field_name).strict_eq(&id_value)) else {
                log::warn(&format!(
                    "[ViBao] cap_nhat_theo_id(\"{}\", \"{}\", {:?}, ...): no matching element found - skipping (the element may have already been removed by another action).",
                    array_name, field_name, id_value
                ));
                return;
            };
            let new_value = expr_eval::eval_with_loop_frame(shared, value_expr, loop_frame, component_id);
            items[pos] = new_value;
        }
        _ => unreachable!("dispatch_array_mutation is only ever called for the 3 function names already matched in dispatch_one"),
    }

    shared
        .borrow_mut()
        .set_state(array_name, VbValue::Array(items));
}

fn eval_id_field_args(
    shared: &SharedState,
    fn_name: &str,
    args: &[Expr],
    loop_frame: Option<&LoopFrame>,
    component_id: Option<&str>,
) -> Option<(String, VbValue)> {
    let Some(field_expr) = args.get(1) else {
        log::warn(&format!("[ViBao] {}(ten_mang, ten_field_id, ...): missing the 2nd argument (the id field name, e.g. \"id\").", fn_name));
        return None;
    };
    let Expr::Literal(vibao_ast::LiteralValue::Str(field_name), _) = field_expr else {
        log::warn(&format!(
            "[ViBao] {}(...): the 2nd argument must be a STRING LITERAL naming the id field (e.g. \"id\"), not an expression - skipping.",
            fn_name
        ));
        return None;
    };
    let Some(id_expr) = args.get(2) else {
        log::warn(&format!("[ViBao] {}(ten_mang, ten_field_id, gia_tri_id, ...): missing the 3rd argument (the id value to find).", fn_name));
        return None;
    };
    let id_value = expr_eval::eval_with_loop_frame(shared, id_expr, loop_frame, component_id);
    Some((field_name.clone(), id_value))
}

fn canonical_toast_kind(kind: &str) -> String {
    match kind {
        "success" => "thanh_cong",
        "error" => "loi",
        "warning" => "canh_bao",
        other => other,
    }
    .to_string()
}

fn eval_opts(
    shared: &SharedState,
    opts: &PropsMap,
    loop_frame: Option<&LoopFrame>,
    component_id: Option<&str>,
) -> BTreeMap<String, VbValue> {
    opts.iter()
        .map(|(k, v)| {
            (
                k.clone(),
                expr_eval::eval_with_loop_frame(shared, v, loop_frame, component_id),
            )
        })
        .collect()
}

async fn dispatch_function_call(
    shared: &SharedState,
    name: &str,
    args: &[VbValue],
    opts: &BTreeMap<String, VbValue>,
) -> VbValue {
    match name {
        "thong_bao" => {
            let msg = args.first().map(|v| v.to_string()).unwrap_or_default();
            let kieu = opts
                .get("kieu")
                .map(|v| canonical_toast_kind(&v.to_string()))
                .unwrap_or_else(|| "info".to_string());
            let thoi_gian = opts
                .get("thoi_gian")
                .map(|v| v.to_num_or_zero())
                .unwrap_or(3000.0);
            super::dom::toast(&msg, &kieu, thoi_gian as i32);
            VbValue::Null
        }
        "canh_bao" => {
            let msg = args.first().map(|v| v.to_string()).unwrap_or_default();
            super::dom::alert(&msg);
            VbValue::Null
        }

        "dieu_huong" => {
            let path = args.first().map(|v| v.to_string()).unwrap_or_default();
            super::dom::navigate(shared, &path);
            VbValue::Null
        }
        "mo_tab_moi" => {
            let path = args.first().map(|v| v.to_string()).unwrap_or_default();
            super::dom::open_tab(&path);
            VbValue::Null
        }

        "mo_modal" => {
            let id = args.first().map(|v| v.to_string()).unwrap_or_default();
            super::dom::open_modal(&id);
            VbValue::Null
        }
        "dong_modal" => {
            let id = args.first().map(|v| v.to_string()).unwrap_or_default();
            super::dom::close_modal(&id);
            VbValue::Null
        }

        "cuon_den" => {
            let target = args.first().map(|v| v.to_string()).unwrap_or_default();
            super::dom::scroll_to(&target);
            VbValue::Null
        }
        "cuon_len_dau" => {
            super::dom::scroll_top();
            VbValue::Null
        }

        "dat_chu_de" => {
            let name = args.first().map(|v| v.to_string()).unwrap_or_default();
            super::dom::set_theme(&name);
            VbValue::Null
        }

        "luu_du_lieu" => {
            let endpoint = args.first().map(|v| v.to_string()).unwrap_or_default();
            let data = args.get(1).cloned().unwrap_or(VbValue::Null);
            let base_url = state::get_base_url(shared);
            let result = api::call(&base_url, "POST", &endpoint, Some(&data), None).await;
            if !result.ok {
                let msg = if state::get_lang(shared) == "en" {
                    format!("Save failed: {}", result.error.as_deref().unwrap_or(""))
                } else {
                    format!("Lưu thất bại: {}", result.error.as_deref().unwrap_or(""))
                };
                super::dom::toast(&msg, "loi", 3000);
            }
            result.data
        }
        "tai_du_lieu" => {
            let endpoint = args.first().map(|v| v.to_string()).unwrap_or_default();
            let base_url = state::get_base_url(shared);
            let result = api::call(&base_url, "GET", &endpoint, None, None).await;
            if !result.ok {
                let msg = if state::get_lang(shared) == "en" {
                    format!(
                        "Failed to load data: {}",
                        result.error.as_deref().unwrap_or("")
                    )
                } else {
                    format!(
                        "Tải dữ liệu thất bại: {}",
                        result.error.as_deref().unwrap_or("")
                    )
                };
                super::dom::toast(&msg, "loi", 3000);
                return VbValue::Null;
            }
            result.data
        }

        "sao_chep" => {
            let text = args.first().map(|v| v.to_string()).unwrap_or_default();
            super::dom::copy_text(&text);
            VbValue::Null
        }

        _ => {
            log::warn(&format!(
                "[ViBao] Action \"{}\" is not yet supported by the action dispatcher.",
                name
            ));
            VbValue::Null
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
    fn test_canonical_toast_kind_maps_english_names() {
        assert_eq!(canonical_toast_kind("success"), "thanh_cong");
        assert_eq!(canonical_toast_kind("error"), "loi");
        assert_eq!(canonical_toast_kind("warning"), "canh_bao");
        assert_eq!(canonical_toast_kind("info"), "info");
        assert_eq!(canonical_toast_kind("thanh_cong"), "thanh_cong");
        assert_eq!(canonical_toast_kind("loi"), "loi");
    }

    #[test]
    fn test_eval_opts_reads_named_options() {
        let shared = state::new_shared_state();
        let opts: PropsMap = vec![("kieu".to_string(), vibao_ast::Expr::literal_str("loi", p()))];
        let result = eval_opts(&shared, &opts, None, None);
        assert_eq!(result.get("kieu").and_then(|v| v.as_str()), Some("loi"));
    }

    #[test]
    fn test_eval_opts_resolves_loop_variable_via_loop_frame() {
        let shared = state::new_shared_state();
        let frame = LoopFrame {
            item_var: "ten".to_string(),
            item_value: VbValue::str("San pham A"),
            index_var: None,
            index_value: None,
        };
        let opts: PropsMap = vec![(
            "msg".to_string(),
            vibao_ast::Expr::Variable("ten".to_string(), p()),
        )];
        let result = eval_opts(&shared, &opts, Some(&frame), None);
        assert_eq!(
            result.get("msg").and_then(|v| v.as_str()),
            Some("San pham A")
        );
    }

    #[test]
    fn test_assign_action_sets_state() {
        let shared = state::new_shared_state();
        let value_expr = vibao_ast::Expr::literal_num(42.0, p());
        let v = expr_eval::eval(&shared, &value_expr);
        shared.borrow_mut().set_state("dem", v);
        assert_eq!(shared.borrow().peek_state("dem").as_num(), Some(42.0));
    }

    fn set_array_state(shared: &SharedState, key: &str, items: Vec<VbValue>) {
        shared.borrow_mut().set_state(key, VbValue::Array(items));
    }

    fn obj_with_id(id: f64, ten: &str) -> VbValue {
        let mut map = BTreeMap::new();
        map.insert("id".to_string(), VbValue::Num(id));
        map.insert("ten".to_string(), VbValue::str(ten));
        VbValue::Object(map)
    }

    #[test]
    fn test_them_vao_mang_appends_to_end() {
        let shared = state::new_shared_state();
        set_array_state(&shared, "tasks", vec![VbValue::str("a"), VbValue::str("b")]);

        let args = vec![
            Expr::Variable("tasks".to_string(), p()),
            Expr::literal_str("c", p()),
        ];
        dispatch_array_mutation(&shared, "them_vao_mang", &args, None, None);

        let result = shared.borrow_mut().get_state("tasks");
        let items = result.as_array().unwrap();
        assert_eq!(items.len(), 3);
        assert_eq!(items[2].as_str(), Some("c"));
    }

    #[test]
    fn test_xoa_theo_id_removes_correct_item_by_id_value() {
        let shared = state::new_shared_state();
        set_array_state(
            &shared,
            "tasks",
            vec![
                obj_with_id(10.0, "a"),
                obj_with_id(20.0, "b"),
                obj_with_id(30.0, "c"),
            ],
        );

        let args = vec![
            Expr::Variable("tasks".to_string(), p()),
            Expr::literal_str("id", p()),
            Expr::literal_num(20.0, p()),
        ];
        dispatch_array_mutation(&shared, "xoa_theo_id", &args, None, None);

        let result = shared.borrow_mut().get_state("tasks");
        let items = result.as_array().unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].get_field("ten").as_str(), Some("a"));
        assert_eq!(items[1].get_field("ten").as_str(), Some("c"));
    }

    #[test]
    fn test_xoa_theo_id_immune_to_stale_position_after_prior_removal() {
        let shared = state::new_shared_state();
        set_array_state(
            &shared,
            "tasks",
            vec![
                obj_with_id(10.0, "a"),
                obj_with_id(20.0, "b"),
                obj_with_id(30.0, "c"),
            ],
        );

        let args1 = vec![
            Expr::Variable("tasks".to_string(), p()),
            Expr::literal_str("id", p()),
            Expr::literal_num(10.0, p()),
        ];
        dispatch_array_mutation(&shared, "xoa_theo_id", &args1, None, None);

        let args2 = vec![
            Expr::Variable("tasks".to_string(), p()),
            Expr::literal_str("id", p()),
            Expr::literal_num(30.0, p()),
        ];
        dispatch_array_mutation(&shared, "xoa_theo_id", &args2, None, None);

        let result = shared.borrow_mut().get_state("tasks");
        let items = result.as_array().unwrap();
        assert_eq!(items.len(), 1, "exactly 1 element must remain");
        assert_eq!(
            items[0].get_field("ten").as_str(),
            Some("b"),
            "the remaining element must be \"b\" (id=20), not wrongly removed"
        );
    }

    #[test]
    fn test_xoa_theo_id_not_found_is_noop() {
        let shared = state::new_shared_state();
        set_array_state(&shared, "tasks", vec![obj_with_id(10.0, "a")]);

        let args = vec![
            Expr::Variable("tasks".to_string(), p()),
            Expr::literal_str("id", p()),
            Expr::literal_num(999.0, p()),
        ];
        dispatch_array_mutation(&shared, "xoa_theo_id", &args, None, None);

        let result = shared.borrow_mut().get_state("tasks");
        assert_eq!(
            result.as_array().unwrap().len(),
            1,
            "the array must not change when no matching id is found"
        );
    }

    #[test]
    fn test_cap_nhat_theo_id_replaces_correct_item_by_id_value() {
        let shared = state::new_shared_state();
        set_array_state(
            &shared,
            "tasks",
            vec![obj_with_id(10.0, "a"), obj_with_id(20.0, "b")],
        );

        let args = vec![
            Expr::Variable("tasks".to_string(), p()),
            Expr::literal_str("id", p()),
            Expr::literal_num(10.0, p()),
            Expr::literal_str("a_moi", p()),
        ];
        dispatch_array_mutation(&shared, "cap_nhat_theo_id", &args, None, None);

        let result = shared.borrow_mut().get_state("tasks");
        let items = result.as_array().unwrap();
        assert_eq!(
            items[0].as_str(),
            Some("a_moi"),
            "the id=10 element must be REPLACED entirely with the new value"
        );
        assert_eq!(
            items[1].get_field("ten").as_str(),
            Some("b"),
            "the other element (id=20) is unaffected"
        );
    }

    #[test]
    fn test_cap_nhat_theo_id_immune_to_stale_position_after_prior_deletion() {
        let shared = state::new_shared_state();
        set_array_state(
            &shared,
            "tasks",
            vec![
                obj_with_id(10.0, "a"),
                obj_with_id(20.0, "b"),
                obj_with_id(30.0, "c"),
            ],
        );

        let args_delete = vec![
            Expr::Variable("tasks".to_string(), p()),
            Expr::literal_str("id", p()),
            Expr::literal_num(10.0, p()),
        ];
        dispatch_array_mutation(&shared, "xoa_theo_id", &args_delete, None, None);

        let args_update = vec![
            Expr::Variable("tasks".to_string(), p()),
            Expr::literal_str("id", p()),
            Expr::literal_num(30.0, p()),
            Expr::literal_str("c_da_sua", p()),
        ];
        dispatch_array_mutation(&shared, "cap_nhat_theo_id", &args_update, None, None);

        let result = shared.borrow_mut().get_state("tasks");
        let items = result.as_array().unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(
            items[0].get_field("ten").as_str(),
            Some("b"),
            "\"b\" (id=20) is untouched"
        );
        assert_eq!(
            items[1].as_str(),
            Some("c_da_sua"),
            "\"c\" (id=30) must be CORRECTLY updated despite shifting position"
        );
    }

    #[test]
    fn test_id_field_name_arg_must_be_string_literal() {
        let shared = state::new_shared_state();
        set_array_state(&shared, "tasks", vec![obj_with_id(10.0, "a")]);

        let args = vec![
            Expr::Variable("tasks".to_string(), p()),
            Expr::literal_num(123.0, p()),
            Expr::literal_num(10.0, p()),
        ];
        dispatch_array_mutation(&shared, "xoa_theo_id", &args, None, None);

        let result = shared.borrow_mut().get_state("tasks");
        assert_eq!(
            result.as_array().unwrap().len(),
            1,
            "the array must not change when the id field name is invalid"
        );
    }

    #[test]
    fn test_array_mutation_requires_bare_variable_as_first_arg() {
        let shared = state::new_shared_state();
        set_array_state(&shared, "tasks", vec![VbValue::str("a")]);

        let args = vec![Expr::literal_num(123.0, p()), Expr::literal_str("x", p())];
        dispatch_array_mutation(&shared, "them_vao_mang", &args, None, None);
    }

    #[test]
    fn test_array_mutation_on_non_array_state_is_noop() {
        let shared = state::new_shared_state();
        shared
            .borrow_mut()
            .set_state("khong_phai_mang", VbValue::Num(5.0));

        let args = vec![
            Expr::Variable("khong_phai_mang".to_string(), p()),
            Expr::literal_str("x", p()),
        ];
        dispatch_array_mutation(&shared, "them_vao_mang", &args, None, None);

        let result = shared.borrow_mut().get_state("khong_phai_mang");
        assert_eq!(
            result.as_num(),
            Some(5.0),
            "the original value is unchanged when it is not an array"
        );
    }

    #[test]
    fn test_them_vao_mang_resolves_loop_variable_in_value_arg() {
        let shared = state::new_shared_state();
        set_array_state(&shared, "ket_qua", vec![]);
        let frame = LoopFrame {
            item_var: "x".to_string(),
            item_value: VbValue::str("gia_tri_tu_vong_lap"),
            index_var: None,
            index_value: None,
        };

        let args = vec![
            Expr::Variable("ket_qua".to_string(), p()),
            Expr::Variable("x".to_string(), p()),
        ];
        dispatch_array_mutation(&shared, "them_vao_mang", &args, Some(&frame), None);

        let result = shared.borrow_mut().get_state("ket_qua");
        assert_eq!(
            result.as_array().unwrap()[0].as_str(),
            Some("gia_tri_tu_vong_lap")
        );
    }

    #[test]
    fn test_cap_nhat_theo_id_resolves_loop_variable_in_id_value_and_new_value() {
        let shared = state::new_shared_state();
        set_array_state(
            &shared,
            "tasks",
            vec![obj_with_id(10.0, "a"), obj_with_id(20.0, "b")],
        );

        let mut task_obj = BTreeMap::new();
        task_obj.insert("id".to_string(), VbValue::Num(20.0));
        let frame = LoopFrame {
            item_var: "task".to_string(),
            item_value: VbValue::Object(task_obj),
            index_var: Some("idx".to_string()),
            index_value: Some(1.0),
        };

        let args = vec![
            Expr::Variable("tasks".to_string(), p()),
            Expr::literal_str("id", p()),
            vibao_ast::Expr::MemberAccess {
                object: Box::new(Expr::Variable("task".to_string(), p())),
                property: "id".to_string(),
                pos: p(),
            },
            Expr::literal_str("b_moi", p()),
        ];
        dispatch_array_mutation(&shared, "cap_nhat_theo_id", &args, Some(&frame), None);

        let result = shared.borrow_mut().get_state("tasks");
        let items = result.as_array().unwrap();
        assert_eq!(
            items[0].get_field("ten").as_str(),
            Some("a"),
            "the other element is unaffected"
        );
        assert_eq!(
            items[1].as_str(),
            Some("b_moi"),
            "must correctly update the element whose id matches \\$task.id (=20)"
        );
    }
}
