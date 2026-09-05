// ============================================================
// VIBAO COMPILER (Rust) — codegen/layout.rs
// Generates CSS for LAYOUT ELEMENTS (flex, grid, box, stack, scroll,
// container, layer, dinh_dau, dinh_man_hinh) and responsive directives
// (@di_dong, @may_tinh_bang, @may_tinh). Equivalent to
// 07-parser-layout.ts in the old TS version.
// ============================================================

use vibao_ast::{Breakpoint, PropKey, PropsMap, ResponsiveNode};
use crate::codegen::css::OrderedMap;
use crate::codegen::expr::get_static_value;

/// Set of tags considered "layout elements" — used in element.rs to
/// decide whether to call resolve_layout_css() (this module) or
/// expand_props() (props.rs) for a specific Element.
#[cfg(test)]
pub const LAYOUT_TAGS: [&str; 9] = [
    "flex",
    "grid",
    "khoi",
    "stack",
    "cuon",
    "can_giua",
    "lop",
    "dinh_dau",
    "dinh_man_hinh",
];

#[cfg(test)]
pub fn is_layout_tag(tag: &str) -> bool {
    LAYOUT_TAGS.contains(&tag)
}

/// PARALLEL version USING SEMANTIC IDENTITY (Tag, via vibao_ast::semantic)
/// instead of matching String directly — parallel with `is_layout_tag()`
/// above, NOT replacing it yet (following the "old path → new path → test
/// → pass, then migrate" principle). `is_layout_tag()` (String-based) is
/// STILL the current official path — this function exists only for
/// CROSS-CHECKING (see the tests below), confirming that
/// `vibao_ast::semantic::tag_spec()` (the NEW real source, combining
/// LAYOUT_TAGS/BUILTIN_COMPLEX/tag_to_html into one table) gives EXACTLY
/// the same result as the old table before considering a full switch.
pub fn is_layout_tag_semantic(tag: vibao_ast::Tag) -> bool {
    vibao_ast::semantic::tag_spec(tag).kind == vibao_ast::semantic::TagKind::Layout
}

/// Resolved CSS for one layout element — uses a custom OrderedMap
/// instead of HashMap to PRESERVE property declaration order, which
/// matters when printing the real CSS string (some properties depend on
/// order, e.g. border-style must come after border-width when both
/// override "border"). Matches PropsMap in ast.rs, which also uses Vec
/// instead of HashMap for the same reason. Avoids external crates
/// (indexmap) to preserve the project's "no network dependency at build
/// time" philosophy documented in Cargo.toml/README.
pub type LayoutCss = OrderedMap;

/// Dispatches by tag to compute the corresponding layout CSS — equivalent
/// to resolveLayoutCSS() in the old TS version.
pub fn resolve_layout_css(tag: &str, props: &PropsMap) -> LayoutCss {
    match tag {
        "flex" => resolve_flex(props),
        "grid" => resolve_grid(props),
        "khoi" => resolve_box(props),
        "stack" => resolve_stack(props),
        "cuon" => resolve_scroll(props),
        "can_giua" => resolve_container(props),
        "lop" => resolve_layer(),
        "dinh_dau" => resolve_sticky_top(props),
        "dinh_man_hinh" => resolve_fixed(props),
        _ => {
            let mut css = LayoutCss::new();
            css.insert("display".to_string(), "block".to_string());
            css
        }
    }
}

// ════════════════════════════════════════════════════════════
// BUG-25 FIX — "unknown prop" warning for Layout Element (AUDIT.md)
// ════════════════════════════════════════════════════════════
//
// Before this function, Layout Element had NO warning mechanism
// equivalent to the `unknown_keys` Simple Element already had
// (`props.rs::expand_props()`, cross-checking semantic `PropKey`/
// `PropSpec`) — mistyping a prop name on `khoi`/`flex`/... fell into the
// silent `_ => {}` branch in EACH `resolve_*` function, setting nothing
// and warning nothing.
//
// DESIGN (intentionally NOT changing the 9 existing `resolve_*` functions
// — see ARCHITECTURE_PROPOSAL.md "STOP AT THE RIGHT PLACE": changing
// those signatures to return unknown_keys would be a large change and
// touch many tests that call each function directly — too risky when the
// only need is to ADD a warning). This NEW, INDEPENDENT function only
// COMPUTES (generates no CSS), and is called IN PARALLEL with
// `resolve_layout_css()` at the call site (`element.rs::gen_layout_element`)
// — exactly like `expand_props()` (props.rs) separates `unknown_keys` into
// a separate result field so the caller (element.rs) decides how to warn.
//
// REAL SOURCE of the table below: semantic `PropKey` values corresponding
// to props CROSS-CHECKED DIRECTLY from the bodies of the 9 `resolve_*`
// functions. Because the table uses identity instead of surface strings,
// both Vietnamese and English locales go through the same allowlist.
// The tests that cross-check REAL BEHAVIOR (not just a static list) are
// `test_known_layout_props_*_all_have_effect` (1 test/tag, sharing helper
// `assert_prop_has_effect()`) in `mod tests` at the end of this file —
// renamed from the original plan (`test_layout_prop_allowlist_matches_
// resolve_fns_exactly`, which does NOT exist). User review found the old
// comment mentioned a non-existent test name, so this was updated to match
// the actual code; see VIBAOC_BUG_NOTES.md NOTE C.
//
// KNOWN LIMITATION (VIBAOC_BUG_NOTES.md NOTE C): the
// `assert_prop_has_effect()` tests only catch one DIRECTION — a key
// declared "known" in the table but having NO real CSS effect. They do
// NOT catch the REVERSE direction: if someone later adds a new match arm
// to `resolve_flex()`/... but FORGETS to add that key to
// `known_layout_props()`, BUG-25 will RECUR for that new key (it will fall
// into unknown_keys even though it is actually "known" according to the
// real resolve_* function), and no test will catch it automatically. Low
// short-term risk (new layout props are rarely added), but keep it in mind
// when reviewing later updates that touch the 9 `resolve_*` functions.
//
// Do NOT reuse `vibao_ast::semantic::prop_spec().applies_to_layout`
// because that field only answers "valid for LAYOUT IN GENERAL" (all 9
// tags grouped together) — too coarse for useful warnings on the exact
// specific tag (e.g. "cot" is valid on "grid" but NOT on "flex", even
// though both are applies_to_layout=true; using only that field would not
// warn for "cot" on "flex" even though it is a real error).
fn known_layout_props(tag: &str) -> &'static [PropKey] {
    match tag {
        "flex" => &[PropKey::Direction, PropKey::Gap, PropKey::RowGap, PropKey::ColumnGap, PropKey::Align, PropKey::AlignItems, PropKey::Wrap, PropKey::Width, PropKey::Height, PropKey::Padding, PropKey::BackgroundColor, PropKey::Radius],
        "grid" => &[PropKey::Columns, PropKey::Rows, PropKey::Gap, PropKey::RowGap, PropKey::ColumnGap, PropKey::Width, PropKey::Padding, PropKey::BackgroundColor],
        "khoi" => &[PropKey::BackgroundColor, PropKey::Width, PropKey::Height, PropKey::MinWidth, PropKey::MaxWidth, PropKey::MinHeight, PropKey::MaxHeight, PropKey::Radius, PropKey::Padding, PropKey::Margin, PropKey::Border, PropKey::Shadow, PropKey::Overflow, PropKey::TranslateX, PropKey::TranslateY, PropKey::ZIndex],
        "stack" => &[PropKey::Align, PropKey::AlignItems, PropKey::Width, PropKey::Height],
        "cuon" => &[PropKey::Direction, PropKey::Height, PropKey::Width],
        "can_giua" => &[PropKey::MaxWidth, PropKey::Padding],
        "lop" => &[],
        "dinh_dau" => &[PropKey::Offset],
        "dinh_man_hinh" => &[PropKey::Position, PropKey::Width, PropKey::Height],
        _ => &[],
    }
}

