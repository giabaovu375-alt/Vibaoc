use std::cell::RefCell;
use std::collections::HashSet;

use vibao_ast::Theme;

use crate::codegen::expr::resolve_value;

thread_local! {
    static THEME_VARIABLES: RefCell<HashSet<String>> = RefCell::new(HashSet::new());
}

const DARK_THEME_NAMES: [&str; 2] = ["toi", "dark"];

pub fn css_var_name(name: &str) -> String {
    format!("--{}", name.replace('_', "-"))
}

pub fn activate(themes: &[Theme]) {
    THEME_VARIABLES.with(|set| {
        let mut set = set.borrow_mut();
        set.clear();
        for theme in themes {
            for variable in &theme.variables {
                set.insert(variable.name.clone());
            }
        }
    });
}

pub fn is_theme_variable(name: &str) -> bool {
    THEME_VARIABLES.with(|set| set.borrow().contains(name))
}

pub fn is_dark_theme_name(name: &str) -> bool {
    DARK_THEME_NAMES.contains(&name)
}

fn escape_attribute_value(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn rule(selector: &str, theme: &Theme) -> String {
    let mut declarations = theme
        .variables
        .iter()
        .map(|variable| {
            format!(
                "{}: {}",
                css_var_name(&variable.name),
                resolve_value(&variable.value).as_css()
            )
        })
        .collect::<Vec<_>>();
    declarations.push(format!("--vb-theme: {}", theme.name));
    let declarations = declarations.join("; ");
    format!("{} {{ {} }}\n", selector, declarations)
}

pub fn generate_css(themes: &[Theme]) -> String {
    let default = match themes.first() {
        Some(theme) => theme,
        None => return String::new(),
    };

    let mut css = String::new();
    css.push_str(&rule(":root", default));

    if let Some(dark) = themes.iter().skip(1).find(|t| is_dark_theme_name(&t.name)) {
        css.push_str("@media (prefers-color-scheme: dark) {\n  ");
        css.push_str(&rule(":root:not([data-vb-theme])", dark));
        css.push_str("}\n");
    }

    for theme in themes {
        let selector = format!(
            ":root[data-vb-theme=\"{}\"]",
            escape_attribute_value(&theme.name)
        );
        css.push_str(&rule(&selector, theme));
    }
    css
}

#[cfg(test)]
mod tests {
    use super::*;
    use vibao_ast::{Expr, Pos, VarDecl};

    fn p() -> Pos {
        Pos { line: 1, column: 1 }
    }

    fn mau(hex: &str) -> Expr {
        Expr::Literal(vibao_ast::LiteralValue::Color(hex.to_string()), p())
    }

    fn theme(name: &str, variables: Vec<(&str, Expr)>) -> Theme {
        Theme {
            name: name.to_string(),
            variables: variables
                .into_iter()
                .map(|(n, value)| VarDecl {
                    name: n.to_string(),
                    value,
                    pos: p(),
                })
                .collect(),
            pos: p(),
        }
    }

    #[test]
    fn ten_bien_css_doi_gach_duoi_thanh_gach_ngang() {
        assert_eq!(css_var_name("mau_chinh"), "--mau-chinh");
        assert_eq!(css_var_name("nen"), "--nen");
    }

    #[test]
    fn kich_hoat_ghi_nho_bien_cua_moi_theme_va_xoa_bien_cu() {
        let themes = vec![
            theme("sang", vec![("nen", mau("#FFFFFF"))]),
            theme(
                "toi",
                vec![("nen", mau("#000000")), ("phu", mau("#111111"))],
            ),
        ];
        activate(&themes);
        assert!(is_theme_variable("nen"));
        assert!(is_theme_variable("phu"));
        assert!(!is_theme_variable("khac"));

        activate(&[]);
        assert!(!is_theme_variable("nen"));
    }

    #[test]
    fn bien_co_dau_cham_khong_phai_bien_theme() {
        activate(&[theme("sang", vec![("nen", mau("#FFFFFF"))])]);
        assert!(!is_theme_variable("nen.con"));
    }

    #[test]
    fn khong_co_theme_thi_khong_sinh_css() {
        assert_eq!(generate_css(&[]), "");
    }

    #[test]
    fn theme_dau_tien_la_mac_dinh_tren_root() {
        let css = generate_css(&[theme(
            "sang",
            vec![("nen", mau("#FFFFFF")), ("chu", mau("#2D3748"))],
        )]);
        assert!(
            css.contains(":root { --nen: #FFFFFF; --chu: #2D3748; --vb-theme: sang }"),
            "{}",
            css
        );
        assert!(!css.contains("prefers-color-scheme"), "{}", css);
    }

    #[test]
    fn theme_toi_tu_ap_dung_theo_he_dieu_hanh() {
        let css = generate_css(&[
            theme("sang", vec![("nen", mau("#FFFFFF"))]),
            theme("toi", vec![("nen", mau("#1A202C"))]),
        ]);
        assert!(
            css.contains("@media (prefers-color-scheme: dark) {\n  :root:not([data-vb-theme]) { --nen: #1A202C; --vb-theme: toi }\n}"),
            "{}",
            css
        );
    }

    #[test]
    fn ten_dark_cung_duoc_coi_la_theme_toi() {
        let css = generate_css(&[
            theme("light", vec![("nen", mau("#FFFFFF"))]),
            theme("dark", vec![("nen", mau("#1A202C"))]),
        ]);
        assert!(css.contains("prefers-color-scheme: dark"), "{}", css);
    }

    #[test]
    fn theme_ten_khac_khong_tu_ap_dung() {
        let css = generate_css(&[
            theme("sang", vec![("nen", mau("#FFFFFF"))]),
            theme("bien", vec![("nen", mau("#0000FF"))]),
        ]);
        assert!(!css.contains("prefers-color-scheme"), "{}", css);
    }

    #[test]
    fn chi_theme_toi_dau_tien_tu_ap_dung() {
        let css = generate_css(&[
            theme("sang", vec![("nen", mau("#FFFFFF"))]),
            theme("toi", vec![("nen", mau("#111111"))]),
            theme("dark", vec![("nen", mau("#222222"))]),
        ]);
        assert_eq!(css.matches("prefers-color-scheme").count(), 1, "{}", css);
        assert!(css.contains("#111111"));
    }

    #[test]
    fn moi_theme_co_selector_theo_thuoc_tinh_de_ep_bang_tay() {
        let css = generate_css(&[
            theme("sang", vec![("nen", mau("#FFFFFF"))]),
            theme("toi", vec![("nen", mau("#1A202C"))]),
        ]);
        assert!(
            css.contains(":root[data-vb-theme=\"sang\"] { --nen: #FFFFFF; --vb-theme: sang }"),
            "{}",
            css
        );
        assert!(
            css.contains(":root[data-vb-theme=\"toi\"] { --nen: #1A202C; --vb-theme: toi }"),
            "{}",
            css
        );
    }

    #[test]
    fn so_khong_don_vi_thanh_px_va_chuoi_giu_nguyen() {
        let css = generate_css(&[theme(
            "sang",
            vec![
                ("khoang", Expr::literal_num(16.0, p())),
                ("phong", Expr::literal_str("Georgia, serif", p())),
            ],
        )]);
        assert!(css.contains("--khoang: 16px"), "{}", css);
        assert!(css.contains("--phong: Georgia, serif"), "{}", css);
    }

    #[test]
    fn ten_theme_co_dau_ngoac_kep_duoc_escape() {
        let css = generate_css(&[theme("a\"b", vec![("nen", mau("#FFFFFF"))])]);
        assert!(css.contains("[data-vb-theme=\"a\\\"b\"]"), "{}", css);
    }
}
