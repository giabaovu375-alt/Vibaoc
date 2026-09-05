// ============================================================
// VIBAO COMPILER (Rust) — codegen/expr.rs
// Compiles an Expr (the expression AST) into either: (1) a registration
// in the expr registry so the WASM runtime computes it itself in Rust
// (register_expr/take_expr_registry - the ONLY path actually used in the
// build pipeline), or (2) an extracted STATIC value known at build time
// (resolve_value/get_static_value, used for static CSS/attributes).
//
// HISTORY: this file USED TO have a set of functions that generated
// plain JS for the browser to eval
// (expr_to_js/expr_to_js_default/expr_to_js_registry/literal_to_js/
// binary_op_js/map_call_fn/map_color_fn/template_to_js/escape_template_text)
// - the ORIGINAL approach from when the runtime was still JS. That
// entire block WAS REMOVED (along with many corresponding tests) after
// confirming: (a) map_call_fn/map_color_fn generated calls to
// `__fmt.*`/`__color.*` - these JS objects do NOT EXIST in the current
// runtime (the runtime computes the equivalent in pure Rust, see
// vibao-runtime::expr_eval) - this once caused a real bug (crashing the
// entire app.js if a global variable used a utility function, see the
// fix history in codegen/mod.rs::gen_app_js); (b) `resolve_value()` (a
// LIVE function, used extensively) used to call these JS-emitting
// functions to stuff a JS string into `ResolvedValue::Dynamic(String)`,
// but a full codebase search confirmed NO caller ever read that string
// (every call site only matched `Dynamic(_)`/called `is_dynamic()`,
// always calling `register_expr(the original expr)` itself instead of
// using the JS translation) - so the variant was changed to `Dynamic`
// (carrying no data), eliminating the need to compute that useless JS
// string entirely.
// ============================================================

use std::cell::RefCell;

use vibao_ast::{Expr, LiteralValue};

thread_local! {
    /// The accumulator holding every Expr registered during the current
    /// build pass. The index in the Vec IS the id used in
    /// "__vb.evalExpr(id)" - simpler than a HashMap since an id only
    /// needs to be unique within a single build, not stable across
    /// different builds.
    static EXPR_REGISTRY: RefCell<Vec<Expr>> = RefCell::new(Vec::new());
}

/// Registers an Expr into the registry, returning its id (index) - this
/// id gets embedded in the HTML/JSON output (e.g.
/// "data-vb-text=\"<id>\"", or inside "data-vb-props") so the WASM
/// runtime can look it up in the registry and compute it itself in Rust
/// (vibao-runtime::expr_eval), generating NO JS at build time.
pub fn register_expr(expr: Expr) -> usize {
    EXPR_REGISTRY.with(|reg| {
        let mut reg = reg.borrow_mut();
        reg.push(expr);
        reg.len() - 1
    })
}

/// Retrieves the entire accumulated registry AND clears it (resetting
/// for the next build pass - important if `main.rs` builds multiple
/// pages within a single process run, to avoid one page's registry
/// leaking into another's). This should be called AFTER all JS for a
/// page has been generated, then the returned result (Vec<Expr>) is
/// serialized to JSON and embedded in the output.
pub fn take_expr_registry() -> Vec<Expr> {
    EXPR_REGISTRY.with(|reg| std::mem::take(&mut *reg.borrow_mut()))
}

// ════════════════════════════════════════════════════════════
// NUMBER -> STRING (shared by resolve_value/get_static_value)
// ════════════════════════════════════════════════════════════

/// Formats a number so it has no trailing decimal part (e.g. 5.0 -> "5",
/// not "5.0").
fn format_number(n: f64) -> String {
    if n.fract() == 0.0 && n.abs() < 1e15 {
        format!("{}", n as i64)
    } else {
        n.to_string()
    }
}

/// Escapes a Rust string into a valid JSON string literal - used by
/// `component.rs::build_props_json()` to escape a prop's KEY inside a
/// JSON object (e.g. {"tieu_de": 0}), unrelated to the removed
/// JS-emitting architecture (the function name stays "json_string"
/// since this really is producing JSON, not raw JS).
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

// ════════════════════════════════════════════════════════════
// RESOLVED VALUE — used in props.rs to decide static vs. dynamic style
// ════════════════════════════════════════════════════════════

/// The result of resolving an Expr in the context of a prop value -
/// either a static constant (known right at compile time) or a dynamic
/// expression (only known at runtime, needing binding via
/// data-vb-bind-*).
#[derive(Debug, Clone)]
pub enum ResolvedValue {
    /// A static string value used directly (not a number/color, e.g. "row").
    Static(String),
    /// A CSS size that already has a unit (e.g. "16px", "50%").
    Size(String),
    /// A color already resolved to a valid CSS value (hex or var(--...)).
    Color(String),
    /// A dynamic expression - every caller ONLY needs to know "this is
    /// dynamic" in order to decide to call `register_expr(expr.clone())`
    /// (registering the ORIGINAL Expr itself, not any already-translated
    /// form), never reading any content back from here. This variant
    /// used to carry a `String` (JS code generated by
    /// `expr_to_js_default`/`template_to_js`, the old JS architecture) -
    /// but a full codebase search confirmed NOT A SINGLE call site ever
    /// read that string (every match site only used `Dynamic(_)`,
    /// discarding the content). The field was removed entirely, since
    /// there's no need to compute a JS string nobody uses (and this also
    /// avoids the risk of that string calling a nonexistent JS API - see
    /// the history of a similar bug fixed in gen_app_js()).
    Dynamic,
}

