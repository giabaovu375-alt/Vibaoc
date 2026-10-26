use std::collections::{HashMap, HashSet};

use vibao_ast::{
    Action, App, Child, ColorValue, ComponentCall, Element, Expr, LiteralValue, LoopKind, Pos,
    PropBlock, TemplatePart, Theme,
};

use crate::validator::ValidationError;

const FORBIDDEN_VALUE_CHARS: [char; 5] = ['{', '}', ';', '<', '>'];

fn head(name: &str) -> &str {
    name.split('.').next().unwrap_or(name)
}

fn valid_theme_name(name: &str) -> bool {
    !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-')
}

pub fn check(app: &App, errors: &mut Vec<ValidationError>) {
    check_declarations(&app.themes, errors);

    let variables: HashSet<String> = app
        .themes
        .iter()
        .flat_map(|theme| theme.variables.iter().map(|v| v.name.clone()))
        .collect();
    let theme_names: Vec<String> = app.themes.iter().map(|theme| theme.name.clone()).collect();

    let mut checker = Checker { variables, theme_names, errors };
    checker.app(app);
}

fn error(errors: &mut Vec<ValidationError>, pos: &Pos, message: String) {
    errors.push(ValidationError { message, line: pos.line, column: pos.column });
}

fn check_declarations(themes: &[Theme], errors: &mut Vec<ValidationError>) {
    let mut seen_names: HashSet<&str> = HashSet::new();
    let mut owner_of_variable: HashMap<&str, &str> = HashMap::new();

    for theme in themes {
        if !valid_theme_name(&theme.name) {
            error(
                errors,
                &theme.pos,
                format!("Theme name \"{}\" may only contain letters, digits, '_' and '-'", theme.name),
            );
        }
        if !seen_names.insert(theme.name.as_str()) {
            error(errors, &theme.pos, format!("Theme \"{}\" is declared more than once", theme.name));
        }
        if theme.variables.is_empty() {
            error(errors, &theme.pos, format!("Theme \"{}\" declares no variables", theme.name));
        }

        let mut declared_here: HashSet<&str> = HashSet::new();
        for variable in &theme.variables {
            if !declared_here.insert(variable.name.as_str()) {
                error(
                    errors,
                    &variable.pos,
                    format!(
                        "Theme \"{}\": variable \"${}\" is declared more than once",
                        theme.name, variable.name
                    ),
                );
            }
            owner_of_variable.entry(variable.name.as_str()).or_insert(theme.name.as_str());
            check_value(theme, &variable.name, &variable.value, &variable.pos, errors);
        }
    }

    for theme in themes {
        let declared: HashSet<&str> = theme.variables.iter().map(|v| v.name.as_str()).collect();
        let mut missing: Vec<(&str, &str)> = owner_of_variable
            .iter()
            .filter(|(name, _)| !declared.contains(**name))
            .map(|(name, owner)| (*name, *owner))
            .collect();
        missing.sort();
        for (name, owner) in missing {
            error(
                errors,
                &theme.pos,
                format!(
                    "Theme \"{}\" is missing variable \"${}\" (declared in theme \"{}\"): every theme must declare the same variables",
                    theme.name, name, owner
                ),
            );
        }
    }
}

fn check_value(theme: &Theme, name: &str, value: &Expr, pos: &Pos, errors: &mut Vec<ValidationError>) {
    match value {
        Expr::Literal(LiteralValue::Str(text), _) => {
            if text.contains(&FORBIDDEN_VALUE_CHARS[..]) {
                error(
                    errors,
                    pos,
                    format!(
                        "Theme \"{}\": the value of \"${}\" must not contain any of {{ }} ; < >",
                        theme.name, name
                    ),
                );
            }
        }
        Expr::Literal(LiteralValue::Num(_, _), _) | Expr::Literal(LiteralValue::Color(_), _) => {}
        _ => error(
            errors,
            pos,
            format!(
                "Theme \"{}\": the value of \"${}\" must be a constant (color, number or text)",
                theme.name, name
            ),
        ),
    }
}

struct Checker<'a> {
    variables: HashSet<String>,
    theme_names: Vec<String>,
    errors: &'a mut Vec<ValidationError>,
}

