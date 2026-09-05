use std::fmt::Write as _;
use std::path::Path;

/// User-facing diagnostic locale. English is always available and is the
/// fallback when no `lang` directive is present or when a source language is
/// not yet implemented.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Locale {
    English,
    Vietnamese,
}

impl Locale {
    pub fn detect_from_source(source: &str) -> Self {
        for raw in source.lines().take(8) {
            let line = raw.trim();
            if line.starts_with("//") || line.starts_with("/*") || line.is_empty() { continue; }
            if let Some(rest) = line.strip_prefix("lang") {
                let rest = rest.trim_start();
                if let Some(rest) = rest.strip_prefix('=') {
                    let rest = rest.trim_start();
                    if let Some(rest) = rest.strip_prefix('"') {
                        if let Some(end) = rest.find('"') {
                            if let Some(locale) = Self::from_code(&rest[..end]) { return locale; }
                        }
                    }
                }
            }
            break;
        }
        Self::English
    }

    pub fn from_code(code: &str) -> Option<Self> {
        match code {
            "en" => Some(Self::English),
            "vi" => Some(Self::Vietnamese),
            _ => None,
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            Self::English => "en",
            Self::Vietnamese => "vi",
        }
    }
}

/// Render a compiler diagnostic with a source span. The message itself is
/// localized at the final user-facing boundary so parser/codegen internals do
/// not need to carry UI-language concerns through every layer.
pub fn render(
    locale: Locale,
    level: &str,
    code: &str,
    stage: &str,
    path: &Path,
    source: &str,
    line: usize,
    column: usize,
    message: &str,
    help: Option<&str>,
) -> String {
    let line = line.max(1);
    let column = column.max(1);
    let lines: Vec<&str> = source.lines().collect();
    let source_line = lines.get(line.saturating_sub(1)).copied().unwrap_or("");
    let caret_column = column.saturating_sub(1).min(source_line.chars().count());
    let width = line.to_string().len();
    let mut out = String::new();
    let message = localize_message(locale, message);
    let stage = localize_stage(locale, stage);

    let _ = writeln!(out, "{level}[{code}] {stage}: {message}");
    let _ = writeln!(out, "  --> {}:{}:{}", path.display(), line, column);
    let _ = writeln!(out, "   |\n{line:>width$} | {source_line}", line = line, width = width);
    let _ = writeln!(
        out,
        "{space:>width$} | ^",
        space = "",
        width = width + caret_column,
    );
    if let Some(help) = help {
        let help = localize_message(locale, help);
        let label = if locale == Locale::Vietnamese { "gợi ý" } else { "help" };
        let _ = writeln!(out, "   |\n   = {label}: {help}");
    }
    out
}

pub fn warning(locale: Locale, code: &str, message: &str) -> String {
    let label = if locale == Locale::Vietnamese { "cảnh báo" } else { "warning" };
    format!("{label}[{code}]: {}", localize_warning(locale, message))
}

pub fn missing_file(locale: Locale, path: &Path, error: &std::io::Error) -> String {
    let message = match locale {
        Locale::English => format!("Could not read '{}': {}", path.display(), error),
        Locale::Vietnamese => format!("Không thể đọc '{}': {}", path.display(), error),
    };
    format!("error[E0001] {}", message)
}

pub fn generic_build_failure(locale: Locale, path: &Path, error: &std::io::Error) -> String {
    let message = match locale {
        Locale::English => format!("Could not write '{}': {}", path.display(), error),
        Locale::Vietnamese => format!("Không thể ghi '{}': {}", path.display(), error),
    };
    format!("error[E0002] {}", message)
}

pub fn usage() -> &'static str {
    "Usage:\n  vibaoc build <file.vbao> [--out <directory>]\n      Compile a ViBao application and write a browser-ready dist/ directory.\n  vibaoc check <file.vbao> [--ast]\n      Validate and inspect a ViBao application without writing output files.\n  vibaoc <file.vbao>\n      Compile <file.vbao> to ./dist/ (tsc-style shorthand).\n  vibaoc --version\n      Print the compiler version.\n  vibaoc --help\n      Show this help."
}

