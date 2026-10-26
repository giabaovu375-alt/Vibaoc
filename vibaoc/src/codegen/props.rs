use vibao_ast::{prop_spec, PropsMap, PropKey, Tag};
use crate::codegen::css::OrderedMap;
use crate::codegen::expr::{register_expr, resolve_value, ResolvedValue};
use crate::codegen::layout::{map_align_items, map_direction, map_justify};

#[derive(Debug, Clone, Default)]
pub struct ExpandedProps {
    pub style: OrderedMap,
    pub attrs: OrderedMap,
    pub dynamic: OrderedMap,
    pub dynamic_attrs: OrderedMap,
    pub class_bindings: Vec<String>,
    pub model: Option<String>,
    pub unknown_keys: Vec<String>,
    pub warnings: Vec<String>,
}

fn is_known_simple_prop(surface_name: &str) -> bool {
    crate::locale::resolve_prop_key(surface_name)
        .map(|key| prop_spec(key).applies_to_simple)
        .unwrap_or(false)
}

fn is_spa_target(value: &str) -> bool {
    value.starts_with('#') || (value.starts_with('/') && !value.starts_with("//"))
}

pub fn expand_props(tag: Tag, props: &PropsMap) -> ExpandedProps {
    let mut out = ExpandedProps::default();

    for (key, expr) in props {
        let resolved = resolve_value(expr);
        let is_dynamic = resolved.is_dynamic();
        let css_val = resolved.as_css();

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
            Some(PropKey::PreserveLines) => {
                if is_dynamic {
                    out.warnings.push(
                        "prop 'giu_dong' với giá trị biểu thức động chưa được hỗ trợ — cùng giới \
                         hạn của 'dam'. Dùng giá trị tĩnh."
                            .to_string(),
                    );
                } else if css_val == "true" {
                    out.style.insert("whiteSpace".to_string(), "pre-wrap".to_string());
                    out.style.insert("overflowWrap".to_string(), "anywhere".to_string());
                }
            }
            Some(PropKey::Align) => {
                if is_dynamic {
                    out.warnings.push(format!(
                        "prop 'can' với giá trị biểu thức động chưa được hỗ trợ — cơ chế binding \
                         hiện tại không thể chuyển tên căn chỉnh tiếng Việt (vd \"giua\") thành \
                         CSS keyword tương ứng (vd \"center\") lúc runtime. Style sẽ KHÔNG được áp \
                         dụng. Dùng giá trị tĩnh, hoặc bọc trong 'neu/khong_thi' ở cấp element.",
                    ));
                } else if matches!(tag, Tag::Text | Tag::H1 | Tag::H2 | Tag::H3 | Tag::P | Tag::Nhan) {
                    out.style.insert("textAlign".to_string(), map_align(&css_val));
                } else {
                    out.style.insert("justifyContent".to_string(), map_justify(&css_val));
                }
            }
            Some(PropKey::LineHeight) => {
                if is_dynamic {
                    out.dynamic.insert("lineHeight".to_string(), register_expr(expr.clone()).to_string());
                } else {
                    let value = match expr {
                        vibao_ast::Expr::Literal(vibao_ast::LiteralValue::Num(n, None), _)
                            if *n > 0.0 && *n <= 5.0 =>
                        {
                            crate::codegen::expr::format_number(*n)
                        }
                        _ => css_val,
                    };
                    out.style.insert("lineHeight".to_string(), value);
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
                if is_dynamic {
                    out.dynamic.insert("flexDirection".to_string(), register_expr(expr.clone()).to_string());
                } else {
                    out.style.insert("display".to_string(), "flex".to_string());
                    out.style.insert("flexDirection".to_string(), map_direction(&css_val).to_string());
                }
            }
            Some(PropKey::Gap) => {
                if is_dynamic {
                    out.dynamic.insert("gap".to_string(), register_expr(expr.clone()).to_string());
                } else {
                    out.style.insert("display".to_string(), "flex".to_string());
                    out.style.insert("gap".to_string(), ensure_px(&css_val));
                }
            }
            Some(PropKey::AlignItems) => {
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
                if is_dynamic {
                    out.dynamic_attrs.insert("href".to_string(), register_expr(expr.clone()).to_string());
                    continue;
                }
                out.attrs.insert("href".to_string(), css_val.clone());
                if is_spa_target(&css_val) {
                    out.attrs.insert("data-vb-link".to_string(), css_val);
                }
            }
            Some(PropKey::Target) => {
                if is_dynamic {
                    out.dynamic_attrs.insert("target".to_string(), register_expr(expr.clone()).to_string());
                } else {
                    out.attrs.insert("target".to_string(), css_val);
                }
            }
            Some(PropKey::Rel) => {
                if is_dynamic {
                    out.dynamic_attrs.insert("rel".to_string(), register_expr(expr.clone()).to_string());
                } else {
                    out.attrs.insert("rel".to_string(), css_val);
                }
            }
            Some(PropKey::HtmlId) => {
            }
            Some(PropKey::Required) => {
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
                let is_plain_variable = matches!(expr, vibao_ast::Expr::Variable(_, _));
                if matches!(tag, Tag::Input | Tag::Textarea | Tag::LuaChon | Tag::ChonMot | Tag::HopKiem) && is_plain_variable {
                    if let vibao_ast::Expr::Variable(name, _) = expr {
                        out.model = Some(name.clone());
                    }
                } else if is_dynamic {
                    out.dynamic_attrs.insert("value".to_string(), register_expr(expr.clone()).to_string());
                } else {
                    out.attrs.insert("value".to_string(), css_val);
                }
            }
            Some(PropKey::OptionValue) => {
                if is_dynamic {
                    out.dynamic_attrs.insert("value".to_string(), register_expr(expr.clone()).to_string());
                } else {
                    out.attrs.insert("value".to_string(), css_val);
                }
            }
            Some(PropKey::Animation) | Some(PropKey::Duration) | Some(PropKey::Delay)
            | Some(PropKey::Repeat) | Some(PropKey::HoverAnimation) | Some(PropKey::ScrollAnimation) => {}
            Some(PropKey::Content) => {
                if is_dynamic {
                    out.dynamic.insert("noi_dung".to_string(), register_expr(expr.clone()).to_string());
                } else {
                    out.attrs.insert("noi_dung".to_string(), css_val);
                }
            }
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

fn size_or_px(resolved: &ResolvedValue, css_val: &str) -> String {
    match resolved {
        ResolvedValue::Size(s) => s.clone(),
        _ => format!("{}px", css_val),
    }
}

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

pub fn expand_spacing(val: &str) -> String {
    if val.is_empty() {
        return "0".to_string();
    }
    val.split_whitespace().map(ensure_px).collect::<Vec<_>>().join(" ")
}

pub fn expand_radius(val: &str) -> String {
    expand_spacing(val)
}

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

pub fn map_align(val: &str) -> String {
    match val {
        "trai" => "left".to_string(),
        "phai" => "right".to_string(),
        "giua" => "center".to_string(),
        "deu" => "justify".to_string(),
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

    #[test]
    fn test_color_prop_static() {
        let props: PropsMap = vec![("mau".to_string(), Expr::Literal(vibao_ast::LiteralValue::Color("#FF0000".to_string()), p()))];
        let out = expand_props(Tag::Text, &props);
        assert_eq!(out.style.get("color"), Some(&"#FF0000".to_string()));
    }

    #[test]
    fn test_color_prop_alias_mau_chu_same_as_mau() {
        let props: PropsMap = vec![("mau_chu".to_string(), Expr::Literal(vibao_ast::LiteralValue::Color("#00FF00".to_string()), p()))];
        let out = expand_props(Tag::Text, &props);
        assert_eq!(out.style.get("color"), Some(&"#00FF00".to_string()));
        assert!(out.unknown_keys.is_empty(), "'mau_chu' không được coi là unknown key");
    }

    #[test]
    fn test_color_prop_dynamic_variable() {
        let props: PropsMap = vec![("mau".to_string(), Expr::Variable("mau_chinh".to_string(), p()))];
        let out = expand_props(Tag::Text, &props);
        let value = out.dynamic.get("color").expect("prop 'mau' dynamic phải có trong out.dynamic");
        assert!(value.parse::<usize>().is_ok(), "giá trị dynamic phải là ID số hợp lệ (dom.rs::parse_expr_id yêu cầu), nhận được: {}", value);
        assert!(!out.style.contains_key("color"));
    }

    #[test]
    fn test_width_uses_number_unit() {
        let props: PropsMap = vec![("rong".to_string(), Expr::literal_num(200.0, p()))];
        let out = expand_props(Tag::Khoi, &props);
        assert_eq!(out.style.get("width"), Some(&"200px".to_string()));
    }

    #[test]
    fn test_width_dynamic_variable_goes_to_dynamic_not_broken_style() {
        let props: PropsMap = vec![("rong".to_string(), Expr::Variable("w".to_string(), p()))];
        let out = expand_props(Tag::Khoi, &props);
        let value = out.dynamic.get("width").expect("prop 'rong' dynamic phải có trong out.dynamic");
        assert!(value.parse::<usize>().is_ok(), "giá trị dynamic phải là ID số hợp lệ, nhận được: {}", value);
        assert!(!out.style.contains_key("width"));
    }

    #[test]
    fn test_unknown_prop_key_collected_for_warning() {
        let props: PropsMap = vec![("mua".to_string(), Expr::Literal(vibao_ast::LiteralValue::Str("do".to_string()), p()))];
        let out = expand_props(Tag::Text, &props);
        assert_eq!(out.unknown_keys, vec!["mua".to_string()]);
        assert_eq!(out.attrs.get("mua"), Some(&"do".to_string()));
    }

    #[test]
    fn test_known_prop_key_not_flagged_as_unknown() {
        let props: PropsMap = vec![("mau".to_string(), Expr::Literal(vibao_ast::LiteralValue::Color("#FF0000".to_string()), p()))];
        let out = expand_props(Tag::Text, &props);
        assert!(out.unknown_keys.is_empty());
    }

    #[test]
    fn test_multiple_unknown_keys_all_collected() {
        let props: PropsMap = vec![
            ("mua".to_string(), Expr::Literal(vibao_ast::LiteralValue::Str("x".to_string()), p())),
            ("cann".to_string(), Expr::Literal(vibao_ast::LiteralValue::Str("y".to_string()), p())),
        ];
        let out = expand_props(Tag::Text, &props);
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
        let out = expand_props(Tag::Khoi, &props);
        assert_eq!(out.style.get("width"), Some(&"50%".to_string()));
    }

    #[test]
    fn test_bold_flag_true() {
        let props: PropsMap = vec![("dam".to_string(), Expr::literal_bool(true, p()))];
        let out = expand_props(Tag::Text, &props);
        assert_eq!(out.style.get("fontWeight"), Some(&"bold".to_string()));
    }

    #[test]
    fn test_bold_flag_false_not_set() {
        let props: PropsMap = vec![("dam".to_string(), Expr::literal_bool(false, p()))];
        let out = expand_props(Tag::Text, &props);
        assert!(!out.style.contains_key("fontWeight"));
    }

    #[test]
    fn test_can_text_align_on_text_tag() {
        let props: PropsMap = vec![("can".to_string(), Expr::literal_str("giua", p()))];
        let out = expand_props(Tag::Text, &props);
        assert_eq!(out.style.get("textAlign"), Some(&"center".to_string()));
    }

    #[test]
    fn test_can_justify_content_on_non_text_tag() {
        let props: PropsMap = vec![("can".to_string(), Expr::literal_str("center", p()))];
        let out = expand_props(Tag::Button, &props);
        assert_eq!(out.style.get("justifyContent"), Some(&"center".to_string()));
    }

    #[test]
    fn test_can_vietnamese_value_on_simple_element_no_longer_broken() {
        let props: PropsMap = vec![("can".to_string(), Expr::literal_str("giua", p()))];
        let out = expand_props(Tag::Button, &props);
        assert_eq!(out.style.get("justifyContent"), Some(&"center".to_string()));
    }

    #[test]
    fn test_doc_vietnamese_deu_on_simple_element_no_longer_broken() {
        let props: PropsMap = vec![("doc".to_string(), Expr::literal_str("deu", p()))];
        let out = expand_props(Tag::Button, &props);
        assert_eq!(out.style.get("alignItems"), Some(&"stretch".to_string()));
    }

    #[test]
    fn test_unknown_prop_goes_to_attrs() {
        let props: PropsMap = vec![("data_custom".to_string(), Expr::literal_str("xyz", p()))];
        let out = expand_props(Tag::Khoi, &props);
        assert_eq!(out.attrs.get("data_custom"), Some(&"xyz".to_string()));
    }

    #[test]
    fn test_den_prop_emits_href_and_data_vb_link() {
        let props: PropsMap = vec![("den".to_string(), Expr::literal_str("/gioi-thieu", p()))];
        let out = expand_props(Tag::Link, &props);
        assert_eq!(out.attrs.get("href"), Some(&"/gioi-thieu".to_string()));
        assert_eq!(out.attrs.get("data-vb-link"), Some(&"/gioi-thieu".to_string()));
    }

    #[test]
    fn test_muc_tieu_and_quan_he_emit_target_and_rel() {
        let props: PropsMap = vec![
            ("den".to_string(), Expr::literal_str("https://example.com", p())),
            ("muc_tieu".to_string(), Expr::literal_str("_blank", p())),
            ("quan_he".to_string(), Expr::literal_str("noreferrer", p())),
        ];
        let out = expand_props(Tag::Link, &props);
        assert_eq!(out.attrs.get("target"), Some(&"_blank".to_string()));
        assert_eq!(out.attrs.get("rel"), Some(&"noreferrer".to_string()));
        assert_eq!(out.attrs.get("href"), Some(&"https://example.com".to_string()));
        assert_eq!(out.attrs.get("data-vb-link"), None);
    }

    #[test]
    fn test_english_target_and_rel_surface_names_also_work() {
        let props: PropsMap = vec![
            ("target".to_string(), Expr::literal_str("_blank", p())),
            ("rel".to_string(), Expr::literal_str("noopener", p())),
        ];
        let out = expand_props(Tag::Link, &props);
        assert_eq!(out.attrs.get("target"), Some(&"_blank".to_string()));
        assert_eq!(out.attrs.get("rel"), Some(&"noopener".to_string()));
    }

    #[test]
    fn test_animation_props_skipped() {
        let props: PropsMap = vec![("hieu_ung".to_string(), Expr::literal_str("fade_in", p()))];
        let out = expand_props(Tag::Khoi, &props);
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

    #[test]
    fn test_dem_dynamic_goes_to_dynamic_not_zero() {
        let props: PropsMap = vec![("dem".to_string(), Expr::Variable("khoang_cach".to_string(), p()))];
        let out = expand_props(Tag::Khoi, &props);
        let value = out.dynamic.get("padding").expect("prop 'dem' dynamic phải có trong out.dynamic");
        assert!(value.parse::<usize>().is_ok());
        assert!(!out.style.contains_key("padding"));
    }

    #[test]
    fn test_radius_dynamic_goes_to_dynamic() {
        let props: PropsMap = vec![("radius".to_string(), Expr::Variable("r".to_string(), p()))];
        let out = expand_props(Tag::Khoi, &props);
        assert!(out.dynamic.contains_key("borderRadius"));
        assert!(!out.style.contains_key("borderRadius"));
    }

    #[test]
    fn test_vien_dynamic_still_sets_static_border_style() {
        let props: PropsMap = vec![("vien".to_string(), Expr::Variable("do_day".to_string(), p()))];
        let out = expand_props(Tag::Khoi, &props);
        assert!(out.dynamic.contains_key("borderWidth"));
        assert_eq!(out.style.get("borderStyle"), Some(&"solid".to_string()));
    }

    #[test]
    fn test_co_font_size_dynamic() {
        let props: PropsMap = vec![("co".to_string(), Expr::Variable("kich_co".to_string(), p()))];
        let out = expand_props(Tag::Text, &props);
        assert!(out.dynamic.contains_key("fontSize"));
        assert!(!out.style.contains_key("fontSize"));
    }

    #[test]
    fn test_line_height_small_bare_number_is_unitless() {
        let props: PropsMap = vec![("hang".to_string(), Expr::literal_num(1.6, p()))];
        let out = expand_props(Tag::Text, &props);
        assert_eq!(out.style.get("lineHeight").map(String::as_str), Some("1.6"));
    }

    #[test]
    fn test_line_height_large_bare_number_stays_pixels() {
        let props: PropsMap = vec![("hang".to_string(), Expr::literal_num(24.0, p()))];
        let out = expand_props(Tag::Text, &props);
        assert_eq!(out.style.get("lineHeight").map(String::as_str), Some("24px"));
    }

    #[test]
    fn test_line_height_with_explicit_unit_is_kept() {
        let props: PropsMap = vec![("hang".to_string(), Expr::Literal(vibao_ast::LiteralValue::Num(2.0, Some("rem".to_string())), p()))];
        let out = expand_props(Tag::Text, &props);
        assert_eq!(out.style.get("lineHeight").map(String::as_str), Some("2rem"));
    }

    #[test]
    fn test_den_dynamic_binds_href_without_spa_interception() {
        let props: PropsMap = vec![("den".to_string(), Expr::Variable("url".to_string(), p()))];
        let out = expand_props(Tag::Link, &props);
        let value = out.dynamic_attrs.get("href").expect("href động phải nằm trong dynamic_attrs");
        assert!(value.parse::<usize>().is_ok());
        assert!(!out.attrs.contains_key("href"));
        assert!(!out.attrs.contains_key("data-vb-link"));
    }

    #[test]
    fn test_is_spa_target() {
        for v in ["/du-an", "/san-pham/1", "/", "#gioi-thieu"] {
            assert!(is_spa_target(v), "{}", v);
        }
        for v in ["https://github.com", "http://a.b", "mailto:a@b.c", "tel:+84123", "//cdn.x/y", "cv.pdf", "files/cv.pdf", ""] {
            assert!(!is_spa_target(v), "{}", v);
        }
    }

    #[test]
    fn test_den_external_url_has_href_but_no_spa_link_flag() {
        for url in ["https://github.com/x", "mailto:a@b.c", "cv.pdf"] {
            let props: PropsMap = vec![("den".to_string(), Expr::literal_str(url, p()))];
            let out = expand_props(Tag::Link, &props);
            assert_eq!(out.attrs.get("href").map(String::as_str), Some(url));
            assert!(!out.attrs.contains_key("data-vb-link"), "{}", url);
        }
    }

    #[test]
    fn test_den_static_keeps_href_and_spa_link_flag() {
        let props: PropsMap = vec![("den".to_string(), Expr::literal_str("#du-an", p()))];
        let out = expand_props(Tag::Link, &props);
        assert_eq!(out.attrs.get("href").map(String::as_str), Some("#du-an"));
        assert_eq!(out.attrs.get("data-vb-link").map(String::as_str), Some("#du-an"));
    }

    #[test]
    fn test_loai_dynamic_goes_to_dynamic_attrs_with_html_attr_name() {
        let props: PropsMap = vec![("loai".to_string(), Expr::Variable("kieu_input".to_string(), p()))];
        let out = expand_props(Tag::Input, &props);
        let value = out.dynamic_attrs.get("type").expect("phải có key 'type' (không phải 'loai') trong dynamic_attrs");
        assert!(value.parse::<usize>().is_ok());
        assert!(!out.attrs.contains_key("type"));
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
        let out = expand_props(Tag::Button, &props);
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
        let out = expand_props(Tag::Button, &props);
        assert!(out.class_bindings.is_empty());
        assert_eq!(out.warnings.len(), 1);
        assert!(out.warnings[0].contains("không được để trống"));
    }

    #[test]
    fn test_class_binding_non_object_warns() {
        let props: PropsMap = vec![("lop".to_string(), Expr::Variable("class_map".to_string(), p()))];
        let out = expand_props(Tag::Button, &props);
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
        let out = expand_props(Tag::Button, &props);
        assert!(out.class_bindings.is_empty());
        assert_eq!(out.warnings.len(), 2);
        assert!(out.warnings.iter().all(|w| w.contains("delimiter nội bộ")));
    }

    #[test]
    fn test_gia_tri_variable_on_input_uses_two_way_model() {
        let props: PropsMap = vec![("gia_tri".to_string(), Expr::Variable("v".to_string(), p()))];
        let out = expand_props(Tag::Input, &props);
        assert_eq!(out.model.as_deref(), Some("v"));
        assert!(!out.dynamic_attrs.contains_key("value"));
        assert!(!out.attrs.contains_key("value"));
    }

    #[test]
    fn test_gia_tri_variable_on_textarea_uses_two_way_model() {
        let props: PropsMap = vec![("gia_tri".to_string(), Expr::Variable("v".to_string(), p()))];
        let out = expand_props(Tag::Textarea, &props);
        assert_eq!(out.model.as_deref(), Some("v"));
        assert!(!out.dynamic_attrs.contains_key("value"));
        assert!(!out.attrs.contains_key("value"));
    }

    #[test]
    fn test_gia_tri_variable_on_select_uses_two_way_model() {
        let props: PropsMap = vec![("gia_tri".to_string(), Expr::Variable("v".to_string(), p()))];
        let out = expand_props(Tag::LuaChon, &props);
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
        let out = expand_props(Tag::Input, &props);
        assert!(out.model.is_none());
        assert!(out.dynamic_attrs.contains_key("value"));
    }

    #[test]
    fn test_gia_tri_variable_on_non_input_keeps_one_way_binding() {
        let props: PropsMap = vec![("gia_tri".to_string(), Expr::Variable("v".to_string(), p()))];
        let out = expand_props(Tag::Button, &props);
        assert!(out.model.is_none());
        assert!(out.dynamic_attrs.contains_key("value"));
    }

    #[test]
    fn test_alt_static_unaffected_by_dynamic_attrs_change() {
        let props: PropsMap = vec![("mo_ta_anh".to_string(), Expr::literal_str("Ảnh minh hoạ", p()))];
        let out = expand_props(Tag::Image, &props);
        assert_eq!(out.attrs.get("alt"), Some(&"Ảnh minh hoạ".to_string()));
        assert!(out.dynamic_attrs.is_empty());
    }

    #[test]
    fn test_dam_dynamic_warns_and_does_not_set_style() {
        let props: PropsMap = vec![("dam".to_string(), Expr::Variable("la_dam".to_string(), p()))];
        let out = expand_props(Tag::Text, &props);
        assert_eq!(out.warnings.len(), 1, "phải cảnh báo đúng 1 lần: {:?}", out.warnings);
        assert!(out.warnings[0].contains("dam"));
        assert!(!out.style.contains_key("fontWeight"), "không được set fontWeight sai giá trị");
        assert!(!out.dynamic.contains_key("fontWeight"), "không được đẩy vào dynamic (sẽ set sai 'true'/'false' làm CSS value)");
    }

    #[test]
    fn test_can_dynamic_warns_and_does_not_set_style() {
        let props: PropsMap = vec![("can".to_string(), Expr::Variable("huong_can".to_string(), p()))];
        let out = expand_props(Tag::Text, &props);
        assert_eq!(out.warnings.len(), 1);
        assert!(!out.style.contains_key("textAlign"));
    }

    #[test]
    fn test_bat_buoc_dynamic_warns_and_does_not_set_attr() {
        let props: PropsMap = vec![("bat_buoc".to_string(), Expr::Variable("bb".to_string(), p()))];
        let out = expand_props(Tag::Input, &props);
        assert_eq!(out.warnings.len(), 1);
        assert!(!out.attrs.contains_key("required"));
        assert!(!out.dynamic_attrs.contains_key("required"), "không được đẩy vào dynamic_attrs (sẽ set 'required=\"false\"' vẫn bị HTML coi là required=true)");
    }

    #[test]
    fn test_static_boolean_props_unaffected_by_warning_change() {
        let props: PropsMap = vec![
            ("dam".to_string(), Expr::literal_bool(true, p())),
            ("can".to_string(), Expr::literal_str("giua", p())),
            ("bat_buoc".to_string(), Expr::literal_bool(true, p())),
        ];
        let out = expand_props(Tag::Text, &props);
        assert!(out.warnings.is_empty(), "không nên có cảnh báo nào cho giá trị tĩnh: {:?}", out.warnings);
        assert_eq!(out.style.get("fontWeight"), Some(&"bold".to_string()));
        assert_eq!(out.style.get("textAlign"), Some(&"center".to_string()));
    }

    #[test]
    fn test_tai_cham_dynamic_warns_and_does_not_set_attr() {
        let props: PropsMap = vec![("tai_cham".to_string(), Expr::Variable("lazy".to_string(), p()))];
        let out = expand_props(Tag::Image, &props);
        assert_eq!(out.warnings.len(), 1, "phải cảnh báo đúng 1 lần: {:?}", out.warnings);
        assert!(out.warnings[0].contains("tai_cham"));
        assert!(!out.attrs.contains_key("loading"), "không được set loading sai (rỗng luôn != \"true\")");
        assert!(!out.dynamic_attrs.contains_key("loading"), "không được đẩy vào dynamic_attrs (loading là attribute đọc 1 lần, không phản ứng động)");
    }

    #[test]
    fn test_tai_cham_static_true_still_works() {
        let props: PropsMap = vec![("tai_cham".to_string(), Expr::literal_bool(true, p()))];
        let out = expand_props(Tag::Image, &props);
        assert!(out.warnings.is_empty());
        assert_eq!(out.attrs.get("loading"), Some(&"lazy".to_string()));
    }

    #[test]
    fn test_tai_cham_static_false_sets_nothing() {
        let props: PropsMap = vec![("tai_cham".to_string(), Expr::literal_bool(false, p()))];
        let out = expand_props(Tag::Image, &props);
        assert!(out.warnings.is_empty());
        assert!(!out.attrs.contains_key("loading"));
    }

    #[test]
    fn test_layout_only_propkey_falls_through_to_passthrough_unchanged() {
        let props: PropsMap = vec![("cot".to_string(), Expr::literal_num(2.0, p()))];
        let out = expand_props(Tag::Text, &props);
        assert!(out.attrs.contains_key("cot"), "phải đi qua passthrough, ghi ra attrs['cot']");
        assert_eq!(
            out.unknown_keys, vec!["cot".to_string()],
            "'cot' là prop layout-only, gõ trên Simple tag 'text' PHẢI được cảnh báo unknown"
        );
    }

    #[test]
    fn test_unresolvable_typo_still_flagged_unknown_after_migrate() {
        let props: PropsMap = vec![("khong_ton_tai_xyz".to_string(), Expr::literal_str("x", p()))];
        let out = expand_props(Tag::Text, &props);
        assert_eq!(out.unknown_keys, vec!["khong_ton_tai_xyz".to_string()]);
    }

    #[test]
    fn test_english_simple_prop_is_not_flagged_unknown() {
        let props: PropsMap = vec![(
            "background_color".to_string(),
            Expr::literal_str("red", p()),
        )];
        let out = expand_props(Tag::Text, &props);
        assert!(out.unknown_keys.is_empty(), "English simple prop không được cảnh báo unknown");
        assert_eq!(out.style.get("backgroundColor"), Some(&"red".to_string()));
    }

    #[test]
    fn test_english_layout_only_prop_is_still_unknown_on_simple_element() {
        let props: PropsMap = vec![(
            "columns".to_string(),
            Expr::literal_num(2.0, p()),
        )];
        let out = expand_props(Tag::Text, &props);
        assert_eq!(out.unknown_keys, vec!["columns".to_string()]);
        assert_eq!(out.attrs.get("columns"), Some(&"2px".to_string()));
    }

    #[test]
    fn test_all_simple_prop_identities_are_marked_simple_in_prop_spec() {
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
            PropKey::Disabled, PropKey::ClassBinding, PropKey::HtmlId, PropKey::To, PropKey::Target,
            PropKey::Rel, PropKey::Content,
            PropKey::Animation, PropKey::Duration, PropKey::Delay, PropKey::Repeat,
            PropKey::HoverAnimation, PropKey::ScrollAnimation,
        ];
        for key in simple_keys {
            assert!(prop_spec(key).applies_to_simple, "{:?} phải áp dụng cho Simple", key);
        }
    }

}