impl ResolvedValue {
    /// Returns the corresponding CSS string for the static variants
    /// (Static/Size/Color) - used when the caller already knows for
    /// certain the value isn't Dynamic. For Dynamic, returns an empty
    /// string (the caller must handle binding itself).
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

/// Resolves an Expr into a ResolvedValue - equivalent to resolveValue()
/// in the old TS version. Number/color literals are handled specially
/// (adding the px unit, resolving a color name to hex); every other
/// expression (variables, operators, function calls...) is always
/// Dynamic since its value can only be determined at runtime.
pub fn resolve_value(expr: &Expr) -> ResolvedValue {
    match expr {
        Expr::Literal(LiteralValue::Color(hex), _) => ResolvedValue::Color(hex.clone()),
        // A number with an explicit CSS unit (e.g. "50%", "10vw") keeps
        // that unit; a bare number (no unit, e.g. "16") defaults to px -
        // matching resolveValue()'s behavior in the old TS version
        // (lit.value always had "px" appended unless the original string
        // already contained %/vw/vh).
        Expr::Literal(LiteralValue::Num(n, Some(unit)), _) => {
            ResolvedValue::Size(format!("{}{}", format_number(*n), unit))
        }
        Expr::Literal(LiteralValue::Num(n, None), _) => {
            ResolvedValue::Size(format!("{}px", format_number(*n)))
        }
        Expr::Literal(LiteralValue::Str(s), _) => ResolvedValue::Static(s.clone()),
        Expr::Literal(LiteralValue::Bool(b), _) => ResolvedValue::Static(b.to_string()),
        // Variable/MemberAccess/Binary/Unary/Call/ColorFunc/Array/Object/
        // TemplateString can never be known at compile time - always
        // Dynamic (the caller will call register_expr(the original
        // expr) itself; no JS translation is needed here - see the note
        // on ResolvedValue::Dynamic).
        _ => ResolvedValue::Dynamic,
    }
}

/// Extracts an Expr's STATIC value as a raw string, used in places that
/// only accept a value known at compile time (e.g. resolveLayoutCSS).
/// Returns "__dynamic__" for a dynamic expression - the same sentinel
/// value used in the old TS version (getStaticValue), so downstream
/// functions can `if v == "__dynamic__"` and skip it instead of
/// crashing.
pub fn get_static_value(expr: &Expr) -> String {
    match expr {
        Expr::Literal(LiteralValue::Str(s), _) => s.clone(),
        // The unit is kept in the returned string (e.g. "50%", not just
        // "50") since layout.rs (px/size/spacing) checks with a regex
        // whether the string already has a unit before deciding whether
        // to append "px" - exactly like the old TS version's use of
        // String(lit.value), which kept the suffix intact.
        Expr::Literal(LiteralValue::Num(n, Some(unit)), _) => format!("{}{}", format_number(*n), unit),
        Expr::Literal(LiteralValue::Num(n, None), _) => format_number(*n),
        Expr::Literal(LiteralValue::Bool(b), _) => b.to_string(),
        Expr::Literal(LiteralValue::Color(c), _) => c.clone(),
        _ => "__dynamic__".to_string(),
    }
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

    // REMOVED: tests for expr_to_js/expr_to_js_default/expr_to_js_registry/
    // map_call_fn/map_color_fn/template_to_js (test_variable_to_js,
    // test_binary_eq_becomes_strict_eq, test_member_access_rong_do_dai,
    // test_call_fn_mapping, test_call_fn_unmapped_kept_as_is,
    // test_color_func_mapping, test_template_string_to_js,
    // test_template_member_path, test_expr_to_js_registry_emits_eval_call,
    // test_expr_to_js_unchanged_for_existing_behavior) - every function
    // they tested has been removed (the old JS architecture, see the
    // note at the top of the file).

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

    // ── Expr registry (Rust/WASM evaluator) ─────────────────────────

    #[test]
    fn test_registry_assigns_sequential_ids() {
        // Each test runs on its own thread (the Rust test framework),
        // and the registry is thread_local, so there's no risk of one
        // test "polluting" another's ids - but to be safe, this doesn't
        // assume ids start at 0, only checking that the 2nd id equals
        // the 1st id + 1 (monotonic increase).
        let e1 = Expr::literal_num(1.0, p());
        let e2 = Expr::literal_num(2.0, p());
        let id1 = register_expr(e1);
        let id2 = register_expr(e2);
        assert_eq!(id2, id1 + 1);
    }

    #[test]
    fn test_take_expr_registry_drains_and_resets() {
        // Clean this thread's registry first (in case another test on
        // the same thread already registered something - cargo test can
        // reuse a thread across multiple sequential tests).
        take_expr_registry();

        register_expr(Expr::literal_num(1.0, p()));
        register_expr(Expr::literal_num(2.0, p()));

        let drained = take_expr_registry();
        assert_eq!(drained.len(), 2);

        // After taking, the registry must be empty again.
        let empty = take_expr_registry();
        assert_eq!(empty.len(), 0);
    }
}
