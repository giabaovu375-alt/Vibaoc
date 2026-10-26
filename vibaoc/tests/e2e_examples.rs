mod common;
use common::build_source;
use std::fs;
use std::path::Path;

fn read_example(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/examples")
        .join(name);
    fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read example fixture '{}': {}", path.display(), e))
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
    let src = read_example("03_bo_dem_co_dau.vbao");
    let (_dir, result) = build_source("ex03", &src);
    result.assert_ok();
    let html = result.html();
    assert!(
        html.contains("data-vb-if="),
        "expected an if-binding, html:\n{}",
        html
    );
    assert!(
        html.contains("data-vb-else"),
        "expected an else-binding, html:\n{}",
        html
    );
}

#[test]
fn example_04_layout_tags_builds_ok_and_generates_expected_display_css() {
    let src = read_example("04_layout_tags.vbao");
    let (_dir, result) = build_source("ex04", &src);
    result.assert_ok();
    let css = result.css();
    assert!(
        css.contains("flex"),
        "expected a flex display rule, css:\n{}",
        css
    );
    assert!(
        css.contains("grid"),
        "expected a grid display rule, css:\n{}",
        css
    );
    assert!(
        css.contains("sticky"),
        "expected a sticky position rule, css:\n{}",
        css
    );
    assert!(
        css.contains("fixed"),
        "expected a fixed position rule, css:\n{}",
        css
    );
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
    assert!(
        js.contains("\"i\"") || js.contains("'i'"),
        "range loop var name 'i' should be preserved"
    );
}

