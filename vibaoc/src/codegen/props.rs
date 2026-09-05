// ============================================================
// VIBAO COMPILER (Rust) — codegen/props.rs
// Expands a PropsMap (a list of Vietnamese key:value pairs on an
// Element) into CSS style, HTML attributes (attrs), and dynamic
// bindings. Equivalent to expandProps() + the ensurePx/
// expandSpacing/expandRadius/mapAlign*/... functions from
// 06-parser-expr.ts (its final section, used specifically for the
// SIMPLE ELEMENT — unlike layout.rs, which is used for the LAYOUT
// ELEMENT).
// ============================================================

use vibao_ast::{prop_spec, PropsMap, PropKey};
use crate::codegen::css::OrderedMap;
use crate::codegen::expr::{register_expr, resolve_value, ResolvedValue};
use crate::codegen::layout::{map_align_items, map_justify};

/// The result of expanding a regular (simple) Element's props - text,
/// button, image, input... Keeps 3 groups separate since each renders
/// into a different part of the HTML tag (style="...", individual
/// attrs, and data-vb-bind-* for parts needing dynamic runtime
/// updates). Uses OrderedMap (not a HashMap) to preserve declaration
/// order - matching how PropsMap/LayoutCss handle order throughout
/// codegen.
#[derive(Debug, Clone, Default)]
pub struct ExpandedProps {
    /// CSS key in camelCase (e.g. "backgroundColor") -> CSS value.
    /// camelCase is used at this intermediate step to match the
    /// original JS style object; css.rs converts it to kebab-case when
    /// printing the real CSS string.
    pub style: OrderedMap,
    /// A normal HTML attr (e.g. alt, placeholder, type, value...).
    pub attrs: OrderedMap,
    /// A CSS/attr key needing dynamic binding -> a JS expression for
    /// the runtime to track.
    ///
    /// IMPORTANT: this field is ONLY for CSS STYLE properties (element.rs
    /// always generates it as `data-vb-style-<kebab-case>`, read by
    /// `bind_all_styles` at runtime - set via CSSOM
    /// `style.set_property`). Do NOT use this field for a normal HTML
    /// attribute (type, value, placeholder, alt, required, disabled...)
    /// - use `dynamic_attrs` below instead, generating
    /// `data-vb-attr-*` (read by `bind_all_attrs`, set via
    /// `Element::set_attribute`). These 2 runtime mechanisms are NOT
    /// interchangeable: style.set_property("type", ...) does nothing at
    /// all (there's no CSS property named "type"), and
    /// set_attribute("color", ...) would create a meaningless HTML
    /// attribute instead of actually changing the real background color
    /// (which must go through CSSOM).
    pub dynamic: OrderedMap,
    /// An HTML ATTRIBUTE key (the REAL attribute name, e.g. "type"/
    /// "value"/"placeholder", NOT the original Vietnamese prop name
    /// like "loai"/"gia_tri") needing dynamic binding -> a JS
    /// expression. element.rs generates this field as
    /// `data-vb-attr-<name>` (read by `bind_all_attrs` at runtime -
    /// set via `Element::set_attribute`, CORRECT for normal HTML attrs
    /// like type/value/placeholder/alt/required/disabled, completely
    /// different from `dynamic` above, which is only for CSS style).
    pub dynamic_attrs: OrderedMap,
    /// Dynamic CSS class toggles: each entry is `expr_id:class_name`,
    /// consumed by runtime `data-vb-class`.
    pub class_bindings: Vec<String>,
    /// A 2-way model binding for an input when the `gia_tri` prop is a
    /// plain Variable. Only holds the state key name; element.rs
    /// generates `data-vb-model` from this field. Kept separate from
    /// `dynamic_attrs` since a model has a completely different runtime
    /// semantic from a 1-way attribute binding.
    pub model: Option<String>,
    /// A prop name that is NOT in ViBao's defined prop list (falls into
    /// the `other` branch below). Still written to `attrs` as before
    /// (no behavior change - allowing arbitrary HTML attrs like
    /// `data-*`/`aria-*` to pass through), but collected separately
    /// here so the caller (element.rs) can raise a build warning for
    /// cases that look like a mistyped prop name (e.g. "mua" instead of
    /// "mau").
    pub unknown_keys: Vec<String>,
    /// Build-time warnings raised DURING expansion - different from
    /// `unknown_keys` (an unrecognized prop name), this is a warning
    /// about a technical LIMITATION when a prop IS CORRECTLY RECOGNIZED
    /// by ViBao but is used with a DYNAMIC expression in a way that
    /// isn't currently supported (see "dam"/"nghieng"/"gach_chan" in
    /// the match below). Not sharing a Vec with `unknown_keys` since
    /// the 2 warning kinds have completely different causes and fixes -
    /// merging them would make it hard to write an accurate message for
    /// both cases in element.rs.
    pub warnings: Vec<String>,
}

/// Checks whether a surface name is a valid prop on a Simple Element.
///
/// `PropKey` is the semantic identity, and `PropSpec` is the metadata
/// source for applicability. No separate surface-string list is kept
/// here: this way a new English locale/alias can never get a false
/// warning just because a secondary table was forgotten. A layout-only
/// prop still resolves but is treated as `unknown` in the Simple
/// context, per the correct semantic boundary.
fn is_known_simple_prop(surface_name: &str) -> bool {
    crate::locale::resolve_prop_key(surface_name)
        .map(|key| prop_spec(key).applies_to_simple)
        .unwrap_or(false)
}