/// Returns prop names that are NOT in the valid prop list for the exact
/// layout tag being checked — used by `element.rs::gen_layout_element`
/// to emit a soft warning (like `ExpandedProps::unknown_keys` in
/// props.rs — does NOT block the build, only warns about likely typos).
pub fn layout_unknown_keys(tag: &str, props: &PropsMap) -> Vec<String> {
    let known = known_layout_props(tag);
    props
        .iter()
        .filter(|(key, _)| {
            match crate::locale::resolve_prop_key(key.as_str()) {
                Some(prop_key) => !known.contains(&prop_key),
                None => true,
            }
        })
        .map(|(key, _)| key.clone())
        .collect()
}

fn resolve_flex(props: &PropsMap) -> LayoutCss {
    let mut css = LayoutCss::new();
    css.insert("display".to_string(), "flex".to_string());
    for (key, expr) in props {
        let v = get_static_value(expr);
        // Migrate PropKey (this code pass, see AUDIT.md "small groups")
        // — changes match keys from raw String to PropKey via
        // `crate::locale::resolve_prop_key()`. Props not belonging to
        // "flex" (e.g. "cot" — Columns, only valid on "grid") fall into
        // `_ => {}` exactly like the old behavior (the old String match
        // also had no arm for those names HERE, even though they are still
        // valid in resolve_grid() separately).
        match crate::locale::resolve_prop_key(key.as_str()) {
            Some(PropKey::Direction) => {
                css.insert(
                    "flexDirection".to_string(),
                    if v == "column" { "column".to_string() } else { "row".to_string() },
                );
            }
            Some(PropKey::Gap) => {
                css.insert("gap".to_string(), px(&v));
            }
            Some(PropKey::RowGap) => {
                css.insert("rowGap".to_string(), px(&v));
            }
            Some(PropKey::ColumnGap) => {
                css.insert("columnGap".to_string(), px(&v));
            }
            Some(PropKey::Align) => {
                css.insert("justifyContent".to_string(), map_justify(&v));
            }
            Some(PropKey::AlignItems) => {
                css.insert("alignItems".to_string(), map_align_items(&v));
            }
            Some(PropKey::Wrap) => {
                if v == "true" {
                    css.insert("flexWrap".to_string(), "wrap".to_string());
                }
            }
            Some(PropKey::Width) => {
                css.insert("width".to_string(), size(&v));
            }
            Some(PropKey::Height) => {
                css.insert("height".to_string(), size(&v));
            }
            Some(PropKey::Padding) => {
                css.insert("padding".to_string(), spacing(&v));
            }
            Some(PropKey::BackgroundColor) => {
                css.insert("backgroundColor".to_string(), v);
            }
            Some(PropKey::Radius) => {
                css.insert("borderRadius".to_string(), radius(&v));
            }
            _ => {}
        }
    }
    css
}

fn resolve_grid(props: &PropsMap) -> LayoutCss {
    let mut css = LayoutCss::new();
    css.insert("display".to_string(), "grid".to_string());
    for (key, expr) in props {
        let v = get_static_value(expr);
        match crate::locale::resolve_prop_key(key.as_str()) {
            Some(PropKey::Columns) => {
                css.insert("gridTemplateColumns".to_string(), repeat_or_raw(&v));
            }
            Some(PropKey::Rows) => {
                css.insert("gridTemplateRows".to_string(), repeat_or_raw(&v));
            }
            Some(PropKey::Gap) => {
                css.insert("gap".to_string(), px(&v));
            }
            Some(PropKey::RowGap) => {
                css.insert("rowGap".to_string(), px(&v));
            }
            Some(PropKey::ColumnGap) => {
                css.insert("columnGap".to_string(), px(&v));
            }
            Some(PropKey::Width) => {
                css.insert("width".to_string(), size(&v));
            }
            Some(PropKey::Padding) => {
                css.insert("padding".to_string(), spacing(&v));
            }
            Some(PropKey::BackgroundColor) => {
                css.insert("backgroundColor".to_string(), v);
            }
            _ => {}
        }
    }
    css
}

fn resolve_box(props: &PropsMap) -> LayoutCss {
    let mut css = LayoutCss::new();
    css.insert("display".to_string(), "block".to_string());
    // BUG E FIX (the layout validation review notes, user review): tran_x
    // (TranslateX) and tran_y (TranslateY) PREVIOUSLY both inserted
    // directly into the same CSS key "transform" in the main loop —
    // `OrderedMap::insert()` overwrites duplicate keys, so declaring BOTH
    // at once (`khoi(tran_x: 10, tran_y: 20)`) made whichever value was
    // processed LATER (depending on source PropsMap order) WIN, while the
    // other disappeared from CSS completely — no warning, no error, just
    // silently incorrect display. The bug predates the PropKey migration
    // (the old String code had the same bug in the two "tran_x"/"tran_y"
    // branches).
    //
    // Fix: COLLECT the two values separately (Option<String>, without
    // inserting directly into css inside the loop), then MERGE into one
    // SINGLE "transform" string after the loop ends — insert only if at
    // least one of the two values exists. This "collect separately then
    // build once" design is INTENTIONAL to make future rotate/scale
    // extension easier (only add one collected variable + one branch to
    // append into transform_parts, without changing the structure again).
    let mut translate_x: Option<String> = None;
    let mut translate_y: Option<String> = None;
    for (key, expr) in props {
        let v = get_static_value(expr);
        match crate::locale::resolve_prop_key(key.as_str()) {
            Some(PropKey::BackgroundColor) => {
                css.insert("backgroundColor".to_string(), v);
            }
            Some(PropKey::Width) => {
                css.insert("width".to_string(), size(&v));
            }
            Some(PropKey::Height) => {
                css.insert("height".to_string(), size(&v));
            }
            Some(PropKey::MinWidth) => {
                css.insert("minWidth".to_string(), size(&v));
            }
            Some(PropKey::MaxWidth) => {
                css.insert("maxWidth".to_string(), size(&v));
            }
            Some(PropKey::MinHeight) => {
                css.insert("minHeight".to_string(), size(&v));
            }
            Some(PropKey::MaxHeight) => {
                css.insert("maxHeight".to_string(), size(&v));
            }
            Some(PropKey::Radius) => {
                css.insert("borderRadius".to_string(), radius(&v));
            }
            Some(PropKey::Padding) => {
                css.insert("padding".to_string(), spacing(&v));
            }
            Some(PropKey::Margin) => {
                css.insert("margin".to_string(), spacing(&v));
            }
            Some(PropKey::Border) => {
                css.insert("border".to_string(), border(&v, props));
            }
            Some(PropKey::Shadow) => {
                css.insert("boxShadow".to_string(), v);
            }
            Some(PropKey::Overflow) => {
                css.insert("overflow".to_string(), v);
            }
            Some(PropKey::TranslateX) => {
                translate_x = Some(px(&v));
            }
            Some(PropKey::TranslateY) => {
                translate_y = Some(px(&v));
            }
            Some(PropKey::ZIndex) => {
                css.insert("zIndex".to_string(), v);
            }
            _ => {}
        }
    }
    // Merge translate_x/translate_y into one single "transform" string —
    // set only when at least one of the two exists, preserving X before Y
    // order (matching the CSS transform convention: translate(x, y)).
    match (translate_x, translate_y) {
        (Some(x), Some(y)) => {
            css.insert("transform".to_string(), format!("translateX({}) translateY({})", x, y));
        }
        (Some(x), None) => {
            css.insert("transform".to_string(), format!("translateX({})", x));
        }
        (None, Some(y)) => {
            css.insert("transform".to_string(), format!("translateY({})", y));
        }
        (None, None) => {}
    }
    css
}