#[test]
fn example_08_collection_loop_english_builds_ok() {
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
    assert_eq!(
        count, 5,
        "expected 5 mounted Card instances (2+2+1 across 3 pages), found {}",
        count
    );
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
fn example_16_action_option_locale_builds_ok() {
    let src = read_example("16_action_option_locale.vbao");
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
    assert!(
        css.contains("@media"),
        "expected a @media rule from @di_dong, css:\n{}",
        css
    );
}

#[test]
fn example_20_expected_build_errors_fails_with_a_duplicate_component_message() {
    let src = read_example("20_expected_build_errors.vbao");
    let (_dir, result) = build_source("ex20", &src);
    result.assert_err();
    assert!(
        result.stderr.contains("TheThe"),
        "lỗi phải nêu tên component bị trùng, stderr:\n{}",
        result.stderr
    );
}

#[test]
fn portfolio_example_builds_and_copies_public_assets() {
    let repo_examples = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../examples");
    let source = repo_examples.join("portfolio.vbao");
    let dir = common::TempDir::new("portfolio");
    let out_dir = dir.path.join("dist");
    let result = common::run_build(&source, &out_dir);
    result.assert_ok();

    assert!(
        out_dir.join("avatar.svg").is_file(),
        "public/avatar.svg must be copied"
    );
    assert!(
        out_dir.join("favicon.svg").is_file(),
        "public/favicon.svg must be copied"
    );

    let html = result.html();
    assert!(
        html.contains("<link rel=\"icon\" type=\"image/svg+xml\" href=\"./favicon.svg\">"),
        "{}",
        html
    );
    assert!(
        html.contains("<meta name=\"description\" content=\"Portfolio của Gia Bảo"),
        "{}",
        html
    );
    assert!(html.contains("<meta property=\"og:title\""), "{}", html);
    assert!(
        html.contains("href=\"https://github.com/giabaovu375-alt\""),
        "{}",
        html
    );
    assert!(
        !html.contains("data-vb-link=\"https://github.com/giabaovu375-alt\""),
        "an external link must not be intercepted by the router: {}",
        html
    );
    assert!(html.contains("data-vb-link=\"#du-an\""), "{}", html);
    for element in [
        "<header ",
        "<main ",
        "<section ",
        "<footer ",
        "<h1 ",
        "<h2 ",
    ] {
        assert!(
            html.contains(element),
            "semantic element {} missing: {}",
            element,
            html
        );
    }

    let css = result.css();
    assert!(
        css.contains("@media (max-width: 639px)") && css.contains("display: none !important;"),
        "responsive overrides must be !important to beat inline layout styles:\n{}",
        css
    );
    assert!(css.contains("[data-vb-loop] { display: none; }"), "{}", css);

    assert!(css.contains(":root { --nen: #FFFFFF;"), "{}", css);
    assert!(
        css.contains("@media (prefers-color-scheme: dark)"),
        "{}",
        css
    );
    assert!(css.contains("--nen: #1a202c"), "{}", css);
    assert!(css.contains("background-color: var(--nen)"), "{}", css);
    assert!(html.contains("var(--nhan)"), "{}", html);
    assert!(
        html.contains("<html lang=\"vi\" data-vb-themes=\"sang,toi\">"),
        "{}",
        html
    );
    assert!(result.js().contains("dat_chu_de"), "{}", result.js());
    assert!(html.contains("Repo mới trên GitHub"), "{}", html);
    assert!(
        result
            .js()
            .contains("api.github.com/users/giabaovu375-alt/repos"),
        "{}",
        result.js()
    );
    assert!(
        result.js().contains("\"background\":true"),
        "{}",
        result.js()
    );
}

#[test]
fn global_style_block_generates_body_css_and_font_link() {
    let src = r#"
lang = "vi";
ung_dung("Style") {
    giao_dien {
        phong_chu: "Georgia, serif"
        tai_phong: "https://fonts.googleapis.com/css2?family=Inter"
        co: 18
        mau: xam_dam
        mau_nen: trang
        cuon_muot: true
        cuon_lui: 64
    }
    trang("/") {
        text("Xin chao")
    }
}
"#;
    let (_dir, result) = common::build_source("global-style", src);
    result.assert_ok();
    let css = result.css();
    assert!(css.contains("font-family: Georgia, serif"), "{}", css);
    assert!(css.contains("font-size: 18px"), "{}", css);
    assert!(css.contains("scroll-behavior: smooth"), "{}", css);
    assert!(css.contains("scroll-padding-top: 64px"), "{}", css);
    let html = result.html();
    assert!(
        html.contains(
            "<link rel=\"stylesheet\" href=\"https://fonts.googleapis.com/css2?family=Inter\">"
        ),
        "{}",
        html
    );
}

#[test]
fn english_global_style_block_is_accepted() {
    let src = r#"
app("Style") {
    global_style {
        font_family: "Georgia, serif"
        smooth_scroll: true
    }
    page("/") {
        text("Hello")
    }
}
"#;
    let (_dir, result) = common::build_source("global-style-en", src);
    result.assert_ok();
    assert!(
        result.css().contains("font-family: Georgia, serif"),
        "{}",
        result.css()
    );
}

#[test]
fn unknown_key_in_global_style_is_a_build_error() {
    let src = r#"
lang = "vi";
ung_dung("Style") {
    giao_dien {
        phong_chuu: "Georgia"
    }
    trang("/") {
        text("Xin chao")
    }
}
"#;
    let (_dir, result) = common::build_source("global-style-bad", src);
    result.assert_err();
    assert!(result.stderr.contains("phong_chuu"), "{}", result.stderr);
}

#[test]
fn chat_groq_example_builds_with_headers_index_and_preserved_lines() {
    let repo_examples = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../examples");
    let source = repo_examples.join("chat_groq.vbao");
    let dir = common::TempDir::new("chat-groq");
    let out_dir = dir.path.join("dist");
    let result = common::run_build(&source, &out_dir);
    result.assert_ok();

    let html = result.html();
    assert!(html.contains("data-vb-on-submit"), "{}", html);
    assert!(html.contains("position:fixed;"), "{}", html);
    assert!(html.contains("white-space:pre-wrap"), "{}", html);
    assert!(html.contains("overflow-wrap:anywhere"), "{}", html);

    let js = result.js();
    assert!(
        js.contains("api.groq.com/openai/v1/chat/completions"),
        "{}",
        js
    );
    assert!(js.contains("Authorization"), "{}", js);
    assert!(js.contains("\"choices\""), "{}", js);
}

#[test]
fn build_voi_site_url_sinh_sitemap_robots_404_va_the_seo() {
    let repo_examples = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../examples");
    let dir = common::TempDir::new("site-url");
    let source = dir.write(
        "app.vbao",
        &std::fs::read_to_string(repo_examples.join("multi_page.vbao")).unwrap(),
    );
    std::fs::create_dir_all(dir.path.join("public")).unwrap();
    std::fs::write(dir.path.join("public").join("og-image.png"), "png").unwrap();
    let out_dir = dir.path.join("dist");
    let result =
        common::run_build_with_args(&source, &out_dir, &["--site-url", "https://vi-du.com/"]);
    result.assert_ok();

    let html = result.html();
    assert!(
        html.contains("<link rel=\"canonical\" href=\"https://vi-du.com/\">"),
        "{}",
        html
    );
    assert!(
        html.contains("<meta property=\"og:image\" content=\"https://vi-du.com/og-image.png\">"),
        "{}",
        html
    );
    assert!(html.contains("summary_large_image"), "{}", html);

    let sitemap = std::fs::read_to_string(out_dir.join("sitemap.xml")).unwrap();
    assert!(
        sitemap.contains("<loc>https://vi-du.com/</loc>"),
        "{}",
        sitemap
    );
    let robots = std::fs::read_to_string(out_dir.join("robots.txt")).unwrap();
    assert!(
        robots.contains("Sitemap: https://vi-du.com/sitemap.xml"),
        "{}",
        robots
    );
    assert_eq!(
        std::fs::read_to_string(out_dir.join("404.html")).unwrap(),
        html
    );
}

#[test]
fn build_khong_co_site_url_khong_sinh_sitemap_va_the_canonical() {
    let repo_examples = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../examples");
    let dir = common::TempDir::new("khong-site-url");
    let out_dir = dir.path.join("dist");
    let result = common::run_build(&repo_examples.join("multi_page.vbao"), &out_dir);
    result.assert_ok();
    assert!(!result.html().contains("canonical"), "{}", result.html());
    assert!(!out_dir.join("sitemap.xml").exists());
    assert!(!out_dir.join("robots.txt").exists());
    assert!(out_dir.join("404.html").exists());
}

#[test]
fn example_21_tinh_nang_0_1_5_builds_with_theme_api_index_and_preserved_lines() {
    let src = read_example("21_tinh_nang_0_1_5.vbao");
    let (_dir, result) = build_source("ex21", &src);
    result.assert_ok();

    let html = result.html();
    assert!(
        html.contains("<html lang=\"vi\" data-vb-themes=\"sang,toi\">"),
        "{}",
        html
    );
    assert!(html.contains("white-space:pre-wrap"), "{}", html);
    assert!(html.contains("Danh sách trống"), "{}", html);

    let css = result.css();
    assert!(css.contains("--vb-theme: sang"), "{}", css);
    assert!(css.contains("--vb-theme: toi"), "{}", css);

    let js = result.js();
    assert!(js.contains("\"Authorization\""), "{}", js);
    assert!(js.contains("\"background\":true"), "{}", js);
    assert!(js.contains("\"property\":\"0\""), "{}", js);
    assert!(js.contains("dat_chu_de"), "{}", js);
}