fn localize_stage(locale: Locale, stage: &str) -> String {
    if locale == Locale::English { return stage.to_string(); }
    match stage {
        "lexer" => "bộ phân tích từ vựng".into(),
        "parser" => "bộ phân tích cú pháp".into(),
        "resolver" => "bộ phân giải module".into(),
        "semantic analysis" => "phân tích ngữ nghĩa".into(),
        "runtime" => "runtime".into(),
        other => other.to_string(),
    }
}

fn localize_message(locale: Locale, message: &str) -> String {
    if locale == Locale::English {
        let mut s = message.to_string();
        for (from, to) in [
            ("định danh '", "identifier '"),
            ("từ khóa '", "keyword '"),
            ("thành phần '", "component '"),
            ("chuỗi \"", "string \""),
            ("số ", "number "),
            ("giá trị luận lý ", "boolean value "),
            ("tên màu '", "color name '"),
            ("biến '$", "variable '$"),
            ("kết thúc file", "end of file"),
        ] { s = s.replace(from, to); }
        return s;
    }


    if let Some(rest) = message.strip_prefix("Expected an application name string, received ") {
        return format!("Cần một chuỗi tên ứng dụng, nhận được {}", localize_message(Locale::English, rest));
    }
    if let Some(rest) = message.strip_prefix("Expected a component name, received ") {
        return format!("Cần tên component, nhận được {}", localize_message(Locale::English, rest));
    }
    if let Some(rest) = message.strip_prefix("Expected a theme identifier, received ") {
        return format!("Cần tên định danh theme, nhận được {}", localize_message(Locale::English, rest));
    }
    if let Some(rest) = message.strip_prefix("Expected a route string, received ") {
        return format!("Cần chuỗi route, nhận được {}", localize_message(Locale::English, rest));
    }
    if let Some(rest) = message.strip_prefix("Expected an identifier, received ") {
        return format!("Cần một định danh, nhận được {}", localize_message(Locale::English, rest));
    }
    if let Some(rest) = message.strip_prefix("Expected a variable name starting with $, received ") {
        return format!("Cần tên biến bắt đầu bằng $, nhận được {}", localize_message(Locale::English, rest));
    }
    if let Some(rest) = message.strip_prefix("Expected a valid range start, received ") {
        return format!("Cần điểm đầu hợp lệ của khoảng, nhận được {}", localize_message(Locale::English, rest));
    }
    if let Some(rest) = message.strip_prefix("Expected a valid range end, received ") {
        return format!("Cần điểm cuối hợp lệ của khoảng, nhận được {}", localize_message(Locale::English, rest));
    }
    if let Some(rest) = message.strip_prefix("Invalid declaration inside the application block: ") {
        return format!("Khai báo không hợp lệ bên trong khối ứng dụng: {}", localize_message(Locale::English, rest));
    }
    if let Some(rest) = message.strip_prefix("Could not parse value: ") {
        return format!("Không thể phân tích giá trị: {}", localize_message(Locale::English, rest));
    }
    if let Some(rest) = message.strip_prefix("Unrecognized character: ") {
        return format!("Ký tự không được nhận diện: {}", rest);
    }
    if message == "The 'lang' declaration must appear at the top of the file" {
        return "Khai báo 'lang' phải nằm ở đầu file".into();
    }
    if message == "Expected ';' after the language declaration" {
        return "Cần ';' sau khai báo ngôn ngữ".into();
    }
    if message == "Expected '=' after 'lang'" {
        return "Cần '=' sau 'lang'".into();
    }
    if message.starts_with("Unsupported source language '") {
        return message
            .replace("Unsupported source language", "Ngôn ngữ source chưa được hỗ trợ")
            .replace("English is always available; supported diagnostic languages", "English luôn được hỗ trợ; các ngôn ngữ diagnostics hiện có");
    }

    // Generic parser phrasing. This intentionally handles wording, not
    // semantics: token descriptions are converted separately below.
    let mut s = message.to_string();
    for (from, to) in [
        ("Expected ", "Cần "),
        ("expected ", "cần "),
        ("received ", "nhận được "),
        ("Received: ", "Nhận được: "),
        ("Could not ", "Không thể "),
        ("could not ", "không thể "),
        ("Invalid ", "Không hợp lệ: "),
        ("Unknown ", "Không xác định: "),
        ("Unsupported ", "Chưa được hỗ trợ: "),
        ("Missing ", "Thiếu "),
        ("must ", "phải "),
        ("inside the application block", "bên trong khối ứng dụng"),
        ("to close ", "để đóng "),
        ("to start ", "để bắt đầu "),
        ("after ", "sau "),
        ("before ", "trước "),
        ("and ", "và "),
        ("or ", "hoặc "),
    ] { s = s.replace(from, to); }
    let s = localize_message(Locale::English, &s);
    s.replace("identifier '", "định danh '")
        .replace("keyword '", "từ khóa '")
        .replace("component '", "thành phần '")
        .replace("string \"", "chuỗi \"")
        .replace("number ", "số ")
        .replace("boolean value ", "giá trị luận lý ")
        .replace("color name '", "tên màu '")
        .replace("variable '$", "biến '$")
        .replace("end of file", "kết thúc file")
}

