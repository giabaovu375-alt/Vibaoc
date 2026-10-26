mod common;
use common::build_source;

#[test]
fn unknown_action_name_is_a_hard_build_error() {
    let (_dir, result) = build_source(
        "typo-action",
        r#"
ung_dung("App") {
    trang("/") {
        button("Nhan") {
            khi_nhan {
                thongbao("Xin chao")
            }
        }
    }
}
"#,
    );
    result.assert_err();
    assert!(
        result.stderr.contains("thongbao"),
        "the error should mention the offending name, stderr:\n{}",
        result.stderr
    );
}

#[test]
fn known_but_unsupported_action_fails_with_a_distinct_message_from_a_typo() {
    let (_dir, result) = build_source(
        "known-unsupported-action",
        r#"
ung_dung("App") {
    trang("/") {
        button("Dang xuat") {
            khi_nhan {
                dang_xuat()
            }
        }
    }
}
"#,
    );
    result.assert_err();
    assert!(result.stderr.contains("dang_xuat"));
    assert!(
        !result.stderr.contains("Unknown action"),
        "a recognized-but-unimplemented action must not be reported as a generic typo, stderr:\n{}",
        result.stderr
    );
}

const NGUON_THEME_HOP_LE: &str = r##"
lang = "vi";
ung_dung("Chủ đề") {
    chu_de sang { $nen = trang  $chu = xam_dam }
    chu_de toi { $nen = "#1a202c"  $chu = "#e2e8f0" }
    giao_dien { mau_nen: $nen, mau: $chu }
    trang("/", mau_nen: $nen) {
        khoi {
            @mau { mau_nen: $nen }
            text("Xin chào", mau: $chu)
        }
    }
}
"##;

#[test]
fn theme_hop_le_duoc_build_va_sinh_css_bien() {
    let (_dir, result) = build_source("theme-hop-le", NGUON_THEME_HOP_LE);
    result.assert_ok();
    let css = result.css();
    assert!(
        css.contains(":root { --nen: #FFFFFF; --chu: #2D3748; --vb-theme: sang }"),
        "{}",
        css
    );
    assert!(
        css.contains("@media (prefers-color-scheme: dark) {\n  :root:not([data-vb-theme]) { --nen: #1a202c; --chu: #e2e8f0; --vb-theme: toi }\n}"),
        "{}",
        css
    );
    assert!(
        css.contains(
            ":root[data-vb-theme=\"toi\"] { --nen: #1a202c; --chu: #e2e8f0; --vb-theme: toi }"
        ),
        "{}",
        css
    );
    assert!(css.contains("background-color: var(--nen)"), "{}", css);
    assert!(css.contains("color: var(--chu)"), "{}", css);
}

#[test]
fn bien_theme_trong_prop_thanh_var_css_khong_thanh_binding_dong() {
    let (_dir, result) = build_source("theme-var", NGUON_THEME_HOP_LE);
    result.assert_ok();
    let html = result.html();
    assert!(html.contains("var(--nen)"), "{}", html);
    assert!(html.contains("var(--chu)"), "{}", html);
    assert!(!html.contains("data-vb-bind-backgroundColor"), "{}", html);
}

#[test]
fn bien_theme_dung_lam_noi_dung_la_loi_build() {
    let nguon = NGUON_THEME_HOP_LE.replace("text(\"Xin chào\", mau: $chu)", "text($chu)");
    let (_dir, result) = build_source("theme-noi-dung", &nguon);
    result.assert_err();
    assert!(
        result.stderr.contains(
            "Biến theme \"$chu\" chỉ dùng được làm toàn bộ giá trị của một prop kiểu dáng"
        ),
        "{}",
        result.stderr
    );
}

