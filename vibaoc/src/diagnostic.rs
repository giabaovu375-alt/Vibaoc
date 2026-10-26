use std::fmt::Write as _;
use std::path::Path;

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
    "Usage:\n  vibaoc build <file.vbao> [--out <directory>] [--site-url <url>]\n      Compile a ViBao application and write a browser-ready dist/ directory.\n      With --site-url it also writes canonical/og tags, sitemap.xml and robots.txt.\n  vibaoc check <file.vbao> [--ast]\n      Validate and inspect a ViBao application without writing output files.\n  vibaoc serve <file.vbao> [--out <directory>] [--port <port>] [--host <address>] [--no-watch]\n      Build, then serve the result locally and rebuild when the source or public/ changes.\n  vibaoc <file.vbao>\n      Compile <file.vbao> to ./dist/ (tsc-style shorthand).\n  vibaoc --version\n      Print the compiler version.\n  vibaoc --help\n      Show this help."
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

    if let Some(localized) = localize_block_message(message) {
        return localized;
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

fn localize_block_message(message: &str) -> Option<String> {
    localize_requires_block(message)
        .or_else(|| localize_misplaced_prop(message))
        .or_else(|| localize_unusable_prop(message))
        .or_else(|| localize_duplicate_block(message))
        .or_else(|| localize_identity_key(message))
        .or_else(|| localize_component_block(message))
        .or_else(|| localize_response_handler(message))
        .or_else(|| localize_theme_message(message))
}

fn fill_pattern(message: &str, english: &str, vietnamese: &str) -> Option<String> {
    let english_parts: Vec<&str> = english.split("{}").collect();
    let mut rest = message.strip_prefix(english_parts[0])?;
    let mut captured: Vec<&str> = Vec::new();
    for (index, part) in english_parts.iter().enumerate().skip(1) {
        let is_last = index == english_parts.len() - 1;
        let end = if is_last {
            if !rest.ends_with(part) {
                return None;
            }
            rest.len() - part.len()
        } else {
            rest.find(part)?
        };
        captured.push(&rest[..end]);
        rest = &rest[end + part.len()..];
    }
    if !rest.is_empty() || captured.iter().any(|value| value.contains('"')) {
        return None;
    }
    let mut out = String::new();
    for (index, part) in vietnamese.split("{}").enumerate() {
        out.push_str(part);
        if let Some(value) = captured.get(index) {
            out.push_str(value);
        }
    }
    Some(out)
}

fn localize_theme_message(message: &str) -> Option<String> {
    const PATTERNS: [(&str, &str); 17] = [
        ("Theme name \"{}\" may only contain letters, digits, '_' and '-'",
         "Tên theme \"{}\" chỉ được chứa chữ cái, chữ số, '_' và '-'"),
        ("Theme \"{}\" is declared more than once",
         "Theme \"{}\" được khai báo nhiều lần"),
        ("Theme \"{}\" declares no variables",
         "Theme \"{}\" không khai báo biến nào"),
        ("Theme \"{}\": variable \"${}\" is declared more than once",
         "Theme \"{}\": biến \"${}\" được khai báo nhiều lần"),
        ("Theme \"{}\" is missing variable \"${}\" (declared in theme \"{}\"): every theme must declare the same variables",
         "Theme \"{}\" thiếu biến \"${}\" (đã khai báo ở theme \"{}\"): mọi theme phải khai báo cùng một tập biến"),
        ("Theme \"{}\": the value of \"${}\" must be a constant (color, number or text)",
         "Theme \"{}\": giá trị của \"${}\" phải là hằng số (màu, số hoặc chuỗi)"),
        ("Theme \"{}\": the value of \"${}\" must not contain any of { } ; < >",
         "Theme \"{}\": giá trị của \"${}\" không được chứa ký tự { } ; < >"),
        ("Theme variable \"${}\" can only be used as the whole value of a style property (color, size, spacing, text, layout or position)",
         "Biến theme \"${}\" chỉ dùng được làm toàn bộ giá trị của một prop kiểu dáng (màu, kích thước, khoảng cách, chữ, bố cục hoặc vị trí)"),
        ("The theme action is used but no theme is declared (chu_de / theme)",
         "Action đổi theme được dùng nhưng chưa khai báo theme nào (chu_de / theme)"),
        ("The theme \"{}\" does not exist: declared themes are {}",
         "Theme \"{}\" không tồn tại: các theme đã khai báo là {}"),
        ("The theme action takes either no argument or one theme name written as a text literal",
         "Action đổi theme nhận 0 đối số hoặc 1 tên theme viết dưới dạng chuỗi"),
        ("Theme variable \"${}\" is a constant and cannot be assigned",
         "Biến theme \"${}\" là hằng số, không thể gán giá trị"),
        ("The state \"${}\" has the same name as a theme variable: rename one of them",
         "State \"${}\" trùng tên với một biến theme: hãy đổi tên một trong hai"),
        ("The variable \"${}\" has the same name as a theme variable: rename one of them",
         "Biến toàn cục \"${}\" trùng tên với một biến theme: hãy đổi tên một trong hai"),
        ("The component parameter \"${}\" has the same name as a theme variable: rename one of them",
         "Tham số component \"${}\" trùng tên với một biến theme: hãy đổi tên một trong hai"),
        ("The loop variable \"${}\" has the same name as a theme variable: rename one of them",
         "Biến vòng lặp \"${}\" trùng tên với một biến theme: hãy đổi tên một trong hai"),
        ("Property \"{}\" is not valid in the global style block (giao_dien). Accepted: phong_chu, co, mau, mau_nen, hang, tai_phong, cuon_muot, cuon_lui.",
         "Prop \"{}\" không hợp lệ trong khối giao_dien. Chấp nhận: phong_chu, co, mau, mau_nen, hang, tai_phong, cuon_muot, cuon_lui."),
    ];
    PATTERNS
        .iter()
        .find_map(|(english, vietnamese)| fill_pattern(message, english, vietnamese))
}

fn localize_response_handler(message: &str) -> Option<String> {
    let rest = message.strip_prefix("Unknown response handler '")?;
    let (name, rest) = rest.split_once("': expected 'thanh_cong' (or 'success') or 'that_bai' (or 'failure')")?;
    if !rest.is_empty() {
        return None;
    }
    Some(format!(
        "Khối xử lý phản hồi '{}' không hợp lệ: cần 'thanh_cong' (hoặc 'success') hay 'that_bai' (hoặc 'failure')",
        name
    ))
}

fn localize_requires_block(message: &str) -> Option<String> {
    let rest = message.strip_prefix("Tag '")?;
    let (tag, rest) = rest.split_once("' requires block syntax for property '")?;
    let (key, rest) = rest.split_once("': ")?;
    if let Some(example) = rest.strip_prefix("write ") {
        let example = example.strip_suffix(" instead of using '(...)'")?;
        return Some(format!(
            "Tag '{}' yêu cầu dùng khối cho prop '{}': viết {} thay vì đặt trong '(...)'",
            tag, key, example
        ));
    }
    if rest == "declare it inside an @block { ... } in the tag body instead of using '(...)'" {
        return Some(format!(
            "Tag '{}' yêu cầu dùng khối cho prop '{}': khai báo trong một khối @ten_khoi {{ ... }} ở thân tag thay vì đặt trong '(...)'",
            tag, key
        ));
    }
    None
}

fn localize_misplaced_prop(message: &str) -> Option<String> {
    let rest = message.strip_prefix("Property '")?;
    let (key, rest) = rest.split_once("' belongs to block @")?;
    let (target, rest) = rest.split_once(", not @")?;
    let (declared, example) = rest.split_once(": move it to ")?;
    Some(format!(
        "Prop '{}' thuộc khối @{}, không phải @{}: chuyển vào {}",
        key, target, declared, example
    ))
}

fn localize_unusable_prop(message: &str) -> Option<String> {
    let rest = message.strip_prefix("Property '")?;
    let (key, rest) = rest.split_once("' cannot be used inside block @")?;
    if let Some(block) = rest.strip_suffix(": animation properties are written in '(...)' of the tag") {
        return Some(format!(
            "Prop '{}' không dùng được trong khối @{}: prop animation viết trong '(...)' của tag",
            key, block
        ));
    }
    Some(format!("Prop '{}' không dùng được trong khối @{}", key, rest))
}

fn localize_duplicate_block(message: &str) -> Option<String> {
    let rest = message.strip_prefix("Block '@")?;
    let (name, rest) = rest.split_once("' is already declared on this tag and cannot be repeated: merge it into a single '@")?;
    rest.strip_suffix("' block")?;
    Some(format!(
        "Khối '@{}' đã được khai báo trên tag này và không thể lặp lại: gộp vào một khối '@{}' duy nhất",
        name, name
    ))
}

fn localize_identity_key(message: &str) -> Option<String> {
    let rest = message.strip_prefix('\'')?;
    let (key, rest) = rest.split_once("' is not valid in block @")?;
    if let Some(block) = rest.strip_suffix(": only 'id', 'lop' and 'the_html' are accepted") {
        return Some(format!(
            "'{}' không hợp lệ trong khối @{}: chỉ chấp nhận 'id', 'lop' và 'the_html'",
            key, block
        ));
    }
    let block = rest.strip_suffix(": only 'id' and 'lop' are accepted")?;
    Some(format!(
        "'{}' không hợp lệ trong khối @{}: chỉ chấp nhận 'id' và 'lop'",
        key, block
    ))
}

fn localize_component_block(message: &str) -> Option<String> {
    let rest = message.strip_prefix("'@")?;
    let (block, rest) = rest.split_once("' is a semantic block of Element and does not apply to component calls: component parameters are free-named, write them directly in the '(...)' of ")?;
    Some(format!(
        "'@{}' là khối ngữ nghĩa của Element, không áp dụng cho lời gọi component: param của component đặt tên tự do, viết trực tiếp trong '(...)' của {}",
        block, rest
    ))
}

const CODEGEN_WARNING_FRAGMENTS_EN: &[(&str, &str)] = &[
    ("(nhóm: màu sắc)",
     "(group: color)"),
    ("(nhóm: kích thước)",
     "(group: size)"),
    ("(nhóm: khoảng cách/viền/hiệu ứng)",
     "(group: spacing/border/effects)"),
    ("(nhóm: kiểu chữ)",
     "(group: typography)"),
    ("(nhóm: bố cục flex/grid)",
     "(group: flex/grid layout)"),
    ("(nhóm: vị trí/biến đổi)",
     "(group: position/transform)"),
    ("(nhóm: chỉ dùng trong khối responsive)",
     "(group: only usable inside a responsive block)"),
    ("(nhóm: media/đối tượng)",
     "(group: media/object)"),
    ("(nhóm: form/input)",
     "(group: form/input)"),
    ("(nhóm: liên kết)",
     "(group: link)"),
    ("(nhóm: nội dung)",
     "(group: content)"),
    ("(nhóm: hiệu ứng chuyển động)",
     "(group: animation)"),
    (" phần tử top-level cho mỗi item (không bọc trong 1 container/box/stack). Vẫn hoạt động đúng, nhưng nên bọc trong 1 container để dễ áp style/layout và debug hơn.",
     " top-level elements per item (not wrapped in a container/box/stack). It still works correctly, but wrapping them in a container makes styling, layout and debugging easier."),
    ("): thân vòng lặp render ra ",
     "): the loop body renders "),
    (") được gọi TRỰC TIẾP làm phần tử top-level của thân vòng lặp, không có \"neu\"/element nào bao ngoài. Vẫn hoạt động đúng sau bản vá hiện tại, nhưng đây là dạng cấu trúc dễ vỡ nhất trong toàn bộ runtime - xem docs/LIMITATIONS.md. Gợi ý: bọc component trong 1 container (ví dụ `khoi(...) {",
     ") is called DIRECTLY as a top-level element of the loop body, with no \"neu\"/element wrapping it. It still works correctly after the current fix, but this is the most fragile structure in the whole runtime - see docs/LIMITATIONS.md. Hint: wrap the component in a container (for example `box(...) {"),
    (") }`) để tránh hoàn toàn dạng cấu trúc này.",
     ") }`) to avoid this structure entirely."),
    ("' là cú pháp hợp lệ của ViBao (",
     "' is valid ViBao syntax ("),
    (") nhưng KHÔNG áp dụng cho thẻ '",
     ") but does NOT apply to tag '"),
    ("' — giá trị vẫn được ghi thẳng thành HTML attribute '",
     "' — the value is still written directly as the HTML attribute '"),
    ("=\"...\"' (không tự động chuyển kebab-case), nên nếu đây không phải gõ sai, có thể bỏ qua cảnh báo này.",
     "=\"...\"' (not automatically converted to kebab-case), so if this is not a typo you can ignore this warning."),
    ("' là cú pháp hợp lệ nhưng KHÔNG có tác dụng trên thẻ layout '",
     "' is valid syntax but has NO effect on layout tag '"),
    ("không có thẻ layout nào hỗ trợ prop này",
     "no layout tag supports this prop"),
    ("chỉ dùng được trên: ",
     "only usable on: "),
    (". Xem docs/LIMITATIONS.md để biết giới hạn của từng loại layout.",
     ". See docs/LIMITATIONS.md for the limits of each layout type."),
    ("' với giá trị biểu thức động chưa được hỗ trợ trên layout tag (cần map giá trị hoặc ghép nhiều CSS property cùng lúc, không thể bind qua 1 data-vb-style- đơn). Style sẽ KHÔNG được áp dụng. Dùng giá trị tĩnh, hoặc bọc trong 'neu/khong_thi' ở cấp element để chọn hẳn 1 nhánh lúc build.",
     "' with a dynamic expression value is not supported on layout tags (the value needs mapping or several CSS properties at once, which a single data-vb-style- binding cannot do). The style will NOT be applied. Use a static value, or wrap it in 'neu/khong_thi' at element level to pick one branch at build time."),
    ("' với giá trị biểu thức động (biến/tính toán) chưa được hỗ trợ — cơ chế binding hiện tại không thể chuyển true/false thành \"bold\"/\"normal\" cho CSS font-weight. Style sẽ KHÔNG được áp dụng lúc runtime. Dùng giá trị tĩnh true/false, hoặc bọc trong 'neu/khong_thi' ở cấp element để chọn hẳn 1 nhánh có 'dam: true' / không có 'dam' lúc build.",
     "' with a dynamic expression value (variable/computation) is not supported — the current binding mechanism cannot turn true/false into \"bold\"/\"normal\" for CSS font-weight. The style will NOT be applied at runtime. Use a static true/false value, or wrap it in 'neu/khong_thi' at element level to pick one branch with 'dam: true' / without 'dam' at build time."),
    ("' với giá trị biểu thức động chưa được hỗ trợ — tương tự giới hạn của 'dam' (xem cảnh báo 'dam' để biết chi tiết + cách khắc phục).",
     "' with a dynamic expression value is not supported — same limitation as 'dam' (see the 'dam' warning for details and the workaround)."),
    ("' với giá trị biểu thức động chưa được hỗ trợ — cùng giới hạn với prop 'dam' (xem cảnh báo 'dam' để biết chi tiết + cách khắc phục).",
     "' with a dynamic expression value is not supported — same limitation as prop 'dam' (see the 'dam' warning for details and the workaround)."),
    ("' với giá trị biểu thức động chưa được hỗ trợ — cơ chế binding hiện tại không thể chuyển tên căn chỉnh tiếng Việt (vd \"giua\") thành CSS keyword tương ứng (vd \"center\") lúc runtime. Style sẽ KHÔNG được áp dụng. Dùng giá trị tĩnh, hoặc bọc trong 'neu/khong_thi' ở cấp element.",
     "' with a dynamic expression value is not supported — the current binding mechanism cannot turn a Vietnamese alignment name (e.g. \"giua\") into the matching CSS keyword (e.g. \"center\") at runtime. The style will NOT be applied. Use a static value, or wrap it in 'neu/khong_thi' at element level."),
    ("' với giá trị biểu thức động chưa được hỗ trợ — cùng giới hạn với prop 'can' (cần map tên căn chỉnh tiếng Việt sang CSS keyword lúc build, không thực hiện được lúc runtime). Style sẽ KHÔNG được áp dụng.",
     "' with a dynamic expression value is not supported — same limitation as prop 'can' (the Vietnamese alignment name must be mapped to a CSS keyword at build time, which cannot be done at runtime). The style will NOT be applied."),
    ("' với giá trị biểu thức động chưa được hỗ trợ — thuộc tính 'loading=\"lazy\"' cần biết trước lúc build (trình duyệt chỉ đọc 1 lần khi element được tạo, không phản ứng lại nếu attribute đổi sau đó). Dùng giá trị tĩnh (true/false), hoặc bọc trong 'neu/khong_thi' ở cấp element.",
     "' with a dynamic expression value is not supported — the 'loading=\"lazy\"' attribute must be known at build time (the browser reads it once when the element is created and does not react if the attribute changes later). Use a static true/false value, or wrap it in 'neu/khong_thi' at element level."),
    ("' với giá trị biểu thức động chưa được hỗ trợ — HTML boolean attribute (required) coi bất kỳ giá trị chuỗi nào (kể cả \"false\") là bật, nên cơ chế binding hiện tại (set_attribute thẳng) sẽ set sai khi giá trị là false. Dùng giá trị tĩnh, hoặc bọc trong 'neu/khong_thi' ở cấp element.",
     "' with a dynamic expression value is not supported — an HTML boolean attribute (required) treats any string value (even \"false\") as on, so the current binding mechanism (a direct set_attribute) would set it wrongly when the value is false. Use a static value, or wrap it in 'neu/khong_thi' at element level."),
    ("' với giá trị biểu thức động chưa được hỗ trợ — cùng giới hạn với prop 'bat_buoc' (xem cảnh báo 'bat_buoc' để biết chi tiết + cách khắc phục).",
     "' with a dynamic expression value is not supported — same limitation as prop 'bat_buoc' (see the 'bat_buoc' warning for details and the workaround)."),
    ("lop: tên class '",
     "lop: class name '"),
    ("' không được chứa ',' hoặc ':' (dùng làm delimiter nội bộ)",
     "' must not contain ',' or ':' (used as internal delimiters)"),
    ("lop: tên class không được để trống",
     "lop: class name must not be empty"),
    ("lop: cần một object, ví dụ lop: { active: $dang_chon }",
     "lop: requires an object, for example lop: { active: $dang_chon }"),
];

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
    for (from, to) in CODEGEN_WARNING_FRAGMENTS_EN {
        s = s.replace(from, to);
    }
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

    fn render_message(locale: Locale, message: &str) -> String {
        render(locale, "error", "E2001", "parser", Path::new("app.vbao"), "x", 1, 1, message, None)
    }

    #[test]
    fn block_messages_stay_english_in_english_locale() {
        let message = "Tag 'box' requires block syntax for property 'padding': write box { @spacing { padding: ... } } instead of using '(...)'";
        assert!(render_message(Locale::English, message).contains(message));
    }

    #[test]
    fn requires_block_message_is_localized_to_vietnamese() {
        let message = "Tag 'khoi' requires block syntax for property 'dem': write khoi { @khoang_cach { dem: ... } } instead of using '(...)'";
        let rendered = render_message(Locale::Vietnamese, message);
        assert!(rendered.contains("Tag 'khoi' yêu cầu dùng khối cho prop 'dem': viết khoi { @khoang_cach { dem: ... } } thay vì đặt trong '(...)'"), "{}", rendered);

        let message = "Tag 'khoi' requires block syntax for property 'abc': declare it inside an @block { ... } in the tag body instead of using '(...)'";
        let rendered = render_message(Locale::Vietnamese, message);
        assert!(rendered.contains("khai báo trong một khối @ten_khoi { ... }"), "{}", rendered);
    }

    #[test]
    fn misplaced_prop_message_is_localized_to_vietnamese() {
        let message = "Property 'co' belongs to block @chu, not @mau: move it to @chu { co: ... }";
        let rendered = render_message(Locale::Vietnamese, message);
        assert!(rendered.contains("Prop 'co' thuộc khối @chu, không phải @mau: chuyển vào @chu { co: ... }"), "{}", rendered);

        let message = "Property 'an' cannot be used inside block @mau";
        let rendered = render_message(Locale::Vietnamese, message);
        assert!(rendered.contains("Prop 'an' không dùng được trong khối @mau"), "{}", rendered);

        let message = "Property 'hieu_ung_cuon' cannot be used inside block @mau: animation properties are written in '(...)' of the tag";
        let rendered = render_message(Locale::Vietnamese, message);
        assert!(rendered.contains("prop animation viết trong '(...)' của tag"), "{}", rendered);
    }

    #[test]
    fn duplicate_identity_and_component_block_messages_are_localized_to_vietnamese() {
        let message = "Block '@mau' is already declared on this tag and cannot be repeated: merge it into a single '@mau' block";
        let rendered = render_message(Locale::Vietnamese, message);
        assert!(rendered.contains("Khối '@mau' đã được khai báo trên tag này"), "{}", rendered);

        let message = "'x' is not valid in block @dinh_danh: only 'id' and 'lop' are accepted";
        let rendered = render_message(Locale::Vietnamese, message);
        assert!(rendered.contains("'x' không hợp lệ trong khối @dinh_danh: chỉ chấp nhận 'id' và 'lop'"), "{}", rendered);

        let message = "'@mau' is a semantic block of Element and does not apply to component calls: component parameters are free-named, write them directly in the '(...)' of The_Card(...)";
        let rendered = render_message(Locale::Vietnamese, message);
        assert!(rendered.contains("param của component đặt tên tự do, viết trực tiếp trong '(...)' của The_Card(...)"), "{}", rendered);
    }

    #[test]
    fn response_handler_message_is_localized_to_vietnamese() {
        let message = "Unknown response handler 'done': expected 'thanh_cong' (or 'success') or 'that_bai' (or 'failure')";
        let rendered = render_message(Locale::Vietnamese, message);
        assert!(rendered.contains("Khối xử lý phản hồi 'done' không hợp lệ"), "{}", rendered);
        assert!(render_message(Locale::English, message).contains(message));
    }

    #[test]
    fn codegen_warnings_are_fully_translated_in_english_locale() {
        let samples = [
            "vong_lap (dòng 3:5): thân vòng lặp render ra 2 phần tử top-level cho mỗi item (không bọc trong 1 container/box/stack). Vẫn hoạt động đúng, nhưng nên bọc trong 1 container để dễ áp style/layout và debug hơn.",
            "prop 'abc' là cú pháp hợp lệ của ViBao (nhóm: kiểu chữ) nhưng KHÔNG áp dụng cho thẻ 'khoi' — giá trị vẫn được ghi thẳng thành HTML attribute 'abc=\"...\"' (không tự động chuyển kebab-case), nên nếu đây không phải gõ sai, có thể bỏ qua cảnh báo này.",
            "prop 'huong' là cú pháp hợp lệ nhưng KHÔNG có tác dụng trên thẻ layout 'khoi' — chỉ dùng được trên: flex, cuon. Xem docs/LIMITATIONS.md để biết giới hạn của từng loại layout.",
            "prop 'nghieng' với giá trị biểu thức động chưa được hỗ trợ — tương tự giới hạn của 'dam' (xem cảnh báo 'dam' để biết chi tiết + cách khắc phục).",
            "tag 'text': lop: tên class 'a,b' không được chứa ',' hoặc ':' (dùng làm delimiter nội bộ)",
            "tag 'text': lop: cần một object, ví dụ lop: { active: $dang_chon }",
        ];
        for sample in samples {
            let rendered = warning(Locale::English, "W0001", sample);
            assert!(
                !rendered
                    .chars()
                    .flat_map(|c| c.to_lowercase())
                    .any(|c| "ăâđêôơưáàảãạéèẻẽẹíìỉĩịóòỏõọúùủũụýỳỷỹỵ".contains(c)),
                "still Vietnamese: {}",
                rendered
            );
        }
        let rendered = warning(Locale::English, "W0001", samples[2]);
        assert!(rendered.contains("has NO effect on layout tag 'khoi'"), "{}", rendered);
        assert!(rendered.contains("only usable on: flex, cuon"), "{}", rendered);
        let rendered = warning(Locale::English, "W0001", samples[1]);
        assert!(rendered.contains("(group: typography)"), "{}", rendered);
    }

    #[test]
    fn codegen_warnings_stay_vietnamese_in_vietnamese_locale() {
        let sample = "prop 'huong' là cú pháp hợp lệ nhưng KHÔNG có tác dụng trên thẻ layout 'khoi' — chỉ dùng được trên: flex, cuon.";
        assert!(warning(Locale::Vietnamese, "W0001", sample).contains(sample));
    }

    #[test]
    fn thong_bao_theme_duoc_dich_sang_tieng_viet() {
        let cac_cap = [
            (
                "Theme \"toi\" is missing variable \"$chu\" (declared in theme \"sang\"): every theme must declare the same variables",
                "Theme \"toi\" thiếu biến \"$chu\" (đã khai báo ở theme \"sang\"): mọi theme phải khai báo cùng một tập biến",
            ),
            (
                "Theme variable \"$nen\" can only be used as the whole value of a style property (color, size, spacing, text, layout or position)",
                "Biến theme \"$nen\" chỉ dùng được làm toàn bộ giá trị của một prop kiểu dáng",
            ),
            (
                "Theme \"sang\": the value of \"$nen\" must not contain any of { } ; < >",
                "Theme \"sang\": giá trị của \"$nen\" không được chứa ký tự { } ; < >",
            ),
            (
                "The loop variable \"$nen\" has the same name as a theme variable: rename one of them",
                "Biến vòng lặp \"$nen\" trùng tên với một biến theme",
            ),
            (
                "Theme variable \"$nen\" is a constant and cannot be assigned",
                "Biến theme \"$nen\" là hằng số, không thể gán giá trị",
            ),
            (
                "Property \"phong_chuu\" is not valid in the global style block (giao_dien). Accepted: phong_chu, co, mau, mau_nen, hang, tai_phong, cuon_muot, cuon_lui.",
                "Prop \"phong_chuu\" không hợp lệ trong khối giao_dien",
            ),
        ];
        for (tieng_anh, mong_doi) in cac_cap {
            let ket_qua = render_message(Locale::Vietnamese, tieng_anh);
            assert!(ket_qua.contains(mong_doi), "{}\n-> {}", tieng_anh, ket_qua);
            assert!(render_message(Locale::English, tieng_anh).contains(tieng_anh));
        }
    }

    #[test]
    fn mau_cu_the_khong_bi_mau_chung_hon_nuot_mat() {
        let ket_qua = render_message(
            Locale::Vietnamese,
            "Theme \"sang\": variable \"$nen\" is declared more than once",
        );
        assert!(ket_qua.contains("Theme \"sang\": biến \"$nen\" được khai báo nhiều lần"), "{}", ket_qua);
        assert!(!ket_qua.contains("variable"), "{}", ket_qua);
    }

    #[test]
    fn mau_dich_khong_khop_thi_tra_ve_none() {
        assert_eq!(fill_pattern("Khác hẳn", "Theme \"{}\" declares no variables", "x {}"), None);
        assert_eq!(
            fill_pattern("Theme \"a\" declares no variables thừa", "Theme \"{}\" declares no variables", "x {}"),
            None
        );
        assert_eq!(
            fill_pattern("Theme \"a\" declares no variables", "Theme \"{}\" declares no variables", "Theme \"{}\" trống").as_deref(),
            Some("Theme \"a\" trống")
        );
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
