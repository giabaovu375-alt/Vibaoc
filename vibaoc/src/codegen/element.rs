// ============================================================
// VIBAO COMPILER (Rust) — codegen/element.rs
// Generates HTML for a specific Element - dispatching by 3 kinds:
// SIMPLE (text, button, image...), LAYOUT (flex, box, grid...), and
// built-in COMPLEX (form, modal, tabs...). Equivalent to the
// "5. ELEMENT GENERATION" section of 11-codegen-core.ts.
// ============================================================

use vibao_ast::{get_prop, AnimationProps, Element, LapValue};
use crate::codegen::css::{camel_to_kebab, esc_attr, esc_html, indent2, layout_css_to_string, layout_css_to_string_inline, style_map_to_string, OrderedMap};
use crate::codegen::expr::{register_expr, resolve_value, ResolvedValue};
use crate::codegen::layout::{is_layout_tag_semantic, resolve_layout_css, layout_unknown_keys, resolve_responsive_css, build_media_query};
use crate::codegen::props::expand_props;

/// The list of 10 built-in complex components - element.rs only
/// generates a placeholder tag for these, with the real content built
/// by the runtime at run time (see component.rs for the corresponding
/// mount step). Matches BUILTIN_COMPLEX in the old TS version.
#[cfg(test)]
pub const BUILTIN_COMPLEX: [&str; 10] = [
    "form",
    "modal",
    "tabs",
    "gap_mo",
    "bang_chuyen",
    "xuong_trang",
    "bang",
    "bieu_do",
    "ban_do",
    "trinh_soan_thao",
];

#[cfg(test)]
pub fn is_builtin_complex(tag: &str) -> bool {
    BUILTIN_COMPLEX.contains(&tag)
}

/// A PARALLEL version using Tag (semantic) - does NOT yet replace
/// `is_builtin_complex()`, only used for cross-checking (see the tests
/// at the end of this file). The same principle applied to
/// is_layout_tag_semantic() in layout.rs.
pub fn is_builtin_complex_semantic(tag: vibao_ast::Tag) -> bool {
    vibao_ast::semantic::tag_spec(tag).kind == vibao_ast::semantic::TagKind::Complex
}

/// Maps a ViBao tag -> the real HTML tag name. A tag not in the table
/// (e.g. layout tags, custom components) defaults to "div". Matches
/// TAG_MAP.
fn tag_to_html(tag: &str) -> &str {
    match tag {
        "text" => "p",
        "h1" => "h1",
        "h2" => "h2",
        "h3" => "h3",
        "p" => "p",
        "nhan" => "span",
        "button" => "button",
        "link" | "lien_ket" => "a",
        "image" => "img",
        "video" => "video",
        "icon" => "span",
        "input" => "input",
        "khoang_cach" => "div",
        "duong_ke" => "hr",
        "vong_quay" => "div",
        "thanh_tien_trinh" => "div",
        "cuon" => "div",
        "can_giua" => "div",
        _ => "div",
    }
}

/// A PARALLEL version using Tag (semantic) - does NOT yet replace
/// `tag_to_html()`. IMPORTANT NOTE: the registry (tag_spec().html_tag)
/// returns "div" for EVERY Layout/Complex tag (preserving the old
/// table's "_ => div" fallback behavior) - but the old table already
/// had some Layout tags (cuon, can_giua) AND Complex tags both fall
/// into "div" for EXACTLY the same reason (either absent from the
/// match, or explicitly matching to "div") - so results still agree
/// despite the different paths (see the cross-check test to confirm
/// this rather than just reasoning about it).
#[cfg(test)]
fn tag_to_html_semantic(tag: vibao_ast::Tag) -> &'static str {
    vibao_ast::semantic::tag_spec(tag).html_tag
}


const SELF_CLOSING: [&str; 4] = ["img", "input", "hr", "br"];

fn is_self_closing(html_tag: &str) -> bool {
    SELF_CLOSING.contains(&html_tag)
}

/// The minimal context needed to pass to an element-generation function
/// - the real Codegen (in mod.rs) implements this trait so genElement()
/// can call back into genChildren() recursively without an import cycle
/// between mod.rs and element.rs. Equivalent to the `codegen` parameter
/// in getContent() in the old TS version.
pub trait ElementCodegenHost {
    /// Generates a new, unique id within the current page, using the
    /// prefix `tag`.
    fn next_id(&mut self, tag: &str) -> String;
    /// Generates HTML for an entire Vec<Child> - recurses back into
    /// genChild in mod.rs.
    fn gen_children(&mut self, children: &[vibao_ast::Child]) -> String;
    /// Registers a CSS rule block (not a media query) into the
    /// stylesheet.
    fn add_css(&mut self, code: &str);
    /// Registers an @media query block into the stylesheet.
    fn add_media_query(&mut self, code: &str);
    /// Registers a build warning (printed to the terminal with a
    /// warning-sign prefix, doesn't stop the build) - used for syntax
    /// that IS VALID to parse but is suspicious in meaning, e.g. an
    /// unrecognized prop name (possibly a typo). Has a default no-op so
    /// OLD implementors (test doubles in component.rs/element.rs) don't
    /// need any changes to keep compiling.
    fn add_warning(&mut self, _msg: String) {}

    /// Registers a `<template>` block that needs to be "hoisted" up to
    /// the page's top level, instead of embedded inline at its call
    /// site.
    ///
    /// BUG ALREADY FIXED (found through a real-browser test - nested
    /// loops rendering wrong, columns repeating chaotically):
    /// `gen_loop()` used to embed `<template id="...">` directly into
    /// the HTML at the position it was generated. With NESTED loops,
    /// this made a CHILD loop's `<template>` sit PHYSICALLY inside its
    /// PARENT loop's `<template>`. When the runtime cloned the parent's
    /// `template.content()` for EACH item (`clone_node_with_deep`), it
    /// accidentally duplicated the child `<template>` too - creating
    /// MULTIPLE COPIES sharing the SAME `id` (a duplicate ID, violating
    /// the HTML standard). `document.getElementById()` always returns
    /// the FIRST match found regardless of context, so every child loop
    /// (no matter which parent item it belonged to) accidentally shared
    /// ONE template - causing exactly the chaotic repeat/overwrite
    /// behavior observed.
    ///
    /// Fixed by SPLITTING the template away from where it's generated:
    /// only a `<div data-vb-loop>` (the container/instance, which NEEDS
    /// to stay in place to repeat per parent item) is left in its
    /// original spot, while `<template>` (the content definition -
    /// only needing to exist ONCE across the whole document) is
    /// collected and printed at the page's top level, as a sibling to
    /// everything else, never nested inside any other template.
    ///
    /// Has a default no-op so OLD implementors (test doubles) don't
    /// need any changes to keep compiling - they typically test a
    /// single, non-nested loop, so the old behavior (skip/don't hoist)
    /// doesn't affect their result.
    fn add_hoisted_template(&mut self, _html: String) {}

    /// Compiles an EventNode into `(attribute_name, action_id)` to embed
    /// directly into the HTML as `data-vb-on-<event>="<id>"`, generating
    /// NO JS. This IS the path used by the current real build pipeline
    /// - see codegen/action.rs::compile_event_handler_registry.
    ///
    /// Has a default calling
    /// `crate::codegen::action::compile_event_handler_registry` directly
    /// - implementors typically do NOT need to override this function,
    /// since it depends on no state of `self`.
    fn compile_event_handler_registry(&self, event: &vibao_ast::EventNode) -> (String, String) {
        crate::codegen::action::compile_event_handler_registry(event)
    }
}

/// The main entry point - dispatches by tag kind. Equivalent to
/// genElement().
///
/// REAL TAG WIRING: `node.tag` is now a Tag (the semantic identity, see
/// vibao_ast::semantic) instead of a String - but is_builtin_complex()/
/// is_layout_tag() below STILL take &str (not yet changed, following
/// the "parallel old path" principle, see the existing
/// is_builtin_complex_semantic()/is_layout_tag_semantic() + their
/// cross-check tests). The _semantic version (taking Tag directly) is
/// used DIRECTLY here - no need to convert back to &str for these 2
/// calls specifically, since a Tag-accepting version already exists for
/// them.
pub fn gen_element(node: &Element, host: &mut dyn ElementCodegenHost) -> String {
    // Converts Tag -> its Vietnamese surface name ONCE, right at the
    // entry point - the rest of codegen (expand_props/resolve_layout_css/
    // tag_to_html/next_id/...) STAYS UNCHANGED, still taking &str as
    // before. This is NOT yet the final destination (the final
    // destination is every downstream function also taking Tag
    // directly, removing this conversion layer entirely) - but it's the
    // SAFEST step right now: isolating the whole change to EXACTLY 1
    // spot, instead of editing hundreds of string-match sites at once.
    let tag_name: &str = crate::locale::vi::tag_display_name_vi(node.tag);
    let id = host.next_id(tag_name);
    if is_builtin_complex_semantic(node.tag) {
        gen_complex_component(node, &id)
    } else if is_layout_tag_semantic(node.tag) {
        gen_layout_element(node, &id, host)
    } else {
        gen_simple_element(node, &id, host)
    }
}

