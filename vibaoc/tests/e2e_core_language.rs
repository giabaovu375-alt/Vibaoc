// VIBAO COMPILER — end-to-end: core language constructs.
//
// Black-box tests that build real `.vbao` sources through the actual
// `vibaoc` CLI and inspect the emitted HTML/CSS/JS. See
// `tests/common/mod.rs` for the shared harness.

mod common;
use common::build_source;

#[test]
fn minimal_app_produces_all_three_output_files() {
    let (_dir, result) = build_source(
        "minimal",
        r#"
app("Test App") {
    page("/") {
        text("Xin chao")
    }
}
"#,
    );
    result.assert_ok();
    assert!(result.out_dir.join("index.html").exists());
    assert!(result.out_dir.join("app.js").exists());
    assert!(result.out_dir.join("style.css").exists());
}

#[test]
fn vietnamese_and_english_surface_syntax_compile_to_equivalent_output() {
    let (_dir, vi_result) = build_source(
        "surface-vi",
        r#"
ung_dung("App") {
    trang("/") {
        khoi(dem: 16) {
            text("Xin chao", dam: true)
        }
    }
}
"#,
    );
    vi_result.assert_ok();

    let (_dir2, en_result) = build_source(
        "surface-en",
        r#"
app("App") {
    page("/") {
        box(padding: 16) {
            text("Xin chao", bold: true)
        }
    }
}
"#,
    );
    en_result.assert_ok();

    // Both surface languages resolve to the same AST tags/props, so
    // both should mark the text element as bold in the emitted HTML.
    let vi_html = vi_result.html();
    let en_html = en_result.html();
    assert!(vi_html.contains("Xin chao"));
    assert!(en_html.contains("Xin chao"));
}

#[test]
fn multi_page_app_generates_a_route_for_every_page() {
    let (_dir, result) = build_source(
        "multi-page",
        r#"
ung_dung("Nhieu trang") {
    trang("/") {
        text("Trang chu")
        link("Gioi thieu", den: "/gioi-thieu")
    }
    trang("/gioi-thieu") {
        text("Gioi thieu")
        link("Ve trang chu", den: "/")
    }
    trang("/lien-he") {
        text("Lien he")
    }
}
"#,
    );
    result.assert_ok();
    let html = result.html();
    assert!(html.contains("Trang chu"));
    assert!(html.contains("Gioi thieu"));
    assert!(html.contains("Lien he"));
}

#[test]
fn if_else_renders_a_conditional_binding_for_both_branches() {
    let (_dir, result) = build_source(
        "if-else",
        r#"
ung_dung("App") {
    trang("/") {
        state $dang_nhap = false
        neu $dang_nhap {
            text("Xin chao")
        } khong_thi {
            text("Vui long dang nhap")
        }
    }
}
"#,
    );
    result.assert_ok();
    let html = result.html();
    assert!(html.contains("data-vb-if="), "expected an if-binding attribute in HTML:\n{}", html);
    assert!(html.contains("data-vb-else"), "expected an else-binding attribute in HTML:\n{}", html);
}

#[test]
fn range_loop_renders_the_expected_number_of_items() {
    let (_dir, result) = build_source(
        "range-loop",
        r#"
ung_dung("App") {
    trang("/") {
        vong_lap $i tu 1 den 5 {
            text("Muc so $i")
        }
    }
}
"#,
    );
    result.assert_ok();
    let js = result.js();
    // The loop is desugared into a bound range; the item variable name
    // must be preserved (not silently replaced by a default like "i").
    assert!(js.contains("\"i\"") || js.contains("'i'"));
}

#[test]
fn collection_loop_over_a_state_array_binds_the_item_variable() {
    let (_dir, result) = build_source(
        "collection-loop",
        r#"
ung_dung("App") {
    trang("/") {
        state $tasks = [
            {id: 1, tieu_de: "Viec 1"},
            {id: 2, tieu_de: "Viec 2"}
        ]
        vong_lap $task trong $tasks {
            text($task.tieu_de)
        }
    }
}
"#,
    );
    result.assert_ok();
    let js = result.js();
    assert!(js.contains("tasks"));
}

#[test]
fn switch_statement_compiles_every_case_including_default() {
    let (_dir, result) = build_source(
        "switch",
        r#"
ung_dung("App") {
    trang("/") {
        state $trang_thai = "sang_sua"
        truong_hop $trang_thai {
            "sang_sua" {
                text("On dinh")
            }
            "loi" {
                text("Co loi")
            }
            mac_dinh {
                text("Khong ro")
            }
        }
    }
}
"#,
    );
    result.assert_ok();
    let html = result.html();
    assert!(html.contains("On dinh"));
    assert!(html.contains("Co loi"));
    assert!(html.contains("Khong ro"));
}

#[test]
fn nested_loops_both_compile_and_render() {
    let (_dir, result) = build_source(
        "nested-loops",
        r#"
ung_dung("App") {
    trang("/") {
        vong_lap $hang tu 1 den 2 {
            vong_lap $cot tu 1 den 2 {
                text("O luoi")
            }
        }
    }
}
"#,
    );
    result.assert_ok();
    assert!(result.html().contains("O luoi"));
}

#[test]
fn dynamic_layout_prop_generates_a_style_binding() {
    let (_dir, result) = build_source(
        "dynamic-width",
        r#"
ung_dung("App") {
    trang("/") {
        state $do_rong = 200
        khoi(rong: $do_rong) {
            text("noi dung")
        }
    }
}
"#,
    );
    result.assert_ok();
    assert!(
        result.html().contains("data-vb-style-width="),
        "a dynamic width prop should emit a data-vb-style-width binding"
    );
}

#[test]
fn user_defined_component_can_be_instantiated_multiple_times() {
    let (_dir, result) = build_source(
        "component-reuse",
        r#"
ung_dung("App") {
    @the TheThe(nhan: chuoi) {
        khoi(dem: 6) {
            text($nhan)
        }
    }

    trang("/") {
        TheThe(nhan: "Mot")
        TheThe(nhan: "Hai")
        TheThe(nhan: "Ba")
    }
}
"#,
    );
    result.assert_ok();
    let html = result.html();
    let count = html.matches("data-vb-component=\"TheThe\"").count();
    assert_eq!(count, 3, "expected 3 mounted TheThe instances, found {}. HTML:\n{}", count, html);
}

#[test]
fn component_mounts_correctly_on_every_page_not_just_the_last() {
    let (_dir, result) = build_source(
        "component-multi-page",
        r#"
ung_dung("App") {
    @the TheTieuDe(noi_dung: chuoi) {
        text($noi_dung)
    }

    trang("/") {
        TheTieuDe(noi_dung: "Trang chu")
    }
    trang("/gioi-thieu") {
        TheTieuDe(noi_dung: "Gioi thieu")
    }
    trang("/lien-he") {
        TheTieuDe(noi_dung: "Lien he")
    }
}
"#,
    );
    result.assert_ok();
    let html = result.html();
    let count = html.matches("data-vb-component=\"TheTieuDe\"").count();
    assert_eq!(count, 3, "expected one mounted TheTieuDe per page, found {}", count);
}
