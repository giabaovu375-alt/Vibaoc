mod common;
use common::build_source;

#[test]
fn lang_directive_switches_diagnostics_to_vietnamese() {
    let (_dir, result) = build_source(
        "lang-directive",
        r#"lang = "vi";

ung_dung("App") {
    trang("/") {
        button("Nhan") {
            khi_nhan {
                thongbao("loi")
            }
        }
    }
}
"#,
    );
    result.assert_err();
    assert!(
        result.stderr.to_lowercase().contains("không xác định") || result.stderr.contains("lỗi ngữ nghĩa"),
        "expected a Vietnamese diagnostic message, stderr:\n{}",
        result.stderr
    );
}

#[test]
fn missing_lang_directive_defaults_diagnostics_to_english() {
    let (_dir, result) = build_source(
        "no-lang-directive",
        r#"
ung_dung("App") {
    trang("/") {
        button("Nhan") {
            khi_nhan {
                thongbao("loi")
            }
        }
    }
}
"#,
    );
    result.assert_err();
    assert!(
        result.stderr.contains("Unknown action") || result.stderr.to_lowercase().contains("unknown"),
        "expected an English diagnostic message by default, stderr:\n{}",
        result.stderr
    );
}

#[test]
fn mixed_vietnamese_and_english_props_are_both_accepted_on_the_same_tag() {
    let (_dir, result) = build_source(
        "mixed-props",
        r#"
app("App") {
    page("/") {
        box {
            @color { mau_nen: xam_nhat }
            @spacing { padding: 16, radius: 8 }

            text("Ca hai locale cung mot the")
        }
    }
}
"#,
    );
    result.assert_ok();
}

#[test]
fn hover_and_scroll_animations_compile_successfully() {
    let (_dir, result) = build_source(
        "animations",
        r#"
ung_dung("App") {
    trang("/") {
        button("Di chuot vao day", hieu_ung_hover: "phong_to", hieu_ung_cuon: "fade_in")
    }
}
"#,
    );
    result.assert_ok();
}

#[test]
fn responsive_mobile_block_overrides_are_emitted_in_css() {
    let (_dir, result) = build_source(
        "responsive",
        r#"
ung_dung("App") {
    trang("/") {
        khoi {
            @kich_thuoc { rong: 800 }
            @di_dong {
                rong: 320
            }
            text("Resize de kiem tra")
        }
    }
}
"#,
    );
    result.assert_ok();
    let css = result.css();
    assert!(
        css.contains("@media"),
        "a `@di_dong` responsive block should emit a media query, css:\n{}",
        css
    );
}

#[test]
fn built_in_color_name_resolves_to_a_css_value() {
    let (_dir, result) = build_source(
        "colors",
        r#"
ung_dung("App") {
    trang("/") {
        text("Chu mau do", mau: do)
    }
}
"#,
    );
    result.assert_ok();
}

#[test]
fn dynamic_class_object_compiles_with_multiple_conditions() {
    let (_dir, result) = build_source(
        "dynamic-class",
        r#"
ung_dung("App") {
    trang("/") {
        state $dang_chon = false
        state $im_lang = true
        button("Nhan", lop: { active: $dang_chon, muted: $im_lang }) {
            khi_nhan { $dang_chon = !$dang_chon }
        }
    }
}
"#,
    );
    result.assert_ok();
}

#[test]
fn template_string_interpolation_does_not_swallow_a_literal_dollar_amount() {
    let (_dir, result) = build_source(
        "template-dollar-literal",
        r#"
ung_dung("App") {
    trang("/") {
        text("Gia: $50")
    }
}
"#,
    );
    result.assert_ok();
    let js = result.js();
    assert!(
        js.contains("50"),
        "the literal amount should appear in app.js's expression registry:\n{}",
        js
    );
    let html = result.html();
    assert!(
        html.contains("data-vb-text="),
        "a template string (even one with a literal-only `$`) should bind dynamically, HTML:\n{}",
        html
    );
}
