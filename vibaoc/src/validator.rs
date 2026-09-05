// ============================================================
// VIBAO COMPILER (Rust) — validator.rs
//
// THE VALIDATION LAYER (semantic analysis pass) — runs AFTER the
// parser + resolver (multi-file merging already done), BEFORE codegen:
//
//   Lexer -> Parser -> Resolver (merges nhap) -> Validator (HERE) -> Codegen
//
// WHY THIS LAYER EXISTS:
// Before this file, semantic validity checks (does a color name exist,
// is a called component actually defined...) were SCATTERED throughout
// codegen itself — each call site decided its own way to handle
// "unexpected" data, and some silently swallowed errors. A real
// example: a nonexistent color name ("xanh_duong") got used verbatim as
// a CSS value, the browser silently ignored that property, and nobody
// knew why the color wasn't showing. Consolidating every rule into one
// layer gives: (1) codegen can assume its input AST is already valid,
// with no need for scattered defensive checks; (2) a dev sees EVERY
// semantic error in one build pass, like tsc, instead of a slow
// fix-build-repeat cycle revealing one error at a time.
//
// PRINCIPLE: the validator does NOT modify the AST, it only READS and
// collects errors.
// ============================================================

use std::collections::{HashMap, HashSet};

use vibao_ast::{
    Action, App, CaseNode, Child, ComponentCall, ComponentDef, Element, EventNode, Expr, IfNode,
    LiteralValue, LoopKind, LoopNode, PropsMap, SwitchNode, Theme,
};

use crate::lexer::color_map;

/// A specific semantic error - kept separate from ParseError/ResolverError
/// since this is a different class of error: the parser fails when
/// SYNTAX is wrong, the validator fails when the syntax is CORRECT but
/// the MEANING is wrong (e.g. "xanh_duong" is a valid identifier just
/// like "xanh", it's only its meaning that's wrong - it doesn't exist).
#[derive(Debug, Clone)]
pub struct ValidationError {
    pub message: String,
    pub line: usize,
    pub column: usize,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "[semantic] {} ({}:{})", self.message, self.line, self.column)
    }
}

/// A prop with a color semantic identity - its value MUST be a hex
/// code, a color name in color_map(), or a variable ($ten)/dynamic
/// expression. Doesn't keep a separate surface-string list so English
/// names/aliases don't get missed.
fn is_color_prop(surface_name: &str) -> bool {
    matches!(
        crate::locale::resolve_prop_key(surface_name),
        Some(vibao_ast::PropKey::Color | vibao_ast::PropKey::BorderColor | vibao_ast::PropKey::BackgroundColor)
    )
}