/// Expands a PropsMap into style/attrs/dynamic for a SIMPLE ELEMENT.
/// `tag` is needed since some props mean different things depending on
/// the tag type (e.g. "can" means text-align on a text tag but
/// justify-content on a layout tag - even though layout uses its own
/// layout.rs, "can" still appears here for simple elements containing
/// text).
pub fn expand_props(tag: &str, props: &PropsMap) -> ExpandedProps {
    let mut out = ExpandedProps::default();

    for (key, expr) in props {
        let resolved = resolve_value(expr);
        let is_dynamic = resolved.is_dynamic();
        let css_val = resolved.as_css();

        // The match key was changed from a raw String (`key.as_str()`)
        // to PropKey (the semantic identity, `vibao_ast::PropKey`) via
        // `locale::resolve_prop_key()` - editing expand_props()
        // directly, with no parallel v2 version, and full test
        // coverage added. The LOGIC INSIDE each branch stays 100%
        // UNCHANGED - only the match arm's PATTERN changed, not its
        // behavior. The `None` branch (an unrecognized prop,
        // resolve_prop_key returning None) KEEPS the old passthrough
        // behavior (the previous `other` branch) - this is the most
        // important thing to preserve: a prop not among the 57 built-in
        // PropKeys must still work as an arbitrary HTML attribute, not
        // an error.
        match crate::locale::resolve_prop_key(key.as_str()) {
            Some(PropKey::BackgroundColor) => {
                if is_dynamic {
                    out.dynamic.insert("backgroundColor".to_string(), register_expr(expr.clone()).to_string());
                } else {
                    out.style.insert("backgroundColor".to_string(), css_val);
                }
            }
            Some(PropKey::Color) => {
                if is_dynamic {
                    out.dynamic.insert("color".to_string(), register_expr(expr.clone()).to_string());
                } else {
                    out.style.insert("color".to_string(), css_val);
                }
            }
            Some(PropKey::BorderColor) => {
                if is_dynamic {
                    out.dynamic.insert("borderColor".to_string(), register_expr(expr.clone()).to_string());
                } else {
                    out.style.insert("borderColor".to_string(), css_val);
                }
            }
            // NOTE: the original TS version had a bug here -
            // width/height/max_rong never checked isDynamic, so
            // "width: $bien" always generated style["width"] = "px"
            // (empty + "px") with NO dynamic binding, losing width
            // entirely at runtime. This Rust version fixes it correctly
            // (routing into dynamic exactly like color/mau above), since
            // faithfully preserving known-wrong behavior wasn't
            // required.
            Some(PropKey::Width) => {
                if is_dynamic {
                    out.dynamic.insert("width".to_string(), register_expr(expr.clone()).to_string());
                } else {
                    out.style.insert("width".to_string(), size_or_px(&resolved, &css_val));
                }
            }
            Some(PropKey::Height) => {
                if is_dynamic {
                    out.dynamic.insert("height".to_string(), register_expr(expr.clone()).to_string());
                } else {
                    out.style.insert("height".to_string(), size_or_px(&resolved, &css_val));
                }
            }
            Some(PropKey::MaxWidth) => {
                if is_dynamic {
                    out.dynamic.insert("maxWidth".to_string(), register_expr(expr.clone()).to_string());
                } else {
                    out.style.insert("maxWidth".to_string(), size_or_px(&resolved, &css_val));
                }
            }
            // BUG ALREADY FIXED: the same class of bug found and fixed
            // for "width"/"height"/"max_rong" above (see the note there)
            // - the props below (radius, dem, le, vien, bong, overflow,
            // tang_z, hang, khoang_chu, bien_doi, font, gap) used to NOT
            // check `is_dynamic`, always falling straight into the
            // STATIC handling branch. With a DYNAMIC expression (e.g.
            // `dem: $khoang_cach`), `resolve_value()` returns
            // `ResolvedValue::Dynamic(_)`, making `css_val` (via
            // `as_css()`) an EMPTY STRING - the style got silently set
            // to an empty/"0" value, and the dynamic value never
            // actually bound into the DOM.
            //
            // The runtime
            // (`vibao-runtime/src/runtime/dom.rs::bind_all_styles`)
            // reads `data-vb-style-<css-prop>` by directly calling
            // `style.set_property(css_prop, v.to_string())` - it does
            // NOT automatically add a "px" unit or split multi-value
            // strings (unlike `ensure_px`/`expand_spacing`/`expand_radius`
            // at this BUILD TIME layer, which only apply to STATIC,
            // already-known values). So when dynamic: (a) props needing
            // only a single plain CSS value (borderRadius/boxShadow/
            // overflow/zIndex/lineHeight/textTransform/fontFamily) - bind
            // directly, no expand_*/ensure_px; (b) `vien` needs special
            // handling since it sets 2 styles at once (borderWidth
            // DYNAMIC + a default borderStyle "solid" ALWAYS STATIC,
            // independent of vien's value).
            Some(PropKey::Radius) => {
                if is_dynamic {
                    out.dynamic.insert("borderRadius".to_string(), register_expr(expr.clone()).to_string());
                } else {
                    out.style.insert("borderRadius".to_string(), expand_radius(&css_val));
                }
            }
            Some(PropKey::Padding) => {
                if is_dynamic {
                    out.dynamic.insert("padding".to_string(), register_expr(expr.clone()).to_string());
                } else {
                    out.style.insert("padding".to_string(), expand_spacing(&css_val));
                }
            }
            Some(PropKey::Margin) => {
                if is_dynamic {
                    out.dynamic.insert("margin".to_string(), register_expr(expr.clone()).to_string());
                } else {
                    out.style.insert("margin".to_string(), expand_spacing(&css_val));
                }
            }
            Some(PropKey::Border) => {
                // "borderStyle: solid" is a STATIC default needed for
                // the border to display (CSS: a border won't show
                // without border-style, no matter what border-width is)
                // - always set regardless of whether `vien` is static or
                // dynamic, must NOT be wrapped in an else branch like
                // borderWidth.
                if is_dynamic {
                    out.dynamic.insert("borderWidth".to_string(), register_expr(expr.clone()).to_string());
                } else {
                    out.style.insert("borderWidth".to_string(), ensure_px(&css_val));
                }
                out.style.entry_or_insert_with("borderStyle", || "solid".to_string());
            }
            Some(PropKey::BorderStyle) => {
                if is_dynamic {
                    out.dynamic.insert("borderStyle".to_string(), register_expr(expr.clone()).to_string());
                } else {
                    out.style.insert("borderStyle".to_string(), css_val);
                }
            }
            Some(PropKey::Shadow) => {
                if is_dynamic {
                    out.dynamic.insert("boxShadow".to_string(), register_expr(expr.clone()).to_string());
                } else {
                    out.style.insert("boxShadow".to_string(), css_val);
                }
            }
            Some(PropKey::Overflow) => {
                if is_dynamic {
                    out.dynamic.insert("overflow".to_string(), register_expr(expr.clone()).to_string());
                } else {
                    out.style.insert("overflow".to_string(), css_val);
                }
            }
            Some(PropKey::ZIndex) => {
                if is_dynamic {
                    out.dynamic.insert("zIndex".to_string(), register_expr(expr.clone()).to_string());
                } else {
                    out.style.insert("zIndex".to_string(), css_val);
                }
            }
            Some(PropKey::FontSize) => {
                if is_dynamic {
                    out.dynamic.insert("fontSize".to_string(), register_expr(expr.clone()).to_string());
                } else {
                    out.style.insert("fontSize".to_string(), ensure_px(&css_val));
                }
            }
            // A TECHNICAL LIMITATION (not a wrong-logic bug - a
            // deliberate decision after weighing tradeoffs):
            // "dam"/"nghieng"/"gach_chan" are BOOLEAN props (true/false),
            // but the corresponding CSS property needs 1 of 2 SPECIFIC
            // VALUES (fontWeight: "bold"/"normal", NOT "true"/"false").
            // The current `dynamic`/`data-vb-style-*` mechanism (see
            // bind_all_styles in the runtime) only sets `v.to_string()`
            // DIRECTLY as the CSS value - there's no "map a bool to one
            // of 2 strings" step in between. Supporting this properly
            // needs 1 of 2 approaches, BOTH BEYOND the scope of a plain
            // bug fix: (a) adding a ternary-style builtin function to the
            // language (affecting the parser + validator + docs, a
            // language design change), or (b) generating an anonymous CSS
            // class + using the bind_class mechanism (already exists, but
            // needs new class/CSS-rule generation infrastructure in
            // codegen). Current decision: KEEP the old static-only
            // behavior (setting nothing if dynamic, instead of setting
            // the wrong "true"/"false" value into CSS), with a build-time
            // WARNING so the dev knows immediately instead of discovering
            // it through DOM debugging. Different from "dem"/"radius"/...
            // above (those props could be cleanly fixed at the root, with
            // no new language feature needed).
            Some(PropKey::Bold) => {
                if is_dynamic {
                    out.warnings.push(format!(
                        "prop 'dam' với giá trị biểu thức động (biến/tính toán) chưa được hỗ trợ — \
                         cơ chế binding hiện tại không thể chuyển true/false thành \"bold\"/\"normal\" \
                         cho CSS font-weight. Style sẽ KHÔNG được áp dụng lúc runtime. Dùng giá trị \
                         tĩnh true/false, hoặc bọc trong 'neu/khong_thi' ở cấp element để chọn hẳn 1 \
                         nhánh có 'dam: true' / không có 'dam' lúc build.",
                    ));
                } else if css_val == "true" {
                    out.style.insert("fontWeight".to_string(), "bold".to_string());
                }
            }
            Some(PropKey::Italic) => {
                if is_dynamic {
                    out.warnings.push(format!(
                        "prop 'nghieng' với giá trị biểu thức động chưa được hỗ trợ — tương tự giới \
                         hạn của 'dam' (xem cảnh báo 'dam' để biết chi tiết + cách khắc phục).",
                    ));
                } else if css_val == "true" {
                    out.style.insert("fontStyle".to_string(), "italic".to_string());
                }
            }
            Some(PropKey::Underline) => {
                if is_dynamic {
                    out.warnings.push(format!(
                        "prop 'gach_chan' với giá trị biểu thức động chưa được hỗ trợ — tương tự \
                         giới hạn của 'dam' (xem cảnh báo 'dam' để biết chi tiết + cách khắc phục).",
                    ));
                } else if css_val == "true" {
                    out.style.insert("textDecoration".to_string(), "underline".to_string());
                }
            }
            Some(PropKey::Align) => {
                // A TECHNICAL LIMITATION similar to "dam"/"nghieng"/
                // "gach_chan" above: "can" needs to MAP a Vietnamese
                // value (e.g. "giua") -> a CSS keyword (e.g. "center")
                // via map_align()/map_justify() - this mapping step only
                // runs at BUILD TIME (the value known ahead of time).
                // With a dynamic expression, the runtime only sets
                // `v.to_string()` directly (which would be "giua", not
                // "center") - invalid CSS, not applied. Same reason as
                // "dam": a build-time warning instead of setting the
                // wrong value.
                const TEXT_TAGS: [&str; 6] = ["text", "h1", "h2", "h3", "p", "nhan"];
                if is_dynamic {
                    out.warnings.push(format!(
                        "prop 'can' với giá trị biểu thức động chưa được hỗ trợ — cơ chế binding \
                         hiện tại không thể chuyển tên căn chỉnh tiếng Việt (vd \"giua\") thành \
                         CSS keyword tương ứng (vd \"center\") lúc runtime. Style sẽ KHÔNG được áp \
                         dụng. Dùng giá trị tĩnh, hoặc bọc trong 'neu/khong_thi' ở cấp element.",
                    ));
                } else if TEXT_TAGS.contains(&tag) {
                    out.style.insert("textAlign".to_string(), map_align(&css_val));
                } else {
                    out.style.insert("justifyContent".to_string(), map_justify(&css_val));
                }
            }
            Some(PropKey::LineHeight) => {
                if is_dynamic {
                    out.dynamic.insert("lineHeight".to_string(), register_expr(expr.clone()).to_string());
                } else {
                    out.style.insert("lineHeight".to_string(), css_val);
                }
            }
            Some(PropKey::LetterSpacing) => {
                if is_dynamic {
                    out.dynamic.insert("letterSpacing".to_string(), register_expr(expr.clone()).to_string());
                } else {
                    out.style.insert("letterSpacing".to_string(), ensure_px(&css_val));
                }
            }
            Some(PropKey::TextTransform) => {
                if is_dynamic {
                    out.dynamic.insert("textTransform".to_string(), register_expr(expr.clone()).to_string());
                } else {
                    out.style.insert("textTransform".to_string(), css_val);
                }
            }
            Some(PropKey::FontFamily) => {
                if is_dynamic {
                    out.dynamic.insert("fontFamily".to_string(), register_expr(expr.clone()).to_string());
                } else {
                    out.style.insert("fontFamily".to_string(), css_val);
                }
            }
            Some(PropKey::Direction) => {
                // The same kind of limitation as "can": the valid values
                // are "row"/"column" (matching real CSS names directly,
                // NOT translated to Vietnamese - see docs/VIBAO_SPEC.md
                // around line 254), so setting directly would in theory
                // be fine, BUT the current branch TREATS EVERY VALUE
                // OTHER THAN "column" as "row" (including
                // empty/unexpected values) - with a dynamic expression,
                // if the runtime value isn't exactly "column" (e.g. Null
                // before it's assigned, or a typo), it SILENTLY falls
                // back to "row" instead of erroring. Setting the dynamic
                // expr directly into CSS (bind_all_styles) loses this
                // exact fallback behavior (every value passes through
                // verbatim - "column" or "row" or any other string) -
                // slightly different from static behavior but still
                // SAFER (CSS itself ignores an invalid flex-direction
                // value), so this could be cleanly fixed here without
                // needing a warning.
                if is_dynamic {
                    out.dynamic.insert("flexDirection".to_string(), register_expr(expr.clone()).to_string());
                } else {
                    out.style.insert(
                        "flexDirection".to_string(),
                        if css_val == "column" { "column".to_string() } else { "row".to_string() },
                    );
                }
            }
            Some(PropKey::Gap) => {
                if is_dynamic {
                    out.dynamic.insert("gap".to_string(), register_expr(expr.clone()).to_string());
                } else {
                    out.style.insert("gap".to_string(), ensure_px(&css_val));
                }
            }
            Some(PropKey::AlignItems) => {
                // The same limitation as "can": map_align_items()
                // translates Vietnamese ("giua", "dau", "cuoi"...) into
                // a CSS keyword - can't be applied at runtime with the
                // current binding mechanism (see the full explanation in
                // the "can" warning above).
                if is_dynamic {
                    out.warnings.push(format!(
                        "prop 'doc' với giá trị biểu thức động chưa được hỗ trợ — cùng giới hạn với \
                         prop 'can' (cần map tên căn chỉnh tiếng Việt sang CSS keyword lúc build, \
                         không thực hiện được lúc runtime). Style sẽ KHÔNG được áp dụng.",
                    ));
                } else {
                    out.style.insert("alignItems".to_string(), map_align_items(&css_val));
                }
            }
            Some(PropKey::Wrap) => {
                // The same limitation as "dam"/"nghieng"/"gach_chan": a
                // boolean needing to map to a fixed CSS value ("wrap"),
                // can't be set directly at runtime.
                if is_dynamic {
                    out.warnings.push(format!(
                        "prop 'boc' với giá trị biểu thức động chưa được hỗ trợ — cùng giới hạn với \
                         prop 'dam' (xem cảnh báo 'dam' để biết chi tiết + cách khắc phục).",
                    ));
                } else if css_val == "true" {
                    out.style.insert("flexWrap".to_string(), "wrap".to_string());
                }
            }
            Some(PropKey::Fit) => {
                if is_dynamic {
                    out.dynamic.insert("objectFit".to_string(), register_expr(expr.clone()).to_string());
                } else {
                    out.style.insert("objectFit".to_string(), css_val);
                }
            }
            Some(PropKey::Source) => {
                // A NEW prop (not a bug fix - an addition per a feature
                // request): image/video used to be COMPLETELY MISSING an
                // official prop for "image/video source" - an
                // unrecognized prop (e.g. typing "src" directly) still
                // worked via the generic pass-through branch (this
                // match's final "other" branch) but with an
                // "unknown_keys" warning, confusing a beginner even
                // though the image still displayed correctly. "nguon" is
                // now an OFFICIAL prop, mapping directly to the HTML
                // "src" attribute.
                if is_dynamic {
                    out.dynamic_attrs.insert("src".to_string(), register_expr(expr.clone()).to_string());
                } else {
                    out.attrs.insert("src".to_string(), css_val);
                }
            }
            Some(PropKey::Alt) => {
                if is_dynamic {
                    out.dynamic_attrs.insert("alt".to_string(), register_expr(expr.clone()).to_string());
                } else {
                    out.attrs.insert("alt".to_string(), css_val);
                }
            }
            Some(PropKey::LazyLoad) => {
                // BUG ALREADY FIXED: this prop used to NOT check
                // is_dynamic (unlike "bat_buoc"/"vo_hieu", the same
                // boolean-like group already reviewed and given a
                // warning earlier) - if "tai_cham: $lazy" (a dynamic
                // expression), css_val was always an EMPTY string (see
                // ResolvedValue::Dynamic::as_css()), never equal to
                // "true", so `loading="lazy"` SILENTLY never got set -
                // nothing set, no warning, no way for the dev to know
                // the build ignored their intent. Now applies the EXACT
                // same pattern already used for "bat_buoc"/"vo_hieu": a
                // clear warning when dynamic, preserving the old static
                // behavior when not dynamic.
                if is_dynamic {
                    out.warnings.push(format!(
                        "prop 'tai_cham' với giá trị biểu thức động chưa được hỗ trợ — thuộc tính \
                         'loading=\"lazy\"' cần biết trước lúc build (trình duyệt chỉ đọc 1 lần khi \
                         element được tạo, không phản ứng lại nếu attribute đổi sau đó). Dùng giá trị \
                         tĩnh (true/false), hoặc bọc trong 'neu/khong_thi' ở cấp element.",
                    ));
                } else if css_val == "true" {
                    out.attrs.insert("loading".to_string(), "lazy".to_string());
                }
            }
            Some(PropKey::Type) => {
                if is_dynamic {
                    out.dynamic_attrs.insert("type".to_string(), register_expr(expr.clone()).to_string());
                } else {
                    out.attrs.insert("type".to_string(), css_val);
                }
            }
            Some(PropKey::Placeholder) => {
                if is_dynamic {
                    out.dynamic_attrs.insert("placeholder".to_string(), register_expr(expr.clone()).to_string());
                } else {
                    out.attrs.insert("placeholder".to_string(), css_val);
                }
            }
            Some(PropKey::To) => {
                // A dedicated prop for link/lien_ket: "den" (to) - the
                // navigation target. Generates data-vb-link so router.rs
                // (runtime) intercepts the click itself and does a real
                // SPA navigation (History API, no reload) - see
                // runtime/router.rs::setup_link_interception. Also sets
                // a normal href so: (1) hovering the link shows the
                // correct URL, (2) it still works if JS/WASM fails or is
                // disabled (progressive enhancement - clicking still
                // reaches the correct page, just via a full reload
                // instead of SPA navigation).
                //
                // A LIMITATION (documented in docs/VIBAO_SPEC.md - see
                // the "den" prop section): only works correctly with a
                // STATIC value. A dynamic route for a link COULD use
                // dynamic_attrs (like alt/loai above) to bind "href"
                // dynamically - but "data-vb-link" (the flag marking it
                // for router click interception) must SIMULTANEOUSLY be
                // a KNOWN STATIC VALUE (the router needs to know right at
                // bind_events time whether this link participates in SPA
                // interception at all - see setup_link_interception),
                // and "does it participate in interception" can't itself
                // depend on the runtime. Since 2 different reasons stack
                // up here (not simply a missing binding mechanism), the
                // recommendation to use button + dieu_huong() for a
                // state-dependent route stands, and dynamic_attrs is NOT
                // extended for "den" specifically in this round.
                out.attrs.insert("href".to_string(), css_val.clone());
                out.attrs.insert("data-vb-link".to_string(), css_val);
            }
            Some(PropKey::Required) => {
                // A LIMITATION: "bat_buoc"/"vo_hieu" map to a real HTML
                // BOOLEAN ATTRIBUTE (required/disabled) - for a boolean
                // attribute, the HTML STANDARD treats an attribute's
                // "PRESENCE" (regardless of its string value, even the
                // literal "false") as true. The current `bind_all_attrs`
                // (runtime) only removes an attribute when the value is
                // VbValue::Null - there's no separate branch for "the
                // value evaluated to false but is NOT Null". If bound
                // dynamically here, "vo_hieu: $co_loi" when $co_loi=false
                // would set_attribute("disabled", "false") - HTML would
                // STILL treat this input as disabled=true (completely
                // wrong intent). A proper fix needs a separate "boolean
                // attribute" branch in bind_all_attrs (outside this pure
                // codegen file's scope), so this warns for now instead of
                // setting it wrong.
                if is_dynamic {
                    out.warnings.push(format!(
                        "prop 'bat_buoc' với giá trị biểu thức động chưa được hỗ trợ — HTML boolean \
                         attribute (required) coi bất kỳ giá trị chuỗi nào (kể cả \"false\") là bật, \
                         nên cơ chế binding hiện tại (set_attribute thẳng) sẽ set sai khi giá trị là \
                         false. Dùng giá trị tĩnh, hoặc bọc trong 'neu/khong_thi' ở cấp element.",
                    ));
                } else if css_val == "true" {
                    out.attrs.insert("required".to_string(), "true".to_string());
                }
            }
            Some(PropKey::Disabled) => {
                // Same limitation as "bat_buoc" above - see that warning
                // for the full details.
                if is_dynamic {
                    out.warnings.push(format!(
                        "prop 'vo_hieu' với giá trị biểu thức động chưa được hỗ trợ — cùng giới hạn \
                         với prop 'bat_buoc' (xem cảnh báo 'bat_buoc' để biết chi tiết + cách khắc phục).",
                    ));
                } else if css_val == "true" {
                    out.attrs.insert("disabled".to_string(), "true".to_string());
                }
            }
            Some(PropKey::ClassBinding) => {
                match expr {
                    vibao_ast::Expr::Object(fields, _) => {
                        for (class_name, condition) in fields {
                            if class_name.trim().is_empty() {
                                out.warnings.push("lop: tên class không được để trống".to_string());
                                continue;
                            }
                            // The runtime `data-vb-class` currently uses
                            // the CSV format `expr_id:class_name`. These 2
                            // delimiters are an internal contract; letting
                            // them leak into a class name would make the
                            // runtime split expr_id/class incorrectly.
                            if class_name.contains(',') || class_name.contains(':') {
                                out.warnings.push(format!(
                                    "lop: tên class '{}' không được chứa ',' hoặc ':' (dùng làm delimiter nội bộ)",
                                    class_name
                                ));
                                continue;
                            }
                            let expr_id = register_expr(condition.clone());
                            out.class_bindings.push(format!("{}:{}", expr_id, class_name));
                        }
                    }
                    _ => {
                        out.warnings.push(
                            "lop: cần một object, ví dụ lop: { active: $dang_chon }".to_string(),
                        );
                    }
                }
            }
            Some(PropKey::Value) => {
                // `gia_tri: $ten` on input/textarea/select is a 2-way
                // binding: the runtime already has a dedicated semantic
                // for `data-vb-model` (state -> value + input -> state),
                // so it CANNOT be shared with the 1-way
                // `data-vb-attr-value`. Only upgraded when the expression
                // is a plain Variable, matching the runtime model's
                // limitation (a single global state key, not an
                // arbitrary expression).
                //
                // For other tags or a more complex dynamic expression,
                // dynamic_attrs is kept as before to preserve the old
                // attribute-binding semantics.
                let is_plain_variable = matches!(expr, vibao_ast::Expr::Variable(_, _));
                if matches!(tag, "input" | "textarea" | "select") && is_plain_variable {
                    if let vibao_ast::Expr::Variable(name, _) = expr {
                        out.model = Some(name.clone());
                    }
                } else if is_dynamic {
                    out.dynamic_attrs.insert("value".to_string(), register_expr(expr.clone()).to_string());
                } else {
                    out.attrs.insert("value".to_string(), css_val);
                }
            }
            // The animation props are handled separately in
            // animation.rs (reading directly from AnimationProps on
            // Element, not through this shared PropsMap) - skipped here
            // to avoid generating a redundant attr by mistake.
            Some(PropKey::Animation) | Some(PropKey::Duration) | Some(PropKey::Delay)
            | Some(PropKey::Repeat) | Some(PropKey::HoverAnimation) | Some(PropKey::ScrollAnimation) => {}
            Some(PropKey::Content) => {
                if is_dynamic {
                    out.dynamic.insert("noi_dung".to_string(), register_expr(expr.clone()).to_string());
                } else {
                    out.attrs.insert("noi_dung".to_string(), css_val);
                }
            }
            // Every REMAINING PropKey (Hidden, MinWidth, MaxHeight,
            // MinHeight, RowGap, ColumnGap, Columns, Rows, TranslateX,
            // TranslateY, Position, Offset - the 12 LAYOUT-ONLY props,
            // see PropSpec) is NOT recognized by `expand_props()`
            // (Simple Element) - passes through exactly like a
            // completely unrecognized prop (the `None` branch below),
            // matching the OLD behavior (the old String match also had
            // NO separate branch for these names in props.rs, they
            // always fell into the "other" branch).
            Some(_) => {
                let key_str = key.as_str();
                if !is_known_simple_prop(key_str) {
                    out.unknown_keys.push(key_str.to_string());
                }
                if is_dynamic {
                    out.dynamic.insert(key_str.to_string(), register_expr(expr.clone()).to_string());
                } else {
                    out.attrs.insert(key_str.to_string(), css_val);
                }
            }
            None => {
                let key_str = key.as_str();
                if !is_known_simple_prop(key_str) {
                    out.unknown_keys.push(key_str.to_string());
                }
                if is_dynamic {
                    out.dynamic.insert(key_str.to_string(), register_expr(expr.clone()).to_string());
                } else {
                    out.attrs.insert(key_str.to_string(), css_val);
                }
            }
        }
    }

    out
}