// ════════════════════════════════════════════════════════════
// SIMPLE ELEMENT
// ════════════════════════════════════════════════════════════

fn gen_simple_element(node: &Element, id: &str, host: &mut dyn ElementCodegenHost) -> String {
    // See the full explanation in gen_element() - converted once, used
    // for both the warning message AND the parameter passed to
    // expand_props()/resolve_responsive_css() below (these functions
    // haven't been changed to take Tag directly yet).
    let tag_name: &str = crate::locale::vi::tag_display_name_vi(node.tag);
    let expanded = expand_props(tag_name, &node.props);
    for w in &expanded.warnings {
        host.add_warning(format!("tag '{}': {}", tag_name, w));
    }
    for key in &expanded.unknown_keys {
        // "data_*"/"aria_*" are deliberate HTML attr passthroughs
        // (generating real data-*/aria-* attrs in codegen) - not warned
        // about in these cases, only warning for a prop that looks like
        // a typo.
        if !(key.starts_with("data_") || key.starts_with("aria_")) {
            host.add_warning(format!(
                "prop '{}' on tag '{}' is not recognized by ViBao - possibly a mistyped prop name. \
                 An unrecognized prop is still written directly as the HTML attribute '{}=\"...\"' (not \
                 automatically converted to kebab-case), so if this is meant to be a custom attribute, \
                 check that the name matches the intended HTML attribute format.",
                key, tag_name, key,
            ));
        }
    }
    let style_str = style_map_to_string(&expanded.style);
    let attrs_str = attrs_to_string_ordered(&expanded.attrs, &["noi_dung"]);
    // BUG ALREADY FIXED: this used to use "data-vb-bind-{}" with a RAW
    // JS STRING value (expr_to_js_default(expr), e.g.
    // "__s.mau ?? ..."). The WASM runtime
    // (vibao-runtime/src/runtime/dom.rs) NEVER reads an attr named
    // "data-vb-bind-*" - it only reads "data-vb-style-<css-prop>" with
    // its VALUE AS AN INTEGER ID pointing into the expr registry (see
    // ATTR_BIND_STYLE_PREFIX in dom.rs). Since these 2 formats didn't
    // match, EVERY dynamic CSS binding (text color changing with state,
    // width changing with a variable...) used to silently crash at
    // runtime (no build error, no clear console error beyond a single
    // easy-to-miss warn line from dom.rs). props.rs now registers the
    // Expr into the registry via register_expr() and stores its ID (a
    // number string) into `dynamic`, so only the attribute name needed
    // to be changed to match.
    let dynamic_attrs = expanded
        .dynamic
        .iter()
        .filter(|(k, _)| k.as_str() != "noi_dung") // noi_dung is handled separately below (data-vb-text, not data-vb-style-*)
        .map(|(k, v)| format!("data-vb-style-{}=\"{}\"", camel_to_kebab(k), esc_attr(v)))
        .collect::<Vec<_>>()
        .join(" ");
    // Similar to dynamic_attrs above but for a normal HTML ATTRIBUTE
    // (type/value/placeholder/alt...) instead of CSS style - see the
    // full explanation at the ExpandedProps::dynamic_attrs field
    // (props.rs). Does NOT go through camel_to_kebab since this is
    // already a real HTML attribute name (e.g. "type"), not a camelCase
    // CSS property name needing kebab-case conversion.
    let dynamic_attr_bindings = expanded
        .dynamic_attrs
        .iter()
        .map(|(k, v)| format!("data-vb-attr-{}=\"{}\"", k, esc_attr(v)))
        .collect::<Vec<_>>()
        .join(" ");
    let model_attr = expanded
        .model
        .as_deref()
        .map(|key| format!("data-vb-model=\"{}\"", esc_attr(key)))
        .unwrap_or_default();
    let class_binding_attr = if expanded.class_bindings.is_empty() {
        String::new()
    } else {
        format!(
            "data-vb-class=\"{}\"",
            esc_attr(&expanded.class_bindings.join(","))
        )
    };
    let anim_attrs = gen_anim_attrs(&node.animation);

    // The CORRECT path for the current pipeline: each EventNode is
    // registered into the action registry, generating one HTML
    // attribute "data-vb-on-<dom-event>=\"<actionId>\"" - generating
    // NO JS at all. The WASM runtime (dom.rs::bind_events) reads this
    // attribute itself while binding the page.
    let event_attrs = node
        .events
        .iter()
        .map(|e| {
            let (attr_name, action_id) = host.compile_event_handler_registry(e);
            format!("{}=\"{}\"", attr_name, action_id)
        })
        .collect::<Vec<_>>()
        .join(" ");

    // Hover/scroll animation goes through gen_anim_attrs() together with
    // a load-in animation (data-vb-anim) - see gen_anim_attrs() below.
    // No JS is generated here (the old JS architecture,
    // action.rs::compile_hover_animation/compile_scroll_animation, has
    // been removed entirely - no longer exists anywhere in the
    // codebase).

    if !node.responsive.is_empty() {
        // BUG B FIX (VIBAOC_BUG_NOTES.md) - a soft warning for an
        // unrecognized prop INSIDE a responsive block, at the
        // vocabulary level (see the full doc-comment at
        // layout.rs::responsive_unknown_keys for the exact scope/
        // limitations - not yet validated against a specific Tag).
        // Shares `responsive_unknown_key_warning()` (a module-level
        // helper, see the end of this file) with the similar call site
        // in gen_layout_element().
        for key in crate::codegen::layout::responsive_unknown_keys(&node.responsive) {
            if let Some(msg) = responsive_unknown_key_warning(&key, tag_name) {
                host.add_warning(msg);
            }
        }
        let bp_css = resolve_responsive_css(tag_name, &node.responsive);
        for bp in &bp_css {
            let mq = build_media_query(&format!("#{}", id), bp);
            if !mq.is_empty() {
                host.add_media_query(&mq);
            }
        }
    }

    let content = get_content(tag_name, &node.props, &node.children, host);
    let html_tag = tag_to_html(tag_name);
    let self_closing = is_self_closing(html_tag);

    let all_attrs = [
        format!("id=\"{}\"", id),
        if style_str.is_empty() { String::new() } else { format!("style=\"{}\"", style_str) },
        attrs_str,
        dynamic_attrs,
        dynamic_attr_bindings,
        model_attr,
        class_binding_attr,
        anim_attrs,
        event_attrs,
        if !node.events.is_empty() { "data-vb-interactive".to_string() } else { String::new() },
    ]
    .into_iter()
    .filter(|s| !s.is_empty())
    .collect::<Vec<_>>()
    .join(" ");

    if self_closing {
        format!("<{} {} />", html_tag, all_attrs)
    } else {
        format!("<{} {}>{}</{}>", html_tag, all_attrs, content, html_tag)
    }
}

/// The tag's inner content: prioritizes the `noi_dung` prop (a text
/// literal or a dynamic binding via <span data-vb-text>), then falls
/// back to nested children. Equivalent to getContent() in the old TS
/// version - NOTE: the original TS version used the key "_content", but
/// the Rust parser (parser/element.rs, in parse_element_rest) names the
/// positional-shorthand content parameter "noi_dung" (e.g.
/// text("Xin chao")) - codegen must match the REAL Rust parser actually
/// running, not the original TS version.
fn get_content(tag: &str, props: &vibao_ast::PropsMap, children: &[vibao_ast::Child], host: &mut dyn ElementCodegenHost) -> String {
    let _ = tag; // kept to match the original signature even though not used separately right now
    if let Some(content_expr) = get_prop(props, "noi_dung") {
        let resolved = resolve_value(content_expr);
        return match resolved {
            ResolvedValue::Dynamic => {
                // BUG ALREADY FIXED: this used to embed a RAW JS STRING
                // (expr_to_js_default) directly as the "data-vb-text"
                // value - but dom.rs::parse_expr_id only accepts an
                // INTEGER (an id pointing into the expr registry), so a
                // dynamic text binding (e.g. displaying "$dem" updating
                // with state) never worked at real runtime - just a
                // silent crash with one easy-to-miss console.warn line.
                // The Expr is now registered into the registry, and its
                // numeric ID is embedded.
                let expr_id = register_expr(content_expr.clone());
                format!("<span data-vb-text=\"{}\"></span>", expr_id)
            }
            ResolvedValue::Static(s) => esc_html(&s),
            // Size/Color have no meaning as text content - the old TS
            // version only returned String(resolved.value) when
            // kind === "static", with every other kind returning ""
            // (see the final branch
            // `String(resolved.kind === "static" ? resolved.value : "")`).
            _ => String::new(),
        };
    }
    if !children.is_empty() {
        format!("\n{}\n", indent2(&host.gen_children(children)))
    } else {
        String::new()
    }
}

