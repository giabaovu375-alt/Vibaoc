mod lexer;
mod locale;
mod parser;
mod codegen;
mod resolver;
mod validator;
mod diagnostic;
mod serve;
mod theme_check;
mod deploy;

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();

    match args.get(1).map(String::as_str) {
        Some("--help") | Some("-h") => {
            println!("{}", diagnostic::usage());
        }
        Some("--version") | Some("-V") => {
            println!("vibaoc {}", env!("CARGO_PKG_VERSION"));
        }
        Some("build") | Some("check") => {
            if args.len() < 3 {
                eprintln!("{}\n\nerror[E0003] Missing input file.", diagnostic::usage());
                process::exit(2);
            }
            let subcommand = args[1].as_str();
            let path = &args[2];
            match subcommand {
                "build" => {
                    let out_dir = parse_out_dir(&args).unwrap_or_else(|| PathBuf::from("dist"));
                    let site_url = flag_value(&args, "--site-url");
                    cmd_build(path, &out_dir, site_url.as_deref());
                }
                "check" => {
                    let ast_only = args.iter().any(|a| a == "--ast");
                    cmd_check(path, ast_only);
                }
                _ => unreachable!(),
            }
        }
        Some("serve") => {
            let path = match args.get(2) {
                Some(path) => path,
                None => {
                    eprintln!("{}\n\nerror[E0003] Missing input file.", diagnostic::usage());
                    process::exit(2);
                }
            };
            let out_dir = parse_out_dir(&args).unwrap_or_else(|| PathBuf::from("dist"));
            let port = match flag_value(&args, "--port") {
                Some(raw) => match raw.parse::<u16>() {
                    Ok(port) => port,
                    Err(_) => {
                        eprintln!("error[E0005] Invalid --port value '{}'. Use a number from 1 to 65535.", raw);
                        process::exit(2);
                    }
                },
                None => 8000,
            };
            let host = flag_value(&args, "--host").unwrap_or_else(|| "127.0.0.1".to_string());
            let locale = cmd_build(path, &out_dir, None);
            serve::run(serve::ServeOptions {
                source: PathBuf::from(path),
                out_dir,
                host,
                port,
                watch: !args.iter().any(|a| a == "--no-watch"),
                locale,
            });
        }
        Some(path) if path.ends_with(".vbao") => {
            cmd_build(path, Path::new("dist"), None);
        }
        Some(command) => {
            eprintln!("error[E0004] Unknown command '{}'.\n\n{}", command, diagnostic::usage());
            process::exit(2);
        }
        None => {
            eprintln!("{}\n\nerror[E0003] Missing command.", diagnostic::usage());
            process::exit(2);
        }
    }
}

fn flag_value(args: &[String], name: &str) -> Option<String> {
    let idx = args.iter().position(|a| a == name)?;
    args.get(idx + 1).cloned()
}

fn parse_out_dir(args: &[String]) -> Option<PathBuf> {
    let idx = args.iter().position(|a| a == "--out")?;
    args.get(idx + 1).map(PathBuf::from)
}