#[test]
fn dat_chu_de_build_duoc_va_trang_khai_bao_danh_sach_theme() {
    let nut = "text(\"Xin chào\", mau: $chu)\n            button(\"Tối\") { khi_nhấn { đặt_chủ_đề(\"toi\") } }\n            button(\"Đổi\") { khi_nhấn { đặt_chủ_đề() } }";
    let nguon = NGUON_THEME_HOP_LE.replace("text(\"Xin chào\", mau: $chu)", nut);
    let (_dir, result) = build_source("dat-chu-de", &nguon);
    result.assert_ok();
    let html = result.html();
    assert!(
        html.contains("<html lang=\"vi\" data-vb-themes=\"sang,toi\">"),
        "{}",
        html
    );
    assert!(
        html.contains("localStorage.getItem(\"vb-theme\")"),
        "{}",
        html
    );
    assert!(result.js().contains("dat_chu_de"), "{}", result.js());
}

#[test]
fn dat_chu_de_voi_theme_khong_ton_tai_la_loi_build() {
    let nut = "text(\"Xin chào\", mau: $chu)\n            button(\"X\") { khi_nhấn { đặt_chủ_đề(\"xanh\") } }";
    let nguon = NGUON_THEME_HOP_LE.replace("text(\"Xin chào\", mau: $chu)", nut);
    let (_dir, result) = build_source("dat-chu-de-sai-ten", &nguon);
    result.assert_err();
    assert!(
        result
            .stderr
            .contains("Theme \"xanh\" không tồn tại: các theme đã khai báo là sang, toi"),
        "{}",
        result.stderr
    );
}

#[test]
fn dat_chu_de_khi_chua_khai_bao_theme_la_loi_build() {
    let nguon = r#"
lang = "vi";
ứng_dụng("Không theme") {
    trang("/") {
        button("X") { khi_nhấn { đặt_chủ_đề() } }
    }
}
"#;
    let (_dir, result) = build_source("dat-chu-de-khong-theme", nguon);
    result.assert_err();
    assert!(
        result
            .stderr
            .contains("Action đổi theme được dùng nhưng chưa khai báo theme nào"),
        "{}",
        result.stderr
    );
}

#[test]
fn theme_thieu_bien_la_loi_build_va_duoc_dich_sang_tieng_viet() {
    let nguon = NGUON_THEME_HOP_LE.replace(
        "$nen = \"#1a202c\"  $chu = \"#e2e8f0\"",
        "$nen = \"#1a202c\"",
    );
    let (_dir, result) = build_source("theme-thieu-bien", &nguon);
    result.assert_err();
    assert!(
        result
            .stderr
            .contains("Theme \"toi\" thiếu biến \"$chu\" (đã khai báo ở theme \"sang\")"),
        "{}",
        result.stderr
    );
}

#[test]
fn bien_theme_trung_ten_state_la_loi_build() {
    let nguon = NGUON_THEME_HOP_LE.replace("khoi {", "state $nen = 1\n        khoi {");
    let (_dir, result) = build_source("theme-trung-state", &nguon);
    result.assert_err();
    assert!(
        result
            .stderr
            .contains("State \"$nen\" trùng tên với một biến theme"),
        "{}",
        result.stderr
    );
}

#[test]
fn khong_khai_bao_theme_thi_khong_sinh_css_theme() {
    let (_dir, result) = build_source(
        "khong-theme",
        r#"
lang = "vi";
ung_dung("Không theme") {
    trang("/") { text("Xin chào") }
}
"#,
    );
    result.assert_ok();
    assert!(!result.css().contains("ViBao themes"), "{}", result.css());
}

