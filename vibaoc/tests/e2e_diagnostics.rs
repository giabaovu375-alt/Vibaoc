// VIBAO COMPILER — end-to-end: build-time diagnostics.
//
// ViBao aims to fail loudly and clearly at compile time rather than
// silently miscompiling or deferring problems to the browser. These
// tests confirm that real, common mistakes produce a failed build with
// an informative message via the actual CLI.

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

#[test]
fn theme_block_is_rejected_with_a_clear_message() {
    let (_dir, result) = build_source(
        "theme-block",
        r#"
ung_dung("App") {
    theme SangToi {
        $mau_chinh = xanh
    }
    trang("/") {
        text("noi dung")
    }
}
"#,
    );
    result.assert_err();
    assert!(result.stderr.to_lowercase().contains("theme"));
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
    assert!(!result.stderr.is_empty(), "a syntax error should produce a diagnostic message");
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
    // A typo'd function name inside an expression (as opposed to a
    // top-level action call) should also be caught at build time
    // rather than silently evaluating to null in the browser.
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
    assert!(ok, "check --ast should succeed on valid source, stderr:\n{}", stderr);
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