fn compile(path: &str) -> (codegen::CodegenOutput, diagnostic::Locale) {
    if !path.ends_with(".vbao") {
        eprintln!(
            "warning: ViBao source files normally use the `.vbao` extension; `{}` does not. Continuing.",
            path
        );
    }

    let source = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{}", diagnostic::missing_file(diagnostic::Locale::English, Path::new(path), &e));
            process::exit(1);
        }
    };

    let source_locale = diagnostic::Locale::detect_from_source(&source);
    let tokens = match lexer::tokenize(&source) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("{}", diagnostic::render(source_locale, "error", "E1001", "lexer", Path::new(path), &source, e.line, e.column, &e.message, None));
            process::exit(1);
        }
    };

    let (program, locale) = match parser::parse_with_locale(tokens) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{}", diagnostic::render(source_locale, "error", "E2001", "parser", Path::new(path), &source, e.line, e.column, &e.message, None));
            process::exit(1);
        }
    };

    let program = match resolver::resolve_with_locale(program, Path::new(path), locale) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{}", diagnostic::render(e.locale, "error", "E3001", "resolver", Path::new(&e.path), &e.source, e.line, e.column, &e.message, None));
            process::exit(1);
        }
    };

    if let Err(errors) = validator::validate(&program.app) {
        for e in &errors {
            eprintln!("{}", diagnostic::render(locale, "error", "E4001", "semantic analysis", Path::new(path), &source, e.line, e.column, &e.message, None));
        }
        let summary = if locale == diagnostic::Locale::Vietnamese {
            format!("error: tìm thấy {} lỗi ngữ nghĩa; dừng build.", errors.len())
        } else {
            format!("error: found {} semantic error(s); build stopped.", errors.len())
        };
        eprintln!("{}", summary);
        process::exit(1);
    }

    let mut gen = codegen::Codegen::new(codegen::CodegenOptions::default());
    let output = gen.generate(&program);

    for (index, w) in output.warnings.iter().enumerate() {
        eprintln!("{}", diagnostic::warning(locale, &format!("W{:04}", index + 1), w));
    }

    (output, locale)
}

fn cmd_check(path: &str, ast_only: bool) {
    if ast_only {
        let source = match fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("{}", diagnostic::missing_file(diagnostic::Locale::English, Path::new(path), &e));
                process::exit(1);
            }
        };
        let source_locale = diagnostic::Locale::detect_from_source(&source);
        let tokens = match lexer::tokenize(&source) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("{}", diagnostic::render(source_locale, "error", "E1001", "lexer", Path::new(path), &source, e.line, e.column, &e.message, None));
                process::exit(1);
            }
        };
        let (program, _locale) = match parser::parse_with_locale(tokens) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("{}", diagnostic::render(source_locale, "error", "E2001", "parser", Path::new(path), &source, e.line, e.column, &e.message, None));
                process::exit(1);
            }
        };
        println!("Parse succeeded.\n");
        println!("{:#?}", program);
        return;
    }

    let (output, _locale) = compile(path);
    println!("=== HTML ===\n{}\n", output.html);
    println!("=== CSS ===\n{}\n", output.css);
    println!("=== JS ===\n{}\n", output.js);
}

fn cmd_build(path: &str, out_dir: &Path, site_url: Option<&str>) -> diagnostic::Locale {
    let (output, locale) = compile(path);

    if let Err(e) = fs::create_dir_all(out_dir) {
        eprintln!("{}", diagnostic::generic_build_failure(locale, out_dir, &e));
        process::exit(1);
    }

    let css_path = out_dir.join("style.css");
    write_file(&css_path, &output.css, locale);

    let js_path = out_dir.join("app.js");
    write_file(&js_path, &output.js, locale);

    let mut routes: Vec<&String> = output.pages.keys().collect();
    routes.sort_by_key(|r| if r.as_str() == "/" { 0 } else { 1 });

    let all_pages_html: String = routes
        .iter()
        .map(|r| output.pages.get(*r).cloned().unwrap_or_default())
        .collect::<Vec<_>>()
        .join("\n");

    let title = output.default_title.as_deref().unwrap_or("ViBao App");
    let description = output.default_description.as_deref().unwrap_or("");
    let public_dir = public_dir_for(Path::new(path));
    let favicon = find_favicon(&public_dir);
    let site_url = site_url.map(deploy::normalize_site_url).filter(|url| !url.is_empty());
    let og_image = deploy::find_og_image(&public_dir);
    let seo_head = match &site_url {
        Some(url) => deploy::head_fragment(url, og_image.as_deref()),
        None => String::new(),
    };
    let full_html = assemble_html_page(&all_pages_html, title, description, favicon.as_deref(), &output.stylesheets, &output.theme_names, &seo_head, locale);
    let index_path = out_dir.join("index.html");
    write_file(&index_path, &full_html, locale);

    if routes.len() > 1 {
        write_file(&out_dir.join("404.html"), &full_html, locale);
    }
    if let Some(url) = &site_url {
        write_file(&out_dir.join("sitemap.xml"), &deploy::sitemap_xml(url, &routes), locale);
        write_file(&out_dir.join("robots.txt"), &deploy::robots_txt(url), locale);
    } else if og_image.is_some() {
        if locale == diagnostic::Locale::Vietnamese {
            eprintln!("Lưu ý: tìm thấy public/{} nhưng chưa có --site-url nên og:image không được ghi (cần URL tuyệt đối).", og_image.as_deref().unwrap_or(""));
        } else {
            eprintln!("Note: found public/{} but no --site-url was given, so og:image is not written (an absolute URL is required).", og_image.as_deref().unwrap_or(""));
        }
    }

    copy_runtime_pkg(out_dir, locale);
    copy_public_dir(&public_dir, out_dir, locale);

    if locale == diagnostic::Locale::Vietnamese {
        println!("Build thành công. Mở file sau trong trình duyệt:");
    } else {
        println!("Build succeeded. Open the following file in a browser:");
    }
    println!("   {}", index_path.display());
    locale
}

