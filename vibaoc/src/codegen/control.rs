// ============================================================
// VIBAO COMPILER (Rust) — codegen/control.rs
// Generates HTML + JS binding for control-flow structures:
// neu/khong_thi (If), chon (Switch), lap (Loop). Equivalent to the "6.
// CONTROL FLOW GENERATION" section of 11-codegen-core.ts +
// compileIfCondition/compileLoopNode/ifToDataAttr/loopToDataAttr from
// 08-parser-control.ts.
// ============================================================

use vibao_ast::{Child, IfNode, LoopKind, LoopNode, SwitchNode};
use crate::codegen::css::{esc_attr, indent, indent2};
use crate::codegen::element::ElementCodegenHost;

// REMOVED: `compile_if_condition()`/`CompiledIf`/`IfAnalysisKind`/
// `analyze_if_condition()` used to be here - this was NOT a bug (unlike
// the other removed code blocks in this file/module), it simply was
// NEVER called: `gen_if()` (below) registers `node.condition` directly
// into the expr registry itself, without going through this function.
// It was kept around with the idea that it "might be useful later for
// runtime optimization" but never had a real caller - keeping code
// nobody calls just adds noise, so it was removed for clarity. If
// classifying If conditions for optimization is needed later, it
// should be redesigned tied to the real runtime (knowing exactly what
// to optimize) instead of keeping unverified speculative code around.

/// Generates the `data-vb-if="<exprId>"` attribute - the id points into
/// the expr registry (pure Rust, NOT raw JS anymore - see the full
/// explanation in `gen_if` below; this used to be one of the most
/// serious bugs ever found: if/else was completely non-functional in
/// the build before it was fixed).
pub fn if_to_data_attr(expr_id: usize) -> String {
    format!("data-vb-if=\"{}\"", expr_id)
}

/// Generates the HTML for a complete IfNode - uses the expr registry
/// (pure Rust), generates NO JS, following the same architecture
/// `gen_switch`/`gen_loop` (control.rs) had already been using.
///
/// A REAL BUG THAT WAS FIXED (found through an actual test build -
/// `__vb.bindIf(...)` appeared in app.js, while NO `window.__vb`
/// existed): this function (and the old if_to_data_attr/gen_if_binding)
/// was a LEFTOVER of the JS-mount architecture that had already been
/// completely removed in an earlier bug-fix round for
/// `gen_switch`/`gen_loop` (see the comment on CompiledLoop below) -
/// but NOBODY ever went back and updated `gen_if` to match that same
/// new architecture, so it still: (1) generated an extra line of JS
/// calling `__vb.bindIf(...)` - CRASHING immediately when run
/// (`__vb is not defined`, since no such object was ever exposed
/// globally); (2) `data-vb-if` carried a value in COMPLETELY THE WRONG
/// FORMAT - a raw JS string (e.g. `"__s.dem <= 0"`) instead of a NUMBER
/// (an expr id) - while the runtime's `bind_if` (dom.rs) always expects
/// `parse_expr_id()` to be able to parse a `usize` from this value.
/// Even setting aside error (1), if/else STILL wouldn't have worked
/// because of error (2): the parse would fail, bind_if() would return
/// early, and nothing would ever toggle display.
///
/// The fix: remove the JS binding entirely, register `node.condition`
/// into the expr registry (register_expr, like switch's
/// subject/case), and generate `data-vb-if="<exprId>"` in the correct
/// numeric format. The runtime (bind_if, REWRITTEN at the same time -
/// see dom.rs) reads this expr id itself AND finds the adjacent
/// `data-vb-else` sibling itself to toggle BOTH branches within the
/// SAME subscription, with no need to learn an else_id through any JS
/// parameter anymore.
pub fn gen_if(node: &IfNode, host: &mut dyn ElementCodegenHost) -> String {
    let condition_id = crate::codegen::expr::register_expr(node.condition.clone());
    let consequent_html = host.gen_children(&node.consequent);
    let alternate_html = node.alternate.as_ref().map(|c| host.gen_children(c)).unwrap_or_default();

    let if_id = host.next_id("if");
    let else_id = host.next_id("else");

    let if_block = format!(
        "<div id=\"{}\" {}>\n{}\n</div>",
        if_id,
        if_to_data_attr(condition_id),
        indent2(&consequent_html)
    );

    // "data-vb-if-ref" is kept as an explicit link (easy to debug via
    // DevTools - looking at the attribute tells you exactly which else
    // matches which if), even though the current runtime's bind_if only
    // relies on the next DOM SIBLING RELATIONSHIP to find its else, and
    // never actually reads this attribute's value back.
    let else_block = if !alternate_html.is_empty() {
        format!(
            "<div id=\"{}\" data-vb-else data-vb-if-ref=\"{}\" style=\"display:none\">\n{}\n</div>",
            else_id, if_id, indent2(&alternate_html)
        )
    } else {
        String::new()
    };

    [if_block, else_block].into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join("\n")
}

