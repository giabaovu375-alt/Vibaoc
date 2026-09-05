
mod lexer;
mod locale;
mod parser;
mod codegen;
mod resolver;
mod validator;
mod diagnostic;

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
                    cmd_build(path, &out_dir);
                }
                "check" => {
                    let ast_only = args.iter().any(|a| a == "--ast");
                    cmd_check(path, ast_only);
                }
                _ => unreachable!(),
            }
        }
        Some(path) if path.ends_with(".vbao") => {
            cmd_build(path, Path::new("dist"));
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


fn cmd_build(path: &str, out_dir: &Path) {
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

    let full_html = assemble_html_page(&all_pages_html, locale);
    let index_path = out_dir.join("index.html");
    write_file(&index_path, &full_html, locale);

    copy_runtime_pkg(out_dir, locale);

    if locale == diagnostic::Locale::Vietnamese {
        println!("Build thành công. Mở file sau trong trình duyệt:");
    } else {
        println!("Build succeeded. Open the following file in a browser:");
    }
    println!("   {}", index_path.display());
}

fn write_file(path: &Path, content: &str, locale: diagnostic::Locale) {
    if let Err(e) = fs::write(path, content) {
        eprintln!("{}", diagnostic::generic_build_failure(locale, path, &e));
        process::exit(1);
    }
}

fn assemble_html_page(body_html: &str, locale: diagnostic::Locale) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="{lang}">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>ViBao App</title>
  <link rel="stylesheet" href="./style.css">
</head>
<body>
{body}
  <script src="./app.js"></script>
</body>
</html>
"#,
        body = body_html,
        lang = locale.code()
    )
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