fn resolve_stack(props: &PropsMap) -> LayoutCss {
    let mut css = LayoutCss::new();
    css.insert("display".to_string(), "grid".to_string());
    css.insert("gridTemplateColumns".to_string(), "1fr".to_string());
    css.insert("gridTemplateRows".to_string(), "1fr".to_string());
    for (key, expr) in props {
        let v = get_static_value(expr);
        match crate::locale::resolve_prop_key(key.as_str()) {
            Some(PropKey::Align) => {
                css.insert("justifyItems".to_string(), map_justify(&v));
            }
            Some(PropKey::AlignItems) => {
                css.insert("alignItems".to_string(), map_align_items(&v));
            }
            Some(PropKey::Width) => {
                css.insert("width".to_string(), size(&v));
            }
            Some(PropKey::Height) => {
                css.insert("height".to_string(), size(&v));
            }
            _ => {}
        }
    }
    css
}

fn resolve_scroll(props: &PropsMap) -> LayoutCss {
    let mut css = LayoutCss::new();
    css.insert("display".to_string(), "block".to_string());
    css.insert("overflow".to_string(), "auto".to_string());
    for (key, expr) in props {
        let v = get_static_value(expr);
        match crate::locale::resolve_prop_key(key.as_str()) {
            Some(PropKey::Direction) => {
                css.insert("overflow".to_string(), "hidden".to_string());
                css.insert(
                    "overflowX".to_string(),
                    if v == "ngang" { "auto".to_string() } else { "hidden".to_string() },
                );
                css.insert(
                    "overflowY".to_string(),
                    if v == "doc" { "auto".to_string() } else { "hidden".to_string() },
                );
            }
            Some(PropKey::Height) => {
                css.insert("height".to_string(), size(&v));
            }
            Some(PropKey::Width) => {
                css.insert("width".to_string(), size(&v));
            }
            _ => {}
        }
    }
    css
}

fn resolve_container(props: &PropsMap) -> LayoutCss {
    let mut css = LayoutCss::new();
    css.insert("display".to_string(), "block".to_string());
    css.insert("width".to_string(), "100%".to_string());
    css.insert("marginLeft".to_string(), "auto".to_string());
    css.insert("marginRight".to_string(), "auto".to_string());
    for (key, expr) in props {
        let v = get_static_value(expr);
        match crate::locale::resolve_prop_key(key.as_str()) {
            Some(PropKey::MaxWidth) => {
                css.insert("maxWidth".to_string(), size(&v));
            }
            Some(PropKey::Padding) => {
                css.insert("padding".to_string(), spacing(&v));
            }
            _ => {}
        }
    }
    css
}

fn resolve_layer() -> LayoutCss {
    let mut css = LayoutCss::new();
    css.insert("display".to_string(), "block".to_string());
    css.insert("position".to_string(), "relative".to_string());
    css.insert("width".to_string(), "100%".to_string());
    css.insert("height".to_string(), "100%".to_string());
    css
}

fn resolve_sticky_top(props: &PropsMap) -> LayoutCss {
    let mut css = LayoutCss::new();
    css.insert("display".to_string(), "block".to_string());
    css.insert("position".to_string(), "sticky".to_string());
    css.insert("top".to_string(), "0".to_string());
    css.insert("zIndex".to_string(), "100".to_string());
    for (key, expr) in props {
        let v = get_static_value(expr);
        if crate::locale::resolve_prop_key(key.as_str()) == Some(PropKey::Offset) {
            css.insert("top".to_string(), px(&v));
        }
    }
    css
}

fn resolve_fixed(props: &PropsMap) -> LayoutCss {
    let mut css = LayoutCss::new();
    css.insert("display".to_string(), "block".to_string());
    css.insert("position".to_string(), "fixed".to_string());
    css.insert("zIndex".to_string(), "200".to_string());
    for (key, expr) in props {
        let v = get_static_value(expr);
        match crate::locale::resolve_prop_key(key.as_str()) {
            Some(PropKey::Position) => match v.as_str() {
                "tren" => {
                    css.insert("top".to_string(), "0".to_string());
                    css.insert("left".to_string(), "0".to_string());
                    css.insert("right".to_string(), "0".to_string());
                }
                "duoi" => {
                    css.insert("bottom".to_string(), "0".to_string());
                    css.insert("left".to_string(), "0".to_string());
                    css.insert("right".to_string(), "0".to_string());
                }
                "trai" => {
                    css.insert("top".to_string(), "0".to_string());
                    css.insert("left".to_string(), "0".to_string());
                    css.insert("bottom".to_string(), "0".to_string());
                }
                "phai" => {
                    css.insert("top".to_string(), "0".to_string());
                    css.insert("right".to_string(), "0".to_string());
                    css.insert("bottom".to_string(), "0".to_string());
                }
                _ => {}
            },
            Some(PropKey::Width) => {
                css.insert("width".to_string(), size(&v));
            }
            Some(PropKey::Height) => {
                css.insert("height".to_string(), size(&v));
            }
            _ => {}
        }
    }
    css
}

// ════════════════════════════════════════════════════════════
// RESPONSIVE (@di_dong, @may_tinh_bang, @may_tinh)
// ════════════════════════════════════════════════════════════

/// Media query CSS for one resolved breakpoint — selector plus @media
/// condition and the list of property→value overrides.
pub struct ResponsiveCss {
    pub media_condition: String,
    pub overrides: LayoutCss,
}

fn breakpoint_media_condition(bp: Breakpoint) -> &'static str {
    match bp {
        Breakpoint::DiDong => "(max-width: 639px)",
        Breakpoint::MayTinhBang => "(min-width: 640px) and (max-width: 1023px)",
        Breakpoint::MayTinh => "(min-width: 1024px)",
    }
}

