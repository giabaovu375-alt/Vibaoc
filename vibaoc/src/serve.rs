use std::env;
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{self, Command};
use std::thread;
use std::time::{Duration, SystemTime};

use crate::diagnostic::Locale;

pub struct ServeOptions {
    pub source: PathBuf,
    pub out_dir: PathBuf,
    pub host: String,
    pub port: u16,
    pub watch: bool,
    pub locale: Locale,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Target {
    File(PathBuf),
    SpaFallback(PathBuf),
    NotFound,
    Forbidden,
}

pub struct Reply {
    pub status: u16,
    pub reason: &'static str,
    pub content_type: &'static str,
    pub body: Vec<u8>,
}

pub fn parse_request_line(line: &str) -> Option<(String, String)> {
    let mut parts = line.split_whitespace();
    let method = parts.next()?;
    let target = parts.next()?;
    Some((method.to_string(), target.to_string()))
}

fn percent_decode(input: &str) -> Option<String> {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = input.get(i + 1..i + 3)?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

pub fn resolve_target(root: &Path, target: &str) -> Target {
    let path_part = target.split(|c| c == '?' || c == '#').next().unwrap_or("");
    let decoded = match percent_decode(path_part) {
        Some(decoded) => decoded,
        None => return Target::Forbidden,
    };

    let mut candidate = root.to_path_buf();
    let mut last_has_extension = false;
    for segment in decoded.split('/') {
        if segment.is_empty() || segment == "." {
            continue;
        }
        if segment == ".."
            || segment.contains('\\')
            || segment.contains(':')
            || segment.contains('\0')
        {
            return Target::Forbidden;
        }
        candidate.push(segment);
        last_has_extension = segment.contains('.');
    }

    if candidate.is_dir() {
        candidate.push("index.html");
        last_has_extension = true;
    }
    if candidate.is_file() {
        return Target::File(candidate);
    }

    let index = root.join("index.html");
    if !last_has_extension && index.is_file() {
        return Target::SpaFallback(index);
    }
    Target::NotFound
}

pub fn mime_for(path: &Path) -> &'static str {
    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .unwrap_or_default();
    match extension.as_str() {
        "html" | "htm" => "text/html; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" => "application/json; charset=utf-8",
        "wasm" => "application/wasm",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "ico" => "image/x-icon",
        "pdf" => "application/pdf",
        "txt" => "text/plain; charset=utf-8",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        _ => "application/octet-stream",
    }
}

fn text_reply(status: u16, reason: &'static str) -> Reply {
    Reply {
        status,
        reason,
        content_type: "text/plain; charset=utf-8",
        body: format!("{} {}", status, reason).into_bytes(),
    }
}

pub fn reply_for(root: &Path, request_line: &str) -> (Reply, bool) {
    let (method, target) = match parse_request_line(request_line) {
        Some(parsed) => parsed,
        None => return (text_reply(400, "Bad Request"), false),
    };
    if method != "GET" && method != "HEAD" {
        return (text_reply(405, "Method Not Allowed"), false);
    }
    let head_only = method == "HEAD";
    let reply = match resolve_target(root, &target) {
        Target::File(path) | Target::SpaFallback(path) => match fs::read(&path) {
            Ok(body) => Reply {
                status: 200,
                reason: "OK",
                content_type: mime_for(&path),
                body,
            },
            Err(_) => text_reply(500, "Internal Server Error"),
        },
        Target::Forbidden => text_reply(403, "Forbidden"),
        Target::NotFound => text_reply(404, "Not Found"),
    };
    (reply, head_only)
}

fn handle_connection(mut stream: TcpStream, root: &Path) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let mut buffer = [0u8; 8192];
    let read = match stream.read(&mut buffer) {
        Ok(n) if n > 0 => n,
        _ => return,
    };
    let request = String::from_utf8_lossy(&buffer[..read]).to_string();
    let first_line = request.lines().next().unwrap_or("");
    let (reply, head_only) = reply_for(root, first_line);
    println!("{} -> {}", first_line, reply.status);

    let header = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
        reply.status,
        reply.reason,
        reply.content_type,
        reply.body.len()
    );
    let _ = stream.write_all(header.as_bytes());
    if !head_only {
        let _ = stream.write_all(&reply.body);
    }
    let _ = stream.flush();
}

fn newest_change(dir: &Path, skip: &Path, depth: usize) -> Option<SystemTime> {
    if depth > 6 {
        return None;
    }
    let mut newest: Option<SystemTime> = None;
    for entry in fs::read_dir(dir).ok()?.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with('.') || name == "target" || name == "node_modules" {
            continue;
        }
        let path = entry.path();
        if fs::canonicalize(&path)
            .map(|canonical| canonical == skip)
            .unwrap_or(false)
        {
            continue;
        }
        let modified = if path.is_dir() {
            newest_change(&path, skip, depth + 1)
        } else {
            entry.metadata().ok().and_then(|m| m.modified().ok())
        };
        newest = newest.max(modified);
    }
    newest
}