impl<'a> Checker<'a> {
    fn is_theme_variable(&self, name: &str) -> bool {
        self.variables.contains(name)
    }

    fn misuse(&mut self, name: &str, pos: &Pos) {
        error(
            self.errors,
            pos,
            format!(
                "Theme variable \"${}\" can only be used as the whole value of a style property (color, size, spacing, text, layout or position)",
                head(name)
            ),
        );
    }

    fn clash(&mut self, kind: &str, name: &str, pos: &Pos) {
        if self.is_theme_variable(name) {
            error(
                self.errors,
                pos,
                format!("The {} \"${}\" has the same name as a theme variable: rename one of them", kind, name),
            );
        }
    }

    fn app(&mut self, app: &App) {
        for variable in &app.variables {
            self.clash("variable", &variable.name, &variable.pos);
            self.expr(&variable.value);
        }
        for (_, value) in &app.global_style {
            self.style_value(value);
        }
        for component in &app.components {
            for param in &component.params {
                self.clash("component parameter", &param.name, &param.pos);
                if let Some(default) = &param.default_value {
                    self.expr(default);
                }
            }
            self.children(&component.children);
        }
        for page in &app.pages {
            if let Some(ColorValue::Func { args, .. }) = &page.mau_nen {
                for arg in args {
                    self.expr(arg);
                }
            }
            for state in &page.states {
                self.clash("state", &state.name, &state.pos);
                self.expr(&state.value);
            }
            for event in &page.events {
                self.actions(&event.body);
            }
            self.children(&page.children);
        }
    }

    fn children(&mut self, children: &[Child]) {
        for child in children {
            match child {
                Child::Element(element) => self.element(element),
                Child::ComponentCall(call) => self.component_call(call),
                Child::If(node) => {
                    self.expr(&node.condition);
                    self.children(&node.consequent);
                    if let Some(alternate) = &node.alternate {
                        self.children(alternate);
                    }
                }
                Child::Switch(node) => {
                    self.expr(&node.subject);
                    for case in &node.cases {
                        self.expr(&case.value);
                        self.children(&case.body);
                    }
                    if let Some(default_case) = &node.default_case {
                        self.children(default_case);
                    }
                }
                Child::Loop(node) => {
                    match &node.kind {
                        LoopKind::Each { iterable, item_var, index_var } => {
                            self.expr(iterable);
                            self.clash("loop variable", item_var, &node.pos);
                            if let Some(index_var) = index_var {
                                self.clash("loop variable", index_var, &node.pos);
                            }
                        }
                        LoopKind::Range { var_name, .. } => self.clash("loop variable", var_name, &node.pos),
                    }
                    self.children(&node.body);
                }
                Child::StateDecl(state) => {
                    self.clash("state", &state.name, &state.pos);
                    self.expr(&state.value);
                }
                Child::VarDecl(variable) => {
                    self.clash("variable", &variable.name, &variable.pos);
                    self.expr(&variable.value);
                }
                Child::PageEvent(event) => self.actions(&event.body),
            }
        }
    }

    fn component_call(&mut self, call: &ComponentCall) {
        for (_, value) in &call.props {
            self.expr(value);
        }
        self.identity(&call.identity);
        self.children(&call.children);
    }

    fn identity(&mut self, identity: &vibao_ast::IdentityProps) {
        if let Some(id) = &identity.id {
            self.expr(id);
        }
        if let Some(class_binding) = &identity.class_binding {
            self.expr(class_binding);
        }
    }

    fn element(&mut self, element: &Element) {
        if let Some(content) = &element.content {
            self.expr(content);
        }
        self.identity(&element.identity);
        for block in &element.blocks {
            let (props, is_style) = match block {
                PropBlock::Mau(p)
                | PropBlock::KichThuoc(p)
                | PropBlock::KhoangCach(p)
                | PropBlock::Chu(p)
                | PropBlock::FlexBoCuc(p)
                | PropBlock::GridBoCuc(p)
                | PropBlock::ViTri(p) => (p, true),
                PropBlock::Media(p) | PropBlock::NhapLieu(p) | PropBlock::LienKet(p) => (p, false),
            };
            for (_, value) in props {
                if is_style {
                    self.style_value(value);
                } else {
                    self.expr(value);
                }
            }
        }
        for (_, value) in &element.unknown_props {
            self.expr(value);
        }
        for node in &element.responsive {
            for (_, value) in &node.overrides {
                self.expr(value);
            }
        }
        for event in &element.events {
            self.actions(&event.body);
        }
        self.children(&element.children);
    }