// ════════════════════════════════════════════════════════════
// SWITCH (chon / truong_hop)
// ════════════════════════════════════════════════════════════

/// Generates the HTML for a SwitchNode - uses the expr registry (pure
/// Rust), generates NO JS. `data-vb-switch="<exprId>"` on the root div
/// carries the subject expression's id; each case div carries
/// `data-vb-case="<exprId_case>"` (the case's value expression, so the
/// runtime compares it itself via expr_eval after computing the
/// subject) or `data-vb-default` for the default branch. The runtime
/// (dom.rs::bind_switch, WRITTEN FRESH alongside this change) reads
/// these attributes itself, computes the subject once, then matches
/// each case in turn, showing exactly the one matching branch.
pub fn gen_switch(node: &SwitchNode, host: &mut dyn ElementCodegenHost) -> String {
    let subject_id = crate::codegen::expr::register_expr(node.subject.clone());
    let switch_id = host.next_id("switch");

    let mut html = format!("<div id=\"{}\" data-vb-switch=\"{}\">\n", switch_id, subject_id);

    for case in &node.cases {
        let value_id = crate::codegen::expr::register_expr(case.value.clone());
        let body_html = host.gen_children(&case.body);
        let case_id = host.next_id("case");
        html.push_str(&format!(
            "  <div id=\"{}\" data-vb-case=\"{}\" style=\"display:none\">\n{}\n  </div>\n",
            case_id,
            value_id,
            indent(&body_html, 4)
        ));
    }

    if let Some(default_case) = &node.default_case {
        let default_html = host.gen_children(default_case);
        let default_id = host.next_id("default");
        html.push_str(&format!(
            "  <div id=\"{}\" data-vb-default style=\"display:none\">\n{}\n  </div>\n",
            default_id,
            indent(&default_html, 4)
        ));
    }

    html.push_str("</div>");
    html
}

// ════════════════════════════════════════════════════════════
// LOOP (lap moi / lap tu)
// ════════════════════════════════════════════════════════════

/// The result of compiling a LoopNode - there is now ONLY 1 variant
/// that actually needs to reach the final output: Each. LoopKind::Range
/// is DESUGARED (transformed) into Each right here, generating NO
/// separate HTML/JSON form for Range at all - why:
///
/// BUG ALREADY FIXED: Each used to generate "data-vb-for"/"data-vb-in"/
/// "data-vb-index" (a raw JS string for the iterable) and Range used to
/// generate "data-vb-range-from"/"-to"/"-var" - but the WASM runtime
/// (vibao-runtime/src/runtime/dom.rs::bind_loop) NEVER reads any of
/// these attributes. It ONLY reads a single attribute, "data-vb-loop",
/// whose value is a JSON string {iterable_expr_id, item_var, index_var,
/// template_id} - the id points into the expr registry (the same
/// mechanism already fixed for style/text binding). The result: EVERY
/// loop (both forms) rendered correct static HTML structure, but NEVER
/// actually iterated over any items at runtime - the
/// <template>/container was always empty.
///
/// Since Range's `from`/`to` are always CONSTANT i64 values known at
/// compile time (not a dynamic Expr), desugaring into a fixed array of
/// integers [from, from+1, ..., to] and then reusing the Each path
/// (already correct after the fix) UNCHANGED is the cheapest, safest
/// approach - no changes needed anywhere in vibao-runtime, no need to
/// teach bind_loop() a separate "range" concept.
pub struct CompiledLoop {
    pub iterable_expr_id: usize,
    pub item_var: String,
    pub index_var: Option<String>,
}