/// The single entry point: receives an `App` that's already been
/// through the resolver, returning `Ok(())` if there are no errors, or
/// `Err(Vec<ValidationError>)` containing EVERY error found in one
/// traversal pass (doesn't stop at the first error).
pub fn validate(app: &App) -> Result<(), Vec<ValidationError>> {
    let mut errors = Vec::new();

    check_themes(&app.themes, &mut errors);

    // App.variables (app-level global variables, `bien $x = gia_tien(1000)`)
    // used to be completely UNVISITED by validate() - the same kind of
    // gap already found/fixed for page.events (a similar BUG-25, a
    // different group). This is the EXACT example used as evidence
    // during the original FunctionName investigation (`codegen/mod.rs`
    // around line 481) - now finally actually validated.
    for var in &app.variables {
        check_expr(&var.value, &mut errors);
    }
    for component in &app.components {
        for param in &component.params {
            if let Some(default) = &param.default_value {
                check_expr(default, &mut errors);
            }
        }
    }

    let component_names = check_duplicate_components(&app.components, &mut errors);

    for component in &app.components {
        check_children(&component.children, &component_names, &mut errors);
    }
    for page in &app.pages {
        check_children(&page.children, &component_names, &mut errors);
        // page.states (page-level state) also used to be unvisited,
        // for the same reason above.
        for state in &page.states {
            check_expr(&state.value, &mut errors);
        }
        // A GAP THAT WAS FIXED (found while wiring ActionName into the
        // validator, see AUDIT.md): `page.events: Vec<PageEvent>`
        // (page-level khi_tai/khi_huy - PageEvent, DIFFERENT from
        // Child::PageEvent, which check_children deliberately skips
        // since it contains no Element/ComponentCall tree) used to be
        // completely unvisited here - Actions inside khi_tai/khi_huy
        // were never checked by check_action_name(), even though
        // element-level events (khi_nhan/khi_doi/...) already were, via
        // check_element(). This is the correct place for page-level
        // events to be visited - not through check_children (that's for
        // Child, and PageEvent isn't a Child).
        for event in &page.events {
            check_actions(&event.body, &mut errors);
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

// ════════════════════════════════════════════════════════════
// THEME — parsed but not implemented; blocked at the validator
// ════════════════════════════════════════════════════════════
//
// `theme` has full lexer keyword, AST support (`Theme { name,
// variables, pos }`), and a parser (`parse_theme()`), but stops there:
// it isn't merged by the resolver through imports, nothing in codegen
// reads `app.themes`, and there's no runtime hook or test for it.
//
// Rather than let a `theme` declaration silently compile into an app
// where it has no effect, the validator rejects it outright with a
// clear message, the same treatment given to other unfinished
// features that would otherwise fail silently at runtime. A likely
// eventual design is a CSS custom property (`var(--name)`) per
// declared variable, but that's inferred from the data shape rather
// than confirmed, so it isn't implemented speculatively here.
fn check_themes(themes: &[Theme], errors: &mut Vec<ValidationError>) {
    for theme in themes {
        errors.push(ValidationError {
            message: format!(
                "Theme declaration \"theme {}\" is not supported by ViBao yet. Remove the theme or use a global variable/state until theme support is implemented.",
                theme.name
            ),
            line: theme.pos.line,
            column: theme.pos.column,
        });
    }
}

/// Checks for duplicate `@the` names - returns the set of valid
/// component names (used to check for "calling an undefined component"
/// later). If a name is duplicated, both are still treated as
/// "existing" - one clear error at the source is better than many
/// confusing "undefined" errors scattered elsewhere.
fn check_duplicate_components(components: &[ComponentDef], errors: &mut Vec<ValidationError>) -> HashSet<String> {
    let mut seen: HashMap<&str, &ComponentDef> = HashMap::new();
    let mut names = HashSet::new();

    for def in components {
        names.insert(def.name.clone());
        if let Some(first) = seen.get(def.name.as_str()) {
            errors.push(ValidationError {
                message: format!("Component \"@the {}\" is defined more than once (first defined at line {}). Rename one definition or remove the duplicate.", def.name, first.pos.line),
                line: def.pos.line,
                column: def.pos.column,
            });
        } else {
            seen.insert(def.name.as_str(), def);
        }
    }

    names
}

/// Recursively visits the entire `Child` tree - must handle EVERY
/// variant (see child.rs) so no Element/ComponentCall nested deep
/// inside an if/switch/loop is missed.
fn check_children(children: &[Child], component_names: &HashSet<String>, errors: &mut Vec<ValidationError>) {
    for child in children {
        match child {
            Child::Element(el) => check_element(el, component_names, errors),
            Child::ComponentCall(call) => check_component_call(call, component_names, errors),
            Child::If(if_node) => check_if(if_node, component_names, errors),
            Child::Switch(switch_node) => check_switch(switch_node, component_names, errors),
            Child::Loop(loop_node) => check_loop(loop_node, component_names, errors),
            // FIXED A GAP: an old comment here was correct for its
            // ORIGINAL PURPOSE (StateDecl/VarDecl/PageEvent contain no
            // Element/ComponentCall subtree) but was WRONG once
            // FunctionName validation was added - VarDecl.value/
            // StateDecl.value ARE Expr, which can contain an Expr::Call
            // needing check_expr(). This is a LOCAL variable/state
            // declaration (inside a Child block, different from
            // App.variables/Page.states, which are visited separately in
            // validate()) - this gap was found while re-reviewing the
            // full Child enum to write tests for the new validation, not
            // pointed out by anyone else.
            Child::StateDecl(decl) => check_expr(&decl.value, errors),
            Child::VarDecl(decl) => check_expr(&decl.value, errors),
            // Child::PageEvent(event) - CONFIRMED: the parser NEVER
            // actually creates this variant inside Child (only the
            // separate Page.events: Vec<PageEvent> field, already
            // visited in validate()) - handling it here is still
            // CORRECT IN PRINCIPLE (if a future path ever creates it,
            // validation is already right, with nothing to remember to
            // fix here) but NO real test currently exercises this branch
            // (currently dead code, not an active gap).
            Child::PageEvent(event) => check_actions(&event.body, errors),
        }
    }
}

fn check_element(el: &Element, component_names: &HashSet<String>, errors: &mut Vec<ValidationError>) {
    let tag_name: &str = crate::locale::vi::tag_display_name_vi(el.tag);
    check_color_props(tag_name, &el.props, errors);
    check_props_map_exprs(&el.props, errors);
    check_events(&el.events, errors);
    check_children(&el.children, component_names, errors);
}

/// Visits EVERY Expr in a PropsMap (not just color props like
/// check_color_props) looking for a nested Expr::Call (e.g.
/// `text(noi_dung: gia_tien($x))`).
fn check_props_map_exprs(props: &PropsMap, errors: &mut Vec<ValidationError>) {
    for (_, expr) in props {
        check_expr(expr, errors);
    }
}

fn check_component_call(call: &ComponentCall, component_names: &HashSet<String>, errors: &mut Vec<ValidationError>) {
    if !component_names.contains(&call.name) {
        errors.push(ValidationError {
            message: format!("Component \"{}\" was used but no \"@the {}\" definition was found. Check the spelling or import it with `nhap {} tu \"...\"`.", call.name, call.name, call.name),
            line: call.pos.line,
            column: call.pos.column,
        });
    }
    check_color_props(&call.name, &call.props, errors);
    check_props_map_exprs(&call.props, errors);
    check_children(&call.children, component_names, errors);
}

fn check_if(if_node: &IfNode, component_names: &HashSet<String>, errors: &mut Vec<ValidationError>) {
    check_expr(&if_node.condition, errors);
    check_children(&if_node.consequent, component_names, errors);
    if let Some(alt) = &if_node.alternate {
        check_children(alt, component_names, errors);
    }
}

fn check_switch(switch_node: &SwitchNode, component_names: &HashSet<String>, errors: &mut Vec<ValidationError>) {
    check_expr(&switch_node.subject, errors);
    for case in &switch_node.cases {
        check_case(case, component_names, errors);
    }
    if let Some(default_body) = &switch_node.default_case {
        check_children(default_body, component_names, errors);
    }
}

fn check_case(case: &CaseNode, component_names: &HashSet<String>, errors: &mut Vec<ValidationError>) {
    check_expr(&case.value, errors);
    check_children(&case.body, component_names, errors);
}

fn check_loop(loop_node: &LoopNode, component_names: &HashSet<String>, errors: &mut Vec<ValidationError>) {
    // Both LoopKind::Each/Range have a body: Vec<Child> that needs
    // checking the same way - kept as an explicit match so if
    // LoopKind ever gains a new variant, the compiler reminds us
    // instead of silently missing a branch.
    match &loop_node.kind {
        LoopKind::Each { iterable, .. } => {
            check_expr(iterable, errors);
            check_children(&loop_node.body, component_names, errors);
        }
        LoopKind::Range { .. } => {
            // Range only has from/to: i64 (a plain integer at parse
            // time, not an Expr) + a counter variable name String -
            // there's no Expr here to check_expr().
            check_children(&loop_node.body, component_names, errors);
        }
    }
}

/// Checks every COLOR prop in a PropsMap - the actual root-cause fix
/// for the "mau: xanh_duong" silently-swallowed bug: an identifier in a
/// color prop's value position that does NOT match any name in
/// color_map() (and isn't a hex/variable) is caught RIGHT HERE, before
/// codegen turns it into a meaningless CSS string.
fn check_color_props(tag_or_name: &str, props: &PropsMap, errors: &mut Vec<ValidationError>) {
    for (key, expr) in props {
        if !is_color_prop(key) {
            continue;
        }
        // Only LiteralValue::Str is suspect - Hex/Variable/ColorFunc are
        // always valid since their syntax already guarantees the correct
        // color format (confirmed by the lexer/parser at parse time).
        if let Expr::Literal(LiteralValue::Str(name), pos) = expr {
            if !color_map().contains_key(name.as_str()) {
                errors.push(ValidationError {
                    message: format!(
                        "Value \"{}\" for color prop \"{}\" on \"{}\" is invalid. Use a hex color, a supported ViBao color name, or a variable ($...).",
                        name, key, tag_or_name,
                    ),
                    line: pos.line,
                    column: pos.column,
                });
            }
        }
    }
}

// ════════════════════════════════════════════════════════════
// ACTION NAME VALIDATION — wiring ActionName into the real validator
// ════════════════════════════════════════════════════════════
//
// DECISION SETTLED (asked the user): calling a nonexistent action is a
// HARD ERROR (blocks the build), CONSISTENT with `check_color_props`
// above (a wrong color name also hard-blocks the build) - DIFFERENT
// from how BUG-25/BUG B handle an unknown prop on layout/responsive
// (a soft warning, doesn't block the build). These 2 standards
// deliberately coexist in the project, depending on the error type:
// - An unknown prop on layout/responsive: the build CAN still produce
//   an "approximately correct" result (an arbitrary CSS property via
//   passthrough) - a warning is enough.
// - A wrong color name / nonexistent action: the build produces a
//   result that is COMPLETELY WRONG (the color doesn't show, the
//   action does nothing) with NO passthrough to fall back on - a hard
//   error is the right call, avoiding a dev debugging blind on an app
//   that "built successfully" but behaves completely wrong.
//
// 3 DISTINCT STATES (a design settled during the ActionName
// investigation, see ARCHITECTURE_PROPOSAL.md) - THIS IS THE FIRST
// PLACE the "Known-but-Unsupported" state is ACTUALLY used to produce
// different behavior (before this it was just standalone data in
// `locale::action_vi::KNOWN_BUT_UNSUPPORTED_ACTIONS_VI`, with no
// function consuming it yet - see VIBAOC_ACTIONNAME_BUGS.md item 3,
// which corrected the wording to match reality at the time):
//   Unknown (a plain typo, e.g. "thongbao" missing its underscore)
//     -> error: "Unknown action... Check the spelling..."
//   Known-but-Unsupported (correctly spelled, but not yet implemented
//   at runtime - CURRENTLY ONLY "dang_xuat")
//     -> a SEPARATE error, clearly explaining why (missing an
//        auth/session model), NOT confused with the "typo" message
//        above.
//   Supported (one of the 15 ActionName values)
//     -> no error.

fn check_events(events: &[EventNode], errors: &mut Vec<ValidationError>) {
    for event in events {
        check_actions(&event.body, errors);
    }
}

fn check_actions(actions: &[Action], errors: &mut Vec<ValidationError>) {
    for action in actions {
        match action {
            Action::FunctionCall { name, args, pos, .. } => {
                check_action_name(name, *pos, errors);
                check_array_mutation_args(name, args, *pos, errors);
                for arg in args {
                    check_expr(arg, errors);
                }
            }
            // Action::ApiCall - "goi_api" was already special-cased by
            // the PARSER (see parser/action.rs around line 107), so
            // "name" no longer exists on this struct at all (it
            // "disappeared" into method/endpoint/...) - but PRECISELY
            // BECAUSE of that, whether the parser CORRECTLY recognized
            // "goi_api" (as opposed to a typo that accidentally MATCHED
            // "goi_api") is no longer something the VALIDATOR needs to
            // check - it's already guaranteed correct right at the
            // PARSER (only the literal string "goi_api" triggers that
            // special branch). Nested callbacks (on_success/on_failure)
            // STILL need to be recursively visited - they contain other
            // Actions (possibly nested FunctionCall/ApiCall/IfAction/
            // Assign).
            Action::ApiCall { endpoint, data, on_success, on_failure, .. } => {
                // endpoint/data contain an Expr, which can have a nested
                // Expr::Call inside (e.g.
                // `goi_api("GET", dieu_huong_url($id))` - even though
                // this example doesn't make much semantic sense, the
                // syntax still allows it).
                check_expr(endpoint, errors);
                if let Some(d) = data {
                    check_expr(d, errors);
                }
                if let Some(body) = on_success {
                    check_actions(body, errors);
                }
                if let Some(body) = on_failure {
                    check_actions(body, errors);
                }
            }
            // A nested IfAction - recurses into both branches, and
            // check_expr() for the condition.
            Action::IfAction { condition, consequent, alternate, .. } => {
                check_expr(condition, errors);
                check_actions(consequent, errors);
                if let Some(alt) = alternate {
                    check_actions(alt, errors);
                }
            }
            // Assign has no action name to check (target is a raw
            // variable name, not part of the ActionName domain) - BUT
            // value IS an Expr, needing check_expr().
            Action::Assign { value, .. } => {
                check_expr(value, errors);
            }
        }
    }
}

// ════════════════════════════════════════════════════════════
// EXPR VALIDATION (FunctionName) — a real recursive check_expr()
// ════════════════════════════════════════════════════════════
//
// This is the ONE recursive `check_expr()` framework, SHARED across
// EVERY position an Expr appears in the AST (11 field positions + 8/10
// recursive Expr variants), instead of scattering validation across
// separate check_* functions (which would create "fake validation
// coverage" - a risk this design deliberately avoids).
//
// SCOPE: ONLY validates `Expr::Call` (FunctionName's target) - the
// other variants (Literal/Variable/MemberAccess/Binary/Unary/ColorFunc/
// Array/Object/TemplateString) have nothing to validate at this
// semantic layer (no finite vocabulary attached to them - MemberAccess
// uses a free-form property: String, Binary/Unary use an operator
// already enumerated at PARSE TIME via BinaryOp/UnaryOp needing no
// re-validation) - but STILL need to be recursed INTO to find a DEEPLY
// NESTED Expr::Call inside them (e.g. `$a + gia_tien($b)` - a Binary
// containing a Call in its right branch).
//
// DISTINCT FROM ColorFunc: `ColorFunc.func: ColorFuncKind` is a
// COMPLETELY DIFFERENT domain from FunctionName (color functions like
// lam_sang/lam_toi, already enumerated EARLIER by the parser via
// resolve_color_func_name() - see parser/expr.rs) - that `func` field
// is NOT touched here, only recursing into `color: Box<Expr>` (the
// color function's argument, which can itself contain another
// Expr::Call).
fn check_expr(expr: &Expr, errors: &mut Vec<ValidationError>) {
    match expr {
        Expr::Call { callee, args, pos } => {
            check_function_name(callee, *pos, errors);
            for arg in args {
                check_expr(arg, errors);
            }
        }
        Expr::MemberAccess { object, .. } => {
            check_expr(object, errors);
        }
        Expr::Binary { left, right, .. } => {
            check_expr(left, errors);
            check_expr(right, errors);
        }
        Expr::Unary { operand, .. } => {
            check_expr(operand, errors);
        }
        Expr::ColorFunc { color, .. } => {
            check_expr(color, errors);
        }
        Expr::Array(items, _) => {
            for item in items {
                check_expr(item, errors);
            }
        }
        Expr::Object(fields, _) => {
            for (_, value) in fields {
                check_expr(value, errors);
            }
        }
        // TemplateString contains no child Expr - TemplatePart::Member is a
        // plain Vec<String> - nothing to recurse into.
        Expr::TemplateString(_, _) => {}
        // Literal/Variable are "leaves" - they contain no child Expr.
        Expr::Literal(_, _) | Expr::Variable(_, _) => {}
    }
}

fn check_function_name(name: &str, pos: vibao_ast::Pos, errors: &mut Vec<ValidationError>) {
    if crate::locale::resolve_function_name(name).is_some() {
        return;
    }
    errors.push(ValidationError {
        message: format!("Unknown function \"{}\". Check the spelling or use a supported ViBao expression function.", name),
        line: pos.line,
        column: pos.column,
    });
}

fn check_action_name(name: &str, pos: vibao_ast::Pos, errors: &mut Vec<ValidationError>) {
    if crate::locale::resolve_action_name(name).is_some() {
        return;
    }
    if crate::locale::action_vi::KNOWN_BUT_UNSUPPORTED_ACTIONS_VI.contains(&name) {
        errors.push(ValidationError {
            message: format!("Action \"{}\" is not supported by ViBao yet. The syntax is recognized, but there is no runtime behavior for it in this release.", name),
            line: pos.line,
            column: pos.column,
        });
        return;
    }
    errors.push(ValidationError {
        message: format!("Unknown action \"{}\". Check the spelling or use an action supported by ViBao.", name),
        line: pos.line,
        column: pos.column,
    });
}

/// A BUG (VIBAOC_VALIDATOR_REVIEW.md item 1, team review): the 3 array
/// CRUD functions (them_vao_mang/xoa_theo_id/cap_nhat_theo_id) REQUIRE
/// their first argument to be a bare state variable (`Expr::Variable`,
/// e.g. `$tasks`) - the real runtime
/// (`action.rs::dispatch_array_mutation`, around line 219) already
/// enforces this, but only via `log::warn(...)` then `return` when it's
/// wrong (a silent failure - no panic, the build still "succeeds", only
/// surfacing when the console is opened at actual runtime).
/// `check_action_name()` (above) ONLY checks the action's NAME, never
/// touching `args` at all - so `them_vao_mang("tasks", "hello")` (a
/// string as the first argument, not a variable) passes the build, and
/// silently does nothing at runtime.
///
/// Confirmed with the user and applying the EXACT UX decision already
/// settled for ActionName (a hard error, consistent with
/// check_action_name/check_color_props) - no need to ask again, since
/// this is the SAME CLASS of problem (an action that doesn't do what
/// was intended, with no passthrough to fall back on).
fn check_array_mutation_args(name: &str, args: &[Expr], pos: vibao_ast::Pos, errors: &mut Vec<ValidationError>) {
    const ARRAY_MUTATION_ACTIONS: [&str; 3] = ["them_vao_mang", "xoa_theo_id", "cap_nhat_theo_id"];
    if !ARRAY_MUTATION_ACTIONS.contains(&name) {
        return;
    }
    let first_is_variable = matches!(args.first(), Some(Expr::Variable(_, _)));
    if !first_is_variable {
        errors.push(ValidationError {
            message: format!("Action \"{}\" requires its first argument to be a state variable (for example $tasks). The runtime cannot safely apply the mutation to another value shape.", name),
            line: pos.line,
            column: pos.column,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vibao_ast::{Page, Pos};

    fn p() -> Pos {
        Pos { line: 1, column: 1 }
    }

    fn empty_app() -> App {
        App {
            name: "Test".to_string(),
            imports: vec![],
            variables: vec![],
            themes: vec![],
            components: vec![],
            pages: vec![],
            pos: p(),
        }
    }

    fn element_with_color_prop(tag: vibao_ast::Tag, key: &str, value: &str) -> Element {
        Element {
            tag,
            props: vec![(key.to_string(), Expr::Literal(LiteralValue::Str(value.to_string()), p()))],
            children: vec![],
            events: vec![],
            responsive: vec![],
            animation: Default::default(),
            pos: p(),
        }
    }

    fn page_with(children: Vec<Child>) -> Page {
        Page {
            route: "/".to_string(),
            name: None,
            mau_nen: None,
            states: vec![],
            children,
            events: vec![],
            pos: p(),
        }
    }

    #[test]
    fn test_english_color_prop_uses_same_validation() {
        let app = {
            let mut app = empty_app();
            app.pages.push(page_with(vec![Child::Element(element_with_color_prop(
                vibao_ast::Tag::Text,
                "background_color",
                "not_a_color",
            ))]));
            app
        };
        let result = validate(&app);
        assert!(result.is_err(), "an English color prop must go through the same color validator");
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.message.contains("background_color")), "errors: {:?}", errors);
    }

    #[test]
    fn test_valid_color_name_passes() {
        let mut app = empty_app();
        app.pages.push(page_with(vec![Child::Element(element_with_color_prop(vibao_ast::Tag::Text, "mau", "xanh"))]));
        assert!(validate(&app).is_ok());
    }

    #[test]
    fn test_unknown_color_name_is_rejected() {
        // The original bug: "xanh_duong" isn't in color_map() (only
        // "xanh" is) - must be caught with a clear error, not slip
        // through to become meaningless CSS in codegen.
        let mut app = empty_app();
        app.pages.push(page_with(vec![Child::Element(element_with_color_prop(vibao_ast::Tag::Text, "mau", "xanh_duong"))]));
        let result = validate(&app);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("xanh_duong"));
    }

    #[test]
    fn test_non_color_prop_not_checked_as_color() {
        // "huong: hang" - "hang" isn't a color name and should NOT be
        // checked, since "huong" isn't in COLOR_PROP_NAMES.
        let mut app = empty_app();
        app.pages.push(page_with(vec![Child::Element(element_with_color_prop(vibao_ast::Tag::Khoi, "huong", "hang"))]));
        assert!(validate(&app).is_ok());
    }

    #[test]
    fn test_undefined_component_call_is_rejected() {
        let mut app = empty_app();
        app.pages.push(page_with(vec![Child::ComponentCall(ComponentCall {
            name: "nut_bam".to_string(),
            props: vec![],
            children: vec![],
            pos: p(),
        })]));
        let result = validate(&app);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.message.contains("nut_bam")));
    }

    #[test]
    fn test_defined_component_call_passes() {
        let mut app = empty_app();
        app.components.push(ComponentDef { name: "nut_bam".to_string(), params: vec![], children: vec![], pos: p() });
        app.pages.push(page_with(vec![Child::ComponentCall(ComponentCall {
            name: "nut_bam".to_string(),
            props: vec![],
            children: vec![],
            pos: p(),
        })]));
        assert!(validate(&app).is_ok());
    }

    #[test]
    fn test_duplicate_component_definition_is_rejected() {
        let mut app = empty_app();
        app.components.push(ComponentDef { name: "nut_bam".to_string(), params: vec![], children: vec![], pos: p() });
        app.components.push(ComponentDef {
            name: "nut_bam".to_string(),
            params: vec![],
            children: vec![],
            pos: Pos { line: 20, column: 1 },
        });
        let result = validate(&app);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.message.contains("defined more than once")));
    }

    #[test]
    fn test_color_error_found_deep_inside_if_switch_loop() {
        // Confirms the recursive traversal doesn't miss an Element
        // nested deep inside an if/switch/loop.
        let bad_element = Child::Element(element_with_color_prop(vibao_ast::Tag::Text, "mau", "mau_khong_ton_tai"));

        let mut app = empty_app();
        app.pages.push(page_with(vec![Child::Loop(Box::new(LoopNode {
            kind: LoopKind::Range { from: 0, to: 3, var_name: "i".to_string() },
            body: vec![Child::If(Box::new(IfNode {
                condition: Expr::Literal(LiteralValue::Bool(true), p()),
                consequent: vec![bad_element],
                alternate: None,
                pos: p(),
            }))],
            pos: p(),
        }))]));

        let result = validate(&app);
        assert!(result.is_err(), "a color error nested deep inside a loop/if must still be detected");
    }

    #[test]
    fn test_multiple_errors_all_collected_not_just_first() {
        // "Collect every error and print them all at once" - 2
        // independent errors in the same validation pass must BOTH
        // appear, not stop at the first one.
        let mut app = empty_app();
        app.pages.push(page_with(vec![
            Child::Element(element_with_color_prop(vibao_ast::Tag::Text, "mau", "mau_sai_1")),
            Child::ComponentCall(ComponentCall {
                name: "component_chua_dinh_nghia".to_string(),
                props: vec![],
                children: vec![],
                pos: p(),
            }),
        ]));

        let result = validate(&app);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 2, "must collect both errors, not stop at the first one");
    }

    // ── Tests for wiring ActionName into the validator (see
    // AUDIT.md "UPDATE - Wiring ActionName into the real validator.rs") ──

    fn function_call_action(name: &str) -> Action {
        Action::FunctionCall {
            name: name.to_string(),
            args: vec![],
            opts: vec![],
            assign_to: None,
            pos: p(),
        }
    }

    fn element_with_event(body: Vec<Action>) -> Element {
        Element {
            tag: vibao_ast::Tag::Button,
            props: vec![],
            children: vec![],
            events: vec![EventNode {
                name: vibao_ast::EventName::OnClick,
                body,
                pos: p(),
            }],
            responsive: vec![],
            animation: Default::default(),
            pos: p(),
        }
    }

    #[test]
    fn test_valid_action_name_passes() {
        let mut app = empty_app();
        app.pages.push(page_with(vec![Child::Element(element_with_event(vec![
            function_call_action("thong_bao"),
        ]))]));
        assert!(validate(&app).is_ok());
    }

    /// An Action::FunctionCall for one of the 3 array CRUD functions,
    /// with a first argument of the CORRECT shape (a bare
    /// Expr::Variable) - used when a test needs to pass BOTH
    /// check_action_name() AND check_array_mutation_args().
    fn array_mutation_action(name: &str, array_var: &str) -> Action {
        Action::FunctionCall {
            name: name.to_string(),
            args: vec![Expr::Variable(array_var.to_string(), p())],
            opts: vec![],
            assign_to: None,
            pos: p(),
        }
    }

    #[test]
    fn test_all_14_function_call_action_names_pass() {
        // RENAMED: the old name "test_all_15_action_names_pass" was
        // WRONG - the array only has 14 elements, missing "goi_api"
        // (which doesn't create an Action::FunctionCall and can't use
        // the function_call_action() helper - see the separate test
        // test_goi_api_action_passes_validation() right below). The new
        // name correctly matches: 14 names that create an
        // Action::FunctionCall.
        //
        // The 3 array CRUD names use array_mutation_action() (a first
        // argument with the CORRECT Expr::Variable shape) instead of
        // function_call_action() (empty args) - since
        // check_array_mutation_args() was just added, empty args would
        // FAIL validation specifically for these 3 names (correctly, as
        // that's exactly the bug that was fixed).
        let simple_names = [
            "thong_bao", "canh_bao", "dieu_huong", "mo_tab_moi",
            "mo_modal", "dong_modal", "cuon_den", "cuon_len_dau",
            "luu_du_lieu", "tai_du_lieu", "sao_chep",
        ];
        for name in simple_names {
            let mut app = empty_app();
            app.pages.push(page_with(vec![Child::Element(element_with_event(vec![
                function_call_action(name),
            ]))]));
            assert!(validate(&app).is_ok(), "'{}' must be valid", name);
        }

        let array_mutation_names = ["them_vao_mang", "xoa_theo_id", "cap_nhat_theo_id"];
        for name in array_mutation_names {
            let mut app = empty_app();
            app.pages.push(page_with(vec![Child::Element(element_with_event(vec![
                array_mutation_action(name, "tasks"),
            ]))]));
            assert!(validate(&app).is_ok(), "'{}' (with a correctly-shaped first argument) must be valid", name);
        }
    }

    #[test]
    fn test_goi_api_action_passes_validation() {
        // "goi_api" does NOT create an Action::FunctionCall (the
        // parser special-cases it into Action::ApiCall - see
        // parser/action.rs), so it can't use the function_call_action()
        // helper - Action::ApiCall is built directly here.
        let mut app = empty_app();
        app.pages.push(page_with(vec![Child::Element(element_with_event(vec![
            Action::ApiCall {
                method: "GET".to_string(),
                endpoint: Expr::literal_str("/api/x", p()),
                data: None,
                assign_to: None,
                on_success: None,
                on_failure: None,
                pos: p(),
            },
        ]))]));
        assert!(validate(&app).is_ok(), "'goi_api' (Action::ApiCall) must be valid");
    }

    #[test]
    fn test_array_mutation_action_with_wrong_first_arg_is_rejected() {
        // BUG FIX: a first argument that is NOT an Expr::Variable (e.g.
        // a string literal) must be rejected at the validator, instead
        // of letting the runtime silently log a warning and move on.
        let mut app = empty_app();
        app.pages.push(page_with(vec![Child::Element(element_with_event(vec![
            Action::FunctionCall {
                name: "them_vao_mang".to_string(),
                args: vec![Expr::literal_str("tasks", p())], // WRONG: a string, not a variable
                opts: vec![],
                assign_to: None,
                pos: p(),
            },
        ]))]));
        let result = validate(&app);
        assert!(result.is_err(), "a wrongly-shaped first argument (a string instead of a variable) must be rejected");
        assert!(result.unwrap_err()[0].message.contains("them_vao_mang"));
    }

    #[test]
    fn test_array_mutation_action_with_no_args_is_rejected() {
        let mut app = empty_app();
        app.pages.push(page_with(vec![Child::Element(element_with_event(vec![
            function_call_action("xoa_theo_id"), // args rỗng
        ]))]));
        assert!(validate(&app).is_err(), "thiếu hoàn toàn đối số phải bị từ chối");
    }

    #[test]
    fn test_non_array_mutation_action_not_checked_for_array_shape() {
        // check_array_mutation_args() only applies to the 3 array CRUD
        // names - other actions (e.g. thong_bao) with empty/any-shaped
        // args are NOT checked by this, only check_action_name()
        // applies to them.
        let mut app = empty_app();
        app.pages.push(page_with(vec![Child::Element(element_with_event(vec![
            function_call_action("thong_bao"), // args rỗng, nhưng KHÔNG phải CRUD mảng
        ]))]));
        assert!(validate(&app).is_ok());
    }

    #[test]
    fn test_typo_action_name_is_rejected_as_unknown() {
        // "thongbao" (missing the underscore) - a plain typo, must be
        // rejected with an "Unknown action" message, not confused with
        // the "not supported yet" message reserved for dang_xuat.
        let mut app = empty_app();
        app.pages.push(page_with(vec![Child::Element(element_with_event(vec![
            function_call_action("thongbao"),
        ]))]));
        let result = validate(&app);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("thongbao"));
        assert!(errors[0].message.contains("Unknown action"));
        assert!(!errors[0].message.contains("not supported by ViBao"));
    }

    #[test]
    fn test_dang_xuat_is_rejected_with_distinct_message() {
        // BUG-27 (AUDIT.md): "dang_xuat" must be rejected, but with a
        // DISTINCT message - not "a plain typo" (since the spelling is
        // correct), but "not supported by ViBao" (missing an auth model).
        let mut app = empty_app();
        app.pages.push(page_with(vec![Child::Element(element_with_event(vec![
            function_call_action("dang_xuat"),
        ]))]));
        let result = validate(&app);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("dang_xuat"));
        assert!(errors[0].message.contains("not supported by ViBao"));
        assert!(!errors[0].message.contains("Unknown action"));
    }

    #[test]
    fn test_dang_xuat_end_to_end_through_real_lexer_and_parser() {
        // An end-to-end test through the REAL path: tokenizes real
        // ViBao source (the string "dang_xuat()"), then
        // Parser::new(...).parse_action(). Confirms: (a) tokenize does
        // NOT error - "dang_xuat" is a normal, valid
        // TokenKind::Identifier; (b) parse_action() does NOT error -
        // expect_identifier_like() accepts both Identifier and
        // Component, without distinguishing; (c) the resulting Action,
        // passed through check_action_name(), STILL produces the
        // CORRECT "not supported by ViBao" message as designed.
        let tokens = crate::lexer::tokenize("dang_xuat()").expect("tokenize must not error for 'dang_xuat()'");
        let mut parser = crate::parser::Parser::new(tokens);
        let action = parser.parse_action().expect("parse_action must not error for 'dang_xuat()'");

        let mut errors = Vec::new();
        check_actions(&[action], &mut errors);
        assert_eq!(errors.len(), 1, "there must be exactly 1 validator error for 'dang_xuat'");
        assert!(errors[0].message.contains("not supported by ViBao"), "message: {}", errors[0].message);
    }

    #[test]
    fn test_action_name_nested_in_if_action_is_checked() {
        // An action nested inside neu/khong_thi (Action::IfAction) must
        // still be checked - not just top-level actions.
        let mut app = empty_app();
        app.pages.push(page_with(vec![Child::Element(element_with_event(vec![
            Action::IfAction {
                condition: Expr::Literal(LiteralValue::Bool(true), p()),
                consequent: vec![function_call_action("thongbao_sai")],
                alternate: None,
                pos: p(),
            },
        ]))]));
        let result = validate(&app);
        assert!(result.is_err(), "an action nested inside IfAction must still be checked");
    }

    #[test]
    fn test_action_name_nested_in_api_call_callback_is_checked() {
        // An action nested inside a goi_api thanh_cong/that_bai block
        // (Action::ApiCall.on_success/on_failure) must still be
        // checked.
        let mut app = empty_app();
        app.pages.push(page_with(vec![Child::Element(element_with_event(vec![
            Action::ApiCall {
                method: "GET".to_string(),
                endpoint: Expr::literal_str("/api/x", p()),
                data: None,
                assign_to: None,
                on_success: Some(vec![function_call_action("thongbao_sai")]),
                on_failure: None,
                pos: p(),
            },
        ]))]));
        let result = validate(&app);
        assert!(result.is_err(), "an action nested inside ApiCall's on_success must still be checked");
    }

    #[test]
    fn test_assign_action_not_checked_as_action_name() {
        // Action::Assign has no "action name" to check - target is a
        // raw variable name, not part of the ActionName domain. This
        // test confirms Assign is NOT mistaken by the validator for an
        // unknown action.
        let mut app = empty_app();
        app.pages.push(page_with(vec![Child::Element(element_with_event(vec![
            Action::Assign {
                target: "bien_bat_ky".to_string(),
                value: Expr::literal_num(1.0, p()),
                pos: p(),
            },
        ]))]));
        assert!(validate(&app).is_ok());
    }

    #[test]
    fn test_page_level_event_action_is_checked() {
        // A GAP THAT WAS FIXED: `page.events` (PAGE-LEVEL khi_tai/khi_huy,
        // PageEvent) used to be completely unvisited - an unknown
        // action inside this block slipped past the validator. This
        // test confirms it's been fixed.
        let mut app = empty_app();
        let mut page = page_with(vec![]);
        page.events.push(vibao_ast::PageEvent {
            name: vibao_ast::PageEventName::OnTai,
            body: vec![function_call_action("thongbao_sai")],
            pos: p(),
        });
        app.pages.push(page);
        let result = validate(&app);
        assert!(result.is_err(), "an unknown action in page.events (khi_tai) must be detected");
    }

    #[test]
    fn test_page_level_event_with_valid_action_passes() {
        let mut app = empty_app();
        let mut page = page_with(vec![]);
        page.events.push(vibao_ast::PageEvent {
            name: vibao_ast::PageEventName::OnTai,
            body: vec![function_call_action("thong_bao")],
            pos: p(),
        });
        app.pages.push(page);
        assert!(validate(&app).is_ok());
    }

    // ── Tests for FunctionName (the recursive check_expr()) ──────────

    fn call_expr(callee: &str, args: Vec<Expr>) -> Expr {
        Expr::Call { callee: callee.to_string(), args, pos: p() }
    }

    #[test]
    fn test_valid_function_call_in_var_decl_passes() {
        // The exact example used as evidence during the original
        // FunctionName investigation (codegen/mod.rs around line 481):
        // `bien $x = gia_tien(1000)`.
        let mut app = empty_app();
        app.variables.push(vibao_ast::VarDecl {
            name: "x".to_string(),
            value: call_expr("gia_tien", vec![Expr::literal_num(1000.0, p())]),
            pos: p(),
        });
        assert!(validate(&app).is_ok());
    }

    #[test]
    fn test_all_6_function_names_pass_in_var_decl() {
        let names = ["gia_tien", "ngay", "rut_gon", "hoa_chu", "phan_tram", "lam_tron"];
        for name in names {
            let mut app = empty_app();
            app.variables.push(vibao_ast::VarDecl {
                name: "x".to_string(),
                value: call_expr(name, vec![]),
                pos: p(),
            });
            assert!(validate(&app).is_ok(), "'{}' must be valid", name);
        }
    }

    #[test]
    fn test_unknown_function_name_in_var_decl_is_rejected() {
        let mut app = empty_app();
        app.variables.push(vibao_ast::VarDecl {
            name: "x".to_string(),
            value: call_expr("gia_tien_sai", vec![]),
            pos: p(),
        });
        let result = validate(&app);
        assert!(result.is_err());
        assert!(result.unwrap_err()[0].message.contains("gia_tien_sai"));
    }

    #[test]
    fn test_unknown_function_call_nested_in_binary_is_detected() {
        // DIRECT EVIDENCE for why recursion MUST reach into
        // Binary/Unary/ColorFunc/Array/Object (not just a direct Call):
        // `$a + gia_tien_sai($b)` - the WRONG Call sits inside Binary's
        // right operand.
        let mut app = empty_app();
        app.variables.push(vibao_ast::VarDecl {
            name: "x".to_string(),
            value: Expr::Binary {
                op: vibao_ast::BinaryOp::Add,
                left: Box::new(Expr::Variable("a".to_string(), p())),
                right: Box::new(call_expr("gia_tien_sai", vec![])),
                pos: p(),
            },
            pos: p(),
        });
        let result = validate(&app);
        assert!(result.is_err(), "Call sai lồng trong Binary phải bị phát hiện");
    }

    #[test]
    fn test_unknown_function_call_nested_in_array_is_detected() {
        let mut app = empty_app();
        app.variables.push(vibao_ast::VarDecl {
            name: "x".to_string(),
            value: Expr::Array(vec![call_expr("gia_tien_sai", vec![])], p()),
            pos: p(),
        });
        assert!(validate(&app).is_err(), "Call sai lồng trong Array phải bị phát hiện");
    }

    #[test]
    fn test_unknown_function_call_nested_in_object_is_detected() {
        let mut app = empty_app();
        app.variables.push(vibao_ast::VarDecl {
            name: "x".to_string(),
            value: Expr::Object(vec![("field".to_string(), call_expr("gia_tien_sai", vec![]))], p()),
            pos: p(),
        });
        assert!(validate(&app).is_err(), "Call sai lồng trong Object phải bị phát hiện");
    }

    #[test]
    fn test_unknown_function_call_nested_in_call_args_is_detected() {
        // gia_tien(rut_gon_sai($x)) - a wrong Call nested inside
        // another Call's args (correct, both go through the recursive
        // check_expr).
        let mut app = empty_app();
        app.variables.push(vibao_ast::VarDecl {
            name: "x".to_string(),
            value: call_expr("gia_tien", vec![call_expr("rut_gon_sai", vec![])]),
            pos: p(),
        });
        assert!(validate(&app).is_err(), "Call sai lồng trong args của Call khác phải bị phát hiện");
    }

    #[test]
    fn test_function_call_in_page_state_is_checked() {
        let mut app = empty_app();
        let mut page = page_with(vec![]);
        page.states.push(vibao_ast::StateDecl {
            name: "x".to_string(),
            value: call_expr("gia_tien_sai", vec![]),
            pos: p(),
        });
        app.pages.push(page);
        assert!(validate(&app).is_err(), "Call sai trong page.states phải bị phát hiện");
    }

    #[test]
    fn test_function_call_in_component_param_default_is_checked() {
        let mut app = empty_app();
        app.components.push(vibao_ast::ComponentDef {
            name: "TheBao".to_string(),
            params: vec![vibao_ast::ParamDef {
                name: "gia".to_string(),
                data_type: vibao_ast::DataType::So,
                default_value: Some(call_expr("gia_tien_sai", vec![])),
                pos: p(),
            }],
            children: vec![],
            pos: p(),
        });
        assert!(validate(&app).is_err(), "Call sai trong default_value của ParamDef phải bị phát hiện");
    }

    #[test]
    fn test_function_call_in_props_map_is_checked() {
        let props: PropsMap = vec![("noi_dung".to_string(), call_expr("gia_tien_sai", vec![]))];
        let el = element_with_color_prop(vibao_ast::Tag::Text, "khong_lien_quan", "trang");
        let mut el = el;
        el.props = props;
        let mut app = empty_app();
        app.pages.push(page_with(vec![Child::Element(el)]));
        assert!(validate(&app).is_err(), "Call sai trong props Element phải bị phát hiện");
    }

    #[test]
    fn test_function_call_in_action_args_is_checked() {
        // thong_bao(gia_tien_sai($x)) - a wrong Call inside a valid
        // Action::FunctionCall's args.
        let mut app = empty_app();
        app.pages.push(page_with(vec![Child::Element(element_with_event(vec![
            Action::FunctionCall {
                name: "thong_bao".to_string(),
                args: vec![call_expr("gia_tien_sai", vec![])],
                opts: vec![],
                assign_to: None,
                pos: p(),
            },
        ]))]));
        assert!(validate(&app).is_err(), "Call sai trong args của Action phải bị phát hiện");
    }

    #[test]
    fn test_function_call_in_if_condition_is_checked() {
        let mut app = empty_app();
        app.pages.push(page_with(vec![Child::If(Box::new(IfNode {
            condition: call_expr("gia_tien_sai", vec![]),
            consequent: vec![],
            alternate: None,
            pos: p(),
        }))]));
        assert!(validate(&app).is_err(), "Call sai trong IfNode.condition phải bị phát hiện");
    }

    #[test]
    fn test_function_call_in_action_if_condition_is_checked() {
        let mut app = empty_app();
        app.pages.push(page_with(vec![Child::Element(element_with_event(vec![
            Action::IfAction {
                condition: call_expr("gia_tien_sai", vec![]),
                consequent: vec![],
                alternate: None,
                pos: p(),
            },
        ]))]));
        assert!(validate(&app).is_err(), "Call sai trong Action::IfAction.condition phải bị phát hiện");
    }

    #[test]
    fn test_function_call_in_assign_value_is_checked() {
        let mut app = empty_app();
        app.pages.push(page_with(vec![Child::Element(element_with_event(vec![
            Action::Assign {
                target: "bien".to_string(),
                value: call_expr("gia_tien_sai", vec![]),
                pos: p(),
            },
        ]))]));
        assert!(validate(&app).is_err(), "Call sai trong Action::Assign.value phải bị phát hiện");
    }

    #[test]
    fn test_function_call_in_api_call_endpoint_and_data_is_checked() {
        let mut app = empty_app();
        app.pages.push(page_with(vec![Child::Element(element_with_event(vec![
            Action::ApiCall {
                method: "GET".to_string(),
                endpoint: call_expr("gia_tien_sai", vec![]),
                data: None,
                assign_to: None,
                on_success: None,
                on_failure: None,
                pos: p(),
            },
        ]))]));
        assert!(validate(&app).is_err(), "Call sai trong ApiCall.endpoint phải bị phát hiện");
    }

    #[test]
    fn test_function_call_in_child_var_decl_is_checked() {
        // A GAP found while re-reviewing the full Child enum:
        // Child::VarDecl (a LOCAL variable declaration inside a block,
        // different from App.variables, visited separately) used to be
        // completely SKIPPED by check_children().
        let mut app = empty_app();
        app.pages.push(page_with(vec![Child::VarDecl(vibao_ast::VarDecl {
            name: "y".to_string(),
            value: call_expr("gia_tien_sai", vec![]),
            pos: p(),
        })]));
        assert!(validate(&app).is_err(), "Call sai trong Child::VarDecl phải bị phát hiện");
    }

    #[test]
    fn test_function_call_in_child_state_decl_is_checked() {
        let mut app = empty_app();
        app.pages.push(page_with(vec![Child::StateDecl(vibao_ast::StateDecl {
            name: "y".to_string(),
            value: call_expr("gia_tien_sai", vec![]),
            pos: p(),
        })]));
        assert!(validate(&app).is_err(), "Call sai trong Child::StateDecl phải bị phát hiện");
    }

    #[test]
    fn test_valid_function_call_in_child_var_decl_passes() {
        let mut app = empty_app();
        app.pages.push(page_with(vec![Child::VarDecl(vibao_ast::VarDecl {
            name: "y".to_string(),
            value: call_expr("gia_tien", vec![Expr::literal_num(1000.0, p())]),
            pos: p(),
        })]));
        assert!(validate(&app).is_ok());
    }



    #[test]
    fn test_app_with_no_theme_passes() {
        let app = empty_app();
        assert!(validate(&app).is_ok());
    }

    #[test]
    fn test_theme_declaration_is_rejected() {
        let mut app = empty_app();
        app.themes.push(Theme {
            name: "sang".to_string(),
            variables: vec![],
            pos: p(),
        });
        let result = validate(&app);
        assert!(result.is_err(), "khai báo theme phải bị từ chối (chưa được ViBao hỗ trợ)");
        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("sang"));
        assert!(errors[0].message.contains("not supported by ViBao"));
    }

    #[test]
    fn test_multiple_themes_all_reported() {
        // Collects every error, doesn't stop at the first theme.
        let mut app = empty_app();
        app.themes.push(Theme { name: "sang".to_string(), variables: vec![], pos: p() });
        app.themes.push(Theme { name: "toi".to_string(), variables: vec![], pos: p() });
        let result = validate(&app);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 2, "phải báo đủ cả 2 theme, không dừng ở cái đầu tiên");
    }

    #[test]
    fn test_theme_error_combined_with_other_errors() {
        // A theme error + another error (a wrong action name) at the
        // same time - both must be collected, neither should hide the
        // other.
        let mut app = empty_app();
        app.themes.push(Theme { name: "sang".to_string(), variables: vec![], pos: p() });
        app.pages.push(page_with(vec![Child::Element(element_with_event(vec![
            function_call_action("thongbao_sai"),
        ]))]));
        let result = validate(&app);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 2, "phải gom cả lỗi theme lẫn lỗi action, không dừng ở lỗi đầu");
    }
}