/// Resolves the list of ResponsiveNode values (parsed from
/// @di_dong { ... } etc.) into CSS overrides for each breakpoint.
/// Equivalent to resolveResponsiveCSS() in the old TS version.
pub fn resolve_responsive_css(_tag: &str, responsive: &[ResponsiveNode]) -> Vec<ResponsiveCss> {
    responsive
        .iter()
        .map(|r| {
            let mut overrides = LayoutCss::new();
            for (key, expr) in &r.overrides {
                let v = get_static_value(expr);
                match crate::locale::resolve_prop_key(key.as_str()) {
                    Some(PropKey::Columns) => {
                        overrides.insert("grid-template-columns".to_string(), repeat_or_raw(&v));
                    }
                    Some(PropKey::Direction) => {
                        overrides.insert(
                            "flex-direction".to_string(),
                            if v == "column" { "column".to_string() } else { "row".to_string() },
                        );
                    }
                    Some(PropKey::FontSize) => {
                        overrides.insert("font-size".to_string(), px(&v));
                    }
                    Some(PropKey::Width) => {
                        overrides.insert("width".to_string(), size(&v));
                    }
                    Some(PropKey::Height) => {
                        overrides.insert("height".to_string(), size(&v));
                    }
                    Some(PropKey::Padding) => {
                        overrides.insert("padding".to_string(), spacing(&v));
                    }
                    Some(PropKey::Hidden) => {
                        if v == "true" {
                            overrides.insert("display".to_string(), "none".to_string());
                        }
                    }
                    // Passthrough (keep the ORIGINAL key from PropsMap, do
                    // NOT infer back from PropKey — the same principle used
                    // in the props.rs migration) — includes BOTH cases:
                    // (a) None (a completely unknown name, not any PropKey)
                    // and (b) Some(_) (a valid PropKey but NOT one of the 7
                    // RESPONSIVE_HANDLED_PROP_KEYS — e.g. PropKey::Color
                    // from "mau" — PRESERVE old behavior: the old String
                    // match also had no dedicated arm for "mau" HERE, so it
                    // fell into "other" exactly like a totally unknown name).
                    _ => {
                        overrides.insert(crate::codegen::css::camel_to_kebab(key), v);
                    }
                }
            }
            ResponsiveCss {
                media_condition: breakpoint_media_condition(r.breakpoint).to_string(),
                overrides,
            }
        })
        .collect()
}

// ════════════════════════════════════════════════════════════
// BUG B FIX (VIBAOC_BUG_NOTES.md, user review) — warning for
// "unknown prop" in responsive blocks (@di_dong/@may_tinh_bang/@may_tinh)
// ════════════════════════════════════════════════════════════
//
// Before this function, the `other =>` branch in `resolve_responsive_css()` (above)
// ACCEPTED EVERY unknown key name and converted it directly into an arbitrary CSS property
// via `camel_to_kebab()` — mistyping "ronnng" instead of "rong" produced
// `ronnng: 100px;` — junk CSS silently ignored by browsers, with no
// build-time warning at all. Same kind of issue as BUG-25 (already fixed for regular
// layout elements), but NOT yet fixed for responsive blocks.
//
// SCOPE OF THIS PASS (discussed with the user, STATED EXPLICITLY to avoid
// misunderstanding this as "full validation done"): ONLY validate at VOCABULARY LEVEL —
// compare keys against the exact 7 names `resolve_responsive_css()` ACTUALLY recognizes
// (cot/huong/co/rong/cao/dem/an — see match arms in the function above,
// NO "gap" even if intuition suggests it). NOT yet validating by
// exact specific TAG (e.g. `text { @di_dong { cot: 2 } }` — "cot" is in
// the valid vocabulary so it is NOT warned, even though "cot" only has real meaning
// on "grid" — `resolve_responsive_css()` is shared by BOTH
// Simple AND Layout Elements, without distinguishing by Tag, so
// validating correctly per Tag requires knowing which props are valid in
// responsive for EVERY Simple tag (text/button/...) as well as Layout tags —
// much more analysis than `known_layout_props()`, which has only 9
// tag).
//
// STATE AFTER THIS FIX (as the user requested to state clearly):
//   Responsive validation
//     ├── unknown vocabulary (definite typo, e.g. "ronnng") → HAS warning
//     └── correct vocabulary but WRONG TAG (e.g. "cot" on "text")
//         → NOT checked yet, ACCEPTABLE in this pass
//
// LEVEL 2 (do LATER, once PropKey is wired into the real lexer/parser — do NOT do
// before that, for the exact reason the user gave): instead of building another
// third String table (surface prop -> which Tag is valid in responsive,
// separately duplicating known_layout_props() work), use
// PropKey + a concept of "Applicability by context" (Grid/Simple/
// Layout/Responsive) as the real semantic foundation for per-Tag
// validation — a clearer basis than a parallel handwritten allowlist.
pub const RESPONSIVE_HANDLED_PROP_KEYS: [PropKey; 7] = [
    PropKey::Columns,
    PropKey::Direction,
    PropKey::FontSize,
    PropKey::Width,
    PropKey::Height,
    PropKey::Padding,
    PropKey::Hidden,
];

/// Vietnamese surface names for the responsive group are kept only as
/// metadata/documentation; real validation uses semantic PropKey so the
/// English locale does not create false positives.
pub fn responsive_prop_is_known(key: &str) -> bool {
    match crate::locale::resolve_prop_key(key) {
        Some(prop) => RESPONSIVE_HANDLED_PROP_KEYS.contains(&prop),
        None => false,
    }
}

/// Returns key names in ALL responsive blocks (all breakpoints,
/// combined) that are NOT in `RESPONSIVE_HANDLED_PROP_KEYS` — used by
/// `element.rs` to emit soft warnings, exactly like the existing
/// `unknown_keys`/`layout_unknown_keys()` mechanism. ONLY validates at vocabulary level (see
/// the comment block above) — does NOT take a `tag` parameter.
///
/// DEDUPE (the layout validation review notes item 1, user review): 1 Element
/// may have MANY `ResponsiveNode`s (1 per breakpoint —
/// @di_dong/@may_tinh_bang/@may_tinh), unlike `layout_unknown_keys()`
/// (only reads a single PropsMap; ViBao syntax does not allow duplicate keys
/// in the same block, so no dedupe is needed). If the SAME mistyped name
/// appears in MANY breakpoints (very easy to do — devs often copy the whole
/// prop block and adjust a few values per breakpoint, leaving the typo intact),
/// before this fix it returned N duplicates, causing `element.rs` to emit N
/// identical warnings for the SAME error — use `dedup_keep_order()`
/// (preserve first-seen order, stable — easier for devs to read
/// than a HashSet with no order guarantee).
pub fn responsive_unknown_keys(responsive: &[ResponsiveNode]) -> Vec<String> {
    let mut result: Vec<String> = responsive
        .iter()
        .flat_map(|r| r.overrides.iter())
        .filter(|(key, _)| !responsive_prop_is_known(key))
        .map(|(key, _)| key.clone())
        .collect();
    dedup_keep_order(&mut result);
    result
}