    fn actions(&mut self, actions: &[Action]) {
        for action in actions {
            match action {
                Action::FunctionCall { name, args, opts, assign_to, pos, .. } => {
                    if name == "dat_chu_de" {
                        self.theme_action(args, pos);
                    }
                    for arg in args {
                        self.expr(arg);
                    }
                    for (_, value) in opts {
                        self.expr(value);
                    }
                    if let Some(target) = assign_to {
                        self.assignment(target, pos);
                    }
                }
                Action::Assign { target, value, pos } => {
                    self.assignment(target, pos);
                    self.expr(value);
                }
                Action::ApiCall { endpoint, data, headers, assign_to, on_success, on_failure, pos, .. } => {
                    self.expr(endpoint);
                    if let Some(data) = data {
                        self.expr(data);
                    }
                    if let Some(headers) = headers {
                        self.expr(headers);
                    }
                    if let Some(target) = assign_to {
                        self.assignment(target, pos);
                    }
                    if let Some(body) = on_success {
                        self.actions(body);
                    }
                    if let Some(body) = on_failure {
                        self.actions(body);
                    }
                }
                Action::IfAction { condition, consequent, alternate, .. } => {
                    self.expr(condition);
                    self.actions(consequent);
                    if let Some(alternate) = alternate {
                        self.actions(alternate);
                    }
                }
            }
        }
    }

    fn theme_action(&mut self, args: &[Expr], pos: &Pos) {
        if self.theme_names.is_empty() {
            error(self.errors, pos, "The theme action is used but no theme is declared (chu_de / theme)".to_string());
            return;
        }
        match args {
            [] => {}
            [Expr::Literal(LiteralValue::Str(name), _)] => {
                if !self.theme_names.contains(name) {
                    let declared = self.theme_names.join(", ");
                    error(
                        self.errors,
                        pos,
                        format!("The theme \"{}\" does not exist: declared themes are {}", name, declared),
                    );
                }
            }
            _ => error(
                self.errors,
                pos,
                "The theme action takes either no argument or one theme name written as a text literal".to_string(),
            ),
        }
    }

    fn assignment(&mut self, target: &str, pos: &Pos) {
        if self.is_theme_variable(head(target)) {
            error(
                self.errors,
                pos,
                format!("Theme variable \"${}\" is a constant and cannot be assigned", head(target)),
            );
        }
    }

    fn style_value(&mut self, value: &Expr) {
        match value {
            Expr::Variable(name, _) if self.is_theme_variable(name) => {}
            other => self.expr(other),
        }
    }