/// Compiles a LoopNode into a CompiledLoop - REGISTERS the iterable
/// Expr into the expr registry (a side effect: calls register_expr)
/// instead of just computing a plain JS string like before, since the
/// runtime now needs a numeric ID to look up the registry at runtime,
/// not a JS string embedded directly in the HTML.
pub fn compile_loop_node(node: &LoopNode) -> CompiledLoop {
    match &node.kind {
        LoopKind::Each { iterable, item_var, index_var } => CompiledLoop {
            iterable_expr_id: crate::codegen::expr::register_expr(iterable.clone()),
            // The old TS version stripped the "$" from a variable name
            // ($item -> item) - our Rust AST already stores item_var
            // WITHOUT the "$" from the moment it's parsed (see
            // parser/control.rs), so .replace("$","") here is redundant
            // but is kept just in case some input still has a leftover
            // "$".
            item_var: item_var.replace('$', ""),
            index_var: index_var.as_ref().map(|v| v.replace('$', "")),
        },
        LoopKind::Range { from, to, var_name } => {
            // Desugar: [from, from+1, ..., to] (INCLUSIVE on both ends -
            // matching the meaning of "tu 1 den 3" as 1, 2, 3, not 1, 2
            // like Rust's exclusive range `1..3`).
            let items: Vec<vibao_ast::Expr> = (*from..=*to)
                .map(|n| vibao_ast::Expr::literal_num(n as f64, vibao_ast::Pos { line: 0, column: 0 }))
                .collect();
            let array_expr = vibao_ast::Expr::Array(items, vibao_ast::Pos { line: 0, column: 0 });
            CompiledLoop {
                iterable_expr_id: crate::codegen::expr::register_expr(array_expr),
                // BUG ALREADY FIXED: "i" used to be hardcoded for EVERY
                // Range loop - completely ignoring the counter variable
                // name a dev declared (e.g. "vong_lap $dem tu 1 den 3").
                // var_name now comes from the AST (LoopKind::Range.var_name
                // - the parser already fills in "i" when nothing is
                // explicitly declared, or the real name when there is
                // one).
                item_var: var_name.clone(),
                index_var: None,
            }
        }
    }
}

/// Generates the "data-vb-loop" attribute (JSON) - in the EXACT format
/// vibao-runtime::dom::bind_loop actually reads. Equivalent to
/// loopToDataAttr() in the old TS version, BUT the old TS/Rust versions
/// used to generate the wrong format - see the explanation on
/// CompiledLoop above.
pub fn loop_to_data_attr(compiled: &CompiledLoop, template_id: &str) -> String {
    #[derive(serde::Serialize)]
    struct LoopSpec<'a> {
        iterable_expr_id: usize,
        item_var: &'a str,
        index_var: Option<&'a str>,
        template_id: &'a str,
    }
    let spec = LoopSpec {
        iterable_expr_id: compiled.iterable_expr_id,
        item_var: &compiled.item_var,
        index_var: compiled.index_var.as_deref(),
        template_id,
    };
    let json = serde_json::to_string(&spec).unwrap_or_else(|_| "{}".to_string());
    format!("data-vb-loop=\"{}\"", esc_attr(&json))
}

/// Counts how many `Child` entries in a list ACTUALLY produce at least
/// one top-level DOM node when rendered - i.e. excluding
/// `StateDecl`/`VarDecl`/`PageEvent` (declarations/statement blocks
/// that don't draw any element by themselves). Used to warn when a
/// `vong_lap` body renders MORE THAN 1 top-level node per item (see the
/// full explanation at its call site inside `gen_loop`).
fn count_dom_producing_children(children: &[Child]) -> usize {
    children
        .iter()
        .filter(|c| {
            !matches!(
                c,
                Child::StateDecl(_) | Child::VarDecl(_) | Child::PageEvent(_)
            )
        })
        .count()
}