/// For a size prop (width/height/max_rong), if ResolvedValue is
/// already Size (already carrying a CSS unit from the literal), it's
/// used as-is; otherwise (a static string that isn't a number, rare)
/// "px" is forced onto it - matching `resolved.kind === "size" ?
/// resolved.css : cssVal + "px"` in the old TS version.
fn size_or_px(resolved: &ResolvedValue, css_val: &str) -> String {
    match resolved {
        ResolvedValue::Size(s) => s.clone(),
        _ => format!("{}px", css_val),
    }
}

// ════════════════════════════════════════════════════════════
// CSS VALUE UTILITIES (ensurePx, spacing, radius, align maps)
// ════════════════════════════════════════════════════════════

/// Adds a "px" suffix to a plain numeric value if it doesn't already
/// have a unit - equivalent to ensurePx() in the old TS version. An
/// empty string returns "0px".
pub fn ensure_px(val: &str) -> String {
    if val.is_empty() {
        return "0px".to_string();
    }
    if is_plain_number(val) {
        format!("{}px", val)
    } else {
        val.to_string()
    }
}

/// Applies ensure_px() to each whitespace-separated part - used for
/// props that can accept multiple border/margin values in CSS
/// shorthand form (e.g. "dem" accepting "16 24" meaning
/// padding: 16px 24px).
pub fn expand_spacing(val: &str) -> String {
    if val.is_empty() {
        return "0".to_string();
    }
    val.split_whitespace().map(ensure_px).collect::<Vec<_>>().join(" ")
}