fn attrs_to_string_ordered(attrs: &OrderedMap, skip: &[&str]) -> String {
    attrs
        .iter()
        .filter(|(k, _)| !skip.contains(&k.as_str()))
        .map(|(k, v)| format!("{}=\"{}\"", k, esc_attr(v)))
        .collect::<Vec<_>>()
        .join(" ")
}

// ════════════════════════════════════════════════════════════
// LAYOUT ELEMENT
// ════════════════════════════════════════════════════════════

fn gen_layout_element(node: &Element, id: &str, host: &mut dyn ElementCodegenHost) -> String {
    let tag_name: &str = crate::locale::vi::tag_display_name_vi(node.tag);
    let layout_css = resolve_layout_css(tag_name, &node.props);
    let style_str = layout_css_to_string_inline(&layout_css);

    // BUG-25 FIX (AUDIT.md) — Layout Element used to stay completely
    // silent when a prop name was mistyped (unlike Simple Element,
    // which already warned through `expand_props().unknown_keys` below).
    // `layout_unknown_keys()` (layout.rs, a NEW function that does NOT
    // change `resolve_layout_css()` or the 9 existing `resolve_*`
    // functions) compares props on the node against the exact valid list
    // for this specific tag — a SOFT warning, exactly like Simple
    // Element's `unknown_keys` mechanism, and does NOT block the build.
    for key in layout_unknown_keys(tag_name, &node.props) {
        // "data_*"/"aria_*" are deliberate HTML attr passthroughs
        // (generating real data-*/aria-* attrs in codegen) — do NOT warn
        // for these cases, using the same filter logic already present in
        // gen_simple_element() (see above). Fixed bug (user review,
        // VIBAOC_BUG_NOTES.md BUG A): the previous code pass lacked this
        // filter, causing `khoi(data_testid: "hero")` to be incorrectly
        // warned as a "mistyped prop name" even though it is a deliberate
        // valid HTML attribute.
        if !(key.starts_with("data_") || key.starts_with("aria_")) {
            host.add_warning(format!(
                "prop '{}' trên thẻ layout '{}' không được ViBao nhận diện — có thể do gõ sai tên prop \
                 hoặc dùng nhầm prop của loại layout khác (vd 'cot' chỉ hợp lệ trên 'grid', không phải '{}'). \
                 Prop lạ trên layout element KHÔNG được ghi ra CSS/attr nào cả (khác simple element).",
                key, tag_name, tag_name
            ));
        }
    }

    // Which props on layout tags can be bound DYNAMICALLY when the value
    // is a plain Variable (not a more complex expression) — matches the
    // `props[key]?.type === "Variable"` check in the old TS version.
    //
    // FIXED BUG: previously ONLY "color" (now "mau_nen") had this
    // dynamic binding mechanism — "width"/"height" (now "rong"/"cao")
    // had no equivalent path, so writing "khoi(rong: $do_rong, ...)"
    // made get_static_value() return the "__dynamic__" sentinel (as
    // designed — it ONLY computes STATIC values), but that "__dynamic__"
    // was inserted directly into the STATIC style string
    // (`style="...width:__dynamic__..."`) without any parallel
    // data-vb-style-width for the runtime to update. Result: the box
    // never changed width when clicking the button, with no console error
    // to identify it (not an "invalid expr id" warning because the
    // runtime did not even know there WAS a binding to handle here).
    //
    // (css_prop_name, prop_key): css_prop_name is used for the
    // "data-vb-style-<css_prop_name>" attribute name — it MUST exactly
    // match the CSSOM kebab-case name (background-color, not color,
    // because layout tags use prop "mau_nen" for BACKGROUND, not text
    // color — see resolve_box in layout.rs: "mau_nen" =>
    // css.insert("backgroundColor", ...)).
    //
    // FIXED BUG (second expansion — found through a REAL TEST BUILD by
    // the user, not just reading code): the previous fix ONLY added
    // color/width/height to this list, but missed EVERY other layout-tag
    // prop (dem/radius/le/bong/overflow/tang_z/gap/...). Meanwhile, the
    // SAME props on SIMPLE ELEMENTS (text/button/input, through
    // props.rs::expand_props, a completely different pipeline from
    // layout.rs) had already had full is_dynamic support. Result:
    // "box(dem: $bien)" (very common — changing padding/radius/width
    // with state) silently did not work dynamically, just like the fixed
    // width/height bug, only affecting 13 other props instead of 2.
    // Extend the list below for EVERY prop with a 1-to-1 mapping to
    // EXACTLY 1 CSS property, THE SAME regardless of which layout tag
    // uses it (checked against each resolve_*() in layout.rs to confirm
    // no tag maps this prop to a different CSS property or depends on an
    // additional companion prop):
    //   dem → padding (flex/grid/box/container, always the same)
    //   radius → borderRadius (flex/box, always the same)
    //   le → margin (box only, no collision with other tags)
    //   bong → boxShadow (box only)
    //   overflow → overflow (box only, straight pass-through value)
    //   tang_z → zIndex (box only)
    //   gap/gap_doc/gap_ngang → gap/rowGap/columnGap (flex/grid, same)
    //   min_rong/max_rong/min_cao/max_cao → minWidth/maxWidth/minHeight/
    //     maxHeight (box/container, same)
    //
    // DO NOT add to the list (keep static-only; needs a separate warning
    // instead of setting the wrong or incomplete style if bound directly
    // — see the warning branch immediately below this const):
    //   - "huong": meaning is COMPLETELY DIFFERENT by tag —
    //     flexDirection in `flex`, but in `scroll` it decides overflow,
    //     overflowX, and overflowY ALL AT ONCE (3 properties, depending
    //     on whether the value is "ngang") — cannot be set through a
    //     single data-vb-style-<prop>.
    //   - "vien": sets the CSS shorthand "border" combined from 3
    //     sources (itself + "kieu_vien" + "mau_vien" read from other
    //     props) — 1 dynamic expr is not enough to recreate this
    //     shorthand.
    //   - "can"/"doc"/"boc": requires mapping Vietnamese/bool values to
    //     CSS keywords at build time (map_justify/map_align_items/
    //     "true"→"wrap") — same limitation as "can"/"doc" already
    //     warned about in props.rs.
    //   - "vi_tri": maps 1 value to 3 different CSS properties depending
    //     on the branch (tren/duoi/trai/phai) — not a 1-to-1 mapping.
    const DYNAMIC_LAYOUT_PROPS: [(&str, &str); 16] = [
        ("background-color", "mau_nen"),
        ("width", "rong"),
        ("height", "cao"),
        ("padding", "dem"),
        ("border-radius", "radius"),
        ("margin", "le"),
        ("box-shadow", "bong"),
        ("overflow", "cuon_tran"),
        ("z-index", "tang_z"),
        ("gap", "gap"),
        ("row-gap", "gap_doc"),
        ("column-gap", "gap_ngang"),
        ("min-width", "min_rong"),
        ("max-width", "max_rong"),
        ("min-height", "min_cao"),
        ("max-height", "max_cao"),
    ];
    let dynamic_layout_attrs = DYNAMIC_LAYOUT_PROPS
        .iter()
        .filter_map(|(css_prop_name, prop_key)| match get_prop(&node.props, prop_key) {
            Some(expr @ vibao_ast::Expr::Variable(_, _)) => {
                let expr_id = register_expr(expr.clone());
                Some(format!("data-vb-style-{}=\"{}\"", css_prop_name, expr_id))
            }
            _ => None,
        })
        .collect::<Vec<_>>()
        .join(" ");
    // Build-time warning for layout props that CANNOT support simple
    // dynamic binding (see the "DO NOT add to the list" explanation
    // above). Without this warning, if a dev uses a dynamic expression
    // for these props on a layout tag, the style would silently be set to
    // the literal "__dynamic__" (meaningless CSS, ignored by browsers).
    const UNSUPPORTED_DYNAMIC_LAYOUT_PROPS: [&str; 5] = ["huong", "vien", "can", "doc", "boc"];
    for prop_key in UNSUPPORTED_DYNAMIC_LAYOUT_PROPS {
        if let Some(vibao_ast::Expr::Variable(_, _)) = get_prop(&node.props, prop_key) {
            host.add_warning(format!(
                "thẻ '{}': prop '{}' với giá trị biểu thức động chưa được hỗ trợ trên layout tag \
                 (cần map giá trị hoặc ghép nhiều CSS property cùng lúc, không thể bind qua 1 \
                 data-vb-style- đơn). Style sẽ KHÔNG được áp dụng. Dùng giá trị tĩnh, hoặc bọc \
                 trong 'neu/khong_thi' ở cấp element để chọn hẳn 1 nhánh lúc build.",
                tag_name, prop_key,
            ));
        }
    }
    let anim_attrs = gen_anim_attrs(&node.animation);

    host.add_css(&layout_css_to_string(&format!("#{}", id), &layout_css));

    if !node.responsive.is_empty() {
        // BUG B FIX (VIBAOC_BUG_NOTES.md) — see the full explanation at
        // the similar call site in gen_simple_element() above. Reuses
        // `responsive_unknown_key_warning()` (a new module-level helper,
        // see the end of this file) to avoid duplicating the logic twice
        // — following the lesson from historical BUG-16 (two independent
        // copies of logic can drift over time).
        for key in crate::codegen::layout::responsive_unknown_keys(&node.responsive) {
            if let Some(msg) = responsive_unknown_key_warning(&key, tag_name) {
                host.add_warning(msg);
            }
        }
        let bp_css = resolve_responsive_css(tag_name, &node.responsive);
        for bp in &bp_css {
            let mq = build_media_query(&format!("#{}", id), bp);
            if !mq.is_empty() {
                host.add_media_query(&mq);
            }
        }
    }
    // host.compile_hover_animation()/compile_scroll_animation() (the old
    // JS mechanism, add_js) used to run IN PARALLEL with gen_anim_attrs()
    // (the new attr path, declared in anim_attrs above) — while
    // gen_simple_element() (the simple element branch, see its note) had
    // already been cleaned up correctly to leave only one path
    // (gen_anim_attrs, NO add_js). As a result, hover/scroll animations
    // on LAYOUT TAGS (khoi/flex/stack/...) were applied TWICE in
    // overlapping ways — once through the data-vb-anim-hover/-scroll attr
    // (read by the WASM runtime itself), and once through hand-written JS
    // embedded into <script>. Two mechanisms attached the same animation,
    // making runtime behavior not clearly defined (depending on how the
    // two mechanisms interacted). Remove the add_js branch here to match
    // the selected pipeline (gen_anim_attrs only). The entire old JS
    // mechanism (compile_hover_animation/compile_scroll_animation and the
    // 2 corresponding methods in trait ElementCodegenHost) was then
    // DELETED COMPLETELY (no longer exists in the codebase) because there
    // is no real call path left — see the history in action.rs.

    // REAL TAG WIRING: compare Tag (semantic identity) directly instead
    // of strings — no need to go through tag_name for these 2 comparisons
    // specifically. This is closer to the final destination (low risk
    // because Tag is Copy + PartialEq, direct comparison, with no chance
    // of spelling drift like strings).
    let children_html = if node.tag == vibao_ast::Tag::Stack {
        node.children
            .iter()
            .map(|c| {
                let html = host.gen_children(std::slice::from_ref(c));
                if html.is_empty() { String::new() } else { wrap_stack_child(&html) }
            })
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        host.gen_children(&node.children)
    };

    let extra_style = if node.tag == vibao_ast::Tag::Lop { "position:relative;" } else { "" };

    format!(
        "<div id=\"{}\" style=\"{}{}\" {} {}>\n{}\n</div>",
        id,
        extra_style,
        style_str,
        dynamic_layout_attrs,
        anim_attrs,
        indent2(&children_html)
    )
}