fn localize_warning(locale: Locale, message: &str) -> String {
    if locale == Locale::Vietnamese { return message.to_string(); }

    if let Some(rest) = message.strip_prefix("prop '") {
        if let Some((key, rest)) = rest.split_once("' trên thẻ layout '") {
            if let Some((tag, _)) = rest.split_once("' không được ViBao nhận diện") {
                return format!(
                    "prop '{}' on layout tag '{}' is not recognized by ViBao — the name may be misspelled or may belong to another layout type. Unknown props on layout elements are omitted from CSS/HTML attributes.",
                    key, tag
                );
            }
        }
        if let Some((key, rest)) = rest.split_once("' trên thẻ '") {
            if let Some((tag, _)) = rest.split_once("' không được ViBao nhận diện") {
                return format!(
                    "prop '{}' on tag '{}' is not recognized by ViBao — the prop name may be misspelled. Unknown props are passed through as HTML attributes.",
                    key, tag
                );
            }
        }
    }

    let mut s = message.to_string();
    for (from, to) in [
        ("thẻ '", "tag '"),
        ("trên thẻ '", "on tag '"),
        ("không được ViBao nhận diện", "is not recognized by ViBao"),
        ("có thể do gõ sai tên prop", "the prop name may be misspelled"),
        ("không được hỗ trợ", "is not supported"),
        ("giá trị biểu thức động", "a dynamic expression value"),
        ("chưa được hỗ trợ", "is not supported yet"),
        ("Style sẽ KHÔNG được áp dụng", "The style will NOT be applied"),
        ("Vẫn hoạt động đúng", "It still works correctly"),
        ("nên bọc trong 1 container", "consider wrapping it in a container"),
        ("bị định nghĩa lại", "was redefined"),
        ("chưa được định nghĩa", "has not been defined"),
        ("dòng ", "line "),
        ("phần tử top-level", "top-level elements"),
        ("không được để trống", "must not be empty"),
        ("cần một object", "requires an object"),
    ] { s = s.replace(from, to); }
    s
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn renders_source_location_and_caret() {
        let rendered = render(
            Locale::English,
            "error",
            "E2001",
            "parser",
            Path::new("app.vbao"),
            "page(\"/\") {\n  text(\"hello\")\n}\n",
            2,
            3,
            "Expected a property name",
            Some("check the spelling of the property"),
        );
        assert!(rendered.contains("error[E2001] parser: Expected a property name"));
        assert!(rendered.contains("--> app.vbao:2:3"));
        assert!(rendered.contains("text(\"hello\")"));
        assert!(rendered.contains("^"));
        assert!(rendered.contains("help: check the spelling of the property"));
    }

    #[test]
    fn vietnamese_locale_changes_user_facing_labels() {
        let rendered = render(
            Locale::Vietnamese,
            "error",
            "E2001",
            "parser",
            Path::new("app.vbao"),
            "ung_dung(\"x\") {\n}\n",
            1,
            1,
            "Expected an application name string, received identifier 'x'",
            Some("check the spelling"),
        );
        assert!(rendered.contains("error[E2001] bộ phân tích cú pháp:"));
        assert!(rendered.contains("Cần một chuỗi tên ứng dụng"));
        assert!(rendered.contains("gợi ý:"));
    }

    #[test]
    fn english_token_fragments_are_english() {
        let rendered = render(
            Locale::English,
            "error",
            "E2001",
            "parser",
            Path::new("app.vbao"),
            "x",
            1,
            1,
            "Expected an identifier, received định danh 'x'",
            None,
        );
        assert!(rendered.contains("received identifier 'x'"));
    }
}
