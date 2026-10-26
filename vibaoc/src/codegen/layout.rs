use crate::codegen::css::OrderedMap;
use crate::codegen::expr::get_static_value;
use vibao_ast::{Breakpoint, PropKey, PropsMap, ResponsiveNode, Tag};

pub fn is_layout_tag_semantic(tag: Tag) -> bool {
    vibao_ast::semantic::tag_spec(tag).kind == vibao_ast::semantic::TagKind::Layout
}

pub type LayoutCss = OrderedMap;

pub fn resolve_layout_css(tag: Tag, props: &PropsMap) -> LayoutCss {
    match tag {
        Tag::Flex => resolve_flex(props),
        Tag::Grid => resolve_grid(props),
        Tag::Khoi => resolve_box(props),
        Tag::Stack => resolve_stack(props),
        Tag::Cuon => resolve_scroll(props),
        Tag::CanGiua => resolve_container(props),
        Tag::Lop => resolve_layer(),
        Tag::DinhDau => resolve_sticky_top(props),
        Tag::DinhManHinh => resolve_fixed(props),
        _ => {
            let mut css = LayoutCss::new();
            css.insert("display".to_string(), "block".to_string());
            css
        }
    }
}

const ALL_LAYOUT_TAGS: [Tag; 9] = [
    Tag::Flex,
    Tag::Grid,
    Tag::Khoi,
    Tag::Stack,
    Tag::Cuon,
    Tag::CanGiua,
    Tag::Lop,
    Tag::DinhDau,
    Tag::DinhManHinh,
];

pub fn tags_where_layout_prop_applies(key: PropKey) -> Vec<Tag> {
    if matches!(key, PropKey::BorderColor | PropKey::BorderStyle) {
        let mut tags: Vec<Tag> = ALL_LAYOUT_TAGS
            .iter()
            .copied()
            .filter(|tag| known_layout_props(*tag).contains(&key))
            .collect();
        tags.push(Tag::Khoi);
        return tags;
    }
    ALL_LAYOUT_TAGS
        .iter()
        .copied()
        .filter(|tag| known_layout_props(*tag).contains(&key))
        .collect()
}

fn known_layout_props(tag: Tag) -> &'static [PropKey] {
    match tag {
        Tag::Flex => &[
            PropKey::Direction,
            PropKey::Gap,
            PropKey::RowGap,
            PropKey::ColumnGap,
            PropKey::Align,
            PropKey::AlignItems,
            PropKey::Wrap,
            PropKey::Width,
            PropKey::Height,
            PropKey::Padding,
            PropKey::BackgroundColor,
            PropKey::Radius,
        ],
        Tag::Grid => &[
            PropKey::Columns,
            PropKey::Rows,
            PropKey::Gap,
            PropKey::RowGap,
            PropKey::ColumnGap,
            PropKey::Width,
            PropKey::Padding,
            PropKey::BackgroundColor,
        ],
        Tag::Khoi => &[
            PropKey::BackgroundColor,
            PropKey::Width,
            PropKey::Height,
            PropKey::MinWidth,
            PropKey::MaxWidth,
            PropKey::MinHeight,
            PropKey::MaxHeight,
            PropKey::Radius,
            PropKey::Padding,
            PropKey::Margin,
            PropKey::Border,
            PropKey::Shadow,
            PropKey::Overflow,
            PropKey::TranslateX,
            PropKey::TranslateY,
            PropKey::ZIndex,
        ],
        Tag::Stack => &[
            PropKey::Align,
            PropKey::AlignItems,
            PropKey::Width,
            PropKey::Height,
        ],
        Tag::Cuon => &[PropKey::Direction, PropKey::Height, PropKey::Width],
        Tag::CanGiua => &[PropKey::MaxWidth, PropKey::Padding],
        Tag::Lop => &[],
        Tag::DinhDau => &[PropKey::Offset],
        Tag::DinhManHinh => &[PropKey::Position, PropKey::Width, PropKey::Height],
        _ => &[],
    }
}

