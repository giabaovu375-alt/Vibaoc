use vibao_ast::{PropKey, PropsMap, Tag};

use crate::codegen::css::camel_to_kebab;
use crate::codegen::expr::resolve_value;
use crate::codegen::layout::px;
use crate::codegen::props::expand_props;

#[derive(Debug, Default)]
pub struct GlobalStyle {
    pub css: String,
    pub stylesheets: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Clone, Copy)]
enum Special {
    StylesheetUrl,
    SmoothScroll,
    ScrollPadding,
}

fn special_key(key: &str) -> Option<Special> {
    match key {
        "tai_phong" | "font_url" => Some(Special::StylesheetUrl),
        "cuon_muot" | "smooth_scroll" => Some(Special::SmoothScroll),
        "cuon_lui" | "scroll_offset" => Some(Special::ScrollPadding),
        _ => None,
    }
}

pub fn is_known_key(key: &str) -> bool {
    special_key(key).is_some()
        || matches!(
            crate::locale::resolve_prop_key(key),
            Some(
                PropKey::FontFamily
                    | PropKey::FontSize
                    | PropKey::Color
                    | PropKey::BackgroundColor
                    | PropKey::LineHeight
            )
        )
}

pub fn build(props: &PropsMap) -> GlobalStyle {
    let mut out = GlobalStyle::default();
    let mut html_decls: Vec<String> = Vec::new();
    let mut body_props: PropsMap = Vec::new();

    for (key, expr) in props {
        let Some(special) = special_key(key) else {
            body_props.push((key.clone(), expr.clone()));
            continue;
        };
        let resolved = resolve_value(expr);
        if resolved.is_dynamic() {
            out.warnings.push(format!(
                "giao_dien: '{}' must be a static value; the value was ignored",
                key
            ));
            continue;
        }
        let value = resolved.as_css();
        match special {
            Special::StylesheetUrl => {
                if !value.is_empty() {
                    out.stylesheets.push(value);
                }
            }
            Special::SmoothScroll => {
                if value == "true" {
                    html_decls.push("scroll-behavior: smooth".to_string());
                }
            }
            Special::ScrollPadding => {
                html_decls.push(format!("scroll-padding-top: {}", px(&value)));
            }
        }
    }

    let expanded = expand_props(Tag::Text, &body_props);
    for (key, _) in expanded.dynamic.iter() {
        out.warnings.push(format!(
            "giao_dien: the value of '{}' depends on state; global style must be static and the value was ignored",
            key
        ));
    }
    let body_decls: Vec<String> = expanded
        .style
        .iter()
        .map(|(k, v)| format!("{}: {}", camel_to_kebab(k), v))
        .collect();

    let mut rules: Vec<String> = Vec::new();
    if !html_decls.is_empty() {
        rules.push(format!("html {{ {} }}", html_decls.join("; ")));
    }
    if !body_decls.is_empty() {
        rules.push(format!("body {{ {} }}", body_decls.join("; ")));
    }
    if !rules.is_empty() {
        out.css = rules.join("\n");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use vibao_ast::{Expr, Pos};

    fn p() -> Pos {
        Pos { line: 1, column: 1 }
    }

    fn s(v: &str) -> Expr {
        Expr::literal_str(v, p())
    }

    #[test]
    fn empty_block_generates_nothing() {
        let out = build(&vec![]);
        assert!(out.css.is_empty());
        assert!(out.stylesheets.is_empty());
        assert!(out.warnings.is_empty());
    }

    #[test]
    fn font_size_and_family_go_to_body() {
        let props: PropsMap = vec![
            ("phong_chu".to_string(), s("Inter, sans-serif")),
            ("co".to_string(), Expr::literal_num(16.0, p())),
        ];
        let out = build(&props);
        assert!(out.css.contains("body {"), "{}", out.css);
        assert!(out.css.contains("font-family: Inter, sans-serif"), "{}", out.css);
        assert!(out.css.contains("font-size: 16px"), "{}", out.css);
        assert!(out.warnings.is_empty(), "{:?}", out.warnings);
    }

    #[test]
    fn english_keys_work_too() {
        let props: PropsMap = vec![("font_family".to_string(), s("Georgia, serif"))];
        let out = build(&props);
        assert!(out.css.contains("font-family: Georgia, serif"), "{}", out.css);
    }

    #[test]
    fn stylesheet_url_is_collected_not_written_as_css() {
        let props: PropsMap = vec![("tai_phong".to_string(), s("https://fonts.googleapis.com/css2?family=Inter"))];
        let out = build(&props);
        assert_eq!(out.stylesheets, vec!["https://fonts.googleapis.com/css2?family=Inter".to_string()]);
        assert!(out.css.is_empty(), "{}", out.css);
    }

    #[test]
    fn smooth_scroll_and_scroll_padding_go_to_html() {
        let props: PropsMap = vec![
            ("cuon_muot".to_string(), s("true")),
            ("cuon_lui".to_string(), Expr::literal_num(72.0, p())),
        ];
        let out = build(&props);
        assert!(out.css.contains("html { scroll-behavior: smooth; scroll-padding-top: 72px }"), "{}", out.css);
    }

    #[test]
    fn smooth_scroll_false_adds_nothing() {
        let props: PropsMap = vec![("cuon_muot".to_string(), s("false"))];
        assert!(build(&props).css.is_empty());
    }

    #[test]
    fn dynamic_value_is_ignored_with_a_warning() {
        let props: PropsMap = vec![("co".to_string(), Expr::Variable("co_chu".to_string(), p()))];
        let out = build(&props);
        assert!(out.css.is_empty(), "{}", out.css);
        assert_eq!(out.warnings.len(), 1, "{:?}", out.warnings);
    }

    #[test]
    fn known_keys() {
        for key in ["phong_chu", "font_family", "co", "mau", "mau_nen", "background_color", "hang", "tai_phong", "font_url", "cuon_muot", "smooth_scroll", "cuon_lui", "scroll_offset"] {
            assert!(is_known_key(key), "{}", key);
        }
        for key in ["rong", "dem", "ten_la", "huong"] {
            assert!(!is_known_key(key), "{}", key);
        }
    }
}