fn wrap_stack_child(html: &str) -> String {
    format!("<div style=\"grid-area:1/1/2/2\">{}</div>", html)
}

// ════════════════════════════════════════════════════════════
// COMPLEX BUILT-IN COMPONENT (placeholder — real content is built by the runtime)
// ════════════════════════════════════════════════════════════

fn gen_complex_component(node: &Element, id: &str) -> String {
    let tag_name: &str = crate::locale::vi::tag_display_name_vi(node.tag);
    format!("<div id=\"{}\" data-vb-component=\"{}\"><!-- {} --></div>", id, tag_name, tag_name)
}

/// Translates a ViBao tag (Vietnamese-localized, e.g. "khoi"/"cuon"/
/// "lop") to the original English name used for class/id (e.g. "box"/
/// "scroll"/"layer") — ONLY for naming generated identifiers (HTML id,
/// CSS class), with NO effect on the lexer/parser or tag_to_html()
/// (which remain Vietnamese-localized normally everywhere else). Tags
/// not in this table (components, simple tags like text/button...) keep
/// the original tag.
///
/// Why a separate table is needed: after Vietnamese-localizing tag names
/// (see vocabulary.rs), automatically generated ids/classes (e.g.
/// "vb-khoi-3") become harder to read and inconsistent with the
/// pre-existing English CSS class convention (e.g. docs/VIBAO_SPEC.md
/// still describes layout tags with English names: box, scroll,
/// container, layer...). Keep generated ids in familiar English names,
/// completely separate from ViBao syntax (always Vietnamese).
pub(crate) fn tag_to_class_name(tag: &str) -> &str {
    match tag {
        "khoi" => "box",
        "cuon" => "scroll",
        "lop" => "layer",
        "khoang_cach" => "spacer",
        "duong_ke" => "divider",
        "can_giua" => "container",
        "dinh_dau" => "sticky",
        "dinh_man_hinh" => "fixed",
        other => other,
    }
}

/// Conventional CSS class name for a layout element — currently not used
/// directly in gen_layout_element() (the old TS version computed it but
/// ultimately used id as the CSS selector, not className). Kept to match
/// the original public makeClassName() API in case another place needs it
/// later (e.g. debug tooling).
#[cfg(test)]
pub fn make_class_name(tag: &str, index: u32) -> String {
    format!("vb-{}-{}", tag_to_class_name(tag), index)
}

// ════════════════════════════════════════════════════════════
// ANIMATION ATTRS
// ════════════════════════════════════════════════════════════

/// Generates all animation-related HTML attributes for an element —
/// covering both load-in animation (hieu_ung, runs immediately on mount)
/// AND hover/scroll (hieu_ung_hover/hieu_ung_cuon, event-driven). All of
/// them are PLAIN ATTRIBUTES and generate no JS — the WASM runtime
/// (dom.rs) reads and handles them itself in Rust (web-sys
/// IntersectionObserver/mouseenter/mouseleave), matching the selected
/// "pure Rust, no JS eval" architecture for the whole runtime. This IS
/// the ONLY path for animation in the current build pipeline — the old JS
/// mechanism (compile_hover_animation/compile_scroll_animation) has been
/// deleted completely from the codebase.
pub fn gen_anim_attrs(anim: &AnimationProps) -> String {
    let mut parts: Vec<String> = Vec::new();

    if let Some(hieu_ung) = &anim.hieu_ung {
        parts.push(format!("data-vb-anim=\"{}\"", hieu_ung));
        if let Some(ms) = anim.thoi_gian {
            parts.push(format!("data-vb-anim-duration=\"{}\"", ms));
        }
        if let Some(ms) = anim.tre {
            parts.push(format!("data-vb-anim-delay=\"{}\"", ms));
        }
        if let Some(lap) = &anim.lap {
            let lap_str = match lap {
                LapValue::Count(n) => n.to_string(),
                LapValue::MaiMai => "infinite".to_string(),
            };
            parts.push(format!("data-vb-anim-repeat=\"{}\"", lap_str));
        }
    }

    // Hover: "<ten_hieu_ung>:<thoi_gian_ms>" — a single attribute holds
    // both values (separated by ":") to simplify the runtime side
    // (only split one string; no need to read a separate -duration
    // attribute like the load-in branch above).
    if let Some(hover) = &anim.hieu_ung_hover {
        let dur = anim.thoi_gian.unwrap_or(300);
        parts.push(format!("data-vb-anim-hover=\"{}:{}\"", hover, dur));
    }

    // Scroll: "<ten_hieu_ung>:<thoi_gian_ms>:<tre_ms>".
    if let Some(scroll) = &anim.hieu_ung_cuon {
        let dur = anim.thoi_gian.unwrap_or(600);
        let delay = anim.tre.unwrap_or(0);
        parts.push(format!("data-vb-anim-scroll=\"{}:{}:{}\"", scroll, dur, delay));
    }

    parts.join(" ")
}