    fn expr(&mut self, value: &Expr) {
        match value {
            Expr::Literal(_, _) => {}
            Expr::Variable(name, pos) => {
                if self.is_theme_variable(head(name)) {
                    self.misuse(name, pos);
                }
            }
            Expr::MemberAccess { object, .. } => self.expr(object),
            Expr::Binary { left, right, .. } => {
                self.expr(left);
                self.expr(right);
            }
            Expr::Unary { operand, .. } => self.expr(operand),
            Expr::Call { args, .. } => {
                for arg in args {
                    self.expr(arg);
                }
            }
            Expr::ColorFunc { color, .. } => self.expr(color),
            Expr::Array(items, _) => {
                for item in items {
                    self.expr(item);
                }
            }
            Expr::Object(fields, _) => {
                for (_, field) in fields {
                    self.expr(field);
                }
            }
            Expr::TemplateString(parts, pos) => {
                for part in parts {
                    match part {
                        TemplatePart::Text(_) => {}
                        TemplatePart::Variable(name) => {
                            if self.is_theme_variable(head(name)) {
                                self.misuse(name, pos);
                            }
                        }
                        TemplatePart::Member(path) => {
                            if let Some(first) = path.first() {
                                if self.is_theme_variable(first) {
                                    self.misuse(first, pos);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kiem_tra(thanh_phan: &str) -> Vec<String> {
        let nguon = format!(
            "lang = \"vi\";\nung_dung(\"Thử\") {{\n{}\n}}\n",
            thanh_phan
        );
        let tokens = crate::lexer::tokenize(&nguon).expect("lexer phải đọc được nguồn");
        let program = crate::parser::parse(tokens).expect("parser phải đọc được nguồn");
        let mut loi = Vec::new();
        check(&program.app, &mut loi);
        loi.into_iter().map(|e| e.message).collect()
    }

    const HAI_THEME: &str = r##"
        chu_de sang { $nen = trang  $chu = xam_dam }
        chu_de toi { $nen = "#1a202c"  $chu = "#e2e8f0" }
    "##;

    fn voi_trang(noi_dung: &str) -> String {
        format!("{}\ntrang(\"/\") {{\n{}\n}}\n", HAI_THEME, noi_dung)
    }

    fn co_loi(loi: &[String], mau: &str) -> bool {
        loi.iter().any(|m| m.contains(mau))
    }

    #[test]
    fn hai_theme_hop_le_khong_co_loi() {
        let loi = kiem_tra(&voi_trang(r#"text("Xin chào")"#));
        assert!(loi.is_empty(), "không được có lỗi: {:?}", loi);
    }

    #[test]
    fn ten_mau_sau_dau_bang_va_dau_hai_cham_deu_thanh_mau() {
        let nguon = "lang = \"vi\";\nung_dung(\"Thử\") {\n chu_de sang { $a = trang  $b: xam_dam  $c = \"#fff\"  $d = 16 }\n trang(\"/\") { text(\"x\") }\n}\n";
        let tokens = crate::lexer::tokenize(nguon).unwrap();
        let app = crate::parser::parse(tokens).unwrap().app;
        let bien = &app.themes[0].variables;
        for (i, ten) in ["a", "b"].iter().enumerate() {
            assert_eq!(bien[i].name, *ten);
            assert!(
                matches!(&bien[i].value, Expr::Literal(LiteralValue::Color(_), _)),
                "biến ${} phải là màu: {:?}",
                ten,
                bien[i].value
            );
        }
        assert!(matches!(&bien[2].value, Expr::Literal(LiteralValue::Str(_), _)));
        assert!(matches!(&bien[3].value, Expr::Literal(LiteralValue::Num(_, _), _)));
    }

    #[test]
    fn dung_lam_gia_tri_cua_prop_kieu_dang_duoc_phep() {
        let loi = kiem_tra(&voi_trang(
            r#"khoi {
                @mau { mau_nen: $nen, mau: $chu }
                @khoang_cach { dem: 12 }
                text("Chữ", mau: $chu)
            }"#,
        ));
        assert!(loi.is_empty(), "{:?}", loi);
    }

    #[test]
    fn dung_trong_khoi_giao_dien_va_mau_nen_trang_duoc_phep() {
        let nguon = format!(
            "giao_dien {{ mau_nen: $nen, mau: $chu }}\n{}\ntrang(\"/\", mau_nen: $nen) {{ text(\"x\") }}",
            HAI_THEME
        );
        let loi = kiem_tra(&nguon);
        assert!(loi.is_empty(), "{:?}", loi);
    }

    #[test]
    fn dung_lam_noi_dung_la_loi() {
        let loi = kiem_tra(&voi_trang(r#"text($nen)"#));
        assert!(co_loi(&loi, "Theme variable \"$nen\" can only be used as the whole value of a style property"), "{:?}", loi);
    }

    #[test]
    fn dung_trong_bieu_thuc_cua_prop_la_loi() {
        let loi = kiem_tra(&voi_trang(r#"khoi { @khoang_cach { dem: $nen + 1 } }"#));
        assert!(co_loi(&loi, "\"$nen\" can only be used"), "{:?}", loi);
    }

    #[test]
    fn dung_trong_noi_suy_chuoi_la_loi() {
        let loi = kiem_tra(&voi_trang(r#"text("Màu là $nen")"#));
        assert!(co_loi(&loi, "\"$nen\" can only be used"), "{:?}", loi);
    }

    #[test]
    fn dung_lam_dieu_kien_la_loi() {
        let loi = kiem_tra(&voi_trang(r#"neu $nen { text("x") }"#));
        assert!(co_loi(&loi, "\"$nen\" can only be used"), "{:?}", loi);
    }

    #[test]
    fn dung_trong_hanh_dong_la_loi() {
        let loi = kiem_tra(&voi_trang(r#"button("Bấm") { khi_nhan { thong_bao($chu) } }"#));
        assert!(co_loi(&loi, "\"$chu\" can only be used"), "{:?}", loi);
    }

    #[test]
    fn dung_lam_tham_so_component_la_loi() {
        let nguon = format!(
            "@the TheThe(ten_mau: mau) {{ text(\"x\") }}\n{}",
            voi_trang(r#"TheThe(ten_mau: $nen)"#)
        );
        let loi = kiem_tra(&nguon);
        assert!(co_loi(&loi, "\"$nen\" can only be used"), "{:?}", loi);
    }

    #[test]
    fn dung_trong_prop_khong_phai_kieu_dang_la_loi() {
        let loi = kiem_tra(&voi_trang(r#"input { @nhap_lieu { gia_tri: $nen } }"#));
        assert!(co_loi(&loi, "\"$nen\" can only be used"), "{:?}", loi);
    }

    #[test]
    fn dung_trong_khoi_responsive_la_loi() {
        let loi = kiem_tra(&voi_trang(r#"khoi { @di_dong { dem: $nen } }"#));
        assert!(co_loi(&loi, "\"$nen\" can only be used"), "{:?}", loi);
    }

    #[test]
    fn truy_cap_thuoc_tinh_cua_bien_theme_la_loi() {
        let loi = kiem_tra(&voi_trang(r#"text($nen.con)"#));
        assert!(co_loi(&loi, "\"$nen\" can only be used"), "{:?}", loi);
    }

    #[test]
    fn gan_gia_tri_cho_bien_theme_la_loi() {
        let loi = kiem_tra(&voi_trang(r#"button("Bấm") { khi_nhan { $nen = "do" } }"#));
        assert!(co_loi(&loi, "Theme variable \"$nen\" is a constant and cannot be assigned"), "{:?}", loi);
    }

    #[test]
    fn trung_ten_voi_state_la_loi() {
        let loi = kiem_tra(&voi_trang(r#"state $nen = 1"#));
        assert!(co_loi(&loi, "The state \"$nen\" has the same name as a theme variable"), "{:?}", loi);
    }

    #[test]
    fn trung_ten_voi_bien_vong_lap_la_loi() {
        let loi = kiem_tra(&voi_trang(r#"state $ds = [1, 2]
            vong_lap $nen trong $ds { text("x") }"#));
        assert!(co_loi(&loi, "The loop variable \"$nen\" has the same name as a theme variable"), "{:?}", loi);
    }

    #[test]
    fn trung_ten_voi_tham_so_component_la_loi() {
        let nguon = format!("@the TheThe(chu: chuoi) {{ text($chu) }}\n{}", HAI_THEME);
        let loi = kiem_tra(&nguon);
        assert!(co_loi(&loi, "The component parameter \"$chu\" has the same name as a theme variable"), "{:?}", loi);
    }

    #[test]
    fn khong_co_theme_thi_khong_kiem_tra_cach_dung() {
        let loi = kiem_tra("trang(\"/\") { state $nen = 1\n text($nen) }");
        assert!(loi.is_empty(), "{:?}", loi);
    }

    const HAI_THEME_NGAN: &str = r##"chu_de sang { $nen = trang }  chu_de toi { $nen = "#000000" }"##;

    #[test]
    fn dat_chu_de_hop_le_voi_ten_co_that_hoac_khong_doi_so() {
        let nguon = format!("{}\n trang(\"/\") {{ button(\"x\") {{ khi_nhan {{ dat_chu_de(\"toi\") }} }} button(\"y\") {{ khi_nhan {{ dat_chu_de() }} }} }}", HAI_THEME_NGAN);
        let loi = kiem_tra(&nguon);
        assert!(loi.is_empty(), "{:?}", loi);
    }

    #[test]
    fn dat_chu_de_voi_ten_khong_ton_tai_la_loi() {
        let nguon = format!("{}\n trang(\"/\") {{ button(\"x\") {{ khi_nhan {{ dat_chu_de(\"xanh\") }} }} }}", HAI_THEME_NGAN);
        let loi = kiem_tra(&nguon);
        assert!(co_loi(&loi, "The theme \"xanh\" does not exist: declared themes are sang, toi"), "{:?}", loi);
    }

    #[test]
    fn dat_chu_de_khong_co_theme_nao_la_loi() {
        let loi = kiem_tra("trang(\"/\") { button(\"x\") { khi_nhan { dat_chu_de() } } }");
        assert!(co_loi(&loi, "no theme is declared"), "{:?}", loi);
    }

    #[test]
    fn dat_chu_de_doi_so_khong_phai_chuoi_la_loi() {
        let nguon = format!("{}\n trang(\"/\") {{ button(\"x\") {{ khi_nhan {{ dat_chu_de(1) }} }} }}", HAI_THEME_NGAN);
        let loi = kiem_tra(&nguon);
        assert!(co_loi(&loi, "takes either no argument or one theme name"), "{:?}", loi);
    }

    #[test]
    fn theme_thieu_bien_so_voi_theme_khac_la_loi() {
        let loi = kiem_tra(
            r##"chu_de sang { $nen = trang  $chu = xam_dam }
               chu_de toi { $nen = "#000000" }
               trang("/") { text("x") }"##,
        );
        assert_eq!(loi.len(), 1, "{:?}", loi);
        assert!(
            loi[0].contains("Theme \"toi\" is missing variable \"$chu\" (declared in theme \"sang\")"),
            "{}",
            loi[0]
        );
    }

    #[test]
    fn bien_khai_bao_hai_lan_trong_mot_theme_la_loi() {
        let loi = kiem_tra(r#"chu_de sang { $nen = trang  $nen = den }  trang("/") { text("x") }"#);
        assert!(co_loi(&loi, "Theme \"sang\": variable \"$nen\" is declared more than once"), "{:?}", loi);
    }

    #[test]
    fn theme_trung_ten_la_loi() {
        let loi = kiem_tra(r#"chu_de sang { $nen = trang }  chu_de sang { $nen = den }  trang("/") { text("x") }"#);
        assert!(co_loi(&loi, "Theme \"sang\" is declared more than once"), "{:?}", loi);
    }

    #[test]
    fn theme_khong_co_bien_la_loi() {
        let loi = kiem_tra(r#"chu_de sang { }  trang("/") { text("x") }"#);
        assert!(co_loi(&loi, "Theme \"sang\" declares no variables"), "{:?}", loi);
    }

    #[test]
    fn gia_tri_khong_phai_hang_so_la_loi() {
        let loi = kiem_tra(r#"chu_de sang { $nen = $khac }  trang("/") { text("x") }"#);
        assert!(co_loi(&loi, "the value of \"$nen\" must be a constant"), "{:?}", loi);
    }

    #[test]
    fn chuoi_chua_ky_tu_nguy_hiem_la_loi() {
        for gia_tri in ["a;b", "a}b", "a{b", "a<b", "a>b"] {
            let nguon = format!("chu_de sang {{ $nen = \"{}\" }}  trang(\"/\") {{ text(\"x\") }}", gia_tri);
            let loi = kiem_tra(&nguon);
            assert!(co_loi(&loi, "must not contain any of { } ; < >"), "{} -> {:?}", gia_tri, loi);
        }
    }

    #[test]
    fn ten_theme_khong_hop_le_la_loi() {
        let loi = kiem_tra(r#"chu_de "a b" { $nen = trang }  trang("/") { text("x") }"#);
        assert!(co_loi(&loi, "Theme name \"a b\" may only contain letters, digits, '_' and '-'"), "{:?}", loi);
    }

    #[test]
    fn ten_theme_co_dau_tieng_viet_hop_le() {
        let loi = kiem_tra(r#"chu_de "tối-đậm" { $nen = trang }  trang("/") { text("x") }"#);
        assert!(loi.is_empty(), "{:?}", loi);
    }
}