/// Generates the HTML (template + container) + JS binding for a
/// complete LoopNode. Equivalent to genLoop().
pub fn gen_loop(node: &LoopNode, host: &mut dyn ElementCodegenHost) -> String {
    let compiled = compile_loop_node(node);
    let loop_id = host.next_id("loop");
    let template_id = host.next_id("tpl");
    let body_html = host.gen_children(&node.body);

    // WARNING (does not stop the build): the loop body renders > 1
    // top-level DOM node PER ITEM (e.g. `vong_lap ... { text(...) \n
    // divider() }` not wrapped in a container/box/stack - this is
    // completely VALID syntax and the runtime
    // (vibao-runtime::dom::bind_loop) handles it CORRECTLY in every
    // case, including this one). This warning is purely about BEST
    // PRACTICE, not a bug: multiple separate top-level nodes in a
    // single item makes it (a) hard to apply shared style/layout to
    // "one item" as a unit, (b) hard to debug in DevTools (there's no
    // single wrapping element representing the item), (c) an earlier
    // compiler version once had a bug that mis-rendered this exact case
    // (see the fix history for `rendered_node_count` in dom.rs) - even
    // though it's now fixed at the root, wrapping in a container is
    // still the SAFER + clearer way to write it, so it's worth
    // suggesting right at build time.
    let dom_child_count = count_dom_producing_children(&node.body);
    if dom_child_count > 1 {
        host.add_warning(format!(
            "vong_lap (dòng {}:{}): thân vòng lặp render ra {} phần tử top-level cho mỗi item (không bọc trong 1 container/box/stack). Vẫn hoạt động đúng, nhưng nên bọc trong 1 container để dễ áp style/layout và debug hơn.",
            node.pos.line, node.pos.column, dom_child_count,
        ));
    }

    // WARNING (does not stop the build; see docs/LIMITATIONS.md for the
    // full write-up): a `@the` component called DIRECTLY as a
    // top-level child of a `vong_lap`'s body - with no `neu`/element
    // wrapping it in between - is a known-fragile pattern at RUNTIME.
    // `vibao-runtime::dom::bind_fragment_with_loop_frame` special-cases
    // exactly this shape (a loop item whose sole top-level node IS a
    // `[data-vb-component]`) to call `bind_component` directly instead
    // of `bind_subtree`, which is required for props like
    // `$item.field` to resolve at all - but that special case has
    // repeatedly been the source of double-bind bugs when the
    // component ALSO sits inside a `truong_hop`/switch branch (see the
    // history in vibao-runtime/src/runtime/dom.rs::is_inside_loop) -
    // fixed at the runtime level for the cases found so far, but the
    // pattern remains inherently more fragile than wrapping the call.
    // Checking ONLY the loop's own direct/top-level children (not
    // recursing into nested `neu`/element bodies) is deliberate: a
    // component sitting inside a `neu`/element wrapper is NOT the
    // fragile shape - `bind_subtree` finds and binds it as an ordinary
    // child there, the same way it would anywhere else outside a loop.
    for child in &node.body {
        if let Child::ComponentCall(call) = child {
            host.add_warning(format!(
                "vong_lap (dòng {}:{}): component \"{}\" (dòng {}:{}) được gọi TRỰC TIẾP làm phần tử top-level của thân vòng lặp, không có \"neu\"/element nào bao ngoài. Vẫn hoạt động đúng sau bản vá hiện tại, nhưng đây là dạng cấu trúc dễ vỡ nhất trong toàn bộ runtime - xem docs/LIMITATIONS.md. Gợi ý: bọc component trong 1 container (ví dụ `khoi(...) {{ {}(...) }}`) để tránh hoàn toàn dạng cấu trúc này.",
                node.pos.line, node.pos.column,
                call.name, call.pos.line, call.pos.column,
                call.name,
            ));
        }
    }

    let template = format!("<template id=\"{}\">\n{}\n</template>", template_id, indent2(&body_html));
    // BUG ALREADY FIXED: `template` used to be concatenated directly
    // into the returned string (embedded inline at the call site) -
    // with NESTED loops, this made a child <template> live physically
    // inside its parent <template>, getting duplicated with a
    // conflicting ID every time clone_node_with_deep() ran for each
    // parent item (see the full explanation on add_hoisted_template in
    // element.rs). The template is now "hoisted" out via
    // add_hoisted_template - the host (the real Codegen) collects every
    // hoisted template and prints them all at the top level of the
    // page, never nested inside any other template no matter how deep
    // the loop nesting goes.
    host.add_hoisted_template(template);

    // BUG ALREADY FIXED: a SEPARATE "data-vb-template" attribute used
    // to also be added here - but dom.rs::bind_loop NEVER reads this
    // separate attribute; it only reads template_id EMBEDDED INSIDE the
    // JSON string of "data-vb-loop" (see loop_to_data_attr above, which
    // now embeds template_id into that JSON itself) - so this redundant
    // attribute was removed entirely.
    let container = format!(
        "<div id=\"{}\" {}></div>",
        loop_id,
        loop_to_data_attr(&compiled, &template_id),
    );

    // BUG ALREADY FIXED: host.add_js(&gen_loop_binding(...)) used to be
    // called here, generating a call to
    // "__vb.bindLoop(...)"/"__vb.bindRange(...)" - but NO function with
    // that name exists anywhere in the entire runtime (the same kind of
    // bug already encountered with "__color" earlier: JS code calling a
    // nonexistent function, crashing with "... is not defined" if it
    // had ever actually run). The WASM runtime already automatically
    // binds every loop via bind_loop() (reading the "data-vb-loop"
    // attribute) as soon as the page is rendered/the router boots - no
    // extra JS call is needed here at all.
    //
    // Only `container` is returned (no longer including `template` -
    // it's already been hoisted above) - the container is the ONLY
    // part that needs to stay in its original position in the HTML
    // tree (so the CSS layout/display order is correct).
    container
}

