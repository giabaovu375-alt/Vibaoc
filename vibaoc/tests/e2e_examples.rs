// VIBAO COMPILER — end-to-end: full example programs (20 .vbao files).
//
// Unlike the other `tests/e2e_*.rs` files (which embed small inline
// snippets), this file drives 20 STANDALONE `.vbao` programs living
// under `tests/examples/` through the real `vibaoc` CLI, the same way
// an actual end user would run `vibaoc build app.vbao`. Each program
// targets a distinct language feature (or a specific combination), and
// deliberately mixes THREE surface styles across the set, per the
// project's bilingual design:
//   - Vietnamese WITH diacritics (dấu)      — e.g. 03, 12
//   - Vietnamese WITHOUT diacritics (không dấu) — e.g. 01, 04, 05, 09, 10
//   - English                                 — e.g. 02, 06, 08, 11, 13...
//
// HOW TO RUN (once cargo/rustc is available):
//   1. Copy this file to vibaoc/tests/e2e_examples.rs
//   2. Copy tests/examples/*.vbao next to it (vibaoc/tests/examples/)
//   3. cargo test --workspace
//
// Every test below was cross-checked BY READING THE COMPILER SOURCE
// (lexer/tables.rs, locale/{vi,en}.rs, prop_vi.rs/prop_en.rs,
// codegen/layout.rs, parser/action.rs, parser/control.rs) at the time
// this suite was written — NOT executed against a real `cargo test`,
// because this environment has no cargo/rustc available (same
// limitation noted in docs/VIBAO_SPEC.md's own header). If a build
// unexpectedly fails, the failure itself is valuable signal — see the
// "BUG NOTE" comments inside each .vbao file and in
// docs/AI_TESTING_NOTES.md for known gaps found by source review that
// this suite is specifically designed to catch/confirm.

mod common;
use common::build_source;
use std::fs;
use std::path::Path;

/// Reads one of the standalone example programs from
/// `tests/examples/<name>`. Panics with a clear message if the fixture
/// is missing, instead of a confusing "file.vbao should exist" I/O
/// error deep inside `common::build_source`.
fn read_example(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/examples").join(name);
    fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!("failed to read example fixture '{}': {}", path.display(), e)
    })
}

#[test]
fn example_01_minimal_app_vi_builds_ok() {
    let src = read_example("01_minimal_app_vi.vbao");
    let (_dir, result) = build_source("ex01", &src);
    result.assert_ok();
    assert!(result.html().contains("Xin chao ViBao"));
}

#[test]
fn example_02_minimal_app_en_builds_ok() {
    let src = read_example("02_minimal_app_en.vbao");
    let (_dir, result) = build_source("ex02", &src);
    result.assert_ok();
    assert!(result.html().contains("Hello ViBao"));
}

#[test]
fn example_03_bo_dem_co_dau_builds_ok() {
    // Full Vietnamese diacritics in string/comment content. State,
    // on_click (khi_nhan), comparison, if/else, numeric add/subtract.
    let src = read_example("03_bo_dem_co_dau.vbao");
    let (_dir, result) = build_source("ex03", &src);
    result.assert_ok();
    let html = result.html();
    assert!(html.contains("data-vb-if="), "expected an if-binding, html:\n{}", html);
    assert!(html.contains("data-vb-else"), "expected an else-binding, html:\n{}", html);
}

#[test]
fn example_04_layout_tags_builds_ok_and_generates_expected_display_css() {
    let src = read_example("04_layout_tags.vbao");
    let (_dir, result) = build_source("ex04", &src);
    result.assert_ok();
    let css = result.css();
    assert!(css.contains("flex"), "expected a flex display rule, css:\n{}", css);
    assert!(css.contains("grid"), "expected a grid display rule, css:\n{}", css);
    assert!(css.contains("sticky"), "expected a sticky position rule, css:\n{}", css);
    assert!(css.contains("fixed"), "expected a fixed position rule, css:\n{}", css);
}

#[test]
fn example_05_switch_truong_hop_builds_ok_and_renders_every_case() {
    let src = read_example("05_switch_truong_hop.vbao");
    let (_dir, result) = build_source("ex05", &src);
    result.assert_ok();
    let html = result.html();
    assert!(html.contains("Dang tai du lieu"));
    assert!(html.contains("Co loi xay ra"));
    assert!(html.contains("Thanh cong"));
    assert!(html.contains("Trang thai khong xac dinh"));
}

#[test]
fn example_06_switch_english_builds_ok_and_renders_every_case() {
    let src = read_example("06_switch_english.vbao");
    let (_dir, result) = build_source("ex06", &src);
    result.assert_ok();
    let html = result.html();
    assert!(html.contains("Loading data"));
    assert!(html.contains("Something went wrong"));
    assert!(html.contains("Success!"));
    assert!(html.contains("Unknown status"));
}

