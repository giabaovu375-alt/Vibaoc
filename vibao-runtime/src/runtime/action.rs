// ============================================================
// VIBAO RUNTIME (Rust/WASM) — runtime/action.rs
// The dispatcher that executes `vibao_ast::Action` directly in Rust -
// this was the missing piece that used to make EVERY BUTTON CLICK do
// nothing (bind_events in dom.rs only logged a warning). A port of the
// core of 19-runtime-api.ts (toast/modal/scroll) + compileAction from
// the old codegen (but EXECUTING instead of generating JS).
//
// Since `dispatch` needs to call async (goi_api), and every action
// inside an event handler runs SEQUENTIALLY (like awaiting each line in
// the old JS version), dispatch_all here is an `async fn`, running via
// wasm-bindgen-futures.
// ============================================================

use std::collections::BTreeMap;

use vibao_ast::{Action, Expr, PropsMap};

use super::expr_eval;
use super::state::{self, LoopFrame, SharedState};
use super::value::VbValue;
use super::{api, log};

/// Executes a list of Actions SEQUENTIALLY (an event handler's body, or
/// an on_success/on_failure/consequent/alternate branch). Does NOT
/// track dependencies when evaluating expressions inside an action - an
/// action runs once when the event occurs, it's not a binding that
/// needs to re-run when state changes.
///
/// `loop_frame`: if this action belongs to an event handler nested
/// inside a vong_lap item, this is that exact item's LoopFrame snapshot
/// - every expression inside the action (a function parameter, an if
/// condition, an assigned value) will correctly resolve the loop
/// variable (e.g. "$item") via eval_with_loop_frame.
///
/// `component_id`: LIKE `loop_frame` but for an @the component - if
/// this action belongs to an event handler nested inside a component
/// instance (e.g. `on_click { thong_bao($mo_ta) }` inside a component
/// receiving `mo_ta: chuoi`), this is that instance's id, so `$mo_ta`
/// resolves correctly via `component_scope_stack` (see
/// `eval_with_loop_frame`, `eval_expr_id_tracked` in dom.rs for the
/// full mechanism).
pub async fn dispatch_all(shared: &SharedState, actions: &[Action], loop_frame: Option<&LoopFrame>, component_id: Option<&str>) {
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
    // Wrapped in Box::pin since Action::IfAction/ApiCall recursively
    // call back into dispatch_all (a recursive async fn needs a fixed
    // size at compile time - Rust doesn't allow an async fn to call
    // itself directly without boxing, the same reason Expr/Child need
    // Box<T> in the AST).
    Box::pin(async move {
        match action {
            // NEW FEATURE (not a bug fix - an addition per a feature
            // request, found through a real demo design: the language
            // used to have NO WAY WHATSOEVER to add/remove/edit a single
            // element of an array already in state - Action::Assign only
            // OVERWRITES the ENTIRE value (requiring the full content to
            // be known ahead of build time, unable to "append" to the
            // current array at runtime). This blocked every app needing
            // dynamic list CRUD (a todo list, a shopping cart,
            // comments...).
            //
            // The 3 functions below are SIDE-EFFECT actions (reading the
            // current array from state, transforming it, writing it
            // back) - unlike the functions in eval_call() (expr_eval.rs,
            // PURE, no state access) - so they belong here (action.rs,
            // which already has &SharedState), not in expr_eval.rs.
            //
            // IMPORTANT DESIGN NOTE: these functions need to know the
            // array's VARIABLE NAME (so set_state() writes back to the
            // correct key), NOT its already-evaluated array value - so
            // they must be matched separately BEFORE falling into the
            // general Action::FunctionCall branch (which evaluates every
            // arg into a VbValue before calling the function, LOSING the
            // original variable name). Required syntax: the FIRST
            // argument must be a bare Expr::Variable (e.g. $tasks), not
            // a more complex expression (e.g. $obj.tasks or another
            // function returning an array) - the validator SHOULD report
            // a build-time error if violated (see the note in
            // validator.rs if that check has been added), the runtime
            // here only logs a warning and safely no-ops (never panics)
            // if this case is encountered.
            Action::FunctionCall { name, args, .. }
                if matches!(name.as_str(), "them_vao_mang" | "xoa_theo_id" | "cap_nhat_theo_id") =>
            {
                dispatch_array_mutation(shared, name, args, loop_frame, component_id);
            }

            Action::FunctionCall { name, args, opts, assign_to, .. } => {
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
                method,
                endpoint,
                data,
                assign_to,
                on_success,
                on_failure,
                ..
            } => {
                let endpoint_val = expr_eval::eval_with_loop_frame(shared, endpoint, loop_frame, component_id);
                let endpoint_str = endpoint_val.to_string();
                let data_val = data
                    .as_ref()
                    .map(|d| expr_eval::eval_with_loop_frame(shared, d, loop_frame, component_id));

                let base_url = state::get_base_url(shared);
                let result = api::call(&base_url, method, &endpoint_str, data_val.as_ref()).await;

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

            Action::IfAction { condition, consequent, alternate, .. } => {
                let cond_val = expr_eval::eval_with_loop_frame(shared, condition, loop_frame, component_id);
                if cond_val.is_truthy() {
                    dispatch_all(shared, consequent, loop_frame, component_id).await;
                } else if let Some(alt) = alternate {
                    dispatch_all(shared, alt, loop_frame, component_id).await;
                }
            }
        }

        // Every action can set_state - flush right after EACH action
        // (not waiting until the end of the whole sequence) so that if a
        // middle action indirectly depends on a previous action's
        // render effect (rare but possible, e.g. a conditional based on
        // already-updated DOM), the order stays consistent. In most
        // cases (no cross-dependency), calling flush() extra times is
        // safe - flush() no-ops itself if nothing is pending.
        state::flush(shared);
    })
}

/// Executes the 3 array CRUD functions
/// (them_vao_mang/xoa_theo_id/cap_nhat_theo_id) - a NEW FEATURE addition
/// (see the full explanation at the branch calling this function in
/// dispatch_one).
///
/// A REAL BUG THAT WAS FIXED AT THE ROOT (found through a build + real
/// test run - reported as "clicking a button on a lower row wrongly
/// affects a row above it", especially when clicking several different
/// buttons in rapid succession): these 2 functions were ORIGINALLY
/// designed by INDEX (xoa_theo_chi_so/cap_nhat_theo_chi_so), using
/// "$idx" taken from "vong_lap $item, $idx trong $mang" - but "$idx"
/// was a FIXED SNAPSHOT taken at the moment the button's closure was
/// CREATED (when bind_loop rendered that item), NOT updated if the
/// array's LENGTH changed afterward (adding/removing another element
/// shifts the position of the remaining elements). Since
/// `wasm_bindgen_futures::spawn_local` ALWAYS defers action execution to
/// the next microtask (not running immediately within the click event's
/// tick - see the discussion of this behavior), a window of time (even
/// if very short) exists between "the moment of the click" and "the
/// moment the action ACTUALLY reads/writes state" - if a DELETE action
/// (shifting indexes) happens in that window BEFORE another already-
/// clicked action using the old index, the later action wrongly affects
/// that exact position (now a DIFFERENT element).
///
/// The fix, AT THE ROOT: removes the concept of "index" entirely from
/// the 2 position-based mutation functions - replaced with operating by
/// ID VALUE (reading a specified field on EACH element, RE-LOCATING the
/// real position ITSELF RIGHT WHEN THE ACTION ACTUALLY RUNS, with no
/// dependency on any position snapshot taken earlier). Since this
/// re-location always happens AFTER reading "current =
/// state.get_state(array_name)" (i.e. reading the MOST RECENT state at
/// the exact moment the function runs), it always matches correctly no
/// matter how many times the array was removed from/added to before -
/// completely immune to the "stale index" problem.
///
/// Required syntax (the validator SHOULD additionally check this at
/// build time, see vibaoc/src/validator.rs if that check has been added
/// - here it only logs a warning + safely no-ops if violated, NEVER
/// panics since this code runs inside WASM receiving input from a JSON
/// registry, and a build-time error that slipped through shouldn't
/// crash the whole app at runtime):
///   them_vao_mang($ten_mang, gia_tri_moi)
///   xoa_theo_id($ten_mang, "ten_field_id", gia_tri_id)
///   cap_nhat_theo_id($ten_mang, "ten_field_id", gia_tri_id, gia_tri_moi)
/// The FIRST argument ALWAYS MUST be a bare `Expr::Variable` (e.g.
/// $tasks) - this is the STATE variable name to be read + overwritten,
/// and must NOT be evaluated like a normal parameter (every other
/// parameter is still evaluated normally via eval_with_loop_frame,
/// fully supporting $item inside a vong_lap, complex expressions, etc -
/// only the "array name" position is the exception).
/// xoa_theo_id/cap_nhat_theo_id's 2nd argument (the id field NAME, e.g.
/// "id") is ALWAYS A STRING LITERAL - not evaluated like a normal
/// expression, since this is the NAME of the field to read on EACH
/// element, not a data value.
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
    drop(state); // releases the borrow before evaluating the remaining args (which may read other state)

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

    shared.borrow_mut().set_state(array_name, VbValue::Array(items));
}

