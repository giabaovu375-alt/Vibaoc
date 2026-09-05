// ============================================================
// VIBAO RUNTIME (Rust/WASM) — runtime/expr_eval.rs
// Directly evaluates a `vibao_ast::Expr` into a `VbValue`, running
// entirely in Rust - this is the part that replaces
// `new Function()`/`eval()` on the JS side in the old version. There's
// no "compile to JS then run JS" step: the Expr tree is walked
// (a tree-walking interpreter) directly every time a value is needed.
//
// Fed by the codegen-side registry (`codegen/expr.rs::expr_to_js_registry`):
// every "dynamic" Expr (Variable, Binary, Call, ...) is assigned an id,
// sent as JSON inside `__vb.boot({ exprRegistry: [...] })`. When JS
// calls `__vb.evalExpr(id)` (see runtime/dom.rs, written afterward), the
// function here looks up the correct Expr by id and evaluates it.
//
// ── TRACKED vs UNTRACKED ────────────────────────────────────────────
// There are 2 public functions: `eval_tracked` (used inside a running
// subscriber - every Variable read automatically records a dependency,
// so the binding re-runs itself when state changes) and `eval`
// (no tracking - used when computing something once, e.g. an initial
// value, or inside a function call whose own argument doesn't need its
// own tracking since the whole parent call is already tracked).
// In practice, every expression eval coming from a binding
// (if/loop/style) should ALWAYS use `eval_tracked` - `eval` only exists
// for rare use cases (e.g. an action handler running once on a button
// click, needing no reactivity).
// ============================================================

use vibao_ast::{BinaryOp, ColorFuncKind, Expr, LiteralValue, TemplatePart, UnaryOp};

use super::state::{SharedState, State};
use super::value::VbValue;

// ════════════════════════════════════════════════════════════
// ENTRY POINTS
// ════════════════════════════════════════════════════════════

/// Evaluates an Expr, WITH dependency tracking - used inside a running
/// subscriber (a dynamic if/loop/style/text binding). `shared` must be
/// running inside a subscriber's context (current_tracking already set
/// by `state::run_subscriber`) for tracking to take effect; otherwise,
/// behavior is still correct (nothing gets tracked, exactly like `eval`
/// below), just losing the auto-re-render feature.
pub fn eval_tracked(shared: &SharedState, expr: &Expr) -> VbValue {
    let mut state = shared.borrow_mut();
    eval_inner(&mut state, expr, true)
}

/// Evaluates an Expr, with NO tracking - used when computing something
/// once (an action handler, an initial value at boot).
pub fn eval(shared: &SharedState, expr: &Expr) -> VbValue {
    let mut state = shared.borrow_mut();
    eval_inner(&mut state, expr, false)
}

/// Evaluates an Expr, with NO tracking, WITH an already-packaged
/// LoopFrame - used by action.rs when an action sits inside a vong_lap
/// item (e.g. "xoa($item)" in a button's on_click, nested in a loop).
/// Pushes the frame onto the loop-scope stack RIGHT BEFORE eval, popping
/// it RIGHT AFTER - following the same principle applied to bindings in
/// dom.rs::eval_expr_id_tracked (each call carries its own frame,
/// without depending on anyone having pushed one beforehand).
///
/// ALREADY FIXED: an action inside a loop used to always call plain
/// `eval()` (with no frame at all), making "$item" resolve wrong/empty
/// - this was exactly the "loop-action" gap noted earlier, now patched
/// by this function.
/// `component_id`: LIKE `loop_frame` - if this action belongs to an
/// event handler nested inside an @the component instance (a component
/// whose on_click references its own props, e.g.
/// `on_click { thong_bao($mo_ta) }` inside a component receiving
/// `mo_ta: chuoi`), the instance's id is passed so `$mo_ta` resolves
/// correctly via `component_scope_stack` - see the
/// `eval_expr_id_tracked` doc-comment (dom.rs) for the full reasoning
/// behind pushing/popping RIGHT AT CALL TIME (not depending on anyone
/// having pushed one beforehand).
pub fn eval_with_loop_frame(
    shared: &SharedState,
    expr: &Expr,
    loop_frame: Option<&super::state::LoopFrame>,
    component_id: Option<&str>,
) -> VbValue {
    if let Some(frame) = loop_frame {
        shared.borrow_mut().push_loop_scope(frame.clone());
    }
    if let Some(cid) = component_id {
        shared.borrow_mut().push_component_scope(cid);
    }

    let mut state = shared.borrow_mut();
    let result = eval_inner(&mut state, expr, false);
    drop(state);

    if let Some(cid) = component_id {
        shared.borrow_mut().pop_component_scope(cid);
    }
    if loop_frame.is_some() {
        shared.borrow_mut().pop_loop_scope();
    }

    result
}