// ════════════════════════════════════════════════════════════
// BUG B FIX — responsive warning message (the layout validation review notes)
// ════════════════════════════════════════════════════════════

/// Returns a warning message for a key NOT in `RESPONSIVE_HANDLED_PROPS`
/// (layout.rs) — shared by both `gen_simple_element()` and
/// `gen_layout_element()` (the 2 call sites of `resolve_responsive_css()`
/// in this file) to avoid duplicating the message logic twice, following
/// the lesson from historical BUG-16 (two independent copies of logic
/// with the same meaning can drift over time if not unified).
///
/// MESSAGE FIX (the layout validation review notes item 2, user review):
/// previously, EVERY key not among the 7 `RESPONSIVE_HANDLED_PROPS` names
/// received the SAME "not recognized by ViBao" message — misleading for
/// 49/57 PropKeys (e.g. "bong"/"vien"/"gap"/"radius"...), which ARE valid
/// props but simply do NOT yet have dedicated handling in responsive blocks,
/// not typos. Now distinguish 2 cases by calling back into
/// `locale::resolve_prop_key()`: (a) key does NOT resolve to any PropKey
/// — likely a typo, so keep the original warning tone; (b) key IS a
/// valid PropKey (in another Simple/Layout context) but lacks dedicated
/// responsive handling — change tone to state it is NOT a typo,
/// only a current feature limitation.
///
/// Returns `None` for keys starting with `data_`/`aria_` (HTML attr passthroughs
/// that are deliberate, with no warning — same filter already used in other
/// `unknown_keys` handling).
///
/// IMPORTANT NOTE (the layout validation review notes, documentation note): suppressing
/// warnings here ONLY means "do not emit a build-time warning for this prop
/// name" — it does NOT mean responsive CAN change data_*/aria_* values
/// data-*/aria-* LÚC RUNTIME theo breakpoint. `resolve_responsive_css()`
/// (where CSS is actually generated) only handles CSS style via `@media`, with
/// absolutely NO mechanism to change HTML ATTRIBUTES (including data-*/aria-*) by
/// breakpoint — if this key reaches `resolve_responsive_css()`, it is still
/// converted into an arbitrary CSS property via passthrough (`camelCase-
/// >kebab-case`, see the final `_ =>` branch in `resolve_responsive_css()`),
/// and NEVER becomes a real HTML attribute. This filter exists only
/// to avoid false warnings when a dev uses data_*/aria_* as an arbitrary
/// CSS custom property (rare in responsive blocks but possible),
/// not as a "responsive attribute binding" feature.
fn responsive_unknown_key_warning(key: &str, tag_name: &str) -> Option<String> {
    if key.starts_with("data_") || key.starts_with("aria_") {
        return None;
    }
    let msg = if crate::locale::resolve_prop_key(key).is_some() {
        format!(
            "prop '{}' trong khối responsive (@di_dong/@may_tinh_bang/@may_tinh) trên thẻ '{}' \
             LÀ 1 prop hợp lệ của ViBao, nhưng CHƯA được hỗ trợ trực tiếp trong ngữ cảnh \
             responsive (chỉ 7 prop được xử lý riêng ở đây: cot/huong/co/rong/cao/dem/an — \
             xem RESPONSIVE_HANDLED_PROPS). Giá trị vẫn được chuyển thành 1 CSS property tuỳ ý \
             qua passthrough (camelCase->kebab-case) nên có thể hoạt động tình cờ đúng, nhưng \
             KHÔNG được đảm bảo — không phải gõ sai, có thể yên tâm bỏ qua nếu kết quả đang \
             hiển thị đúng ý muốn.",
            key, tag_name
        )
    } else {
        format!(
            "prop '{}' trong khối responsive (@di_dong/@may_tinh_bang/@may_tinh) trên thẻ '{}' \
             không được ViBao nhận diện — có thể do gõ sai tên prop. Giá trị VẪN được chuyển \
             thành 1 CSS property tuỳ ý (qua camelCase->kebab-case, hành vi passthrough giữ \
             nguyên như cũ để không phá vỡ các trường hợp cố ý dùng CSS property tuỳ ý), nên \
             nếu đây không phải gõ sai, có thể bỏ qua cảnh báo này.",
            key, tag_name
        )
    };
    Some(msg)
}

