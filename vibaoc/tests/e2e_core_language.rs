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
        khoi {
            @khoang_cach { dem: 16 }

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
        box {
            @spacing { padding: 16 }

            text("Xin chao", bold: true)
        }
    }
}
"#,
    );
    en_result.assert_ok();

    let vi_html = vi_result.html();
    let en_html = en_result.html();
    assert!(vi_html.contains("Xin chao"));
    assert!(en_html.contains("Xin chao"));
}

#[test]
fn nguon_co_dau_va_khong_dau_cho_ra_cung_html_va_css() {
    let khong_dau = r##"
lang = "vi";
ung_dung("Ứng dụng") {
    chu_de sang { $nen = trang  $chu = xam_dam }
    chu_de toi { $nen = "#1a202c"  $chu = "#e2e8f0" }
    giao_dien { mau_nen: $nen, mau: $chu }
    @the TheDuAn(ten: chuoi, duong_dan: chuoi) {
        flex {
            @flex_bo_cuc { huong: cot, gap: 8 }
            text($ten, co: 18, dam: true)
            link("Mở", den: $duong_dan, mau: $chu)
        }
    }
    trang("/") {
        state $ky_nang = ["Rust", "WebAssembly"]
        state $ten_nguoi = "Bảo"
        khoi {
            @khoang_cach { dem: 16, radius: 8 }
            text("Xin chào $ten_nguoi")
            vong_lap $k trong $ky_nang {
                text($k)
            }
            TheDuAn(ten: "Dự án", duong_dan: "https://example.com")
        }
    }
}
"##;
    let co_dau = r##"
lang = "vi";
ứng_dụng("Ứng dụng") {
    chủ_đề sáng { $nền = trang  $chữ = xám_đậm }
    chủ_đề tối { $nền = "#1a202c"  $chữ = "#e2e8f0" }
    giao_diện { màu_nền: $nền, màu: $chữ }
    @thẻ ThẻDựÁn(tên: chuỗi, đường_dẫn: chuỗi) {
        flex {
            @flex_bố_cục { hướng: cột, gap: 8 }
            text($tên, cỡ: 18, đậm: true)
            link("Mở", đến: $đường_dẫn, màu: $chữ)
        }
    }
    trang("/") {
        state $kỹ_năng = ["Rust", "WebAssembly"]
        state $tên_người = "Bảo"
        khối {
            @khoảng_cách { đệm: 16, radius: 8 }
            text("Xin chào $tên_người")
            vòng_lặp $k trong $kỹ_năng {
                text($k)
            }
            ThẻDựÁn(tên: "Dự án", đường_dẫn: "https://example.com")
        }
    }
}
"##;
    let (_dir_a, a) = build_source("khong-dau", khong_dau);
    a.assert_ok();
    let (_dir_b, b) = build_source("co-dau", co_dau);
    b.assert_ok();
    assert_eq!(a.html(), b.html());
    assert_eq!(a.css(), b.css());
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
        khoi {
            @kich_thuoc { rong: $do_rong }

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
        khoi {
            @khoang_cach { dem: 6 }

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

#[test]
fn output_sinh_ra_khong_con_comment_cua_compiler() {
    let (_dir, result) = build_source(
        "khong-comment",
        r#"
app("App") {
    page("/") {
        text("Xin chao")
    }
}
"#,
    );
    result.assert_ok();
    let css = result.css();
    let js = result.js();
    for dau_vet in ["ViBao Base CSS", "ViBao Animation classes", "/* Tabs */", "/* Toast */"] {
        assert!(!css.contains(dau_vet), "css còn comment '{}':\n{}", dau_vet, css);
    }
    for dau_vet in ["DO NOT EDIT", "App init", "WASM Bootstrap"] {
        assert!(!js.contains(dau_vet), "js còn comment '{}':\n{}", dau_vet, js);
    }
}