fn write_file(path: &Path, content: &str, locale: diagnostic::Locale) {
    if let Err(e) = fs::write(path, content) {
        eprintln!("{}", diagnostic::generic_build_failure(locale, path, &e));
        process::exit(1);
    }
}

fn public_dir_for(source_file: &Path) -> PathBuf {
    source_file
        .parent()
        .filter(|dir| !dir.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
        .join("public")
}

fn find_favicon(public_dir: &Path) -> Option<String> {
    ["favicon.svg", "favicon.png", "favicon.ico"]
        .iter()
        .find(|name| public_dir.join(name).is_file())
        .map(|name| name.to_string())
}

fn favicon_mime(file_name: &str) -> &'static str {
    match Path::new(file_name).extension().and_then(|e| e.to_str()) {
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        _ => "image/x-icon",
    }
}

const THEME_RESTORE_SCRIPT: &str = "\n  <script>try{var r=document.documentElement,t=localStorage.getItem(\"vb-theme\");if(t&&r.getAttribute(\"data-vb-themes\").split(\",\").indexOf(t)>=0)r.setAttribute(\"data-vb-theme\",t)}catch(e){}</script>";

fn assemble_html_page(
    body_html: &str,
    title: &str,
    description: &str,
    favicon: Option<&str>,
    stylesheets: &[String],
    themes: &[String],
    seo_head: &str,
    locale: diagnostic::Locale,
) -> String {
    let title_html = codegen::css::esc_html(title);
    let title_attr = codegen::css::esc_attr(title);
    let description_attr = codegen::css::esc_attr(description);
    let og_description = if description.is_empty() {
        String::new()
    } else {
        format!("\n  <meta property=\"og:description\" content=\"{}\">", description_attr)
    };
    let favicon_link = match favicon {
        Some(name) => format!(
            "\n  <link rel=\"icon\" type=\"{}\" href=\"./{}\">",
            favicon_mime(name),
            codegen::css::esc_attr(name)
        ),
        None => String::new(),
    };
    let mut extra_styles = String::new();
    if stylesheets.iter().any(|url| url.contains("fonts.googleapis.com")) {
        extra_styles.push_str("\n  <link rel=\"preconnect\" href=\"https://fonts.googleapis.com\">");
        extra_styles.push_str("\n  <link rel=\"preconnect\" href=\"https://fonts.gstatic.com\" crossorigin>");
    }
    for url in stylesheets {
        extra_styles.push_str(&format!("\n  <link rel=\"stylesheet\" href=\"{}\">", codegen::css::esc_attr(url)));
    }
    let (html_attrs, theme_script) = if themes.len() >= 2 {
        (format!(" data-vb-themes=\"{}\"", codegen::css::esc_attr(&themes.join(","))), THEME_RESTORE_SCRIPT)
    } else {
        (String::new(), "")
    };
    format!(
        r#"<!DOCTYPE html>
<html lang="{lang}"{html_attrs}>
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>{title}</title>
  <meta name="description" content="{description}">
  <meta property="og:type" content="website">
  <meta property="og:title" content="{title_attr}">{og_description}{seo_head}{favicon_link}{theme_script}{extra_styles}
  <link rel="stylesheet" href="./style.css">
</head>
<body>
{body}
  <script src="./app.js"></script>
</body>
</html>
"#,
        body = body_html,
        title = title_html,
        title_attr = title_attr,
        description = description_attr,
        og_description = og_description,
        favicon_link = favicon_link,
        extra_styles = extra_styles,
        html_attrs = html_attrs,
        seo_head = seo_head,
        theme_script = theme_script,
        lang = locale.code()
    )
}