// ════════════════════════════════════════════════════════════
// UNIT TESTS
// ════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use vibao_ast::{Child, Element, Expr, LoopKind, LoopNode, Pos, ResponsiveNode, Breakpoint};

    fn p() -> Pos {
        Pos { line: 1, column: 1 }
    }

    /// COMPLETE list of 41 valid tag names in the Vietnamese locale —
    /// shared by the String-based vs Tag-based cross-check tests below.
    const ALL_TAG_NAMES: [&str; 41] = [
        "text", "h1", "h2", "h3", "p", "nhan",
        "image", "video", "icon",
        "button", "input", "link", "lien_ket",
        "flex", "grid", "stack", "khoi", "cuon", "can_giua", "lop", "dinh_dau", "dinh_man_hinh",
        "khoang_cach", "duong_ke",
        "form", "nhom_input", "chon_mot", "hop_kiem", "lua_chon",
        "modal", "tabs", "gap_mo", "bang_chuyen", "xuong_trang", "vong_quay",
        "thanh_tien_trinh", "bang", "bieu_do", "ban_do", "thanh_dieu_huong", "trinh_soan_thao",
    ];

    #[test]
    fn test_is_builtin_complex_semantic_matches_string_based_for_every_known_tag() {
        for name in ALL_TAG_NAMES {
            let tag = crate::locale::vi::tag_name_vi(name)
                .unwrap_or_else(|| panic!("locale::vi::tag_name_vi(\"{}\") trả None", name));
            assert_eq!(
                is_builtin_complex(name),
                is_builtin_complex_semantic(tag),
                "LỆCH cho tag '{}'", name
            );
        }
    }

    #[test]
    fn test_tag_to_html_semantic_matches_string_based_for_every_known_tag() {
        for name in ALL_TAG_NAMES {
            let tag = crate::locale::vi::tag_name_vi(name)
                .unwrap_or_else(|| panic!("locale::vi::tag_name_vi(\"{}\") trả None", name));
            assert_eq!(
                tag_to_html(name),
                tag_to_html_semantic(tag),
                "LỆCH cho tag '{}': String-based=\"{}\" nhưng Tag-based=\"{}\"",
                name, tag_to_html(name), tag_to_html_semantic(tag)
            );
        }
    }

    /// Minimal fake host for tests — no real Codegen is needed to check
    /// element.rs's pure logic.
    struct FakeHost {
        counter: u32,
        js: Vec<String>,
        css: Vec<String>,
        media: Vec<String>,
        warnings: Vec<String>,
        hoisted_templates: Vec<String>,
    }

    impl FakeHost {
        fn new() -> Self {
            FakeHost { counter: 0, js: vec![], css: vec![], media: vec![], warnings: vec![], hoisted_templates: vec![] }
        }
    }

    impl ElementCodegenHost for FakeHost {
        fn next_id(&mut self, tag: &str) -> String {
            self.counter += 1;
            format!("vb-{}-{}", tag, self.counter)
        }
        fn gen_children(&mut self, children: &[Child]) -> String {
            children
                .iter()
                .map(|c| match c {
                    Child::Element(el) => gen_element(el, self),
                    // FIXED BUG (in this test double itself): previously
                    // Child::Loop/If/Switch fell into the `_ =>
                    // String::new()` branch — meaning tests using
                    // FakeHost could not correctly simulate recursion
                    // through nested loop/if/switch (e.g. could not catch
                    // the "template nested inside template" bug because
                    // the inner loop was never actually called). Dispatch
                    // exactly like the real Codegen::gen_child.
                    Child::Loop(node) => crate::codegen::control::gen_loop(node, self),
                    Child::If(node) => crate::codegen::control::gen_if(node, self),
                    Child::Switch(node) => crate::codegen::control::gen_switch(node, self),
                    _ => String::new(),
                })
                .collect::<Vec<_>>()
                .join("\n")
        }
        fn add_css(&mut self, code: &str) {
            self.css.push(code.to_string());
        }
        fn add_media_query(&mut self, code: &str) {
            self.media.push(code.to_string());
        }
        fn add_warning(&mut self, msg: String) {
            self.warnings.push(msg);
        }
        fn add_hoisted_template(&mut self, html: String) {
            self.hoisted_templates.push(html);
        }
    }

    fn make_element(tag: vibao_ast::Tag, props: vibao_ast::PropsMap) -> Element {
        Element {
            tag,
            props,
            children: vec![],
            events: vec![],
            responsive: vec![],
            animation: AnimationProps::default(),
            pos: p(),
        }
    }

    #[test]
    fn test_static_text_is_html_escaped() {
        let props: vibao_ast::PropsMap = vec![(
            "noi_dung".to_string(),
            Expr::Literal(vibao_ast::LiteralValue::Str("A < B & C > D".to_string()), p()),
        )];
        let el = make_element(vibao_ast::Tag::Text, props);
        let mut host = FakeHost::new();
        let html = gen_element(&el, &mut host);
        assert!(html.contains("A &lt; B &amp; C &gt; D"));
        assert!(!html.contains("A < B & C > D"));
    }

    #[test]
    fn test_static_attributes_are_html_attribute_escaped() {
        let mut attrs = OrderedMap::new();
        attrs.insert("alt".to_string(), "Ảnh \"đẹp\" <test>".to_string());
        let html = attrs_to_string_ordered(&attrs, &[]);
        assert_eq!(html, "alt=\"Ảnh &quot;đẹp&quot; &lt;test&gt;\"");
    }

    #[test]
    fn test_gen_simple_element_warns_on_unknown_prop() {
        let props: vibao_ast::PropsMap = vec![(
            "mua".to_string(),
            Expr::Literal(vibao_ast::LiteralValue::Str("do".to_string()), p()),
        )];
        let el = make_element(vibao_ast::Tag::Text, props);
        let mut host = FakeHost::new();
        gen_element(&el, &mut host);
        assert_eq!(host.warnings.len(), 1);
        assert!(host.warnings[0].contains("mua"));
        assert!(host.warnings[0].contains("text"));
    }

    #[test]
    fn test_gen_simple_element_no_warning_for_data_prefixed_prop() {
        // "data_*" is a deliberate HTML attr passthrough — not a
        // mistyped prop name, so it must NOT be warned.
        let props: vibao_ast::PropsMap = vec![(
            "data_testid".to_string(),
            Expr::Literal(vibao_ast::LiteralValue::Str("hero".to_string()), p()),
        )];
        let el = make_element(vibao_ast::Tag::Text, props);
        let mut host = FakeHost::new();
        gen_element(&el, &mut host);
        assert!(host.warnings.is_empty());
    }

    #[test]
    fn test_gen_simple_element_no_warning_for_known_prop() {
        let props: vibao_ast::PropsMap = vec![(
            "mau".to_string(),
            Expr::Literal(vibao_ast::LiteralValue::Color("#FF0000".to_string()), p()),
        )];
        let el = make_element(vibao_ast::Tag::Text, props);
        let mut host = FakeHost::new();
        gen_element(&el, &mut host);
        assert!(host.warnings.is_empty());
    }

    #[test]
    fn test_tag_to_html_mapping() {
        assert_eq!(tag_to_html("text"), "p");
        assert_eq!(tag_to_html("link"), "a");
        assert_eq!(tag_to_html("lien_ket"), "a");
        assert_eq!(tag_to_html("image"), "img");
        assert_eq!(tag_to_html("unknown_custom_tag"), "div");
    }

    #[test]
    fn test_self_closing_tags() {
        assert!(is_self_closing("img"));
        assert!(is_self_closing("input"));
        assert!(!is_self_closing("div"));
        assert!(!is_self_closing("p"));
    }

    #[test]
    fn test_simple_element_text_content() {
        let mut host = FakeHost::new();
        let el = make_element(vibao_ast::Tag::Text, vec![("noi_dung".to_string(), Expr::literal_str("Xin chào", p()))]);
        let html = gen_simple_element(&el, "vb-text-1", &mut host);
        assert!(html.starts_with("<p "));
        assert!(html.contains(">Xin chào</p>"));
    }

    #[test]
    fn test_simple_element_self_closing_image() {
        let mut host = FakeHost::new();
        let el = make_element(vibao_ast::Tag::Image, vec![("mo_ta_anh".to_string(), Expr::literal_str("logo", p()))]);
        let html = gen_simple_element(&el, "vb-image-1", &mut host);
        assert!(html.starts_with("<img "));
        assert!(html.ends_with("/>"));
    }

    #[test]
    fn test_simple_element_dynamic_content_uses_span_binding() {
        let mut host = FakeHost::new();
        let el = make_element(vibao_ast::Tag::Text, vec![("noi_dung".to_string(), Expr::Variable("ten".to_string(), p()))]);
        let html = gen_simple_element(&el, "vb-text-1", &mut host);
        assert!(html.contains("data-vb-text="));
    }

    #[test]
    fn test_layout_element_is_div_with_css_registered() {
        let mut host = FakeHost::new();
        let el = make_element(vibao_ast::Tag::Flex, vec![]);
        let html = gen_layout_element(&el, "vb-flex-1", &mut host);
        assert!(html.starts_with("<div id=\"vb-flex-1\""));
        assert_eq!(host.css.len(), 1);
        assert!(host.css[0].contains("display: flex;"));
    }

    #[test]
    fn test_layout_element_layer_gets_relative_position() {
        let mut host = FakeHost::new();
        let el = make_element(vibao_ast::Tag::Lop, vec![]);
        let html = gen_layout_element(&el, "vb-layer-1", &mut host);
        assert!(html.contains("position:relative;"));
    }

    #[test]
    fn test_layout_element_warns_on_unknown_prop() {
        // Direct evidence that BUG-25 works through the real
        // gen_layout_element() path (not only layout_unknown_keys() in
        // isolation as tested in layout.rs) — mistyping "magin" on
        // "khoi".
        let props: vibao_ast::PropsMap = vec![(
            "magin".to_string(),
            Expr::Literal(vibao_ast::LiteralValue::Str("8".to_string()), p()),
        )];
        let el = make_element(vibao_ast::Tag::Khoi, props);
        let mut host = FakeHost::new();
        gen_layout_element(&el, "vb-box-1", &mut host);
        assert_eq!(host.warnings.len(), 1);
        assert!(host.warnings[0].contains("magin"));
        assert!(host.warnings[0].contains("khoi"));
    }

    #[test]
    fn test_layout_element_no_warning_for_data_prefixed_prop() {
        // Fixed bug (user review, VIBAOC_BUG_NOTES.md BUG A):
        // gen_layout_element() previously lacked the data_*/aria_* filter,
        // causing valid passthrough attrs to be incorrectly warned as
        // mistyped prop names. Same logic that already existed in
        // gen_simple_element().
        let props: vibao_ast::PropsMap = vec![(
            "data_testid".to_string(),
            Expr::Literal(vibao_ast::LiteralValue::Str("hero".to_string()), p()),
        )];
        let el = make_element(vibao_ast::Tag::Khoi, props);
        let mut host = FakeHost::new();
        gen_layout_element(&el, "vb-box-1", &mut host);
        assert!(host.warnings.is_empty());
    }

    #[test]
    fn test_layout_element_no_warning_for_aria_prefixed_prop() {
        let props: vibao_ast::PropsMap = vec![(
            "aria_hidden".to_string(),
            Expr::Literal(vibao_ast::LiteralValue::Str("true".to_string()), p()),
        )];
        let el = make_element(vibao_ast::Tag::Flex, props);
        let mut host = FakeHost::new();
        gen_layout_element(&el, "vb-flex-1", &mut host);
        assert!(host.warnings.is_empty());
    }

    /// Helper for responsive tests — make_element() does not allow
    /// setting the `responsive` field, so create Element directly here.
    fn make_element_with_responsive(
        tag: vibao_ast::Tag,
        responsive: Vec<ResponsiveNode>,
    ) -> Element {
        Element {
            tag,
            props: vec![],
            children: vec![],
            events: vec![],
            responsive,
            animation: AnimationProps::default(),
            pos: p(),
        }
    }

    #[test]
    fn test_responsive_warns_on_unknown_prop_simple_element() {
        // BUG B FIX (VIBAOC_BUG_NOTES.md) — direct evidence through
        // the real gen_simple_element() path: mistyping "ronnng" instead of
        // "rong" in an @di_dong block is now warned.
        let el = make_element_with_responsive(
            vibao_ast::Tag::Text,
            vec![ResponsiveNode {
                breakpoint: Breakpoint::DiDong,
                overrides: vec![(
                    "ronnng".to_string(),
                    Expr::literal_num(100.0, p()),
                )],
                pos: p(),
            }],
        );
        let mut host = FakeHost::new();
        gen_simple_element(&el, "vb-text-1", &mut host);
        assert_eq!(host.warnings.len(), 1);
        assert!(host.warnings[0].contains("ronnng"));
    }

    #[test]
    fn test_responsive_warns_on_unknown_prop_layout_element() {
        // Same bug, gen_layout_element() path — two separate wiring
        // points in element.rs, so each point needs its own test.
        let el = make_element_with_responsive(
            vibao_ast::Tag::Khoi,
            vec![ResponsiveNode {
                breakpoint: Breakpoint::DiDong,
                overrides: vec![(
                    "ronnng".to_string(),
                    Expr::literal_num(100.0, p()),
                )],
                pos: p(),
            }],
        );
        let mut host = FakeHost::new();
        gen_layout_element(&el, "vb-box-1", &mut host);
        assert_eq!(host.warnings.len(), 1);
        assert!(host.warnings[0].contains("ronnng"));
    }

    #[test]
    fn test_responsive_known_vocabulary_does_not_warn() {
        // Cross-check the exact 7 real names (layout.rs::RESPONSIVE_HANDLED_PROPS)
        // — there is NO "gap" even if intuition suggests it (only regular layout
        // elements have "gap"; @di_dong does not handle this name specially,
        // see resolve_responsive_css()).
        let overrides: vibao_ast::PropsMap = vec![
            ("cot".to_string(), Expr::literal_num(2.0, p())),
            ("huong".to_string(), Expr::Literal(vibao_ast::LiteralValue::Str("row".to_string()), p())),
            ("co".to_string(), Expr::literal_num(14.0, p())),
            ("rong".to_string(), Expr::literal_num(100.0, p())),
            ("cao".to_string(), Expr::literal_num(100.0, p())),
            ("dem".to_string(), Expr::literal_num(8.0, p())),
            ("an".to_string(), Expr::Literal(vibao_ast::LiteralValue::Str("true".to_string()), p())),
        ];
        let el = make_element_with_responsive(
            vibao_ast::Tag::Text,
            vec![ResponsiveNode { breakpoint: Breakpoint::DiDong, overrides, pos: p() }],
        );
        let mut host = FakeHost::new();
        gen_simple_element(&el, "vb-text-1", &mut host);
        assert!(host.warnings.is_empty(), "7 prop hợp lệ không được cảnh báo: {:?}", host.warnings);
    }

    #[test]
    fn test_responsive_no_warning_for_data_prefixed_prop() {
        let el = make_element_with_responsive(
            vibao_ast::Tag::Text,
            vec![ResponsiveNode {
                breakpoint: Breakpoint::DiDong,
                overrides: vec![(
                    "data_testid".to_string(),
                    Expr::Literal(vibao_ast::LiteralValue::Str("hero".to_string()), p()),
                )],
                pos: p(),
            }],
        );
        let mut host = FakeHost::new();
        gen_simple_element(&el, "vb-text-1", &mut host);
        assert!(host.warnings.is_empty());
    }

    #[test]
    fn test_responsive_wrong_tag_for_known_vocab_not_flagged_documented_limitation() {
        // DOCUMENTED LIMITATION (not a bug — see the full doc-comment
        // in layout.rs::responsive_unknown_keys): "cot" is in the valid
        // vocabulary, so it is NOT warned even when placed on
        // "text" (Simple tag; "cot" — grid-template-columns — only has
        // real meaning on "grid"). Validating by exact tag is Level 2
        // (done after PropKey is wired into the semantic layer), NOT done in
        // this pass — this test confirms the documented limitation, not
        // accidental behavior.
        let el = make_element_with_responsive(
            vibao_ast::Tag::Text,
            vec![ResponsiveNode {
                breakpoint: Breakpoint::DiDong,
                overrides: vec![(
                    "cot".to_string(),
                    Expr::literal_num(2.0, p()),
                )],
                pos: p(),
            }],
        );
        let mut host = FakeHost::new();
        gen_simple_element(&el, "vb-text-1", &mut host);
        assert!(host.warnings.is_empty(), "giới hạn đã biết: mức vocabulary không bắt được sai-tag");
    }

    #[test]
    fn test_responsive_warning_message_distinguishes_valid_propkey_from_typo() {
        // MESSAGE FIX (the layout validation review notes item 2, user
        // review): previously "bong" (valid PropKey::Shadow, only not yet
        // supported in responsive) and "ronnng" (typo, not any PropKey)
        // received the SAME "not recognized by ViBao" warning —
        // misleading for the first case. This test confirms the 2 messages
        // have clearly different tones.
        let el_valid_propkey = make_element_with_responsive(
            vibao_ast::Tag::Khoi,
            vec![ResponsiveNode {
                breakpoint: Breakpoint::DiDong,
                overrides: vec![("bong".to_string(), Expr::literal_str("true", p()))],
                pos: p(),
            }],
        );
        let mut host1 = FakeHost::new();
        gen_layout_element(&el_valid_propkey, "vb-box-1", &mut host1);
        assert_eq!(host1.warnings.len(), 1);
        assert!(
            host1.warnings[0].contains("hợp lệ"),
            "'bong' là PropKey hợp lệ, message phải nói rõ KHÔNG phải gõ sai: {}",
            host1.warnings[0]
        );
        assert!(
            !host1.warnings[0].contains("có thể do gõ sai"),
            "'bong' không nên bị nghi ngờ là gõ sai: {}",
            host1.warnings[0]
        );

        let el_typo = make_element_with_responsive(
            vibao_ast::Tag::Khoi,
            vec![ResponsiveNode {
                breakpoint: Breakpoint::DiDong,
                overrides: vec![("bonng".to_string(), Expr::literal_str("true", p()))],
                pos: p(),
            }],
        );
        let mut host2 = FakeHost::new();
        gen_layout_element(&el_typo, "vb-box-2", &mut host2);
        assert_eq!(host2.warnings.len(), 1);
        assert!(
            host2.warnings[0].contains("có thể do gõ sai"),
            "'bonng' không phải PropKey nào, message phải nghi ngờ gõ sai: {}",
            host2.warnings[0]
        );
    }

    #[test]
    fn test_responsive_unknown_keys_deduped_across_breakpoints() {
        // DEDUPE FIX (the layout validation review notes item 1, user
        // review): previously the same mistyped name appearing in MULTIPLE
        // breakpoints (@di_dong AND @may_tinh_bang) generated N duplicate
        // warnings for the SAME error. This test confirms there is only 1
        // warning even though "ronnng" appears in 2 breakpoints.
        let el = make_element_with_responsive(
            vibao_ast::Tag::Text,
            vec![
                ResponsiveNode {
                    breakpoint: Breakpoint::DiDong,
                    overrides: vec![("ronnng".to_string(), Expr::literal_num(100.0, p()))],
                    pos: p(),
                },
                ResponsiveNode {
                    breakpoint: Breakpoint::MayTinhBang,
                    overrides: vec![("ronnng".to_string(), Expr::literal_num(200.0, p()))],
                    pos: p(),
                },
            ],
        );
        let mut host = FakeHost::new();
        gen_simple_element(&el, "vb-text-1", &mut host);
        assert_eq!(host.warnings.len(), 1, "phải dedupe, chỉ 1 cảnh báo dù 'ronnng' xuất hiện ở 2 breakpoint");
    }

    #[test]
    fn test_layout_element_dynamic_width_generates_binding_not_dead_static_value() {
        // Regression test for a real bug (found through build + testing in
        // a real browser): khoi(rong: $do_rong) did not change width when
        // clicking a button, because previously only "mau_nen" had dynamic
        // binding on layout tags — "rong" fell straight into static CSS as
        // "width:__dynamic__" (meaningless, ignored by browsers), with no
        // parallel data-vb-style-width for the runtime to update.
        let mut host = FakeHost::new();
        let el = make_element(
            vibao_ast::Tag::Khoi,
            vec![("rong".to_string(), Expr::Variable("do_rong".to_string(), p()))],
        );
        let html = gen_layout_element(&el, "vb-box-1", &mut host);
        assert!(html.contains("data-vb-style-width=\""), "phải có binding động cho width: {}", html);
        assert!(!html.contains("__dynamic__"), "sentinel không được lọt vào output cuối: {}", html);
    }

    #[test]
    fn test_layout_element_hover_animation_no_longer_double_applied() {
        // FIXED BUG: gen_layout_element() used to call BOTH
        // host.compile_hover_animation() (add_js, old JS mechanism) AND
        // gen_anim_attrs() (data-vb-anim-hover attr, new mechanism) for
        // the SAME hover animation — two overlapping mechanisms.
        // gen_simple_element() had already been cleaned up correctly
        // (attr only), so layout element must now match: attr is present,
        // add_js is NOT called anymore.
        let mut host = FakeHost::new();
        let mut el = make_element(vibao_ast::Tag::Khoi, vec![]);
        el.animation.hieu_ung_hover = Some("phong_to".to_string());
        let html = gen_layout_element(&el, "vb-box-1", &mut host);
        assert!(html.contains("data-vb-anim-hover=\"phong_to:300\""), "phải có attr hover: {}", html);
        assert!(host.js.is_empty(), "KHÔNG được add_js cho hover animation (cơ chế cũ đã bỏ): {:?}", host.js);
    }

    #[test]
    fn test_layout_element_scroll_animation_no_longer_double_applied() {
        let mut host = FakeHost::new();
        let mut el = make_element(vibao_ast::Tag::Khoi, vec![]);
        el.animation.hieu_ung_cuon = Some("truot_len".to_string());
        let html = gen_layout_element(&el, "vb-box-1", &mut host);
        assert!(html.contains("data-vb-anim-scroll=\"truot_len:600:0\""), "phải có attr scroll: {}", html);
        assert!(host.js.is_empty(), "KHÔNG được add_js cho scroll animation (cơ chế cũ đã bỏ): {:?}", host.js);
    }

    #[test]
    fn test_complex_component_placeholder() {
        let el = make_element(vibao_ast::Tag::Modal, vec![]);
        let html = gen_complex_component(&el, "vb-modal-1");
        assert_eq!(html, "<div id=\"vb-modal-1\" data-vb-component=\"modal\"><!-- modal --></div>");
    }

    #[test]
    fn test_gen_element_dispatches_to_complex_for_builtin() {
        let mut host = FakeHost::new();
        let el = make_element(vibao_ast::Tag::Tabs, vec![]);
        let html = gen_element(&el, &mut host);
        assert!(html.contains("data-vb-component=\"tabs\""));
    }

    #[test]
    fn test_gen_element_dispatches_to_layout_for_flex() {
        let mut host = FakeHost::new();
        let el = make_element(vibao_ast::Tag::Flex, vec![]);
        let html = gen_element(&el, &mut host);
        assert!(html.starts_with("<div id=\"vb-flex-1\""));
    }

    #[test]
    fn test_gen_anim_attrs_hover_only() {
        let anim = AnimationProps {
            hieu_ung: None,
            thoi_gian: Some(400),
            tre: None,
            lap: None,
            hieu_ung_hover: Some("phong_to".to_string()),
            hieu_ung_cuon: None,
        };
        let out = gen_anim_attrs(&anim);
        assert_eq!(out, "data-vb-anim-hover=\"phong_to:400\"");
    }

    #[test]
    fn test_gen_anim_attrs_hover_default_duration() {
        let anim = AnimationProps {
            hieu_ung_hover: Some("phong_to".to_string()),
            ..AnimationProps::default()
        };
        let out = gen_anim_attrs(&anim);
        assert_eq!(out, "data-vb-anim-hover=\"phong_to:300\"");
    }

    #[test]
    fn test_gen_anim_attrs_scroll_with_delay() {
        let anim = AnimationProps {
            hieu_ung_cuon: Some("truot_len".to_string()),
            thoi_gian: Some(500),
            tre: Some(150),
            ..AnimationProps::default()
        };
        let out = gen_anim_attrs(&anim);
        assert_eq!(out, "data-vb-anim-scroll=\"truot_len:500:150\"");
    }

    #[test]
    fn test_gen_anim_attrs_combines_load_and_hover() {
        let anim = AnimationProps {
            hieu_ung: Some("fade_in".to_string()),
            thoi_gian: Some(200),
            hieu_ung_hover: Some("phong_to".to_string()),
            ..AnimationProps::default()
        };
        let out = gen_anim_attrs(&anim);
        assert!(out.contains("data-vb-anim=\"fade_in\""));
        assert!(out.contains("data-vb-anim-hover=\"phong_to:200\""));
    }

    #[test]
    fn test_gen_anim_attrs_empty_without_hieu_ung() {
        let anim = AnimationProps::default();
        assert_eq!(gen_anim_attrs(&anim), "");
    }

    #[test]
    fn test_gen_anim_attrs_full() {
        let anim = AnimationProps {
            hieu_ung: Some("fade_in".to_string()),
            thoi_gian: Some(500),
            tre: Some(100),
            lap: Some(LapValue::MaiMai),
            hieu_ung_hover: None,
            hieu_ung_cuon: None,
        };
        let out = gen_anim_attrs(&anim);
        assert!(out.contains("data-vb-anim=\"fade_in\""));
        assert!(out.contains("data-vb-anim-duration=\"500\""));
        assert!(out.contains("data-vb-anim-delay=\"100\""));
        assert!(out.contains("data-vb-anim-repeat=\"infinite\""));
    }

    #[test]
    fn test_make_class_name() {
        assert_eq!(make_class_name("khoi", 3), "vb-box-3");
    }

    #[test]
    fn test_gen_loop_hoists_template_instead_of_inlining() {
        // FIXED BUG: previously <template> was embedded inline at the
        // gen_loop() call site — with nested loops, this caused the child
        // <template> to physically sit inside the parent <template>, getting
        // duplicated with the same ID when the runtime cloned parent template
        // content for each item (see the full explanation in add_hoisted_template). Now
        // <template> must be hoisted out through host.add_hoisted_template(),
        // and must NOT appear in the directly returned HTML string.
        let mut host = FakeHost::new();
        let node = LoopNode {
            kind: LoopKind::Range { from: 1, to: 3, var_name: "i".to_string() },
            body: vec![Child::Element(make_element(vibao_ast::Tag::Text, vec![]))],
            pos: p(),
        };
        let html = crate::codegen::control::gen_loop(&node, &mut host);

        assert!(!html.contains("<template"), "template không được nằm trong HTML trả về trực tiếp: {}", html);
        assert!(html.contains("data-vb-loop="), "phải có container binding: {}", html);
        assert_eq!(host.hoisted_templates.len(), 1, "phải có đúng 1 template được hoisted");
        assert!(host.hoisted_templates[0].contains("<template"));
    }

    #[test]
    fn test_nested_gen_loop_produces_two_separate_hoisted_templates_with_distinct_ids() {
        // Directly test the scenario that caused the real bug: the OUTER loop has
        // a body containing one INNER loop. Both templates must be hoisted
        // separately with 2 DIFFERENT IDs — no template may be
        // nested inside the other template.
        let mut host = FakeHost::new();
        let inner_loop = Child::Loop(Box::new(LoopNode {
            kind: LoopKind::Range { from: 1, to: 2, var_name: "i".to_string() },
            body: vec![Child::Element(make_element(vibao_ast::Tag::Text, vec![]))],
            pos: p(),
        }));
        let outer_node = LoopNode {
            kind: LoopKind::Range { from: 1, to: 3, var_name: "i".to_string() },
            body: vec![inner_loop],
            pos: p(),
        };
        let outer_html = crate::codegen::control::gen_loop(&outer_node, &mut host);

        // The OUTER loop returned HTML must not contain any <template>
        // — neither its own nor the inner loop's (both must have been
        // hoisted out).
        assert!(!outer_html.contains("<template"), "outer_html: {}", outer_html);

        // There must be EXACTLY 2 hoisted templates (1 for the outer loop, 1 for the
        // inner loop — hoisted while gen_children() processes the outer
        // loop body, BEFORE the outer loop hoists its own template
        // — order is unimportant, only that both are separate).
        assert_eq!(host.hoisted_templates.len(), 2, "phải có đúng 2 template tách biệt, không lồng nhau");

        // Confirm the 2 templates have DIFFERENT IDs (no duplicates).
        let extract_id = |html: &str| -> String {
            let start = html.find("id=\"").unwrap() + 4;
            let end = html[start..].find('"').unwrap() + start;
            html[start..end].to_string()
        };
        let id1 = extract_id(&host.hoisted_templates[0]);
        let id2 = extract_id(&host.hoisted_templates[1]);
        assert_ne!(id1, id2, "2 template hoisted không được trùng ID");

        // Final confirmation: each hoisted template contains exactly 1
        // opening <template>, with no other one nested inside it.
        for t in &host.hoisted_templates {
            assert_eq!(t.matches("<template").count(), 1, "mỗi template hoisted phải chỉ chứa đúng 1 <template>, không lồng cái khác: {}", t);
        }
    }
}