fn watch(source: PathBuf, out_dir: PathBuf, locale: Locale) {
    let root = source
        .parent()
        .filter(|dir| !dir.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();
    let skip = fs::canonicalize(&out_dir).unwrap_or_else(|_| out_dir.clone());
    let mut last = newest_change(&root, &skip, 0);
    loop {
        thread::sleep(Duration::from_millis(700));
        let current = newest_change(&root, &skip, 0);
        if current == last {
            continue;
        }
        last = current;
        if locale == Locale::Vietnamese {
            println!("Phát hiện thay đổi, đang build lại...");
        } else {
            println!("Change detected, rebuilding...");
        }
        let exe = match env::current_exe() {
            Ok(exe) => exe,
            Err(_) => continue,
        };
        let status = Command::new(exe)
            .arg("build")
            .arg(&source)
            .arg("--out")
            .arg(&out_dir)
            .status();
        let ok = matches!(status, Ok(s) if s.success());
        match (ok, locale == Locale::Vietnamese) {
            (true, true) => println!("Đã build lại. Tải lại trang (Ctrl+Shift+R) để xem."),
            (true, false) => println!("Rebuilt. Reload the page (Ctrl+Shift+R) to see it."),
            (false, true) => println!("Build lỗi, giữ nguyên bản trước. Sửa lỗi rồi lưu lại."),
            (false, false) => {
                println!("Build failed, keeping the previous output. Fix the error and save again.")
            }
        }
    }
}

pub fn run(options: ServeOptions) {
    let address = format!("{}:{}", options.host, options.port);
    let listener = match TcpListener::bind(&address) {
        Ok(listener) => listener,
        Err(e) => {
            if options.locale == Locale::Vietnamese {
                eprintln!(
                    "error: không mở được {} ({}). Thử đổi cổng bằng --port.",
                    address, e
                );
            } else {
                eprintln!(
                    "error: cannot listen on {} ({}). Try another port with --port.",
                    address, e
                );
            }
            process::exit(1);
        }
    };

    if options.locale == Locale::Vietnamese {
        println!(
            "Đang chạy server tại http://{}:{}/ (Ctrl+C để dừng)",
            options.host, options.port
        );
        if options.host == "0.0.0.0" {
            println!(
                "Truy cập từ thiết bị khác trong cùng mạng bằng http://<IP máy này>:{}/",
                options.port
            );
        }
        if options.watch {
            println!("Tự build lại khi file nguồn hoặc public/ thay đổi.");
        }
    } else {
        println!(
            "Serving at http://{}:{}/ (Ctrl+C to stop)",
            options.host, options.port
        );
        if options.host == "0.0.0.0" {
            println!("Open it from another device on the same network with http://<this machine's IP>:{}/", options.port);
        }
        if options.watch {
            println!("Rebuilding automatically when the source or public/ changes.");
        }
    }

    if options.watch {
        let source = options.source.clone();
        let out_dir = options.out_dir.clone();
        let locale = options.locale;
        thread::spawn(move || watch(source, out_dir, locale));
    }

    let root = options.out_dir.clone();
    for stream in listener.incoming().flatten() {
        let root = root.clone();
        thread::spawn(move || handle_connection(stream, &root));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn thu_muc_tam(nhan: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = env::temp_dir().join(format!("vibaoc-serve-{}-{}", nhan, nanos));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn dung_site(nhan: &str) -> PathBuf {
        let root = thu_muc_tam(nhan);
        fs::write(root.join("index.html"), "<p>Trang chủ</p>").unwrap();
        fs::write(root.join("app.js"), "console.log('xin chào')").unwrap();
        fs::create_dir_all(root.join("pkg")).unwrap();
        fs::write(root.join("pkg").join("runtime_bg.wasm"), b"\0asm").unwrap();
        fs::write(root.join("ảnh đại diện.svg"), "<svg/>").unwrap();
        root
    }

    #[test]
    fn tach_dong_yeu_cau_lay_phuong_thuc_va_duong_dan() {
        assert_eq!(
            parse_request_line("GET /du-an?x=1 HTTP/1.1"),
            Some(("GET".to_string(), "/du-an?x=1".to_string()))
        );
        assert_eq!(parse_request_line(""), None);
        assert_eq!(parse_request_line("GET"), None);
    }

    #[test]
    fn giai_ma_phan_tram_ky_tu_tieng_viet() {
        assert_eq!(percent_decode("%E1%BA%A3nh").as_deref(), Some("ảnh"));
        assert_eq!(percent_decode("a%20b").as_deref(), Some("a b"));
        assert_eq!(percent_decode("%zz"), None);
        assert_eq!(percent_decode("%4"), None);
    }

    #[test]
    fn trang_chu_tra_ve_index_html() {
        let root = dung_site("trang-chu");
        assert_eq!(
            resolve_target(&root, "/"),
            Target::File(root.join("index.html"))
        );
        assert_eq!(
            resolve_target(&root, "/?v=2"),
            Target::File(root.join("index.html"))
        );
    }

    #[test]
    fn file_tinh_va_file_trong_thu_muc_con_duoc_phuc_vu() {
        let root = dung_site("file-tinh");
        assert_eq!(
            resolve_target(&root, "/app.js"),
            Target::File(root.join("app.js"))
        );
        assert_eq!(
            resolve_target(&root, "/pkg/runtime_bg.wasm"),
            Target::File(root.join("pkg").join("runtime_bg.wasm"))
        );
    }

    #[test]
    fn ten_file_co_dau_va_khoang_trang_duoc_giai_ma() {
        let root = dung_site("co-dau");
        assert_eq!(
            resolve_target(&root, "/%E1%BA%A3nh%20%C4%91%E1%BA%A1i%20di%E1%BB%87n.svg"),
            Target::File(root.join("ảnh đại diện.svg"))
        );
    }

    #[test]
    fn duong_dan_khong_co_phan_mo_rong_quay_ve_index_html_cho_spa() {
        let root = dung_site("spa");
        assert_eq!(
            resolve_target(&root, "/du-an/3"),
            Target::SpaFallback(root.join("index.html"))
        );
    }

    #[test]
    fn file_co_phan_mo_rong_nhung_khong_ton_tai_la_404() {
        let root = dung_site("404");
        assert_eq!(resolve_target(&root, "/khong-co.png"), Target::NotFound);
    }

    #[test]
    fn chan_thoat_ra_ngoai_thu_muc_goc() {
        let root = dung_site("chan-thoat");
        for target in [
            "/../Cargo.toml",
            "/%2e%2e/etc/passwd",
            "/a/../../b",
            "/a\\b",
            "/C:/windows",
        ] {
            assert_eq!(
                resolve_target(&root, target),
                Target::Forbidden,
                "{}",
                target
            );
        }
    }

    #[test]
    fn kieu_mime_cua_wasm_va_cac_file_pho_bien() {
        assert_eq!(mime_for(Path::new("a.wasm")), "application/wasm");
        assert_eq!(
            mime_for(Path::new("a.JS")),
            "text/javascript; charset=utf-8"
        );
        assert_eq!(mime_for(Path::new("a.svg")), "image/svg+xml");
        assert_eq!(mime_for(Path::new("a.html")), "text/html; charset=utf-8");
        assert_eq!(
            mime_for(Path::new("a.khong_biet")),
            "application/octet-stream"
        );
        assert_eq!(
            mime_for(Path::new("khong-co-duoi")),
            "application/octet-stream"
        );
    }

    #[test]
    fn tra_loi_get_head_va_cac_phuong_thuc_khac() {
        let root = dung_site("tra-loi");
        let (reply, head_only) = reply_for(&root, "GET / HTTP/1.1");
        assert_eq!(reply.status, 200);
        assert_eq!(reply.body, "<p>Trang chủ</p>".as_bytes());
        assert!(!head_only);

        let (reply, head_only) = reply_for(&root, "HEAD /app.js HTTP/1.1");
        assert_eq!(reply.status, 200);
        assert!(head_only);

        assert_eq!(reply_for(&root, "POST / HTTP/1.1").0.status, 405);
        assert_eq!(reply_for(&root, "").0.status, 400);
        assert_eq!(reply_for(&root, "GET /../x HTTP/1.1").0.status, 403);
        assert_eq!(reply_for(&root, "GET /khong-co.png HTTP/1.1").0.status, 404);
    }

    #[test]
    fn phat_hien_file_moi_sua_va_bo_qua_thu_muc_dau_ra() {
        let root = thu_muc_tam("theo-doi");
        let out = root.join("dist");
        fs::create_dir_all(&out).unwrap();
        fs::write(root.join("app.vbao"), "a").unwrap();
        let skip = fs::canonicalize(&out).unwrap();
        let truoc = newest_change(&root, &skip, 0);
        assert!(truoc.is_some());

        thread::sleep(Duration::from_millis(1100));
        fs::write(out.join("index.html"), "đầu ra mới").unwrap();
        assert_eq!(
            newest_change(&root, &skip, 0),
            truoc,
            "thay đổi trong thư mục đầu ra phải bị bỏ qua"
        );

        fs::write(root.join("app.vbao"), "b").unwrap();
        assert_ne!(
            newest_change(&root, &skip, 0),
            truoc,
            "sửa file nguồn phải được phát hiện"
        );
    }
}