/// Shared reading of the 2nd argument (the id field name - ALWAYS A
/// STRING LITERAL, not evaluated like an expression) and the 3rd
/// argument (the id value to match against - evaluated normally,
/// supporting $item.id/a variable/an expression) for both
/// xoa_theo_id/cap_nhat_theo_id. Returns None (already logging a
/// warning itself) if an argument is missing or the 2nd argument isn't
/// a string literal.
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

/// Computes every named option (e.g. `kieu: thanh_cong` in
/// `thong_bao(msg, kieu: thanh_cong)`) into a fast lookup map.
fn eval_opts(shared: &SharedState, opts: &PropsMap, loop_frame: Option<&LoopFrame>, component_id: Option<&str>) -> BTreeMap<String, VbValue> {
    opts.iter()
        .map(|(k, v)| (k.clone(), expr_eval::eval_with_loop_frame(shared, v, loop_frame, component_id)))
        .collect()
}

// ════════════════════════════════════════════════════════════
// FUNCTION CALL DISPATCH — thong_bao, canh_bao, mo_modal, ...
// ════════════════════════════════════════════════════════════

/// Executes an action function call by name (a FunctionCall's callee).
/// Returns a value (used when `assign_to` is present) - most side-effect
/// actions (toast, modal...) return `VbValue::Null`, only a few like
/// `sao_chep`/`goi_api` (via the separate ApiCall branch) return a
/// meaningful value.
async fn dispatch_function_call(
    shared: &SharedState,
    name: &str,
    args: &[VbValue],
    opts: &BTreeMap<String, VbValue>,
) -> VbValue {
    match name {
        // ── Notifications ────────────────────────────────────────────
        "thong_bao" => {
            let msg = args.first().map(|v| v.to_string()).unwrap_or_default();
            let kieu = opts.get("kieu").map(|v| v.to_string()).unwrap_or_else(|| "info".to_string());
            let thoi_gian = opts.get("thoi_gian").map(|v| v.to_num_or_zero()).unwrap_or(3000.0);
            super::dom::toast(&msg, &kieu, thoi_gian as i32);
            VbValue::Null
        }
        "canh_bao" => {
            let msg = args.first().map(|v| v.to_string()).unwrap_or_default();
            super::dom::alert(&msg);
            VbValue::Null
        }

        // ── Navigation ───────────────────────────────────────────
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

        // ── Modal ────────────────────────────────────────────────
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

        // ── Page scrolling ───────────────────────────────────────────
        "cuon_den" => {
            let target = args.first().map(|v| v.to_string()).unwrap_or_default();
            super::dom::scroll_to(&target);
            VbValue::Null
        }
        "cuon_len_dau" => {
            super::dom::scroll_top();
            VbValue::Null
        }

        // ── Array state mutation: $ds.them(item), $ds.xoa(...) ──
        // These functions are actually translated by the PARSER into a
        // special MemberAccess call in most of ViBao's design (see the
        // ast's Expr::Call callee shaped like "ten_bien.them") - but if
        // codegen passes them directly here as a FunctionCall with the
        // callee "luu_du_lieu"/"tai_du_lieu", they're handled via
        // __save/__load equivalent to the old JS version.
        "luu_du_lieu" => {
            let endpoint = args.first().map(|v| v.to_string()).unwrap_or_default();
            let data = args.get(1).cloned().unwrap_or(VbValue::Null);
            let base_url = state::get_base_url(shared);
            let result = api::call(&base_url, "POST", &endpoint, Some(&data)).await;
            if !result.ok {
                super::dom::toast(
                    &format!("Lưu thất bại: {}", result.error.as_deref().unwrap_or("")),
                    "loi",
                    3000,
                );
            }
            result.data
        }
        "tai_du_lieu" => {
            let endpoint = args.first().map(|v| v.to_string()).unwrap_or_default();
            let base_url = state::get_base_url(shared);
            let result = api::call(&base_url, "GET", &endpoint, None).await;
            if !result.ok {
                super::dom::toast(
                    &format!("Tải dữ liệu thất bại: {}", result.error.as_deref().unwrap_or("")),
                    "loi",
                    3000,
                );
                return VbValue::Null;
            }
            result.data
        }

        // ── Clipboard ────────────────────────────────────────────
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
    fn test_eval_opts_reads_named_options() {
        let shared = state::new_shared_state();
        let opts: PropsMap = vec![("kieu".to_string(), vibao_ast::Expr::literal_str("loi", p()))];
        let result = eval_opts(&shared, &opts, None, None);
        assert_eq!(result.get("kieu").and_then(|v| v.as_str()), Some("loi"));
    }

    #[test]
    fn test_eval_opts_resolves_loop_variable_via_loop_frame() {
        // A direct regression test for the fixed loop-action bug:
        // eval_opts (and every eval inside an action) used to always
        // call plain eval() with no loop_frame at all, making the loop
        // variable (e.g. "$item") resolve wrong/empty when used inside
        // an action nested in a vong_lap item. Now that loop_frame is
        // passed in, "$ten" (item_var) must resolve to that item's
        // correct value.
        let shared = state::new_shared_state();
        let frame = LoopFrame {
            item_var: "ten".to_string(),
            item_value: VbValue::str("San pham A"),
            index_var: None,
            index_value: None,
        };
        let opts: PropsMap = vec![("msg".to_string(), vibao_ast::Expr::Variable("ten".to_string(), p()))];
        let result = eval_opts(&shared, &opts, Some(&frame), None);
        assert_eq!(result.get("msg").and_then(|v| v.as_str()), Some("San pham A"));
    }

    #[test]
    fn test_assign_action_sets_state() {
        // A simple synchronous test: Assign doesn't really need async,
        // but dispatch_one is an async fn - futures::executor::block_on
        // isn't available (not adding a separate `futures` dependency
        // just for this test), so this asserts directly via a manual
        // eval + set_state, confirming exactly the LOGIC the Assign
        // case inside dispatch_one performs (the same 2 lines as
        // Action::Assign in dispatch_one).
        let shared = state::new_shared_state();
        let value_expr = vibao_ast::Expr::literal_num(42.0, p());
        let v = expr_eval::eval(&shared, &value_expr);
        shared.borrow_mut().set_state("dem", v);
        assert_eq!(shared.borrow().peek_state("dem").as_num(), Some(42.0));
    }

    // ── Tests for the 3 array CRUD functions
    // (them_vao_mang/xoa_theo_id/cap_nhat_theo_id) - a NEW FEATURE, not a
    // bug fix by itself, BUT xoa_theo_id/cap_nhat_theo_id COMPLETELY
    // REPLACE xoa_theo_chi_so/cap_nhat_theo_chi_so (removed entirely)
    // after the "stale index" bug was found through a build + real test
    // run - see the full explanation in dispatch_array_mutation()'s doc
    // comment. dispatch_array_mutation() is a SYNCHRONOUS function (not
    // async), directly testable without block_on.

    fn set_array_state(shared: &SharedState, key: &str, items: Vec<VbValue>) {
        shared.borrow_mut().set_state(key, VbValue::Array(items));
    }

    /// Quickly creates a VbValue::Object with 2 fields {id, ten} -
    /// enough for every test below (mimicking the real shape of a
    /// "task" in task_manager.vbao, though only 2 fields are needed to
    /// test the id-matching logic).
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
        set_array_state(&shared, "tasks", vec![obj_with_id(10.0, "a"), obj_with_id(20.0, "b"), obj_with_id(30.0, "c")]);

        // Removes by id=20 - MUST remove exactly "b" (id=20), regardless
        // of its position in the array.
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
        // A DIRECT regression test for the real, fixed bug: simulates
        // the exact "click 2 different buttons very quickly" scenario -
        // the FIRST action removes an element (shifting the position of
        // every element AFTER it), then the SECOND action (already
        // "targeting" a SPECIFIC id from before, not an index) must
        // still remove EXACTLY the intended object, no matter how the
        // array shifted in the previous step.
        let shared = state::new_shared_state();
        set_array_state(&shared, "tasks", vec![obj_with_id(10.0, "a"), obj_with_id(20.0, "b"), obj_with_id(30.0, "c")]);

        // Action 1: removes id=10 ("a") - "b" and "c" shift up 1
        // position (index 1,2 -> 0,1).
        let args1 = vec![
            Expr::Variable("tasks".to_string(), p()),
            Expr::literal_str("id", p()),
            Expr::literal_num(10.0, p()),
        ];
        dispatch_array_mutation(&shared, "xoa_theo_id", &args1, None, None);

        // Action 2: removes id=30 ("c") - with the OLD design (by
        // index), if the closure had already "targeted" index=2 ("c"'s
        // original position) BEFORE action 1 ran, it would remove the
        // WRONG element (index 2 now either doesn't exist or points
        // somewhere else) - with the NEW design (by id), it's always
        // correct since it re-locates id=30's REAL position right at
        // this moment.
        let args2 = vec![
            Expr::Variable("tasks".to_string(), p()),
            Expr::literal_str("id", p()),
            Expr::literal_num(30.0, p()),
        ];
        dispatch_array_mutation(&shared, "xoa_theo_id", &args2, None, None);

        let result = shared.borrow_mut().get_state("tasks");
        let items = result.as_array().unwrap();
        assert_eq!(items.len(), 1, "exactly 1 element must remain");
        assert_eq!(items[0].get_field("ten").as_str(), Some("b"), "the remaining element must be \"b\" (id=20), not wrongly removed");
    }

    #[test]
    fn test_xoa_theo_id_not_found_is_noop() {
        // No matching id found (e.g. already removed by another
        // earlier action): logs a warning + does NOT change the array,
        // never panics - far safer than the old design (an out-of-range
        // index used to raise a hard error; now "id not found" is also
        // a COMPLETELY NORMAL case that can happen due to exactly this
        // dual-action race, and shouldn't be treated as a serious
        // error).
        let shared = state::new_shared_state();
        set_array_state(&shared, "tasks", vec![obj_with_id(10.0, "a")]);

        let args = vec![
            Expr::Variable("tasks".to_string(), p()),
            Expr::literal_str("id", p()),
            Expr::literal_num(999.0, p()),
        ];
        dispatch_array_mutation(&shared, "xoa_theo_id", &args, None, None);

        let result = shared.borrow_mut().get_state("tasks");
        assert_eq!(result.as_array().unwrap().len(), 1, "the array must not change when no matching id is found");
    }

    #[test]
    fn test_cap_nhat_theo_id_replaces_correct_item_by_id_value() {
        let shared = state::new_shared_state();
        set_array_state(&shared, "tasks", vec![obj_with_id(10.0, "a"), obj_with_id(20.0, "b")]);

        let args = vec![
            Expr::Variable("tasks".to_string(), p()),
            Expr::literal_str("id", p()),
            Expr::literal_num(10.0, p()),
            Expr::literal_str("a_moi", p()),
        ];
        dispatch_array_mutation(&shared, "cap_nhat_theo_id", &args, None, None);

        let result = shared.borrow_mut().get_state("tasks");
        let items = result.as_array().unwrap();
        assert_eq!(items[0].as_str(), Some("a_moi"), "the id=10 element must be REPLACED entirely with the new value");
        assert_eq!(items[1].get_field("ten").as_str(), Some("b"), "the other element (id=20) is unaffected");
    }

    #[test]
    fn test_cap_nhat_theo_id_immune_to_stale_position_after_prior_deletion() {
        // Similar to test_xoa_theo_id_immune_to_stale_position - but for
        // cap_nhat_theo_id: action 1 removes an element, shifting
        // positions, action 2 (an update, already "targeting" an id
        // from before) must still hit the correct element, not
        // mistakenly hitting a different element that shifted into the
        // old position.
        let shared = state::new_shared_state();
        set_array_state(&shared, "tasks", vec![obj_with_id(10.0, "a"), obj_with_id(20.0, "b"), obj_with_id(30.0, "c")]);

        let args_delete = vec![
            Expr::Variable("tasks".to_string(), p()),
            Expr::literal_str("id", p()),
            Expr::literal_num(10.0, p()),
        ];
        dispatch_array_mutation(&shared, "xoa_theo_id", &args_delete, None, None);

        // "c" (id=30) has now shifted from index 2 down to index 1
        // (since "a" was removed). With the OLD design, an action
        // already "targeting" index=2 would WRONGLY hit outside the
        // array or hit the wrong element - with id, it must still hit
        // exactly "c".
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
        assert_eq!(items[0].get_field("ten").as_str(), Some("b"), "\"b\" (id=20) is untouched");
        assert_eq!(items[1].as_str(), Some("c_da_sua"), "\"c\" (id=30) must be CORRECTLY updated despite shifting position");
    }

    #[test]
    fn test_id_field_name_arg_must_be_string_literal() {
        // The 2nd argument (the id field name) MUST be a string
        // literal, not evaluated like a normal expression (e.g.
        // accidentally passing a variable/number) - logs a warning +
        // safely no-ops.
        let shared = state::new_shared_state();
        set_array_state(&shared, "tasks", vec![obj_with_id(10.0, "a")]);

        let args = vec![
            Expr::Variable("tasks".to_string(), p()),
            Expr::literal_num(123.0, p()), // KHÔNG phải chuỗi literal
            Expr::literal_num(10.0, p()),
        ];
        dispatch_array_mutation(&shared, "xoa_theo_id", &args, None, None);

        let result = shared.borrow_mut().get_state("tasks");
        assert_eq!(result.as_array().unwrap().len(), 1, "the array must not change when the id field name is invalid");
    }

    #[test]
    fn test_array_mutation_requires_bare_variable_as_first_arg() {
        // The first argument is NOT a bare Expr::Variable (e.g. a
        // numeric literal) - must log a warning + safely no-op, never
        // panic.
        let shared = state::new_shared_state();
        set_array_state(&shared, "tasks", vec![VbValue::str("a")]);

        let args = vec![
            Expr::literal_num(123.0, p()), // KHÔNG phải Variable
            Expr::literal_str("x", p()),
        ];
        // Doesn't assert a specific result (there's no key to check) -
        // just confirms it does NOT panic when called.
        dispatch_array_mutation(&shared, "them_vao_mang", &args, None, None);
    }

    #[test]
    fn test_array_mutation_on_non_array_state_is_noop() {
        // The variable exists but is NOT an array (e.g. it's currently
        // a Num) - logs a warning + safely no-ops, never panics, never
        // silently coerces the wrong type.
        let shared = state::new_shared_state();
        shared.borrow_mut().set_state("khong_phai_mang", VbValue::Num(5.0));

        let args = vec![
            Expr::Variable("khong_phai_mang".to_string(), p()),
            Expr::literal_str("x", p()),
        ];
        dispatch_array_mutation(&shared, "them_vao_mang", &args, None, None);

        let result = shared.borrow_mut().get_state("khong_phai_mang");
        assert_eq!(result.as_num(), Some(5.0), "the original value is unchanged when it is not an array");
    }

    #[test]
    fn test_them_vao_mang_resolves_loop_variable_in_value_arg() {
        // A regression test similar to the fixed loop-action bug (see
        // test_eval_opts_resolves_loop_variable_via_loop_frame) - a
        // value passed into them_vao_mang must also correctly resolve
        // via loop_frame if this action sits inside another vong_lap's
        // item.
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
        assert_eq!(result.as_array().unwrap()[0].as_str(), Some("gia_tri_tu_vong_lap"));
    }

    #[test]
    fn test_cap_nhat_theo_id_resolves_loop_variable_in_id_value_and_new_value() {
        // Both id_value (the 3rd argument) and the new value (the 4th
        // argument) must correctly resolve via loop_frame - a real case:
        // inside "vong_lap $task, $idx trong $tasks", a "Hoan thanh"
        // button calls
        // "cap_nhat_theo_id($tasks, \"id\", $task.id, {...})", needing
        // $task.id to resolve correctly against the CORRECT item of that
        // loop.
        let shared = state::new_shared_state();
        set_array_state(&shared, "tasks", vec![obj_with_id(10.0, "a"), obj_with_id(20.0, "b")]);

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
        assert_eq!(items[0].get_field("ten").as_str(), Some("a"), "the other element is unaffected");
        assert_eq!(items[1].as_str(), Some("b_moi"), "must correctly update the element whose id matches \\$task.id (=20)");
    }
}