/// Removes duplicate elements while PRESERVING first-seen order
/// (unlike standard-library `Vec::dedup()` — that only removes adjacent
/// duplicates, not enough when two duplicates are far apart in the Vec, exactly
/// the real case here: unknown names in @di_dong and @may_tinh are not adjacent
/// in the flat_map result).
fn dedup_keep_order(v: &mut Vec<String>) {
    let mut seen = std::collections::HashSet::new();
    v.retain(|item| seen.insert(item.clone()));
}

/// Builds one complete @media block for a selector — returns an empty
/// string if there are no overrides (equivalent to the old TS check
/// `Object.keys(overrides).length === 0`, so callers can filter out empty
/// blocks).
pub fn build_media_query(selector: &str, bp_css: &ResponsiveCss) -> String {
    if bp_css.overrides.is_empty() {
        return String::new();
    }
    let rules = bp_css
        .overrides
        .iter()
        .map(|(k, v)| format!("    {}: {};", k, v))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "@media {} {{\n  {} {{\n{}\n  }}\n}}",
        bp_css.media_condition, selector, rules
    )
}

// ════════════════════════════════════════════════════════════
// CSS VALUE HELPERS (px, size, spacing, radius, border, align maps)
// ════════════════════════════════════════════════════════════

/// If the string is "__dynamic__" (sentinel from get_static_value) or
/// empty, returns it unchanged — layout CSS does not support dynamic
/// binding for layout props (unlike props.rs; layout tags rarely need
/// runtime changes). If it is a plain number, appends "px".
pub fn px(val: &str) -> String {
    if val.is_empty() || val == "__dynamic__" {
        return val.to_string();
    }
    if is_plain_number(val) {
        format!("{}px", val)
    } else {
        val.to_string()
    }
}

/// Like px(), but accepts only non-negative numbers, specifically for
/// size. Negative numbers are disallowed; a valid decimal must have at
/// least 1 digit and at most 1 dot.
pub fn size(val: &str) -> String {
    if val.is_empty() || val == "__dynamic__" {
        return val.to_string();
    }
    if is_unsigned_decimal(val) {
        format!("{}px", val)
    } else {
        val.to_string()
    }
}

pub fn spacing(val: &str) -> String {
    if val.is_empty() || val == "__dynamic__" {
        return val.to_string();
    }
    val.split_whitespace().map(px).collect::<Vec<_>>().join(" ")
}

pub fn radius(val: &str) -> String {
    spacing(val)
}

fn border(val: &str, props: &PropsMap) -> String {
    let width = px(val);
    let style = props
        .iter()
        .find(|(k, _)| k == "kieu_vien")
        .map(|(_, e)| get_static_value(e))
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "solid".to_string());
    let color = props
        .iter()
        .find(|(k, _)| k == "mau_vien")
        .map(|(_, e)| get_static_value(e))
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "#000".to_string());
    format!("{} {} {}", width, style, color)
}

fn repeat_or_raw(val: &str) -> String {
    // Grid repeat() requires a positive integer column count. Numeric
    // string 0 has no meaning for repeat() and previously was wrapped as
    // `repeat(0, 1fr)`, creating invalid CSS without a clear signal. Keep
    // 0/raw unchanged to avoid inventing new semantics; callers can still
    // report/warn at the appropriate layer.
    if !val.is_empty() && val.chars().all(|c| c.is_ascii_digit()) && val != "0" {
        format!("repeat({}, 1fr)", val)
    } else {
        val.to_string()
    }
}

/// Checks a decimal number with an optional sign at the start. The "-"
/// sign is allowed only at the start; the absolute-value portion must have
/// at least 1 digit and at most 1 dot.
fn is_plain_number(val: &str) -> bool {
    if val.is_empty() {
        return false;
    }
    let rest = val.strip_prefix('-').unwrap_or(val);
    is_unsigned_decimal(rest)
}

/// Simple decimal: allows at most 1 `.` but requires at least 1 digit.
/// Therefore `1.25`, `.5`, and `1.` are valid; `.`, `..`, and `1.2.3`
/// are no longer mistaken for numbers.
fn is_unsigned_decimal(val: &str) -> bool {
    !val.is_empty()
        && val.chars().any(|c| c.is_ascii_digit())
        && val.chars().filter(|&c| c == '.').count() <= 1
        && val.chars().all(|c| c.is_ascii_digit() || c == '.')
}

pub fn map_justify(val: &str) -> String {
    match val {
        "start" => "flex-start".to_string(),
        "end" => "flex-end".to_string(),
        "center" | "giua" => "center".to_string(),
        "space-between" => "space-between".to_string(),
        "space-around" => "space-around".to_string(),
        other => other.to_string(),
    }
}