pub fn layout_unknown_keys(tag: Tag, props: &PropsMap) -> Vec<String> {
    let known = known_layout_props(tag);
    props
        .iter()
        .filter(
            |(key, _)| match crate::locale::resolve_prop_key(key.as_str()) {
                Some(PropKey::HtmlId) => false,
                Some(PropKey::BorderColor) | Some(PropKey::BorderStyle) if tag == Tag::Khoi => {
                    false
                }
                Some(prop_key) => !known.contains(&prop_key),
                None => true,
            },
        )
        .map(|(key, _)| key.clone())
        .collect()
}

fn resolve_flex(props: &PropsMap) -> LayoutCss {
    let mut css = LayoutCss::new();
    css.insert("display".to_string(), "flex".to_string());
    for (key, expr) in props {
        let v = get_static_value(expr);
        match crate::locale::resolve_prop_key(key.as_str()) {
            Some(PropKey::Direction) => {
                css.insert("flexDirection".to_string(), map_direction(&v).to_string());
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
    match (translate_x, translate_y) {
        (Some(x), Some(y)) => {
            css.insert(
                "transform".to_string(),
                format!("translateX({}) translateY({})", x, y),
            );
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
                    if v == "ngang" {
                        "auto".to_string()
                    } else {
                        "hidden".to_string()
                    },
                );
                css.insert(
                    "overflowY".to_string(),
                    if v == "doc" {
                        "auto".to_string()
                    } else {
                        "hidden".to_string()
                    },
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
                        overrides
                            .insert("flex-direction".to_string(), map_direction(&v).to_string());
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
                    Some(PropKey::Gap) => {
                        overrides.insert("gap".to_string(), px(&v));
                    }
                    Some(PropKey::MaxWidth) => {
                        overrides.insert("max-width".to_string(), size(&v));
                    }
                    Some(PropKey::Margin) => {
                        overrides.insert("margin".to_string(), spacing(&v));
                    }
                    Some(PropKey::Hidden) => {
                        if v == "true" {
                            overrides.insert("display".to_string(), "none".to_string());
                        }
                    }
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

pub const RESPONSIVE_HANDLED_PROP_KEYS: [PropKey; 10] = [
    PropKey::Columns,
    PropKey::Direction,
    PropKey::FontSize,
    PropKey::Width,
    PropKey::Height,
    PropKey::Padding,
    PropKey::Hidden,
    PropKey::Gap,
    PropKey::MaxWidth,
    PropKey::Margin,
];

pub fn responsive_prop_is_known(key: &str) -> bool {
    match crate::locale::resolve_prop_key(key) {
        Some(prop) => RESPONSIVE_HANDLED_PROP_KEYS.contains(&prop),
        None => false,
    }
}

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

fn dedup_keep_order(v: &mut Vec<String>) {
    let mut seen = std::collections::HashSet::new();
    v.retain(|item| seen.insert(item.clone()));
}

pub fn build_media_query(selector: &str, bp_css: &ResponsiveCss) -> String {
    if bp_css.overrides.is_empty() {
        return String::new();
    }
    let rules = bp_css
        .overrides
        .iter()
        .map(|(k, v)| format!("    {}: {} !important;", k, v))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "@media {} {{\n  {} {{\n{}\n  }}\n}}",
        bp_css.media_condition, selector, rules
    )
}

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
        .find(|(k, _)| crate::locale::resolve_prop_key(k.as_str()) == Some(PropKey::BorderStyle))
        .map(|(_, e)| get_static_value(e))
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "solid".to_string());
    let color = props
        .iter()
        .find(|(k, _)| crate::locale::resolve_prop_key(k.as_str()) == Some(PropKey::BorderColor))
        .map(|(_, e)| get_static_value(e))
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "#000".to_string());
    format!("{} {} {}", width, style, color)
}

fn repeat_or_raw(val: &str) -> String {
    if !val.is_empty() && val.chars().all(|c| c.is_ascii_digit()) && val != "0" {
        format!("repeat({}, 1fr)", val)
    } else {
        val.to_string()
    }
}

fn is_plain_number(val: &str) -> bool {
    if val.is_empty() {
        return false;
    }
    let rest = val.strip_prefix('-').unwrap_or(val);
    is_unsigned_decimal(rest)
}

fn is_unsigned_decimal(val: &str) -> bool {
    !val.is_empty()
        && val.chars().any(|c| c.is_ascii_digit())
        && val.chars().filter(|&c| c == '.').count() <= 1
        && val.chars().all(|c| c.is_ascii_digit() || c == '.')
}

pub fn map_direction(val: &str) -> &'static str {
    match val {
        "column" | "cot" | "doc" => "column",
        _ => "row",
    }
}

pub fn map_justify(val: &str) -> String {
    match val {
        "start" | "trai" => "flex-start".to_string(),
        "end" | "phai" => "flex-end".to_string(),
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
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vibao_ast::{Expr, Pos};

    fn p() -> Pos {
        Pos { line: 1, column: 1 }
    }

    fn assert_prop_has_effect(tag: Tag, key: &str, value: Expr) {
        let baseline = resolve_layout_css(tag, &vec![]);
        let with_prop = resolve_layout_css(tag, &vec![(key.to_string(), value)]);
        let baseline_vec: Vec<(String, String)> = baseline
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        let with_prop_vec: Vec<(String, String)> = with_prop
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        assert_ne!(
            baseline_vec, with_prop_vec,
            "prop '{}' được khai 'known' cho tag {:?} nhưng KHÔNG làm CSS thay đổi — \
             known_layout_props() có thể đã lệch khỏi resolve_{:?}() thật",
            key, tag, tag
        );
    }

    fn str_val(s: &str) -> Expr {
        Expr::literal_str(s, p())
    }

    #[test]
    fn map_justify_nhan_trai_phai_tieng_viet() {
        assert_eq!(map_justify("trai"), "flex-start");
        assert_eq!(map_justify("phai"), "flex-end");
        assert_eq!(map_justify("giua"), "center");
        assert_eq!(map_justify("baseline"), "baseline");
    }

    #[test]
    fn test_known_layout_props_flex_all_have_effect() {
        assert_prop_has_effect(Tag::Flex, "huong", str_val("column"));
        assert_prop_has_effect(Tag::Flex, "gap", str_val("8"));
        assert_prop_has_effect(Tag::Flex, "gap_doc", str_val("8"));
        assert_prop_has_effect(Tag::Flex, "gap_ngang", str_val("8"));
        assert_prop_has_effect(Tag::Flex, "can", str_val("giua"));
        assert_prop_has_effect(Tag::Flex, "doc", str_val("giua"));
        assert_prop_has_effect(Tag::Flex, "boc", str_val("true"));
        assert_prop_has_effect(Tag::Flex, "rong", str_val("100"));
        assert_prop_has_effect(Tag::Flex, "cao", str_val("100"));
        assert_prop_has_effect(Tag::Flex, "dem", str_val("8"));
        assert_prop_has_effect(Tag::Flex, "mau_nen", str_val("trang"));
        assert_prop_has_effect(Tag::Flex, "radius", str_val("8"));
    }

    #[test]
    fn test_known_layout_props_grid_all_have_effect() {
        assert_prop_has_effect(Tag::Grid, "cot", str_val("2"));
        assert_prop_has_effect(Tag::Grid, "hang_luoi", str_val("2"));
        assert_prop_has_effect(Tag::Grid, "gap", str_val("8"));
        assert_prop_has_effect(Tag::Grid, "gap_doc", str_val("8"));
        assert_prop_has_effect(Tag::Grid, "gap_ngang", str_val("8"));
        assert_prop_has_effect(Tag::Grid, "rong", str_val("100"));
        assert_prop_has_effect(Tag::Grid, "dem", str_val("8"));
        assert_prop_has_effect(Tag::Grid, "mau_nen", str_val("trang"));
    }

    #[test]
    fn test_known_layout_props_khoi_all_have_effect() {
        assert_prop_has_effect(Tag::Khoi, "mau_nen", str_val("trang"));
        assert_prop_has_effect(Tag::Khoi, "rong", str_val("100"));
        assert_prop_has_effect(Tag::Khoi, "cao", str_val("100"));
        assert_prop_has_effect(Tag::Khoi, "min_rong", str_val("10"));
        assert_prop_has_effect(Tag::Khoi, "max_rong", str_val("200"));
        assert_prop_has_effect(Tag::Khoi, "min_cao", str_val("10"));
        assert_prop_has_effect(Tag::Khoi, "max_cao", str_val("200"));
        assert_prop_has_effect(Tag::Khoi, "radius", str_val("8"));
        assert_prop_has_effect(Tag::Khoi, "dem", str_val("8"));
        assert_prop_has_effect(Tag::Khoi, "le", str_val("8"));
        assert_prop_has_effect(Tag::Khoi, "vien", str_val("1"));
        assert_prop_has_effect(Tag::Khoi, "bong", str_val("true"));
        assert_prop_has_effect(Tag::Khoi, "cuon_tran", str_val("an"));
        assert_prop_has_effect(Tag::Khoi, "tran_x", str_val("10"));
        assert_prop_has_effect(Tag::Khoi, "tran_y", str_val("10"));
        assert_prop_has_effect(Tag::Khoi, "tang_z", str_val("5"));
    }

    #[test]
    fn test_translate_x_and_y_together_both_present_in_transform() {
        let props: PropsMap = vec![
            ("tran_x".to_string(), str_val("10")),
            ("tran_y".to_string(), str_val("20")),
        ];
        let css = resolve_box(&props);
        let transform = css.get("transform").expect("phải có key 'transform'");
        assert!(
            transform.contains("translateX(10px)"),
            "thiếu translateX trong: {}",
            transform
        );
        assert!(
            transform.contains("translateY(20px)"),
            "thiếu translateY trong: {}",
            transform
        );
    }

    #[test]
    fn test_translate_x_and_y_together_order_independent() {
        let props: PropsMap = vec![
            ("tran_y".to_string(), str_val("20")),
            ("tran_x".to_string(), str_val("10")),
        ];
        let css = resolve_box(&props);
        let transform = css.get("transform").expect("phải có key 'transform'");
        assert!(
            transform.contains("translateX(10px)"),
            "thiếu translateX trong: {}",
            transform
        );
        assert!(
            transform.contains("translateY(20px)"),
            "thiếu translateY trong: {}",
            transform
        );
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
        assert_prop_has_effect(Tag::Stack, "can", str_val("giua"));
        assert_prop_has_effect(Tag::Stack, "doc", str_val("giua"));
        assert_prop_has_effect(Tag::Stack, "rong", str_val("100"));
        assert_prop_has_effect(Tag::Stack, "cao", str_val("100"));
    }

    #[test]
    fn test_known_layout_props_cuon_all_have_effect() {
        assert_prop_has_effect(Tag::Cuon, "huong", str_val("doc"));
        assert_prop_has_effect(Tag::Cuon, "cao", str_val("100"));
        assert_prop_has_effect(Tag::Cuon, "rong", str_val("100"));
    }

    #[test]
    fn test_known_layout_props_can_giua_all_have_effect() {
        assert_prop_has_effect(Tag::CanGiua, "max_rong", str_val("800"));
        assert_prop_has_effect(Tag::CanGiua, "dem", str_val("8"));
    }

    #[test]
    fn test_known_layout_props_dinh_dau_all_have_effect() {
        assert_prop_has_effect(Tag::DinhDau, "offset", str_val("10"));
    }

    #[test]
    fn test_known_layout_props_dinh_man_hinh_all_have_effect() {
        assert_prop_has_effect(Tag::DinhManHinh, "vi_tri", str_val("tren"));
        assert_prop_has_effect(Tag::DinhManHinh, "rong", str_val("100"));
        assert_prop_has_effect(Tag::DinhManHinh, "cao", str_val("100"));
    }

    #[test]
    fn test_known_layout_props_match_prop_spec_applicability() {
        for tag in ALL_LAYOUT_TAGS {
            for key in known_layout_props(tag) {
                let spec = vibao_ast::prop_spec(*key);
                assert!(
                    spec.applies_to_layout,
                    "{:?} trong allowlist {:?} nhưng PropSpec không cho Layout",
                    key, tag
                );
                assert!(
                    !spec.responsive_only,
                    "{:?} trong allowlist thường {:?} nhưng lại responsive_only",
                    key, tag
                );
            }
        }
    }

    fn layout_capable_prop_surface_name(key: PropKey) -> Option<&'static str> {
        match key {
            PropKey::BackgroundColor => Some("mau_nen"),
            PropKey::Width => Some("rong"),
            PropKey::Height => Some("cao"),
            PropKey::MinWidth => Some("min_rong"),
            PropKey::MaxWidth => Some("max_rong"),
            PropKey::MinHeight => Some("min_cao"),
            PropKey::MaxHeight => Some("max_cao"),
            PropKey::Radius => Some("radius"),
            PropKey::Padding => Some("dem"),
            PropKey::Margin => Some("le"),
            PropKey::Border => Some("vien"),
            PropKey::Shadow => Some("bong"),
            PropKey::Overflow => Some("cuon_tran"),
            PropKey::ZIndex => Some("tang_z"),
            PropKey::Direction => Some("huong"),
            PropKey::Gap => Some("gap"),
            PropKey::RowGap => Some("gap_doc"),
            PropKey::ColumnGap => Some("gap_ngang"),
            PropKey::Align => Some("can"),
            PropKey::AlignItems => Some("doc"),
            PropKey::Wrap => Some("boc"),
            PropKey::Columns => Some("cot"),
            PropKey::Rows => Some("hang_luoi"),
            PropKey::TranslateX => Some("tran_x"),
            PropKey::TranslateY => Some("tran_y"),
            PropKey::Position => Some("vi_tri"),
            PropKey::Offset => Some("offset"),
            _ => None,
        }
    }

    #[test]
    fn test_layout_resolve_fns_never_react_to_undeclared_props() {
        for tag in ALL_LAYOUT_TAGS {
            let known = known_layout_props(tag);
            for key in [
                PropKey::BackgroundColor,
                PropKey::Width,
                PropKey::Height,
                PropKey::MinWidth,
                PropKey::MaxWidth,
                PropKey::MinHeight,
                PropKey::MaxHeight,
                PropKey::Radius,
                PropKey::Padding,
                PropKey::Margin,
                PropKey::Border,
                PropKey::Shadow,
                PropKey::Overflow,
                PropKey::ZIndex,
                PropKey::Direction,
                PropKey::Gap,
                PropKey::RowGap,
                PropKey::ColumnGap,
                PropKey::Align,
                PropKey::AlignItems,
                PropKey::Wrap,
                PropKey::Columns,
                PropKey::Rows,
                PropKey::TranslateX,
                PropKey::TranslateY,
                PropKey::Position,
                PropKey::Offset,
            ] {
                if known.contains(&key) {
                    continue;
                }
                let Some(surface_name) = layout_capable_prop_surface_name(key) else {
                    continue;
                };
                let baseline = resolve_layout_css(tag, &vec![]);
                let with_prop =
                    resolve_layout_css(tag, &vec![(surface_name.to_string(), str_val("8"))]);
                let baseline_vec: Vec<(String, String)> = baseline
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();
                let with_prop_vec: Vec<(String, String)> = with_prop
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();
                assert_eq!(
                    baseline_vec, with_prop_vec,
                    "resolve_layout_css({:?}, ...) THAY ĐỔI CSS khi set '{}' ({:?}), \
                     nhưng known_layout_props({:?}) KHÔNG khai báo prop này — \
                     warning message sẽ báo sai '{}' là prop lạ dù nó thực ra có tác dụng",
                    tag, surface_name, key, tag, surface_name
                );
            }
        }
    }

    #[test]
    fn test_layout_unknown_keys_flags_typo() {
        let props: PropsMap = vec![("magin".to_string(), str_val("8"))];
        let unknown = layout_unknown_keys(Tag::Khoi, &props);
        assert_eq!(unknown, vec!["magin".to_string()]);
    }

    #[test]
    fn test_layout_unknown_keys_accepts_english_props() {
        let props: PropsMap = vec![
            ("width".to_string(), str_val("100")),
            ("background_color".to_string(), str_val("red")),
        ];
        assert!(layout_unknown_keys(Tag::Flex, &props).is_empty());
    }

    #[test]
    fn test_layout_unknown_keys_still_flags_unknown_english_prop() {
        let props: PropsMap = vec![("widht".to_string(), str_val("100"))];
        assert_eq!(
            layout_unknown_keys(Tag::Flex, &props),
            vec!["widht".to_string()]
        );
    }

    #[test]
    fn test_layout_unknown_keys_empty_for_all_known_props() {
        let props: PropsMap = vec![
            ("mau_nen".to_string(), str_val("trang")),
            ("rong".to_string(), str_val("100")),
            ("le".to_string(), str_val("8")),
        ];
        assert!(layout_unknown_keys(Tag::Khoi, &props).is_empty());
    }

    #[test]
    fn test_layout_unknown_keys_lop_flags_everything() {
        let props: PropsMap = vec![("rong".to_string(), str_val("100"))];
        assert_eq!(
            layout_unknown_keys(Tag::Lop, &props),
            vec!["rong".to_string()]
        );
    }

    #[test]
    fn test_grid_only_props_are_unknown_on_flex() {
        let props: PropsMap = vec![("cot".to_string(), str_val("2"))];
        assert_eq!(
            layout_unknown_keys(Tag::Flex, &props),
            vec!["cot".to_string()]
        );
    }

    #[test]
    fn test_flex_only_props_are_unknown_on_grid() {
        let props: PropsMap = vec![("boc".to_string(), str_val("true"))];
        assert_eq!(
            layout_unknown_keys(Tag::Grid, &props),
            vec!["boc".to_string()]
        );
    }

    #[test]
    fn test_grid_alias_mau_chu_not_recognized_same_as_mau() {
        let props: PropsMap = vec![("mau_chu".to_string(), str_val("do"))];
        let baseline = resolve_grid(&vec![]);
        let with_prop = resolve_grid(&props);
        let baseline_vec: Vec<(String, String)> = baseline
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        let with_prop_vec: Vec<(String, String)> = with_prop
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        assert_eq!(
            baseline_vec, with_prop_vec,
            "'mau_chu' không hợp lệ trên grid, CSS phải không đổi"
        );
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
    fn test_responsive_gap_max_width_and_margin_get_units() {
        let responsive = vec![ResponsiveNode {
            breakpoint: Breakpoint::DiDong,
            overrides: vec![
                ("gap".to_string(), str_val("12")),
                ("max_rong".to_string(), str_val("320")),
                ("le".to_string(), str_val("8 16")),
            ],
            pos: vibao_ast::Pos { line: 1, column: 1 },
        }];
        assert!(responsive_unknown_keys(&responsive).is_empty());
        let result = resolve_responsive_css("flex", &responsive);
        let css = &result[0].overrides;
        assert_eq!(css.get("gap"), Some(&"12px".to_string()));
        assert_eq!(css.get("max-width"), Some(&"320px".to_string()));
        assert_eq!(css.get("margin"), Some(&"8px 16px".to_string()));
    }

    #[test]
    fn test_responsive_direction_accepts_vietnamese_values() {
        let responsive = vec![ResponsiveNode {
            breakpoint: Breakpoint::DiDong,
            overrides: vec![("huong".to_string(), str_val("cot"))],
            pos: vibao_ast::Pos { line: 1, column: 1 },
        }];
        let result = resolve_responsive_css("flex", &responsive);
        let css = &result[0].overrides;
        assert_eq!(css.get("flex-direction"), Some(&"column".to_string()));
    }

    #[test]
    fn test_responsive_english_typo_is_still_unknown() {
        let responsive = vec![ResponsiveNode {
            breakpoint: Breakpoint::DiDong,
            overrides: vec![("widht".to_string(), str_val("100"))],
            pos: vibao_ast::Pos { line: 1, column: 1 },
        }];

        assert_eq!(
            responsive_unknown_keys(&responsive),
            vec!["widht".to_string()]
        );
    }

    #[test]
    fn test_responsive_css_passthrough_uses_original_key_not_inferred_from_propkey() {
        let responsive = vec![ResponsiveNode {
            breakpoint: Breakpoint::DiDong,
            overrides: vec![("mau_chu".to_string(), str_val("do"))],
            pos: vibao_ast::Pos { line: 1, column: 1 },
        }];
        let result = resolve_responsive_css("text", &responsive);
        assert_eq!(result.len(), 1);
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
        assert_eq!(map_align_items("baseline"), "baseline");
    }

    #[test]
    fn test_doc_prop_end_to_end_no_invalid_css_value() {
        let props: PropsMap = vec![("doc".to_string(), Expr::literal_str("deu", p()))];
        let css = resolve_flex(&props);
        assert_eq!(css.get("alignItems"), Some(&"stretch".to_string()));
    }

    #[test]
    fn test_map_direction_accepts_vietnamese_and_css_names() {
        for v in ["column", "cot", "doc"] {
            assert_eq!(map_direction(v), "column", "{}", v);
        }
        for v in ["row", "hang", "ngang", "khong_biet", ""] {
            assert_eq!(map_direction(v), "row", "{}", v);
        }
    }

    #[test]
    fn test_flex_huong_cot_is_column() {
        let props: PropsMap = vec![("huong".to_string(), Expr::literal_str("cot", p()))];
        let css = resolve_flex(&props);
        assert_eq!(css.get("flexDirection"), Some(&"column".to_string()));
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
        assert_eq!(
            css.get("gridTemplateColumns"),
            Some(&"repeat(3, 1fr)".to_string())
        );
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
            (
                "mau_vien".to_string(),
                Expr::Literal(vibao_ast::LiteralValue::Color("#FF0000".to_string()), p()),
            ),
        ];
        let css = resolve_box(&props);
        assert_eq!(css.get("border"), Some(&"2px dashed #FF0000".to_string()));
    }

    #[test]
    fn test_box_border_uses_english_border_style_and_border_color() {
        let props: PropsMap = vec![
            ("vien".to_string(), Expr::literal_num(2.0, p())),
            ("border_style".to_string(), Expr::literal_str("dashed", p())),
            (
                "border_color".to_string(),
                Expr::Literal(vibao_ast::LiteralValue::Color("#FF0000".to_string()), p()),
            ),
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
        assert!(out.contains("display: none !important;"));
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
        assert_eq!(px("1-2"), "1-2");
    }

    #[test]
    fn test_size_helper_rejects_negative() {
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
    fn test_tags_where_layout_prop_applies() {
        let tags = tags_where_layout_prop_applies(PropKey::Direction);
        assert!(tags.contains(&Tag::Flex));
        assert!(tags.contains(&Tag::Cuon));
        assert!(!tags.contains(&Tag::Khoi));
    }

    #[test]
    fn test_mau_vien_and_kieu_vien_are_not_flagged_unknown_on_khoi() {
        let props: PropsMap = vec![
            ("mau_vien".to_string(), str_val("do")),
            ("kieu_vien".to_string(), str_val("dashed")),
        ];
        assert_eq!(layout_unknown_keys(Tag::Khoi, &props), Vec::<String>::new());
    }

    #[test]
    fn test_tags_where_layout_prop_applies_includes_khoi_for_border_color() {
        let tags = tags_where_layout_prop_applies(PropKey::BorderColor);
        assert!(tags.contains(&Tag::Khoi));
    }
}