// ════════════════════════════════════════════════════════════
// CORE TREE-WALKING EVALUATOR
// ════════════════════════════════════════════════════════════

/// `tracked` decides whether Variable/MemberAccess reads go through the
/// dependency-recording path (`scope_resolve_tracked`) or not
/// (`scope_resolve`). Receives `&mut State` (not `&SharedState`) since
/// internal recursion doesn't need to repeatedly re-borrow the
/// `RefCell` - avoiding overhead + avoiding the risk of "already
/// borrowed" if `shared.borrow_mut()` accidentally gets called nested
/// twice.
/// Evaluates an Expr, with NO tracking, and needing ONLY `&State`
/// (not `&mut State`) - used SPECIFICALLY by the getter closures of
/// `register_props()`/`component_props` (see state.rs::prop_scope, with
/// the fixed signature `Box<dyn Fn(&State) -> VbValue>`).
///
/// WHY THIS SEPARATE FUNCTION IS NEEDED (instead of reusing
/// `eval`/`eval_tracked` above): both `eval`/`eval_tracked` call
/// `shared.borrow_mut()` RIGHT AS THEY ENTER. But a component prop's
/// getter is always called FROM INSIDE another already-running
/// `eval_inner` pass (state is already borrowed - `prop_scope()` is
/// called by `scope_resolve()`/`scope_resolve_tracked()`, and those 2
/// functions are themselves called by `eval_inner` when it hits an
/// `Expr::Variable`) - if the getter closure called back into
/// `eval`/`eval_tracked` (i.e. `shared.borrow_mut()` AGAIN on the SAME
/// `Rc<RefCell<State>>`), that would be a double-borrow, causing the
/// `RefCell` to PANIC AT RUNTIME ("already mutably borrowed"). This
/// function takes `&State` directly, already provided by
/// `prop_scope(&self, ...)`, borrowing nothing else itself.
///
/// A KNOWN LIMITATION (inherited from `register_props`'s own design -
/// not specific to this function): since only `&State` (immutable) is
/// available, the `Expr::Variable` branch here uses
/// `state.scope_resolve()` - the NO-TRACKING version. This means if a
/// prop's value at the moment a component was called references a
/// global variable (e.g. `tieu_de: $ten_bien_global` instead of a
/// string literal), the value read out is still CORRECT at mount time,
/// but will NOT auto-update if that variable changes later (no
/// reactivity through props - only static literal props, matching how
/// ViBao component calls are actually used in practice, are guaranteed
/// to work fully, including reactively).
pub fn eval_readonly(state: &State, expr: &Expr) -> VbValue {
    eval_readonly_with_frame(state, expr, None)
}

/// Same as `eval_readonly` above, but additionally resolves `name`/
/// `name.path...` against `extra_frame` FIRST (before falling through
/// to `state.scope_resolve`), if given.
///
/// WHY THIS EXISTS (root cause of "component called directly inside a
/// vong_lap, itself passing `$item.field` as a prop, always resolves to
/// Null/empty inside the component's own body" - see the long
/// investigation note on `bind_component` in dom.rs for the full
/// history): a component prop getter (`register_props`'s
/// `Box<dyn Fn(&State) -> VbValue>`) is not called at bind time - it's
/// stored away and only invoked LATER, lazily, whenever some binding
/// inside the component's own body (its OWN text/if/... on ITS OWN
/// params) asks `prop_scope(propName)` for the current value. By that
/// time, the loop item that the prop expression (`$bv.tieu_de`) needs
/// to read from is NO LONGER on `state.loop_scope_stack` - nothing ever
/// pushed it there for this call, since `bind_component`'s getter used
/// to call plain `eval_readonly` (using ONLY the global stack,
/// unaware of which specific loop item this exact component instance
/// belongs to). `bind_text`/`bind_if`/etc. don't have this problem: they
/// package their OWN `loop_frame` into their closure and PUSH it onto
/// the global stack (via `eval_expr_id_tracked`) every single time
/// their closure runs, however long after bind time that may be. A
/// component prop getter must do the same thing, but it can't use that
/// push/pop mechanism directly (see the doc-comment on `eval_readonly`
/// above: getters only ever receive `&State`, not `&SharedState`/
/// `&mut State`, precisely to avoid a double-borrow panic - there is no
/// way to `push_loop_scope`/`pop_loop_scope` through a shared `&State`
/// reference). Instead, the getter captures the same `LoopFrame` it was
/// created with (by value, via `.clone()`) and passes it here as
/// `extra_frame` on every call - checked FIRST, ahead of whatever
/// (unrelated, or simply empty) loop frame the global stack happens to
/// be carrying at that exact moment.
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