#[test]
fn unclosed_block_is_reported_as_a_parse_error_not_a_panic() {
    let (_dir, result) = build_source(
        "unclosed-block",
        r#"
ung_dung("App") {
    trang("/") {
        text("noi dung")
"#,
    );
    result.assert_err();
    assert!(
        !result.stderr.is_empty(),
        "a syntax error should produce a diagnostic message"
    );
}

#[test]
fn missing_input_file_is_reported_without_a_panic() {
    let (dir, _placeholder) = build_source("dummy", "app(\"x\") { page(\"/\") { text(\"x\") } }");
    let missing = dir.path.join("does-not-exist.vbao");
    let out_dir = dir.path.join("dist2");
    let result = common::run_build(&missing, &out_dir);
    result.assert_err();
}

#[test]
fn duplicate_component_definition_is_rejected_with_a_clear_message() {
    let (_dir, result) = build_source(
        "duplicate-component",
        r#"
ung_dung("App") {
    @the TheThe(nhan: chuoi) {
        text($nhan)
    }
    @the TheThe(nhan: chuoi) {
        text("Ban sao")
    }
    trang("/") {
        TheThe(nhan: "Xin chao")
    }
}
"#,
    );
    result.assert_err();
    assert!(
        result.stderr.contains("TheThe"),
        "the error should name the duplicated component, stderr:\n{}",
        result.stderr
    );
}

#[test]
fn unknown_function_call_in_an_expression_does_not_silently_produce_null() {
    let (_dir, result) = build_source(
        "unknown-function-expr",
        r#"
ung_dung("App") {
    trang("/") {
        state $gia = 1000
        text(gia_tienn($gia))
    }
}
"#,
    );
    result.assert_err();
    assert!(
        result.stderr.contains("gia_tienn"),
        "the error should mention the unknown function name, stderr:\n{}",
        result.stderr
    );
}

#[test]
fn english_action_option_keys_are_normalized_in_the_ast() {
    let dir = common::TempDir::new("action-opts");
    let src = dir.write(
        "app.vbao",
        r#"
app("App") {
    page("/") {
        button("Go") {
            on_click { notify("hi", type: warning, duration: 5000) }
        }
    }
}
"#,
    );
    let (ok, stdout, stderr) = common::run_check_ast(&src);
    assert!(ok, "check --ast should succeed, stderr:\n{}", stderr);
    assert!(
        stdout.contains("\"kieu\""),
        "AST should hold the canonical key kieu:\n{}",
        stdout
    );
    assert!(
        stdout.contains("\"thoi_gian\""),
        "AST should hold the canonical key thoi_gian:\n{}",
        stdout
    );
    assert!(
        !stdout.contains("\"duration\""),
        "English key must not survive into the AST:\n{}",
        stdout
    );
}

#[test]
fn unknown_api_response_handler_is_a_hard_build_error() {
    let src = r#"
app("App") {
    page("/") {
        button("Go") {
            on_click {
                api_call("GET", "/x") {
                    done { notify("hi") }
                }
            }
        }
    }
}
"#;
    let (_dir, result) = build_source("bad-handler", src);
    result.assert_err();
    assert!(
        result.stderr.contains("Unknown response handler 'done'"),
        "stderr:\n{}",
        result.stderr
    );
}

#[test]
fn check_ast_flag_prints_a_parsed_program_for_valid_source() {
    let dir = common::TempDir::new("check-ast");
    let src = dir.write(
        "app.vbao",
        r#"
ung_dung("App") {
    trang("/") {
        text("Xin chao")
    }
}
"#,
    );
    let (ok, stdout, stderr) = common::run_check_ast(&src);
    assert!(
        ok,
        "check --ast should succeed on valid source, stderr:\n{}",
        stderr
    );
    assert!(stdout.contains("Parse succeeded"));
}

#[test]
fn check_ast_flag_reports_a_lexer_or_parser_error_for_invalid_source() {
    let dir = common::TempDir::new("check-ast-invalid");
    let src = dir.write("app.vbao", "ung_dung(\"App\" { trang(");
    let (ok, _stdout, stderr) = common::run_check_ast(&src);
    assert!(!ok);
    assert!(!stderr.is_empty());
}

#[test]
fn version_in_ra_dung_phien_ban_cua_goi() {
    assert_eq!(
        common::run_version(),
        format!("vibaoc {}", env!("CARGO_PKG_VERSION"))
    );
}