/// expandRadius shares its logic with expand_spacing in the old TS
/// version (it's just an alias) - kept as its own function name to
/// match the calling semantics at its use sites.
pub fn expand_radius(val: &str) -> String {
    expand_spacing(val)
}

/// Matches the original regex `/^-?[\d.]+$/` exactly: at most 1 "-",
/// ONLY in the first position, with the rest being all digits/dots. (A
/// bug already fixed compared to the first draft: using a plain .all()
/// would have allowed "-" in the middle of a string like "1-2", not
/// matching the original regex's meaning.)
fn is_plain_number(val: &str) -> bool {
    if val.is_empty() {
        return false;
    }
    let rest = val.strip_prefix('-').unwrap_or(val);
    !rest.is_empty()
        && rest.chars().any(|c| c.is_ascii_digit())
        && rest.chars().filter(|&c| c == '.').count() <= 1
        && rest.chars().all(|c| c.is_ascii_digit() || c == '.')
}

/// Maps a Vietnamese alignment name (used for text-align) to a CSS value.
pub fn map_align(val: &str) -> String {
    match val {
        "trai" => "left".to_string(),
        "phai" => "right".to_string(),
        "giua" => "center".to_string(),
        "deu" => "justify".to_string(),
        other => other.to_string(),
    }
}