/// Resolves a bare `Expr::Variable(name)` the same way `State::scope_resolve`
/// does (item var / index var / "item.sub.path" against a loop frame,
/// falling back to `prop_scope`), except checking `extra_frame` (the
/// frame a component prop getter was created with, if any) BEFORE the
/// live global `loop_scope_stack` - see `eval_readonly_with_frame`'s
/// doc-comment above for why this extra frame is needed at all.
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
                // NOTE: does not consult extra_frame - `get_path` reads
                // straight from global state/vars, matching the
                // pre-existing behavior of this specific branch (a
                // dotted "Member" template part was already never
                // loop-scope-aware, even in plain, non-component
                // bindings - out of scope for this fix, which is
                // specifically about `$item.field`-as-prop resolving to
                // Null inside a component's body).
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

// ════════════════════════════════════════════════════════════
// LITERAL
// ════════════════════════════════════════════════════════════

fn eval_literal(lit: &LiteralValue) -> VbValue {
    match lit {
        // A CSS unit (if any) has no meaning inside a pure JS/logic
        // expression (e.g. $n - 1) - only the numeric part is used. This
        // is CONSISTENT with `codegen/expr.rs::literal_to_js`, which
        // also drops the unit when generating JS for a pure arithmetic
        // expression context.
        LiteralValue::Num(n, _unit) => VbValue::Num(*n),
        LiteralValue::Str(s) => VbValue::Str(s.clone()),
        LiteralValue::Bool(b) => VbValue::Bool(*b),
        LiteralValue::Color(c) => VbValue::Str(c.clone()),
    }
}

// ════════════════════════════════════════════════════════════
// BINARY OP
// ════════════════════════════════════════════════════════════