#[test]
fn example_07_vong_lap_range_builds_ok() {
    let src = read_example("07_vong_lap_range.vbao");
    let (_dir, result) = build_source("ex07", &src);
    result.assert_ok();
    let js = result.js();
    assert!(js.contains("\"i\"") || js.contains("'i'"), "range loop var name 'i' should be preserved");
}

#[test]
fn example_08_collection_loop_english_builds_ok() {
    // Documents the real gap: "loop" (en) is fine, but "trong" (the
    // collection-loop connector) has NO English form - confirmed by
    // reading parser/control.rs (`self.check_ident("trong")` is the
    // ONLY check, no "in" branch exists anywhere).
    let src = read_example("08_collection_loop_english.vbao");
    let (_dir, result) = build_source("ex08", &src);
    result.assert_ok();
    assert!(result.js().contains("products"));
}

#[test]
fn example_09_vong_lap_index_builds_ok() {
    let src = read_example("09_vong_lap_index.vbao");
    let (_dir, result) = build_source("ex09", &src);
    result.assert_ok();
}

#[test]
fn example_10_array_crud_builds_ok() {
    let src = read_example("10_array_crud.vbao");
    let (_dir, result) = build_source("ex10", &src);
    result.assert_ok();
}

#[test]
fn example_11_array_crud_english_builds_ok() {
    let src = read_example("11_array_crud_english.vbao");
    let (_dir, result) = build_source("ex11", &src);
    result.assert_ok();
}

#[test]
fn example_12_component_the_co_dau_builds_ok() {
    // Higher-risk area per docs/VIBAO_SPEC.md section 9: a component
    // calling ANOTHER component (TheThanhVien -> TheNhan), combined
    // with if/else inside @the, is explicitly flagged as under-tested
    // upstream ("chưa test trường hợp lồng nhiều tầng"). This test's
    // main value is confirming whether that combination still compiles
    // after any future change.
    let src = read_example("12_component_the_co_dau.vbao");
    let (_dir, result) = build_source("ex12", &src);
    result.assert_ok();
    let html = result.html();
    assert!(html.contains("data-vb-component=\"TheThanhVien\""));
    assert!(html.contains("data-vb-component=\"TheNhan\""));
}

#[test]
fn example_13_component_multi_page_english_mounts_on_every_page() {
    let src = read_example("13_component_multi_page_english.vbao");
    let (_dir, result) = build_source("ex13", &src);
    result.assert_ok();
    let html = result.html();
    let count = html.matches("data-vb-component=\"Card\"").count();
    assert_eq!(count, 5, "expected 5 mounted Card instances (2+2+1 across 3 pages), found {}", count);
}

#[test]
fn example_14_dynamic_route_param_builds_ok() {
    let src = read_example("14_dynamic_route_param.vbao");
    let (_dir, result) = build_source("ex14", &src);
    result.assert_ok();
    assert!(result.js().contains("san-pham"));
}

#[test]
fn example_15_page_lifecycle_english_builds_ok() {
    let src = read_example("15_page_lifecycle_english.vbao");
    let (_dir, result) = build_source("ex15", &src);
    result.assert_ok();
}

#[test]
fn example_16_bug_repro_action_option_locale_builds_ok_but_documents_a_behavior_gap() {
    // Both buttons compile successfully (this is not a syntax error) —
    // the point of this fixture is that the FIRST button's `type:`/
    // `duration:` options are silently ignored at runtime (only
    // "kieu"/"thoi_gian" are ever read by
    // vibao-runtime::action::dispatch_function_call). This can only be
    // confirmed by reading the runtime source; a build-success check is
    // the most this black-box harness can assert without a real
    // browser/WASM environment.
    let src = read_example("16_bug_repro_action_option_locale.vbao");
    let (_dir, result) = build_source("ex16", &src);
    result.assert_ok();
}

#[test]
fn example_17_actions_navigate_scroll_api_english_builds_ok() {
    let src = read_example("17_actions_navigate_scroll_api_english.vbao");
    let (_dir, result) = build_source("ex17", &src);
    result.assert_ok();
}

#[test]
fn example_18_input_binding_class_animation_builds_ok() {
    let src = read_example("18_input_binding_class_animation.vbao");
    let (_dir, result) = build_source("ex18", &src);
    result.assert_ok();
}

#[test]
fn example_19_responsive_and_colors_builds_ok_and_emits_media_query() {
    let src = read_example("19_responsive_and_colors.vbao");
    let (_dir, result) = build_source("ex19", &src);
    result.assert_ok();
    let css = result.css();
    assert!(css.contains("@media"), "expected a @media rule from @di_dong, css:\n{}", css);
}

#[test]
fn example_20_expected_build_errors_fails_with_a_theme_block_message() {
    // This fixture is DELIBERATELY invalid (see the file's own header
    // comment for 5 more invalid variants documented but not active).
    // The currently-active error is an unsupported `theme` block.
    let src = read_example("20_expected_build_errors.vbao");
    let (_dir, result) = build_source("ex20", &src);
    result.assert_err();
    assert!(result.stderr.to_lowercase().contains("theme"));
}