const RESERVED_OUTPUT_NAMES: [&str; 4] = ["index.html", "app.js", "style.css", "pkg"];

fn copy_public_dir(public_dir: &Path, out_dir: &Path, locale: diagnostic::Locale) {
    if !public_dir.is_dir() {
        return;
    }
    let out_canonical = fs::canonicalize(out_dir).ok();
    if let Err(e) = copy_dir_contents(public_dir, out_dir, true, out_canonical.as_deref(), locale) {
        eprintln!("{}", diagnostic::generic_build_failure(locale, out_dir, &e));
        process::exit(1);
    }
}

fn copy_dir_contents(
    src: &Path,
    dest: &Path,
    top_level: bool,
    out_canonical: Option<&Path>,
    locale: diagnostic::Locale,
) -> std::io::Result<()> {
    fs::create_dir_all(dest)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let name = entry.file_name();
        let from = entry.path();
        let to = dest.join(&name);

        if let Some(out) = out_canonical {
            if fs::canonicalize(&from).ok().as_deref() == Some(out) {
                continue;
            }
        }
        if top_level && RESERVED_OUTPUT_NAMES.iter().any(|reserved| name.to_string_lossy() == *reserved) {
            if locale == diagnostic::Locale::Vietnamese {
                eprintln!("{}", diagnostic::warning(locale, "W9001", &format!("bỏ qua 'public/{}': trùng tên với file do ViBao sinh ra", name.to_string_lossy())));
            } else {
                eprintln!("{}", diagnostic::warning(locale, "W9001", &format!("skipping 'public/{}': the name is reserved for a file generated by ViBao", name.to_string_lossy())));
            }
            continue;
        }
        if entry.file_type()?.is_dir() {
            copy_dir_contents(&from, &to, false, out_canonical, locale)?;
        } else {
            fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

fn copy_runtime_pkg(out_dir: &Path, locale: diagnostic::Locale) {
    let src_dir = runtime_pkg_dir();
    let files = ["vibao_runtime.js", "vibao_runtime_bg.wasm"];
    let dest_pkg_dir = out_dir.join("pkg");

    if !files.iter().all(|file| src_dir.join(file).is_file()) {
        let message = if locale == diagnostic::Locale::Vietnamese {
            format!("Không tìm thấy gói runtime ViBao. '{}' phải chứa vibao_runtime.js và vibao_runtime_bg.wasm.", src_dir.display())
        } else {
            format!("ViBao runtime package was not found. Expected '{}' to contain vibao_runtime.js and vibao_runtime_bg.wasm.", src_dir.display())
        };
        eprintln!("error[E5001] runtime: {}", message);
        eprintln!("  = {}: {}", if locale == diagnostic::Locale::Vietnamese { "gợi ý" } else { "help" }, if locale == diagnostic::Locale::Vietnamese { "chạy 'scripts/build-runtime.sh' khi build từ source, hoặc cài bản release đã đóng gói runtime." } else { "run 'scripts/build-runtime.sh' in the source tree, or install a release build that already bundles the runtime." });
        process::exit(1);
    }

    if let Err(e) = fs::create_dir_all(&dest_pkg_dir) {
        eprintln!("{}", diagnostic::generic_build_failure(locale, &dest_pkg_dir, &e));
        process::exit(1);
    }

    for file in files {
        let src = src_dir.join(file);
        let dest = dest_pkg_dir.join(file);
        if let Err(e) = fs::copy(&src, &dest) {
            if locale == diagnostic::Locale::Vietnamese {
                eprintln!("error[E5002] runtime: không thể sao chép '{}' tới '{}': {}", src.display(), dest.display(), e);
            } else {
                eprintln!("error[E5002] runtime: could not copy '{}' to '{}': {}", src.display(), dest.display(), e);
            }
            process::exit(1);
        }
    }
}

fn runtime_pkg_dir() -> PathBuf {
    if let Ok(path) = env::var("VIBAO_PKG_DIR") {
        return PathBuf::from(path);
    }

    let exe_dir = default_pkg_dir();
    if exe_dir.join("vibao_runtime.js").is_file() && exe_dir.join("vibao_runtime_bg.wasm").is_file() {
        return exe_dir;
    }

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let candidates = [
        manifest_dir.join("../vibao-runtime/pkg"),
        manifest_dir.join("../../vibao-runtime/pkg"),
        manifest_dir.join("pkg"),
    ];
    candidates
        .into_iter()
        .find(|path| path.join("vibao_runtime.js").is_file() && path.join("vibao_runtime_bg.wasm").is_file())
        .unwrap_or(exe_dir)
}

fn default_pkg_dir() -> PathBuf {
    env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|p| p.join("pkg")))
        .unwrap_or_else(|| PathBuf::from("pkg"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(label: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("vibaoc-main-{}-{}", label, nanos));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn html_shell_has_static_description_and_open_graph_tags() {
        let html = assemble_html_page("<p>x</p>", "Trang \"A\" <b>", "Mô tả & thử", None, &[], &[], "", diagnostic::Locale::English);
        assert!(html.contains("<title>Trang \"A\" &lt;b&gt;</title>"), "{}", html);
        assert!(html.contains("<meta name=\"description\" content=\"Mô tả &amp; thử\">"), "{}", html);
        assert!(html.contains("<meta property=\"og:title\" content=\"Trang &quot;A&quot; &lt;b&gt;\">"), "{}", html);
        assert!(html.contains("<meta property=\"og:description\" content=\"Mô tả &amp; thử\">"), "{}", html);
        assert!(!html.contains("rel=\"icon\""));
    }

    #[test]
    fn html_shell_with_themes_declares_them_and_restores_the_saved_one() {
        let themes = vec!["sang".to_string(), "toi".to_string()];
        let html = assemble_html_page("", "T", "", None, &[], &themes, "", diagnostic::Locale::Vietnamese);
        assert!(html.contains("<html lang=\"vi\" data-vb-themes=\"sang,toi\">"), "{}", html);
        assert!(html.contains("localStorage.getItem(\"vb-theme\")"), "{}", html);
        assert!(html.find("localStorage").unwrap() < html.find("style.css").unwrap(), "{}", html);

        let one = assemble_html_page("", "T", "", None, &[], &["sang".to_string()], "", diagnostic::Locale::Vietnamese);
        assert!(!one.contains("data-vb-themes"), "{}", one);
        assert!(!one.contains("localStorage"), "{}", one);
    }

    #[test]
    fn html_shell_chen_the_seo_sau_og_title() {
        let seo = deploy::head_fragment("https://a.com", None);
        let html = assemble_html_page("", "T", "", None, &[], &[], &seo, diagnostic::Locale::Vietnamese);
        assert!(html.contains("<link rel=\"canonical\" href=\"https://a.com/\">"), "{}", html);
        assert!(html.find("og:title").unwrap() < html.find("canonical").unwrap(), "{}", html);
    }

    #[test]
    fn html_shell_omits_og_description_when_empty() {
        let html = assemble_html_page("", "T", "", None, &[], &[], "", diagnostic::Locale::English);
        assert!(html.contains("<meta name=\"description\" content=\"\">"));
        assert!(!html.contains("og:description"));
    }

    #[test]
    fn html_shell_links_the_favicon_with_its_mime_type() {
        let html = assemble_html_page("", "T", "", Some("favicon.svg"), &[], &[], "", diagnostic::Locale::Vietnamese);
        assert!(html.contains("<link rel=\"icon\" type=\"image/svg+xml\" href=\"./favicon.svg\">"), "{}", html);
        assert!(html.contains("<html lang=\"vi\">"));
        assert_eq!(favicon_mime("favicon.png"), "image/png");
        assert_eq!(favicon_mime("favicon.ico"), "image/x-icon");
    }

    #[test]
    fn html_shell_links_requested_stylesheets_and_preconnects_google_fonts() {
        let urls = vec!["https://fonts.googleapis.com/css2?family=Inter&display=swap".to_string()];
        let html = assemble_html_page("", "T", "", None, &urls, &[], "", diagnostic::Locale::English);
        assert!(html.contains("<link rel=\"preconnect\" href=\"https://fonts.googleapis.com\">"), "{}", html);
        assert!(html.contains("<link rel=\"preconnect\" href=\"https://fonts.gstatic.com\" crossorigin>"), "{}", html);
        assert!(html.contains("<link rel=\"stylesheet\" href=\"https://fonts.googleapis.com/css2?family=Inter&amp;display=swap\">"), "{}", html);
        let own = html.find("href=\"./style.css\"").unwrap();
        let font = html.find("fonts.googleapis.com/css2").unwrap();
        assert!(font < own, "the font stylesheet must come before style.css");
    }

    #[test]
    fn html_shell_without_stylesheets_has_no_extra_links() {
        let html = assemble_html_page("", "T", "", None, &[], &[], "", diagnostic::Locale::English);
        assert!(!html.contains("preconnect"));
        assert_eq!(html.matches("rel=\"stylesheet\"").count(), 1);
    }

    #[test]
    fn public_dir_is_next_to_the_source_file() {
        assert_eq!(public_dir_for(Path::new("app.vbao")), PathBuf::from("./public"));
        assert_eq!(public_dir_for(Path::new("site/app.vbao")), PathBuf::from("site/public"));
    }

    #[test]
    fn favicon_is_found_in_preference_order() {
        let dir = temp_dir("favicon");
        assert_eq!(find_favicon(&dir), None);
        fs::write(dir.join("favicon.ico"), b"x").unwrap();
        assert_eq!(find_favicon(&dir).as_deref(), Some("favicon.ico"));
        fs::write(dir.join("favicon.svg"), b"x").unwrap();
        assert_eq!(find_favicon(&dir).as_deref(), Some("favicon.svg"));
    }

    #[test]
    fn public_dir_is_copied_recursively_and_reserved_names_are_skipped() {
        let root = temp_dir("copy");
        let public = root.join("public");
        fs::create_dir_all(public.join("images")).unwrap();
        fs::write(public.join("images").join("a.png"), b"png").unwrap();
        fs::write(public.join("cv.pdf"), b"pdf").unwrap();
        fs::write(public.join("index.html"), b"must not overwrite").unwrap();
        let out = root.join("dist");
        fs::create_dir_all(&out).unwrap();
        fs::write(out.join("index.html"), b"generated").unwrap();

        copy_public_dir(&public, &out, diagnostic::Locale::English);

        assert_eq!(fs::read(out.join("images").join("a.png")).unwrap(), b"png");
        assert_eq!(fs::read(out.join("cv.pdf")).unwrap(), b"pdf");
        assert_eq!(fs::read(out.join("index.html")).unwrap(), b"generated");
    }

    #[test]
    fn output_directory_inside_public_is_not_copied_into_itself() {
        let root = temp_dir("nested");
        let public = root.join("public");
        fs::create_dir_all(&public).unwrap();
        fs::write(public.join("a.txt"), b"a").unwrap();
        let out = public.join("dist");
        fs::create_dir_all(&out).unwrap();

        copy_public_dir(&public, &out, diagnostic::Locale::English);

        assert_eq!(fs::read(out.join("a.txt")).unwrap(), b"a");
        assert!(!out.join("dist").exists());
    }

    #[test]
    fn missing_public_dir_is_not_an_error() {
        let root = temp_dir("none");
        let out = root.join("dist");
        fs::create_dir_all(&out).unwrap();
        copy_public_dir(&root.join("public"), &out, diagnostic::Locale::English);
        assert_eq!(fs::read_dir(&out).unwrap().count(), 0);
    }
}