// ════════════════════════════════════════════════════════════
// UNIT TESTS
// ════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use vibao_ast::{BinaryOp, Expr, LiteralValue, Pos};

    fn p() -> Pos {
        Pos { line: 1, column: 1 }
    }

    #[test]
    fn test_if_to_data_attr_uses_numeric_expr_id() {
        // BUG ALREADY FIXED: if_to_data_attr() used to receive a raw JS
        // string (e.g. `__s.x === "y"`) and need its quotes escaped to
        // be valid HTML - but the runtime's bind_if() always expects the
        // attribute value to be an INTEGER (an expr id, looked up in
        // the registry), not a JS string. The old test (checking quote
        // escaping) no longer means anything with the new architecture -
        // replaced with a test confirming the exact numeric format
        // bind_if() actually parses (see dom.rs::parse_expr_id).
        let attr = if_to_data_attr(42);
        assert_eq!(attr, "data-vb-if=\"42\"");
    }

    #[test]
    fn test_gen_if_registers_condition_expr_and_no_js_binding() {
        // An END-TO-END regression test for the real bug (reported via a
        // test build: `__vb` didn't exist, `data-vb-if` used to carry a
        // JS string instead of an expr id). gen_if() must now NOT call
        // host.add_js() (no JS binding generated at all), and the root
        // div MUST carry data-vb-if="<number>".
        struct FakeHost {
            counter: u32,
            js_calls: Vec<String>,
        }
        impl FakeHost {
            fn new() -> Self {
                FakeHost { counter: 0, js_calls: vec![] }
            }
        }
        impl ElementCodegenHost for FakeHost {
            fn next_id(&mut self, tag: &str) -> String {
                self.counter += 1;
                format!("vb-{}-{}", tag, self.counter)
            }
            fn gen_children(&mut self, _children: &[Child]) -> String {
                String::new()
            }
            fn add_css(&mut self, _code: &str) {}
            fn add_media_query(&mut self, _code: &str) {}
            fn add_warning(&mut self, _msg: String) {}
            fn add_hoisted_template(&mut self, _html: String) {}
        }

        // BinaryOp/Expr/LiteralValue are already imported at the top of the test module
        let node = IfNode {
            condition: Expr::Binary {
                op: BinaryOp::Gte,
                left: Box::new(Expr::Variable("dem".to_string(), p())),
                right: Box::new(Expr::Literal(LiteralValue::Num(10.0, None), p())),
                pos: p(),
            },
            consequent: vec![],
            alternate: None,
            pos: p(),
        };

        let mut host = FakeHost::new();
        let html = gen_if(&node, &mut host);

        assert!(
            host.js_calls.is_empty(),
            "gen_if must not generate any JS at all (the new architecture uses the expr registry, no JS binding) - actual: {:?}",
            host.js_calls
        );
        assert!(html.contains("data-vb-if=\""), "the HTML must contain data-vb-if: {}", html);
        // The value inside data-vb-if must be a NUMBER (an expr id) -
        // extracted and asserted to parse as a usize, without hardcoding
        // a specific number (register_expr can return a different id
        // depending on call order within the test suite/global
        // registry).
        let start = html.find("data-vb-if=\"").unwrap() + "data-vb-if=\"".len();
        let end = html[start..].find('"').unwrap() + start;
        let id_str = &html[start..end];
        assert!(
            id_str.parse::<usize>().is_ok(),
            "data-vb-if must contain an integer (an expr id), actual: \"{}\"",
            id_str
        );
    }

    #[test]
    fn test_gen_if_with_alternate_generates_else_div_as_sibling() {
        // A regression test for the "else" part - the runtime's
        // bind_if() (dom.rs) finds the [data-vb-else] sibling itself,
        // RIGHT AFTER the [data-vb-if] div via next_element_sibling(),
        // so the sibling order/relationship in the generated HTML MUST
        // be correct: if_block THEN IMMEDIATELY AFTER else_block, with
        // nothing in between.
        struct FakeHost { counter: u32 }
        impl FakeHost { fn new() -> Self { FakeHost { counter: 0 } } }
        impl ElementCodegenHost for FakeHost {
            fn next_id(&mut self, tag: &str) -> String {
                self.counter += 1;
                format!("vb-{}-{}", tag, self.counter)
            }
            fn gen_children(&mut self, children: &[Child]) -> String {
                if children.is_empty() { String::new() } else { "<span>alt</span>".to_string() }
            }
            fn add_css(&mut self, _code: &str) {}
            fn add_media_query(&mut self, _code: &str) {}
            fn add_warning(&mut self, _msg: String) {}
            fn add_hoisted_template(&mut self, _html: String) {}
        }

        // Expr is already imported at the top of the test module (the "use vibao_ast::{BinaryOp, Expr, LiteralValue, Pos};" line)
        let node = IfNode {
            condition: Expr::Variable("da_gui".to_string(), p()),
            consequent: vec![],
            alternate: Some(vec![Child::StateDecl(vibao_ast::StateDecl {
                name: "khong_dung".to_string(),
                value: Expr::literal_num(0.0, p()),
                pos: p(),
            })]),
            pos: p(),
        };
        let mut host = FakeHost::new();
        let html = gen_if(&node, &mut host);

        let if_pos = html.find("data-vb-if=\"").expect("must have an if div");
        let else_pos = html.find("data-vb-else").expect("must have an else div when alternate is present");
        assert!(if_pos < else_pos, "the if div must appear BEFORE the else div in the HTML: {}", html);
        assert!(html.contains("style=\"display:none\""), "else must be hidden by default: {}", html);
    }

    #[test]
    fn test_compile_loop_each_strips_dollar_sign() {
        let node = LoopNode {
            kind: LoopKind::Each {
                iterable: Expr::Variable("ds".to_string(), p()),
                item_var: "item".to_string(),
                index_var: None,
            },
            body: vec![],
            pos: p(),
        };
        let compiled = compile_loop_node(&node);
        assert_eq!(compiled.item_var, "item");
        assert!(compiled.index_var.is_none());
    }

    #[test]
    fn test_compile_loop_range_desugars_with_i_as_item_var() {
        // BUG ALREADY FIXED: Range no longer keeps a separate from/to -
        // it desugars into an Each iterating over an array [1,2,3,4,5]
        // registered into the registry (see the explanation on
        // CompiledLoop). This test now only confirms item_var defaults
        // to "i" (var_name not explicitly declared) and that a valid
        // iterable_expr_id gets registered - from/to can't be asserted
        // directly anymore since they've "disappeared" into the
        // registry as an Expr::Array.
        let node = LoopNode {
            kind: LoopKind::Range { from: 1, to: 5, var_name: "i".to_string() },
            body: vec![],
            pos: p(),
        };
        let compiled = compile_loop_node(&node);
        assert_eq!(compiled.item_var, "i");
        assert!(compiled.index_var.is_none());
    }

    #[test]
    fn test_compile_loop_range_uses_custom_counter_variable_name() {
        // BUG ALREADY FIXED (see RUNTIME_BOUNDARY_BUGS.md - the "range
        // loop loses its loop variable" bug): compile_loop_node() used
        // to HARDCODE item_var = "i" for EVERY Range loop, completely
        // ignoring the counter variable name a dev declared (e.g.
        // "vong_lap $dem tu 1 den 3"). The test above
        // (test_compile_loop_range_desugars_with_i_as_item_var) could
        // NOT self-detect this bug since it happened to use the exact
        // name "i" - this test uses a DIFFERENT name to confirm
        // var_name actually flows all the way from the AST to
        // CompiledLoop.item_var, without being replaced by a hardcoded
        // constant anywhere along the way.
        let node = LoopNode {
            kind: LoopKind::Range { from: 1, to: 5, var_name: "dem".to_string() },
            body: vec![],
            pos: p(),
        };
        let compiled = compile_loop_node(&node);
        assert_eq!(compiled.item_var, "dem");
    }

    #[test]
    fn test_loop_to_data_attr_each_produces_valid_json_with_registry_id() {
        // BUG ALREADY FIXED: this test used to expect
        // "data-vb-for"/"data-vb-in"/"data-vb-index" - but
        // dom.rs::bind_loop doesn't read these attributes at all, only
        // "data-vb-loop" (JSON). The test now confirms the correct
        // attribute + the correct JSON fields bind_loop needs.
        let node = LoopNode {
            kind: LoopKind::Each {
                iterable: Expr::Variable("ds".to_string(), p()),
                item_var: "item".to_string(),
                index_var: Some("idx".to_string()),
            },
            body: vec![],
            pos: p(),
        };
        let compiled = compile_loop_node(&node);
        let attr = loop_to_data_attr(&compiled, "vb-tpl-1");
        assert!(attr.starts_with("data-vb-loop=\""), "must use the correct data-vb-loop attribute name: {}", attr);
        // The attr's value has been through esc_attr() (" -> &quot;),
        // so it's parsed back to check the JSON content correctly,
        // instead of comparing raw strings (fragile if the escaping
        // ever changes).
        let json_start = attr.find('"').unwrap() + 1;
        let json_raw = &attr[json_start..attr.len() - 1];
        let json_unescaped = json_raw.replace("&quot;", "\"");
        assert!(json_unescaped.contains("\"item_var\":\"item\""));
        assert!(json_unescaped.contains("\"index_var\":\"idx\""));
        assert!(json_unescaped.contains("\"template_id\":\"vb-tpl-1\""));
        assert!(json_unescaped.contains("\"iterable_expr_id\":"));
    }

    #[test]
    fn test_range_loop_desugars_to_each_with_correct_array() {
        // BUG ALREADY FIXED: Range used to generate "data-vb-range-*"
        // (which nobody read). Range now MUST desugar into an Each
        // iterating over the array [from..=to] - this checks the
        // registry actually contains that exact array.
        let node = LoopNode {
            kind: LoopKind::Range { from: 1, to: 3, var_name: "i".to_string() },
            body: vec![],
            pos: p(),
        };
        let compiled = compile_loop_node(&node);
        assert_eq!(compiled.item_var, "i");
        assert!(compiled.index_var.is_none());

        // Looks up the registry to confirm the exact array [1,2,3] was
        // registered. attr has already been through esc_attr()
        // (" -> &quot;), so it needs unescaping before checking the JSON
        // content - the same approach used in the test above
        // (test_loop_to_data_attr_each_produces_valid_json_with_registry_id).
        // Missing this unescape step was exactly why this test initially
        // FAILED even though the real logic was already correct.
        let attr = loop_to_data_attr(&compiled, "vb-tpl-2");
        let json_unescaped = attr.replace("&quot;", "\"");
        assert!(json_unescaped.contains("\"iterable_expr_id\":"), "actual attr: {}", attr);
    }

    // ── A regression test for the "loop renders > 1 top-level node"
    // warning ── A related real bug (fixed in
    // vibao-runtime/src/runtime/dom.rs): bind_loop() used to count how
    // many nodes to remove on re-render using the number of ITEMS
    // instead of the actual number of DOM NODES, so if one item
    // rendered > 1 top-level node, a subsequent re-render would remove
    // TOO FEW, leaving DOM garbage behind. The runtime now counts
    // correctly regardless of this case, but this build-time warning is
    // still useful as an additional layer of defense (best practice),
    // independent from the runtime fix - 2 different layers of
    // protection for the same risk.

    /// A minimal fake host, JUST enough for the tests in this file -
    /// doesn't reuse element.rs's FakeHost since that's a separate
    /// struct (a private test module, can't be imported across files),
    /// and here only warnings + next_id need to be tracked, no real
    /// js/css/animation is needed.
    struct FakeHost {
        counter: u32,
        warnings: Vec<String>,
    }

    impl FakeHost {
        fn new() -> Self {
            FakeHost { counter: 0, warnings: vec![] }
        }
    }

    impl ElementCodegenHost for FakeHost {
        fn next_id(&mut self, tag: &str) -> String {
            self.counter += 1;
            format!("vb-{}-{}", tag, self.counter)
        }
        fn gen_children(&mut self, _children: &[vibao_ast::Child]) -> String {
            String::new()
        }
        fn add_css(&mut self, _code: &str) {}
        fn add_media_query(&mut self, _code: &str) {}
        fn add_warning(&mut self, msg: String) {
            self.warnings.push(msg);
        }
        fn add_hoisted_template(&mut self, _html: String) {}
    }

    fn dummy_element_child(tag: vibao_ast::Tag) -> Child {
        Child::Element(vibao_ast::Element {
            tag,
            props: vec![],
            children: vec![],
            events: vec![],
            responsive: vec![],
            animation: Default::default(),
            pos: p(),
        })
    }

    #[test]
    fn test_count_dom_producing_children_ignores_state_and_var_decl() {
        // StateDecl/VarDecl don't draw any node by themselves - they
        // shouldn't be counted as "top-level elements" even though they
        // appear in a Vec<Child>.
        let children = vec![
            Child::StateDecl(vibao_ast::StateDecl {
                name: "x".to_string(),
                value: Expr::literal_num(0.0, p()),
                pos: p(),
            }),
            dummy_element_child(vibao_ast::Tag::Text),
        ];
        assert_eq!(count_dom_producing_children(&children), 1);
    }

    #[test]
    fn test_count_dom_producing_children_counts_multiple_elements() {
        let children = vec![dummy_element_child(vibao_ast::Tag::Text), dummy_element_child(vibao_ast::Tag::DuongKe)];
        assert_eq!(count_dom_producing_children(&children), 2);
    }

    #[test]
    fn test_gen_loop_warns_when_body_has_multiple_top_level_elements() {
        let node = LoopNode {
            kind: LoopKind::Each {
                iterable: Expr::Variable("ds".to_string(), p()),
                item_var: "item".to_string(),
                index_var: None,
            },
            body: vec![dummy_element_child(vibao_ast::Tag::Text), dummy_element_child(vibao_ast::Tag::DuongKe)],
            pos: p(),
        };
        let mut host = FakeHost::new();
        gen_loop(&node, &mut host);
        assert_eq!(
            host.warnings.len(),
            1,
            "must produce exactly 1 warning when the loop body has 2 top-level elements"
        );
        assert!(host.warnings[0].contains("2 phần tử top-level"), "warning content: {}", host.warnings[0]);
    }

    #[test]
    fn test_gen_loop_no_warning_when_body_has_single_top_level_element() {
        let node = LoopNode {
            kind: LoopKind::Each {
                iterable: Expr::Variable("ds".to_string(), p()),
                item_var: "item".to_string(),
                index_var: None,
            },
            body: vec![dummy_element_child(vibao_ast::Tag::Khoi)],
            pos: p(),
        };
        let mut host = FakeHost::new();
        gen_loop(&node, &mut host);
        assert!(
            host.warnings.is_empty(),
            "should not warn when the loop body has only 1 top-level element (the common, correct case): {:?}",
            host.warnings
        );
    }

    // ── Regression tests for the "component called directly inside a
    // vong_lap, with no neu/element wrapper" build-time warning ──
    // Mirrors the exact real-world shape from blog_app.vbao's "tat_ca"
    // switch case: `vong_lap $bv trong $bai_viet { TheBaiViet(...) }` -
    // the component IS the loop body's sole top-level child, nothing
    // wraps it.

    fn dummy_component_call(name: &str) -> Child {
        Child::ComponentCall(vibao_ast::ComponentCall {
            name: name.to_string(),
            props: vec![],
            children: vec![],
            pos: p(),
        })
    }

    #[test]
    fn test_gen_loop_warns_when_component_called_directly_with_no_wrapper() {
        let node = LoopNode {
            kind: LoopKind::Each {
                iterable: Expr::Variable("bai_viet".to_string(), p()),
                item_var: "bv".to_string(),
                index_var: None,
            },
            body: vec![dummy_component_call("TheBaiViet")],
            pos: p(),
        };
        let mut host = FakeHost::new();
        gen_loop(&node, &mut host);
        assert_eq!(
            host.warnings.len(),
            1,
            "must produce exactly 1 warning when a component is the loop's sole top-level child: {:?}",
            host.warnings
        );
        assert!(
            host.warnings[0].contains("TheBaiViet"),
            "warning should name the component: {}",
            host.warnings[0]
        );
        assert!(
            host.warnings[0].contains("TRỰC TIẾP"),
            "warning content: {}",
            host.warnings[0]
        );
    }

    #[test]
    fn test_gen_loop_no_warning_when_component_is_wrapped_in_neu() {
        // Mirrors blog_app.vbao's "da_xuat_ban"/"nhap" cases:
        // `vong_lap $bv trong $bai_viet { neu $bv.da_xuat_ban { TheBaiViet(...) } }`
        // - the loop's OWN top-level child is the `neu`/If node, NOT the
        // component itself (the component sits one level deeper, inside
        // the If's body) - this is the SAFE shape (bind_subtree finds
        // and binds it as an ordinary child), so it must NOT warn.
        let if_node = IfNode {
            condition: Expr::Variable("bv.da_xuat_ban".to_string(), p()),
            consequent: vec![dummy_component_call("TheBaiViet")],
            alternate: None,
            pos: p(),
        };
        let node = LoopNode {
            kind: LoopKind::Each {
                iterable: Expr::Variable("bai_viet".to_string(), p()),
                item_var: "bv".to_string(),
                index_var: None,
            },
            body: vec![Child::If(Box::new(if_node))],
            pos: p(),
        };
        let mut host = FakeHost::new();
        gen_loop(&node, &mut host);
        assert!(
            host.warnings.is_empty(),
            "should not warn when the component sits inside a neu/If wrapper, not directly: {:?}",
            host.warnings
        );
    }

    #[test]
    fn test_gen_loop_warns_once_per_direct_component_call_not_per_loop() {
        // 2 components both called directly (unusual, but valid) - must
        // produce a warning naming each offending component, plus the
        // separate "multiple top-level elements per item" warning since
        // the loop body has 2 children.
        let node = LoopNode {
            kind: LoopKind::Each {
                iterable: Expr::Variable("ds".to_string(), p()),
                item_var: "x".to_string(),
                index_var: None,
            },
            body: vec![dummy_component_call("TheA"), dummy_component_call("TheB")],
            pos: p(),
        };
        let mut host = FakeHost::new();
        gen_loop(&node, &mut host);
        assert_eq!(host.warnings.len(), 3, "actual warnings: {:?}", host.warnings);
        assert!(host.warnings.iter().any(|w| w.contains("TheA")));
        assert!(host.warnings.iter().any(|w| w.contains("TheB")));
    }
}