pub fn map_align_items(val: &str) -> String {
    match val {
        "start" => "flex-start".to_string(),
        "end" => "flex-end".to_string(),
        "center" | "giua" => "center".to_string(),
        "stretch" | "deu" => "stretch".to_string(),
        // FIXED BUG: the old "other => other.to_string()" branch caused
        // Vietnamese values that matched no case (e.g. "deu") to be
        // printed DIRECTLY into CSS as "align-items:deu" — an INVALID CSS
        // value (align-items has no "even"/space-between concept; that is
        // a justify-content concept on a different axis). Browsers
        // silently ignored the invalid property (no error), making layout
        // drift with no obvious cause — found by test-building a real app
        // (dist_ver_0_0_6). "deu" now maps to "stretch" (the closest
        // valid CSS meaning: items stretch across the cross axis, closest
        // to the intended "even" meaning among available align-items
        // choices).
        //
        // For any other UNRECOGNIZED value (not a known Vietnamese typo),
        // still pass it through verbatim — allowing Devs to write a valid
        // CSS value directly (e.g. "baseline") when this table has not
        // explicitly listed it yet, instead of blocking it hard.
        other => other.to_string(),
    }
}

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

    /// Test that cross-checks REAL BEHAVIOR (not just hand-copying a list) for
    /// `known_layout_props()`/`layout_unknown_keys()` (BUG-25 fix) — for
    /// EACH prop declared "known" for a tag, call
    /// `resolve_layout_css()` (the OLD, UNCHANGED function) directly with a sensible value and
    /// confirm the CSS output ACTUALLY CHANGES compared with the empty baseline — if a
    /// prop is declared "known" but does NOT change CSS at all, that indicates
    /// the `known_layout_props()` table has drifted from the real behavior of the 9
    /// `resolve_*` functions; this test catches that immediately (stronger than comparing
    /// a static String list, because it cross-checks through the EXACT execution path
    /// used by `element.rs`).
    fn assert_prop_has_effect(tag: &str, key: &str, value: Expr) {
        let baseline = resolve_layout_css(tag, &vec![]);
        let with_prop = resolve_layout_css(tag, &vec![(key.to_string(), value)]);
        // OrderedMap does not derive PartialEq (see css.rs) — compare through
        // Vec collected from iter(), enough to detect content differences.
        let baseline_vec: Vec<(String, String)> =
            baseline.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
        let with_prop_vec: Vec<(String, String)> =
            with_prop.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
        assert_ne!(
            baseline_vec, with_prop_vec,
            "prop '{}' được khai 'known' cho tag '{}' nhưng KHÔNG làm CSS thay đổi — \
             known_layout_props() có thể đã lệch khỏi resolve_{}() thật",
            key, tag, tag
        );
    }

    fn str_val(s: &str) -> Expr {
        Expr::literal_str(s, p())
    }

    #[test]
    fn test_known_layout_props_flex_all_have_effect() {
        assert_prop_has_effect("flex", "huong", str_val("column"));
        assert_prop_has_effect("flex", "gap", str_val("8"));
        assert_prop_has_effect("flex", "gap_doc", str_val("8"));
        assert_prop_has_effect("flex", "gap_ngang", str_val("8"));
        assert_prop_has_effect("flex", "can", str_val("giua"));
        assert_prop_has_effect("flex", "doc", str_val("giua"));
        assert_prop_has_effect("flex", "boc", str_val("true"));
        assert_prop_has_effect("flex", "rong", str_val("100"));
        assert_prop_has_effect("flex", "cao", str_val("100"));
        assert_prop_has_effect("flex", "dem", str_val("8"));
        assert_prop_has_effect("flex", "mau_nen", str_val("trang"));
        assert_prop_has_effect("flex", "radius", str_val("8"));
    }

    #[test]
    fn test_known_layout_props_grid_all_have_effect() {
        assert_prop_has_effect("grid", "cot", str_val("2"));
        assert_prop_has_effect("grid", "hang_luoi", str_val("2"));
        assert_prop_has_effect("grid", "gap", str_val("8"));
        assert_prop_has_effect("grid", "gap_doc", str_val("8"));
        assert_prop_has_effect("grid", "gap_ngang", str_val("8"));
        assert_prop_has_effect("grid", "rong", str_val("100"));
        assert_prop_has_effect("grid", "dem", str_val("8"));
        assert_prop_has_effect("grid", "mau_nen", str_val("trang"));
    }

    #[test]
    fn test_known_layout_props_khoi_all_have_effect() {
        assert_prop_has_effect("khoi", "mau_nen", str_val("trang"));
        assert_prop_has_effect("khoi", "rong", str_val("100"));
        assert_prop_has_effect("khoi", "cao", str_val("100"));
        assert_prop_has_effect("khoi", "min_rong", str_val("10"));
        assert_prop_has_effect("khoi", "max_rong", str_val("200"));
        assert_prop_has_effect("khoi", "min_cao", str_val("10"));
        assert_prop_has_effect("khoi", "max_cao", str_val("200"));
        assert_prop_has_effect("khoi", "radius", str_val("8"));
        assert_prop_has_effect("khoi", "dem", str_val("8"));
        assert_prop_has_effect("khoi", "le", str_val("8"));
        assert_prop_has_effect("khoi", "vien", str_val("1"));
        assert_prop_has_effect("khoi", "bong", str_val("true"));
        assert_prop_has_effect("khoi", "cuon_tran", str_val("an"));
        assert_prop_has_effect("khoi", "tran_x", str_val("10"));
        assert_prop_has_effect("khoi", "tran_y", str_val("10"));
        assert_prop_has_effect("khoi", "tang_z", str_val("5"));
    }

    #[test]
    fn test_translate_x_and_y_together_both_present_in_transform() {
        // BUG E FIX (the layout validation review notes, user review): previously
        // tran_x/tran_y both wrote to the same CSS key "transform"; the value
        // processed later won, and the other value silently disappeared. This test
        // confirms BOTH values are present in CSS output when both
        // props are declared together.
        let props: PropsMap = vec![
            ("tran_x".to_string(), str_val("10")),
            ("tran_y".to_string(), str_val("20")),
        ];
        let css = resolve_box(&props);
        let transform = css.get("transform").expect("phải có key 'transform'");
        assert!(transform.contains("translateX(10px)"), "thiếu translateX trong: {}", transform);
        assert!(transform.contains("translateY(20px)"), "thiếu translateY trong: {}", transform);
    }

    #[test]
    fn test_translate_x_and_y_together_order_independent() {
        // Change declaration order in PropsMap (tran_y before tran_x) — previously
        // order decided which value SURVIVED (bug); now both
        // must be present regardless of declaration order.
        let props: PropsMap = vec![
            ("tran_y".to_string(), str_val("20")),
            ("tran_x".to_string(), str_val("10")),
        ];
        let css = resolve_box(&props);
        let transform = css.get("transform").expect("phải có key 'transform'");
        assert!(transform.contains("translateX(10px)"), "thiếu translateX trong: {}", transform);
        assert!(transform.contains("translateY(20px)"), "thiếu translateY trong: {}", transform);
    }

    #[test]
    fn test_translate_x_only_no_translate_y_in_transform() {
        let props: PropsMap = vec![("tran_x".to_string(), str_val("10"))];
        let css = resolve_box(&props);
        let transform = css.get("transform").expect("phải có key 'transform'");
        assert_eq!(transform, "translateX(10px)");
    }

    #[test]
    fn test_translate_y_only_no_translate_x_in_transform() {
        let props: PropsMap = vec![("tran_y".to_string(), str_val("20"))];
        let css = resolve_box(&props);
        let transform = css.get("transform").expect("phải có key 'transform'");
        assert_eq!(transform, "translateY(20px)");
    }

    #[test]
    fn test_known_layout_props_stack_all_have_effect() {
        assert_prop_has_effect("stack", "can", str_val("giua"));
        assert_prop_has_effect("stack", "doc", str_val("giua"));
        assert_prop_has_effect("stack", "rong", str_val("100"));
        assert_prop_has_effect("stack", "cao", str_val("100"));
    }

    #[test]
    fn test_known_layout_props_cuon_all_have_effect() {
        assert_prop_has_effect("cuon", "huong", str_val("doc"));
        assert_prop_has_effect("cuon", "cao", str_val("100"));
        assert_prop_has_effect("cuon", "rong", str_val("100"));
    }

    #[test]
    fn test_known_layout_props_can_giua_all_have_effect() {
        assert_prop_has_effect("can_giua", "max_rong", str_val("800"));
        assert_prop_has_effect("can_giua", "dem", str_val("8"));
    }

    #[test]
    fn test_known_layout_props_dinh_dau_all_have_effect() {
        assert_prop_has_effect("dinh_dau", "offset", str_val("10"));
    }

    #[test]
    fn test_known_layout_props_dinh_man_hinh_all_have_effect() {
        assert_prop_has_effect("dinh_man_hinh", "vi_tri", str_val("tren"));
        assert_prop_has_effect("dinh_man_hinh", "rong", str_val("100"));
        assert_prop_has_effect("dinh_man_hinh", "cao", str_val("100"));
    }

    #[test]
    fn test_known_layout_props_match_prop_spec_applicability() {
        // Tag-specific allowlists remain the source for exact layout-tag
        // applicability, but every identity in them must at least be a
        // layout-capable PropKey. This prevents the semantic registry and
        // layout codegen from drifting apart.
        for tag in [
            "flex", "grid", "khoi", "stack", "cuon", "can_giua",
            "lop", "dinh_dau", "dinh_man_hinh",
        ] {
            for key in known_layout_props(tag) {
                let spec = vibao_ast::prop_spec(*key);
                assert!(spec.applies_to_layout, "{:?} trong allowlist '{}' nhưng PropSpec không cho Layout", key, tag);
                assert!(!spec.responsive_only, "{:?} trong allowlist thường '{}' nhưng lại responsive_only", key, tag);
            }
        }
    }

    #[test]
    fn test_layout_unknown_keys_flags_typo() {
        // Direct evidence that BUG-25 IS FIXED: mistyping "magin" instead
        // of "le" on "khoi" is now DETECTED (previously it fell into
        // the silent `_ => {}` branch, with no warning).
        let props: PropsMap = vec![("magin".to_string(), str_val("8"))];
        let unknown = layout_unknown_keys("khoi", &props);
        assert_eq!(unknown, vec!["magin".to_string()]);
    }

    #[test]
    fn test_layout_unknown_keys_accepts_english_props() {
        let props: PropsMap = vec![
            ("width".to_string(), str_val("100")),
            ("background_color".to_string(), str_val("red")),
        ];
        assert!(layout_unknown_keys("flex", &props).is_empty());
    }

    #[test]
    fn test_layout_unknown_keys_still_flags_unknown_english_prop() {
        let props: PropsMap = vec![("widht".to_string(), str_val("100"))];
        assert_eq!(layout_unknown_keys("flex", &props), vec!["widht".to_string()]);
    }

    #[test]
    fn test_layout_unknown_keys_empty_for_all_known_props() {
        let props: PropsMap = vec![
            ("mau_nen".to_string(), str_val("trang")),
            ("rong".to_string(), str_val("100")),
            ("le".to_string(), str_val("8")),
        ];
        assert!(layout_unknown_keys("khoi", &props).is_empty());
    }

    #[test]
    fn test_layout_unknown_keys_lop_flags_everything() {
        // "lop" (layer) accepts no props — EVERY prop written on it
        // must be warned as unknown.
        let props: PropsMap = vec![("rong".to_string(), str_val("100"))];
        assert_eq!(layout_unknown_keys("lop", &props), vec!["rong".to_string()]);
    }

    /// Cross-check: a prop "known" for THIS tag must NOT bleed
    /// into ANOTHER tag if it is not actually valid there — catches the specific issue
    /// mentioned in the `known_layout_props()` doc-comment: "cot" is valid on
    /// grid but NOT valid on flex.
    #[test]
    fn test_grid_only_props_are_unknown_on_flex() {
        let props: PropsMap = vec![("cot".to_string(), str_val("2"))];
        assert_eq!(layout_unknown_keys("flex", &props), vec!["cot".to_string()]);
    }

    #[test]
    fn test_flex_only_props_are_unknown_on_grid() {
        let props: PropsMap = vec![("boc".to_string(), str_val("true"))];
        assert_eq!(layout_unknown_keys("grid", &props), vec!["boc".to_string()]);
    }

    // ── Tests for migrating to PropKey (this code pass, see AUDIT.md) —
    // changes match keys from raw String to PropKey in the 9 resolve_* functions
    // ──────────────────────────────────────────────────────────

    #[test]
    fn test_grid_alias_mau_chu_not_recognized_same_as_mau() {
        // "mau_chu" (alias of "mau", same PropKey::Color) must NOT be
        // recognized by resolve_grid() — exactly like "mau" is also NOT
        // recognized (Color is not in known_layout_props("grid"),
        // only "mau_nen" — BackgroundColor — is valid). Confirm
        // the PropKey migration does not accidentally make "mau"/"mau_chu" bleed into
        // the BackgroundColor branch because aliases were merged.
        let props: PropsMap = vec![("mau_chu".to_string(), str_val("do"))];
        let baseline = resolve_grid(&vec![]);
        let with_prop = resolve_grid(&props);
        let baseline_vec: Vec<(String, String)> = baseline.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
        let with_prop_vec: Vec<(String, String)> = with_prop.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
        assert_eq!(baseline_vec, with_prop_vec, "'mau_chu' không hợp lệ trên grid, CSS phải không đổi");
    }

    #[test]
    fn test_responsive_english_prop_is_not_false_positive() {
        let responsive = vec![ResponsiveNode {
            breakpoint: Breakpoint::DiDong,
            overrides: vec![
                ("width".to_string(), str_val("100")),
                ("height".to_string(), str_val("50")),
                ("hidden".to_string(), str_val("true")),
            ],
            pos: vibao_ast::Pos { line: 1, column: 1 },
        }];

        assert!(responsive_unknown_keys(&responsive).is_empty());
    }

    #[test]
    fn test_responsive_english_typo_is_still_unknown() {
        let responsive = vec![ResponsiveNode {
            breakpoint: Breakpoint::DiDong,
            overrides: vec![("widht".to_string(), str_val("100"))],
            pos: vibao_ast::Pos { line: 1, column: 1 },
        }];

        assert_eq!(responsive_unknown_keys(&responsive), vec!["widht".to_string()]);
    }

    #[test]
    fn test_responsive_css_passthrough_uses_original_key_not_inferred_from_propkey() {
        // Important regression: the passthrough branch (_ => ...) in
        // resolve_responsive_css() MUST use the ORIGINAL key from PropsMap
        // (camel_to_kebab(key)), NOT infer back from PropKey — if
        // inferred incorrectly (e.g. always printing "mau" even though the user typed "mau_chu"),
        // the arbitrary CSS property generated would have the wrong name compared with what the user typed.
        let responsive = vec![ResponsiveNode {
            breakpoint: Breakpoint::DiDong,
            overrides: vec![("mau_chu".to_string(), str_val("do"))],
            pos: vibao_ast::Pos { line: 1, column: 1 },
        }];
        let result = resolve_responsive_css("text", &responsive);
        assert_eq!(result.len(), 1);
        // "mau_chu" is not in RESPONSIVE_HANDLED_PROP_KEYS (7 names), so
        // it goes through passthrough — camel_to_kebab("mau_chu") preserves
        // "mau_chu" (already has underscores, not camelCase).
        assert!(result[0].overrides.iter().any(|(k, _)| k == "mau_chu"));
    }

    #[test]
    fn test_flex_default_display() {
        let props: PropsMap = vec![];
        let css = resolve_flex(&props);
        assert_eq!(css.get("display"), Some(&"flex".to_string()));
    }

    #[test]
    fn test_map_align_items_deu_maps_to_stretch() {
        // Direct regression test for the fixed bug: previously "deu" did not
        // match any case in map_align_items and was printed straight into CSS as
        // "align-items:deu" — an INVALID value, silently ignored by browsers,
        // causing wrong layout (found by test-building a real app).
        assert_eq!(map_align_items("deu"), "stretch");
    }

    #[test]
    fn test_map_align_items_known_values() {
        assert_eq!(map_align_items("start"), "flex-start");
        assert_eq!(map_align_items("end"), "flex-end");
        assert_eq!(map_align_items("center"), "center");
        assert_eq!(map_align_items("giua"), "center");
        assert_eq!(map_align_items("stretch"), "stretch");
    }

    #[test]
    fn test_map_align_items_unknown_passthrough() {
        // Unknown values NOT in the list of known Vietnamese mistakes
        // still pass through verbatim — allowing Devs to directly write another valid
        // CSS value (e.g. baseline) not explicitly listed yet.
        assert_eq!(map_align_items("baseline"), "baseline");
    }

    #[test]
    fn test_doc_prop_end_to_end_no_invalid_css_value() {
        // Higher-level regression test: simulates exactly how the real bug happened —
        // through resolve_flex() with prop "doc: deu" (not calling
        // map_align_items directly) — confirms generated CSS no longer contains "deu"
        // (invalid value), but is now "stretch".
        let props: PropsMap = vec![("doc".to_string(), Expr::literal_str("deu", p()))];
        let css = resolve_flex(&props);
        assert_eq!(css.get("alignItems"), Some(&"stretch".to_string()));
    }

    #[test]
    fn test_flex_huong_column() {
        let props: PropsMap = vec![("huong".to_string(), Expr::literal_str("column", p()))];
        let css = resolve_flex(&props);
        assert_eq!(css.get("flexDirection"), Some(&"column".to_string()));
    }

    #[test]
    fn test_grid_cot_numeric_becomes_repeat() {
        let props: PropsMap = vec![("cot".to_string(), Expr::literal_str("3", p()))];
        let css = resolve_grid(&props);
        assert_eq!(css.get("gridTemplateColumns"), Some(&"repeat(3, 1fr)".to_string()));
    }

    #[test]
    fn test_grid_cot_raw_value_kept() {
        let props: PropsMap = vec![("cot".to_string(), Expr::literal_str("1fr 2fr", p()))];
        let css = resolve_grid(&props);
        assert_eq!(css.get("gridTemplateColumns"), Some(&"1fr 2fr".to_string()));
    }

    #[test]
    fn test_box_border_uses_kieu_vien_and_mau_vien() {
        let props: PropsMap = vec![
            ("vien".to_string(), Expr::literal_num(2.0, p())),
            ("kieu_vien".to_string(), Expr::literal_str("dashed", p())),
            ("mau_vien".to_string(), Expr::Literal(vibao_ast::LiteralValue::Color("#FF0000".to_string()), p())),
        ];
        let css = resolve_box(&props);
        assert_eq!(css.get("border"), Some(&"2px dashed #FF0000".to_string()));
    }

    #[test]
    fn test_stack_forces_grid_1fr() {
        let props: PropsMap = vec![];
        let css = resolve_stack(&props);
        assert_eq!(css.get("gridTemplateColumns"), Some(&"1fr".to_string()));
        assert_eq!(css.get("gridTemplateRows"), Some(&"1fr".to_string()));
    }

    #[test]
    fn test_fixed_vi_tri_tren() {
        let props: PropsMap = vec![("vi_tri".to_string(), Expr::literal_str("tren", p()))];
        let css = resolve_fixed(&props);
        assert_eq!(css.get("top"), Some(&"0".to_string()));
        assert_eq!(css.get("left"), Some(&"0".to_string()));
        assert_eq!(css.get("right"), Some(&"0".to_string()));
    }

    #[test]
    fn test_build_media_query_empty_overrides_returns_empty_string() {
        let bp_css = ResponsiveCss {
            media_condition: "(max-width: 639px)".to_string(),
            overrides: LayoutCss::new(),
        };
        assert_eq!(build_media_query("#foo", &bp_css), "");
    }

    #[test]
    fn test_build_media_query_with_overrides() {
        let mut overrides = LayoutCss::new();
        overrides.insert("display".to_string(), "none".to_string());
        let bp_css = ResponsiveCss {
            media_condition: "(max-width: 639px)".to_string(),
            overrides,
        };
        let out = build_media_query("#vb-box-1", &bp_css);
        assert!(out.contains("@media (max-width: 639px)"));
        assert!(out.contains("#vb-box-1"));
        assert!(out.contains("display: none;"));
    }

    #[test]
    fn test_px_helper() {
        assert_eq!(px("16"), "16px");
        assert_eq!(px("-16"), "-16px");
        assert_eq!(px("50%"), "50%");
        assert_eq!(px("__dynamic__"), "__dynamic__");
    }

    #[test]
    fn test_px_helper_rejects_dash_in_middle() {
        // Regression test for the same bug fixed in props.rs — see the note at
        // is_plain_number().
        assert_eq!(px("1-2"), "1-2");
    }

    #[test]
    fn test_size_helper_rejects_negative() {
        // size() does not accept negative numbers; this is the helper's old contract.
        assert_eq!(size("16"), "16px");
        assert_eq!(size("-16"), "-16");
    }

    #[test]
    fn test_size_helper_rejects_malformed_decimals() {
        assert_eq!(size("1.25"), "1.25px");
        assert_eq!(size(".5"), ".5px");
        assert_eq!(size("1.2.3"), "1.2.3");
        assert_eq!(size("."), ".");
        assert_eq!(size(".."), "..");
    }

    #[test]
    fn test_repeat_or_raw_requires_positive_integer() {
        assert_eq!(repeat_or_raw("3"), "repeat(3, 1fr)");
        assert_eq!(repeat_or_raw("0"), "0");
        assert_eq!(repeat_or_raw("1.5"), "1.5");
        assert_eq!(repeat_or_raw("abc"), "abc");
    }

    #[test]
    fn test_is_layout_tag() {
        assert!(is_layout_tag("flex"));
        assert!(is_layout_tag("khoi"));
        assert!(!is_layout_tag("text"));
        assert!(!is_layout_tag("button"));
    }

    #[test]
    fn test_is_layout_tag_semantic_matches_string_based_for_every_known_tag() {
        // FULL cross-check: for EVERY valid tag name (going through
        // locale::vi::tag_name_vi() to obtain Tag), is_layout_tag_semantic()
        // (NEW source, based on vibao_ast::semantic::tag_spec()) must give
        // EXACTLY THE SAME result as is_layout_tag() (OLD source, String-based) —
        // this is a REQUIRED confirmation step before considering replacing
        // the old path, following the "old path -> new path -> test
        // -> pass, then migrate" principle. The name list is taken directly from
        // locale::vi (not hand-copied), so if locale/registry drift
        // anywhere, this test catches it immediately.
        let all_names = [
            "text", "h1", "h2", "h3", "p", "nhan",
            "image", "video", "icon",
            "button", "input", "link", "lien_ket",
            "flex", "grid", "stack", "khoi", "cuon", "can_giua", "lop", "dinh_dau", "dinh_man_hinh",
            "khoang_cach", "duong_ke",
            "form", "nhom_input", "chon_mot", "hop_kiem", "lua_chon",
            "modal", "tabs", "gap_mo", "bang_chuyen", "xuong_trang", "vong_quay",
            "thanh_tien_trinh", "bang", "bieu_do", "ban_do", "thanh_dieu_huong", "trinh_soan_thao",
        ];
        for name in all_names {
            let tag = crate::locale::vi::tag_name_vi(name)
                .unwrap_or_else(|| panic!("locale::vi::tag_name_vi(\"{}\") trả None — thiếu entry", name));
            let old_result = is_layout_tag(name);
            let new_result = is_layout_tag_semantic(tag);
            assert_eq!(
                old_result, new_result,
                "LỆCH cho tag '{}': is_layout_tag()={} nhưng is_layout_tag_semantic()={}",
                name, old_result, new_result
            );
        }
    }
}