fn eval_binary(op: BinaryOp, l: &VbValue, r: &VbValue) -> VbValue {
    match op {
        BinaryOp::Add => eval_add(l, r),
        BinaryOp::Sub => VbValue::Num(l.to_num_or_zero() - r.to_num_or_zero()),
        BinaryOp::Mul => VbValue::Num(l.to_num_or_zero() * r.to_num_or_zero()),
        BinaryOp::Div => VbValue::Num(l.to_num_or_zero() / r.to_num_or_zero()),
        BinaryOp::Mod => VbValue::Num(l.to_num_or_zero() % r.to_num_or_zero()),
        // Matches the JS codegen behavior (always generating "===" /
        // "!=="") - a STRICT comparison by type + value, with no
        // implicit type coercion.
        BinaryOp::Eq => VbValue::Bool(l.strict_eq(r)),
        BinaryOp::Neq => VbValue::Bool(!l.strict_eq(r)),
        BinaryOp::Gt => VbValue::Bool(l.partial_cmp_loose(r) == std::cmp::Ordering::Greater),
        // BUG ALREADY FIXED: Gte/Lte used to be written as
        // "!= Less" / "!= Greater" - correct in pure mathematical terms
        // ("a >= b" <=> "!(a < b)") but WRONG whenever
        // partial_cmp_loose() fell into its Ordering::Equal fallback
        // (used for cases that AREN'T actually comparable, e.g. NaN on
        // one side, or a string that doesn't parse as a number - Equal
        // there is only a safe default value, NOT meaning "these 2
        // values are equal"). The old formula derived Gte/Lte = true for
        // every "not comparable" case, e.g. "NaN >= 10" or
        // "'abc' >= 10" returned TRUE - wrong compared to real JS
        // (Number.NaN >= 10 is always false). The real-world
        // consequence: a state variable not yet assigned a number
        // (Null), or one that accidentally received a non-numeric
        // string, could make `neu $x >= N` take the WRONG branch instead
        // of false like JS.
        //
        // The fix: uses is_nan_like() to explicitly rule out the "not
        // comparable" case BEFORE composing the Greater-or-Equal logic -
        // that case now always evaluates to false for both Gte/Lte,
        // matching real JS correctly.
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

/// "+" adds numerically when both operands can behave as numbers;
/// otherwise it concatenates strings. This keeps text composition like
/// `"Xin chao " + $ten` working while avoiding the common counter bug
/// where an unset state value (`Null`) or a numeric-looking string from
/// prior state writes turns `$count = $count + 1` into string growth.
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

// ════════════════════════════════════════════════════════════
// FUNCTION CALLS (__fmt.*, lam_tron, and a fallback for unknown functions)
// ════════════════════════════════════════════════════════════

/// Maps a ViBao utility function name to its corresponding Rust
/// behavior - equivalent to `__fmt.*` on the old JS side AND
/// `map_call_fn` on the codegen side (but EXECUTING the result here
/// instead of just renaming it to generate JS).
///
/// A function not in the table below (a custom action, calling a
/// component...) returns `VbValue::Null` - the evaluator is NOT the
/// place to execute side effects (opening a modal, calling an API...),
/// those belong to `runtime/dom.rs`/`api.rs` (actions, not
/// expressions). If an Expr::Call ends up here with a side-effect-group
/// callee, that's a codegen bug that wrongly registered an action expr
/// into the expression registry - not something the evaluator can fix
/// itself, so returning Null instead of guessing is the safest choice.
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
            // A simple VND currency format: thousands grouped with dots,
            // an "d" suffix - equivalent to __fmt.giaTien in the old JS
            // version.
            let n = args.first().map(|v| v.to_num_or_zero()).unwrap_or(0.0);
            VbValue::Str(format!("{}đ", group_thousands(n as i64)))
        }
        "ngay" => {
            // Date formatting: a minimal version, only formatting an
            // existing ISO string
            // into dd/mm/yyyy if the input is already an ISO string
            // "yyyy-mm-dd...". There's NO full Date/timezone logic here -
            // the evaluator running inside WASM has no direct access to
            // JS's Date (would need js-sys for "now"); for an existing
            // date string, manual parsing is enough for most display use
            // cases.
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

/// Groups digits by thousands with a dot separator - e.g. 1234567 -> "1.234.567".
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

/// Parses an ISO string "yyyy-mm-dd" (optionally with
/// "Thh:mm:ss...") into "dd/mm/yyyy". Returns the original string
/// unchanged if the format doesn't match.
fn format_iso_date(s: &str) -> VbValue {
    let date_part = s.split('T').next().unwrap_or(s);
    let parts: Vec<&str> = date_part.split('-').collect();
    if parts.len() == 3 {
        VbValue::Str(format!("{}/{}/{}", parts[2], parts[1], parts[0]))
    } else {
        VbValue::Str(s.to_string())
    }
}

// ════════════════════════════════════════════════════════════
// COLOR FUNCTIONS (trong_suot, lam_sang, lam_toi)
// ════════════════════════════════════════════════════════════

/// Applies a color function to a hex code "#RRGGBB", returning a new
/// hex/rgba code. `amount` is in the range 0-100. If `color` isn't a
/// valid hex string, returns the original value unchanged (safe, never
/// panics).
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
            // Transparency: returns rgba() with alpha = 1 - amount/100.
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

// ════════════════════════════════════════════════════════════
// TEMPLATE STRING
// ════════════════════════════════════════════════════════════

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
                    // get_path has no separate tracked variant - each
                    // part is dug through manually here so the "root"
                    // part is tracked correctly.
                    let mut parts_iter = path.iter();
                    let root_name = match parts_iter.next() {
                        Some(p) => p,
                        // An empty path (shouldn't happen - the parser
                        // always creates a Member with at least 1
                        // element) - safely skipped, nothing to append
                        // to the output for this element.
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

// ════════════════════════════════════════════════════════════
// UNIT TESTS
// ════════════════════════════════════════════════════════════

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
        // 100% brightening from black -> full white.
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
        assert_eq!(*seen.borrow(), 2.0); // tự re-run vì evalTracked đã track "n"
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

    // ════════════════════════════════════════════════════════════
    // REGRESSION: component prop referencing a loop item
    // (`TheXyz(tieu_de: $bv.tieu_de)` called directly inside
    // `vong_lap $bv trong $list { ... }`) must resolve correctly even
    // though the getter runs LONG AFTER the loop's own push/pop of
    // `$bv` onto the global loop_scope_stack has already happened -
    // see the full explanation on `eval_readonly_with_frame` above and
    // on `bind_component` in dom.rs.
    // ════════════════════════════════════════════════════════════

    #[test]
    fn test_eval_readonly_with_frame_resolves_field_from_captured_frame() {
        use super::super::state::LoopFrame;

        let shared = setup();
        // The global loop_scope_stack is EMPTY here - simulating the
        // real bug scenario: a component prop getter runs lazily, well
        // after bind_loop's own push_loop_scope/pop_loop_scope for this
        // exact item has already completed.
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

        // $bv.tieu_de
        let e = Expr::MemberAccess {
            object: Box::new(Expr::Variable("bv".to_string(), p())),
            property: "tieu_de".to_string(),
            pos: p(),
        };

        // Without the captured frame (the OLD, buggy behavior): with an
        // empty loop_scope_stack, "$bv" resolves to Null, so
        // "$bv.tieu_de" is also Null - reproducing the exact reported
        // bug.
        assert_eq!(eval_readonly(&state_ref, &e), VbValue::Null);

        // WITH the captured frame passed through explicitly (the FIX):
        // resolves to the real value regardless of what the global
        // stack currently holds.
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

        // "$ten" doesn't match extra_frame's item_var ("bv"), so it
        // must still fall through to global state exactly like plain
        // eval_readonly.
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