// map_justify()/map_align_items() are now UNIFIED into layout.rs
// (imported at the top of this file) - there used to be 2 independent
// versions with the same name here and in layout.rs, with drifted
// behavior: the layout.rs version was already fixed to accept
// "giua"/"deu" (Vietnamese), while the version here (now removed)
// still had the old bug, only accepting plain English CSS keywords -
// causing `text(can: giua)`/`button(doc: giua)` to generate invalid
// CSS while `khoi(can: giua)` (via layout.rs) worked correctly.

// ════════════════════════════════════════════════════════════
// UNIT TESTS
// ════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use vibao_ast::{Expr, Pos};

    fn p() -> Pos {
        Pos { line: 1, column: 1 }
    }

    #[test]
    fn test_color_prop_static() {
        let props: PropsMap = vec![("mau".to_string(), Expr::Literal(vibao_ast::LiteralValue::Color("#FF0000".to_string()), p()))];
        let out = expand_props("text", &props);
        assert_eq!(out.style.get("color"), Some(&"#FF0000".to_string()));
    }

    #[test]
    fn test_color_prop_alias_mau_chu_same_as_mau() {
        // Migrating to PropKey unified "mau" and "mau_chu" into the SAME
        // PropKey::Color - this test confirms the migration did NOT lose
        // the "mau_chu" alias (a second name, equivalent to "mau",
        // already existing before - see PropKey/PropSpec).
        let props: PropsMap = vec![("mau_chu".to_string(), Expr::Literal(vibao_ast::LiteralValue::Color("#00FF00".to_string()), p()))];
        let out = expand_props("text", &props);
        assert_eq!(out.style.get("color"), Some(&"#00FF00".to_string()));
        assert!(out.unknown_keys.is_empty(), "'mau_chu' không được coi là unknown key");
    }

    #[test]
    fn test_color_prop_dynamic_variable() {
        // BUG ALREADY FIXED: the value stored in `dynamic` used to be a
        // RAW JS STRING ("__s.mau_chinh") - but the WASM runtime
        // (dom.rs) can only read a NUMERIC ID pointing into the expr
        // registry, not a JS string. The value must now be a valid
        // number string (parseable as usize) - not asserting a specific
        // hardcoded value since register_expr() uses thread_local, and
        // the real ID depends on the order other tests ran before it on
        // the same thread.
        let props: PropsMap = vec![("mau".to_string(), Expr::Variable("mau_chinh".to_string(), p()))];
        let out = expand_props("text", &props);
        let value = out.dynamic.get("color").expect("prop 'mau' dynamic phải có trong out.dynamic");
        assert!(value.parse::<usize>().is_ok(), "giá trị dynamic phải là ID số hợp lệ (dom.rs::parse_expr_id yêu cầu), nhận được: {}", value);
        assert!(!out.style.contains_key("color"));
    }

    #[test]
    fn test_width_uses_number_unit() {
        let props: PropsMap = vec![("rong".to_string(), Expr::literal_num(200.0, p()))];
        let out = expand_props("khoi", &props);
        assert_eq!(out.style.get("width"), Some(&"200px".to_string()));
    }

    #[test]
    fn test_width_dynamic_variable_goes_to_dynamic_not_broken_style() {
        // A regression test for the fixed bug: the original TS version
        // never handled a dynamic width, always generating
        // style["width"] = "px" (broken). This Rust version must route
        // it into dynamic like color/mau, never setting a garbage
        // style. The value must be a valid numeric ID - see the
        // explanation in test_color_prop_dynamic_variable above.
        let props: PropsMap = vec![("rong".to_string(), Expr::Variable("w".to_string(), p()))];
        let out = expand_props("khoi", &props);
        let value = out.dynamic.get("width").expect("prop 'rong' dynamic phải có trong out.dynamic");
        assert!(value.parse::<usize>().is_ok(), "giá trị dynamic phải là ID số hợp lệ, nhận được: {}", value);
        assert!(!out.style.contains_key("width"));
    }

    #[test]
    fn test_unknown_prop_key_collected_for_warning() {
        // BUG ALREADY FIXED: a mistyped prop name (e.g. "mua" instead of
        // "mau") used to be silently swallowed as an arbitrary HTML
        // attr, with no way to know it was a typo. unknown_keys must now
        // contain the unrecognized prop name.
        let props: PropsMap = vec![("mua".to_string(), Expr::Literal(vibao_ast::LiteralValue::Str("do".to_string()), p()))];
        let out = expand_props("text", &props);
        assert_eq!(out.unknown_keys, vec!["mua".to_string()]);
        // The old behavior (passthrough into attrs) is preserved - not broken.
        assert_eq!(out.attrs.get("mua"), Some(&"do".to_string()));
    }

    #[test]
    fn test_known_prop_key_not_flagged_as_unknown() {
        let props: PropsMap = vec![("mau".to_string(), Expr::Literal(vibao_ast::LiteralValue::Color("#FF0000".to_string()), p()))];
        let out = expand_props("text", &props);
        assert!(out.unknown_keys.is_empty());
    }

    #[test]
    fn test_multiple_unknown_keys_all_collected() {
        let props: PropsMap = vec![
            ("mua".to_string(), Expr::Literal(vibao_ast::LiteralValue::Str("x".to_string()), p())),
            ("cann".to_string(), Expr::Literal(vibao_ast::LiteralValue::Str("y".to_string()), p())),
        ];
        let out = expand_props("text", &props);
        assert_eq!(out.unknown_keys.len(), 2);
        assert!(out.unknown_keys.contains(&"mua".to_string()));
        assert!(out.unknown_keys.contains(&"cann".to_string()));
    }


    #[test]
    fn test_width_percent_unit_preserved() {
        let props: PropsMap = vec![(
            "rong".to_string(),
            Expr::literal_num_with_unit(50.0, Some("%".to_string()), p()),
        )];
        let out = expand_props("khoi", &props);
        assert_eq!(out.style.get("width"), Some(&"50%".to_string()));
    }

    #[test]
    fn test_bold_flag_true() {
        let props: PropsMap = vec![("dam".to_string(), Expr::literal_bool(true, p()))];
        let out = expand_props("text", &props);
        assert_eq!(out.style.get("fontWeight"), Some(&"bold".to_string()));
    }

    #[test]
    fn test_bold_flag_false_not_set() {
        let props: PropsMap = vec![("dam".to_string(), Expr::literal_bool(false, p()))];
        let out = expand_props("text", &props);
        assert!(!out.style.contains_key("fontWeight"));
    }

    #[test]
    fn test_can_text_align_on_text_tag() {
        let props: PropsMap = vec![("can".to_string(), Expr::literal_str("giua", p()))];
        let out = expand_props("text", &props);
        assert_eq!(out.style.get("textAlign"), Some(&"center".to_string()));
    }

    #[test]
    fn test_can_justify_content_on_non_text_tag() {
        let props: PropsMap = vec![("can".to_string(), Expr::literal_str("center", p()))];
        let out = expand_props("button", &props);
        assert_eq!(out.style.get("justifyContent"), Some(&"center".to_string()));
    }

    #[test]
    fn test_can_vietnamese_value_on_simple_element_no_longer_broken() {
        // BUG ALREADY FIXED (BUG-16 in the audit): map_justify() in
        // THIS FILE used to have its own version, drifted from
        // layout.rs - only accepting English CSS keywords, so
        // "can: giua" on a simple element (unlike a layout tag like
        // khoi/flex) generated INVALID CSS "justify-content:giua",
        // silently ignored by the browser. Now uses map_justify() shared
        // from layout.rs (which already accepted "giua"->"center"), so
        // this must match correctly.
        let props: PropsMap = vec![("can".to_string(), Expr::literal_str("giua", p()))];
        let out = expand_props("button", &props);
        assert_eq!(out.style.get("justifyContent"), Some(&"center".to_string()));
    }

    #[test]
    fn test_doc_vietnamese_deu_on_simple_element_no_longer_broken() {
        // The same bug, same fix, but for map_align_items()/the "doc"
        // prop - "deu" used to pass through as "align-items:deu"
        // (invalid) on a simple element even though layout.rs had
        // already been fixed.
        let props: PropsMap = vec![("doc".to_string(), Expr::literal_str("deu", p()))];
        let out = expand_props("button", &props);
        assert_eq!(out.style.get("alignItems"), Some(&"stretch".to_string()));
    }

    #[test]
    fn test_unknown_prop_goes_to_attrs() {
        let props: PropsMap = vec![("data_custom".to_string(), Expr::literal_str("xyz", p()))];
        let out = expand_props("khoi", &props);
        assert_eq!(out.attrs.get("data_custom"), Some(&"xyz".to_string()));
    }

    #[test]
    fn test_den_prop_emits_href_and_data_vb_link() {
        let props: PropsMap = vec![("den".to_string(), Expr::literal_str("/gioi-thieu", p()))];
        let out = expand_props("link", &props);
        assert_eq!(out.attrs.get("href"), Some(&"/gioi-thieu".to_string()));
        assert_eq!(out.attrs.get("data-vb-link"), Some(&"/gioi-thieu".to_string()));
    }

    #[test]
    fn test_animation_props_skipped() {
        let props: PropsMap = vec![("hieu_ung".to_string(), Expr::literal_str("fade_in", p()))];
        let out = expand_props("khoi", &props);
        assert!(out.attrs.is_empty());
        assert!(out.style.is_empty());
        assert!(out.dynamic.is_empty());
    }

    #[test]
    fn test_ensure_px_plain_number() {
        assert_eq!(ensure_px("16"), "16px");
        assert_eq!(ensure_px(""), "0px");
        assert_eq!(ensure_px("50%"), "50%");
    }

    #[test]
    fn test_ensure_px_rejects_dash_in_middle() {
        // A regression test: '-' is only allowed at the start of the string.
        assert_eq!(ensure_px("1-2"), "1-2");
        assert_eq!(ensure_px("-16"), "-16px");
    }

    #[test]
    fn test_ensure_px_rejects_malformed_decimals() {
        assert_eq!(ensure_px("1.25"), "1.25px");
        assert_eq!(ensure_px(".5"), ".5px");
        assert_eq!(ensure_px("1.2.3"), "1.2.3");
        assert_eq!(ensure_px("."), ".");
        assert_eq!(ensure_px(".."), "..");
    }

    #[test]
    fn test_expand_spacing_multiple_values() {
        assert_eq!(expand_spacing("16 24"), "16px 24px");
    }

    // ── Tests for the props with a newly added is_dynamic check (a
    // systemic bug: 12+ CSS props not checking is_dynamic like
    // width/height, see the "BUG ALREADY FIXED" note in expand_props
    // above) ──

    #[test]
    fn test_dem_dynamic_goes_to_dynamic_not_zero() {
        // Before the fix: "dem: $khoang_cach" would set
        // style["padding"] = expand_spacing("") = "0" silently
        // (completely losing the dynamic binding). It must now go into
        // `dynamic`, not touching `style`.
        let props: PropsMap = vec![("dem".to_string(), Expr::Variable("khoang_cach".to_string(), p()))];
        let out = expand_props("khoi", &props);
        let value = out.dynamic.get("padding").expect("prop 'dem' dynamic phải có trong out.dynamic");
        assert!(value.parse::<usize>().is_ok());
        assert!(!out.style.contains_key("padding"));
    }

    #[test]
    fn test_radius_dynamic_goes_to_dynamic() {
        let props: PropsMap = vec![("radius".to_string(), Expr::Variable("r".to_string(), p()))];
        let out = expand_props("khoi", &props);
        assert!(out.dynamic.contains_key("borderRadius"));
        assert!(!out.style.contains_key("borderRadius"));
    }

    #[test]
    fn test_vien_dynamic_still_sets_static_border_style() {
        // A dynamic "vien": borderWidth must go into `dynamic`, BUT
        // borderStyle (defaulting to "solid") must still be set
        // STATICALLY - otherwise the border won't show even with a
        // dynamic borderWidth > 0 (CSS needs border-style for a border
        // to display, regardless of border-width).
        let props: PropsMap = vec![("vien".to_string(), Expr::Variable("do_day".to_string(), p()))];
        let out = expand_props("khoi", &props);
        assert!(out.dynamic.contains_key("borderWidth"));
        assert_eq!(out.style.get("borderStyle"), Some(&"solid".to_string()));
    }

    #[test]
    fn test_co_font_size_dynamic() {
        let props: PropsMap = vec![("co".to_string(), Expr::Variable("kich_co".to_string(), p()))];
        let out = expand_props("text", &props);
        assert!(out.dynamic.contains_key("fontSize"));
        assert!(!out.style.contains_key("fontSize"));
    }

    // ── Tests for dynamic_attrs (a dynamic HTML attribute - different
    // from `dynamic`, which is only for CSS style, see the
    // ExpandedProps::dynamic_attrs note) ──

    #[test]
    fn test_loai_dynamic_goes_to_dynamic_attrs_with_html_attr_name() {
        // "loai" (the Vietnamese prop name) must map to "type" (the
        // real HTML attribute name) EVEN when dynamic - the key in
        // dynamic_attrs must be "type", not "loai".
        let props: PropsMap = vec![("loai".to_string(), Expr::Variable("kieu_input".to_string(), p()))];
        let out = expand_props("input", &props);
        let value = out.dynamic_attrs.get("type").expect("phải có key 'type' (không phải 'loai') trong dynamic_attrs");
        assert!(value.parse::<usize>().is_ok());
        assert!(!out.attrs.contains_key("type"));
        // Confirms it doesn't leak into `dynamic` (which is for CSS style) -
        // "type" isn't a valid CSS property; setting it there by
        // mistake would generate data-vb-style-type (meaningless, the
        // runtime does nothing with it).
        assert!(!out.dynamic.contains_key("type"));
        assert!(!out.dynamic.contains_key("loai"));
    }

    #[test]
    fn test_class_binding_object_registers_each_condition() {
        let props: PropsMap = vec![(
            "lop".to_string(),
            Expr::Object(
                vec![
                    ("active".to_string(), Expr::Variable("dang_chon".to_string(), p())),
                    ("muted".to_string(), Expr::Variable("im_lang".to_string(), p())),
                ],
                p(),
            ),
        )];
        let out = expand_props("button", &props);
        assert_eq!(out.class_bindings.len(), 2);
        assert!(out.class_bindings.iter().all(|v| v.contains(':')));
        assert!(out.class_bindings[0].ends_with(":active"));
        assert!(out.class_bindings[1].ends_with(":muted"));
        assert!(out.warnings.is_empty());
    }

    #[test]
    fn test_class_binding_empty_name_warns() {
        let props: PropsMap = vec![(
            "lop".to_string(),
            Expr::Object(vec![("".to_string(), Expr::Variable("x".to_string(), p()))], p()),
        )];
        let out = expand_props("button", &props);
        assert!(out.class_bindings.is_empty());
        assert_eq!(out.warnings.len(), 1);
        assert!(out.warnings[0].contains("không được để trống"));
    }

    #[test]
    fn test_class_binding_non_object_warns() {
        let props: PropsMap = vec![("lop".to_string(), Expr::Variable("class_map".to_string(), p()))];
        let out = expand_props("button", &props);
        assert!(out.class_bindings.is_empty());
        assert_eq!(out.warnings.len(), 1);
        assert!(out.warnings[0].contains("cần một object"));
    }

    #[test]
    fn test_class_binding_delimiters_warn_and_do_not_register() {
        let props: PropsMap = vec![(
            "lop".to_string(),
            Expr::Object(
                vec![
                    ("hover:bg-blue-500".to_string(), Expr::Variable("hover".to_string(), p())),
                    ("bad,name".to_string(), Expr::Variable("bad".to_string(), p())),
                ],
                p(),
            ),
        )];
        let out = expand_props("button", &props);
        assert!(out.class_bindings.is_empty());
        assert_eq!(out.warnings.len(), 2);
        assert!(out.warnings.iter().all(|w| w.contains("delimiter nội bộ")));
    }

    #[test]
    fn test_gia_tri_variable_on_input_uses_two_way_model() {
        let props: PropsMap = vec![("gia_tri".to_string(), Expr::Variable("v".to_string(), p()))];
        let out = expand_props("input", &props);
        assert_eq!(out.model.as_deref(), Some("v"));
        assert!(!out.dynamic_attrs.contains_key("value"));
        assert!(!out.attrs.contains_key("value"));
    }

    #[test]
    fn test_gia_tri_variable_on_textarea_uses_two_way_model() {
        let props: PropsMap = vec![("gia_tri".to_string(), Expr::Variable("v".to_string(), p()))];
        let out = expand_props("textarea", &props);
        assert_eq!(out.model.as_deref(), Some("v"));
        assert!(!out.dynamic_attrs.contains_key("value"));
        assert!(!out.attrs.contains_key("value"));
    }

    #[test]
    fn test_gia_tri_variable_on_select_uses_two_way_model() {
        let props: PropsMap = vec![("gia_tri".to_string(), Expr::Variable("v".to_string(), p()))];
        let out = expand_props("select", &props);
        assert_eq!(out.model.as_deref(), Some("v"));
        assert!(!out.dynamic_attrs.contains_key("value"));
        assert!(!out.attrs.contains_key("value"));
    }

    #[test]
    fn test_gia_tri_complex_expression_on_input_stays_one_way() {
        let props: PropsMap = vec![(
            "gia_tri".to_string(),
            Expr::Binary {
                op: vibao_ast::BinaryOp::Add,
                left: Box::new(Expr::Variable("a".to_string(), p())),
                right: Box::new(Expr::literal_num(1.0, p())),
                pos: p(),
            },
        )];
        let out = expand_props("input", &props);
        assert!(out.model.is_none());
        assert!(out.dynamic_attrs.contains_key("value"));
    }

    #[test]
    fn test_gia_tri_variable_on_non_input_keeps_one_way_binding() {
        let props: PropsMap = vec![("gia_tri".to_string(), Expr::Variable("v".to_string(), p()))];
        let out = expand_props("button", &props);
        assert!(out.model.is_none());
        assert!(out.dynamic_attrs.contains_key("value"));
    }

    #[test]
    fn test_alt_static_unaffected_by_dynamic_attrs_change() {
        // A regression check: confirms the STATIC case (the most
        // common one) isn't broken by adding the is_dynamic branch.
        let props: PropsMap = vec![("mo_ta_anh".to_string(), Expr::literal_str("Ảnh minh hoạ", p()))];
        let out = expand_props("image", &props);
        assert_eq!(out.attrs.get("alt"), Some(&"Ảnh minh hoạ".to_string()));
        assert!(out.dynamic_attrs.is_empty());
    }

    // ── Tests for the build-time warning (a correctly-recognized prop
    // that does NOT support dynamic values since it needs a build-time
    // value mapping - see the "A TECHNICAL LIMITATION" note on
    // "dam"/"can"/"bat_buoc" in expand_props) ──

    #[test]
    fn test_dam_dynamic_warns_and_does_not_set_style() {
        let props: PropsMap = vec![("dam".to_string(), Expr::Variable("la_dam".to_string(), p()))];
        let out = expand_props("text", &props);
        assert_eq!(out.warnings.len(), 1, "phải cảnh báo đúng 1 lần: {:?}", out.warnings);
        assert!(out.warnings[0].contains("dam"));
        assert!(!out.style.contains_key("fontWeight"), "không được set fontWeight sai giá trị");
        assert!(!out.dynamic.contains_key("fontWeight"), "không được đẩy vào dynamic (sẽ set sai 'true'/'false' làm CSS value)");
    }

    #[test]
    fn test_can_dynamic_warns_and_does_not_set_style() {
        let props: PropsMap = vec![("can".to_string(), Expr::Variable("huong_can".to_string(), p()))];
        let out = expand_props("text", &props);
        assert_eq!(out.warnings.len(), 1);
        assert!(!out.style.contains_key("textAlign"));
    }

    #[test]
    fn test_bat_buoc_dynamic_warns_and_does_not_set_attr() {
        let props: PropsMap = vec![("bat_buoc".to_string(), Expr::Variable("bb".to_string(), p()))];
        let out = expand_props("input", &props);
        assert_eq!(out.warnings.len(), 1);
        assert!(!out.attrs.contains_key("required"));
        assert!(!out.dynamic_attrs.contains_key("required"), "không được đẩy vào dynamic_attrs (sẽ set 'required=\"false\"' vẫn bị HTML coi là required=true)");
    }

    #[test]
    fn test_static_boolean_props_unaffected_by_warning_change() {
        // A regression check: the static (common) case for
        // dam/can/bat_buoc must still work exactly as before, with no
        // warning raised.
        let props: PropsMap = vec![
            ("dam".to_string(), Expr::literal_bool(true, p())),
            ("can".to_string(), Expr::literal_str("giua", p())),
            ("bat_buoc".to_string(), Expr::literal_bool(true, p())),
        ];
        let out = expand_props("text", &props);
        assert!(out.warnings.is_empty(), "không nên có cảnh báo nào cho giá trị tĩnh: {:?}", out.warnings);
        assert_eq!(out.style.get("fontWeight"), Some(&"bold".to_string()));
        assert_eq!(out.style.get("textAlign"), Some(&"center".to_string()));
    }

    // ── tai_cham — BUG ALREADY FIXED: missing is_dynamic (a gap found
    // when reviewing the boolean-like group, unlike
    // "bat_buoc"/"vo_hieu" which already had it) ────

    #[test]
    fn test_tai_cham_dynamic_warns_and_does_not_set_attr() {
        let props: PropsMap = vec![("tai_cham".to_string(), Expr::Variable("lazy".to_string(), p()))];
        let out = expand_props("image", &props);
        assert_eq!(out.warnings.len(), 1, "phải cảnh báo đúng 1 lần: {:?}", out.warnings);
        assert!(out.warnings[0].contains("tai_cham"));
        assert!(!out.attrs.contains_key("loading"), "không được set loading sai (rỗng luôn != \"true\")");
        assert!(!out.dynamic_attrs.contains_key("loading"), "không được đẩy vào dynamic_attrs (loading là attribute đọc 1 lần, không phản ứng động)");
    }

    #[test]
    fn test_tai_cham_static_true_still_works() {
        // A regression check: the static (most common) case must still
        // work exactly as it did before the fix.
        let props: PropsMap = vec![("tai_cham".to_string(), Expr::literal_bool(true, p()))];
        let out = expand_props("image", &props);
        assert!(out.warnings.is_empty());
        assert_eq!(out.attrs.get("loading"), Some(&"lazy".to_string()));
    }

    #[test]
    fn test_tai_cham_static_false_sets_nothing() {
        let props: PropsMap = vec![("tai_cham".to_string(), Expr::literal_bool(false, p()))];
        let out = expand_props("image", &props);
        assert!(out.warnings.is_empty());
        assert!(!out.attrs.contains_key("loading"));
    }

    // ── Test cho semantic PropKey + PropSpec boundary ───────────────
    // (migrated from a raw String to PropKey, see
    // AUDIT.md/ARCHITECTURE_PROPOSAL.md "working in small groups")
    // ──────────────────────────────────────

    #[test]
    fn test_layout_only_propkey_falls_through_to_passthrough_unchanged() {
        // 12 PropKey layout-only (Hidden/MinWidth/MaxHeight/MinHeight/
        // RowGap/ColumnGap/Columns/Rows/TranslateX/TranslateY/Position/
        // Offset) have NO separate branch in expand_props() - they
        // MUST fall through to the passthrough exactly like the old
        // String match (which also had no branch for these names before
        // the migration).
        //
        // A CORRECTED TEST (not a code bug - running `cargo test`
        // revealed this test's OWN assertion was WRONG): an earlier
        // version asserted "'cot' is a known prop so it isn't unknown" -
        // WRONG, re-verified directly against the `PropKey`/`PropSpec`
        // semantics: "cot" is NOT among them (it's one of the 12
        // layout-only names, completely separate from the 45
        // Simple/Both names). The CORRECT behavior (and it was already
        // correct both before/after the migration - unchanged): "cot"
        // typed on a Simple tag "text" STILL goes through the
        // passthrough (written to attrs) BUT IS treated as
        // unknown_keys - a reasonable soft warning (a prop that only
        // means something on "grid" being mistyped on "text" deserves
        // a warning, and shouldn't be treated as "known" just because
        // it's a valid PropKey in a DIFFERENT CONTEXT).
        let props: PropsMap = vec![("cot".to_string(), Expr::literal_num(2.0, p()))];
        let out = expand_props("text", &props);
        assert!(out.attrs.contains_key("cot"), "phải đi qua passthrough, ghi ra attrs['cot']");
        assert_eq!(
            out.unknown_keys, vec!["cot".to_string()],
            "'cot' là prop layout-only, gõ trên Simple tag 'text' PHẢI được cảnh báo unknown"
        );
    }

    #[test]
    fn test_unresolvable_typo_still_flagged_unknown_after_migrate() {
        // A regression check for the core behavior that must be
        // preserved through the migration: a name that doesn't exist at
        // all (not a PropKey, not in the semantic registry) must still
        // be treated as unknown - exactly like before the migration
        // (the old "other" branch).
        let props: PropsMap = vec![("khong_ton_tai_xyz".to_string(), Expr::literal_str("x", p()))];
        let out = expand_props("text", &props);
        assert_eq!(out.unknown_keys, vec!["khong_ton_tai_xyz".to_string()]);
    }

    #[test]
    fn test_english_simple_prop_is_not_flagged_unknown() {
        // English surface syntax must use the same semantic identity as
        // Vietnamese. This specifically protects the warning path, which
        // previously depended on a separate Vietnamese-only string table.
        let props: PropsMap = vec![(
            "background_color".to_string(),
            Expr::literal_str("red", p()),
        )];
        let out = expand_props("text", &props);
        assert!(out.unknown_keys.is_empty(), "English simple prop không được cảnh báo unknown");
        assert_eq!(out.style.get("backgroundColor"), Some(&"red".to_string()));
    }

    #[test]
    fn test_english_layout_only_prop_is_still_unknown_on_simple_element() {
        // A valid semantic PropKey is not automatically valid everywhere.
        // `width` is shared, while `columns`/`cot` is layout-only.
        let props: PropsMap = vec![(
            "columns".to_string(),
            Expr::literal_num(2.0, p()),
        )];
        let out = expand_props("text", &props);
        assert_eq!(out.unknown_keys, vec!["columns".to_string()]);
        assert_eq!(out.attrs.get("columns"), Some(&"2px".to_string()));
    }

    #[test]
    fn test_all_simple_prop_identities_are_marked_simple_in_prop_spec() {
        // Exhaustiveness of the PropSpec match is compile-time; this test
        // guards the important invariant that every identity used by the
        // simple-element path is actually marked as such.
        use vibao_ast::PropKey;
        let simple_keys = [
            PropKey::BackgroundColor, PropKey::Color, PropKey::BorderColor,
            PropKey::Width, PropKey::Height, PropKey::MaxWidth, PropKey::Radius,
            PropKey::Padding, PropKey::Margin, PropKey::Border, PropKey::BorderStyle,
            PropKey::Shadow, PropKey::Overflow, PropKey::ZIndex, PropKey::FontSize,
            PropKey::Bold, PropKey::Italic, PropKey::Underline, PropKey::Align,
            PropKey::LineHeight, PropKey::LetterSpacing, PropKey::TextTransform,
            PropKey::FontFamily, PropKey::Direction, PropKey::Gap, PropKey::AlignItems,
            PropKey::Wrap, PropKey::Fit, PropKey::Source, PropKey::Alt, PropKey::LazyLoad,
            PropKey::Type, PropKey::Placeholder, PropKey::Value, PropKey::Required,
            PropKey::Disabled, PropKey::ClassBinding, PropKey::To, PropKey::Content,
            PropKey::Animation, PropKey::Duration, PropKey::Delay, PropKey::Repeat,
            PropKey::HoverAnimation, PropKey::ScrollAnimation,
        ];
        for key in simple_keys {
            assert!(prop_spec(key).applies_to_simple, "{:?} phải áp dụng cho Simple", key);
        }
    }

}
