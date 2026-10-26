use std::collections::{HashMap, HashSet};

use vibao_ast::{
    Action, App, CaseNode, Child, ComponentCall, ComponentDef, Element, EventNode, Expr, IfNode,
    LiteralValue, LoopKind, LoopNode, PropsMap, SwitchNode,
};

use crate::lexer::color_map;

#[derive(Debug, Clone)]
pub struct ValidationError {
    pub message: String,
    pub line: usize,
    pub column: usize,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "[semantic] {} ({}:{})",
            self.message, self.line, self.column
        )
    }
}

fn is_color_prop(surface_name: &str) -> bool {
    matches!(
        crate::locale::resolve_prop_key(surface_name),
        Some(
            vibao_ast::PropKey::Color
                | vibao_ast::PropKey::BorderColor
                | vibao_ast::PropKey::BackgroundColor
        )
    )
}

pub fn validate(app: &App) -> Result<(), Vec<ValidationError>> {
    let mut errors = Vec::new();

    crate::theme_check::check(app, &mut errors);
    check_global_style(app, &mut errors);

    for var in &app.variables {
        check_expr(&var.value, &mut errors);
    }
    for component in &app.components {
        for param in &component.params {
            if let Some(default) = &param.default_value {
                check_expr(default, &mut errors);
            }
        }
    }

    let component_names = check_duplicate_components(&app.components, &mut errors);

    for component in &app.components {
        check_children(&component.children, &component_names, &mut errors);
    }
    for page in &app.pages {
        check_children(&page.children, &component_names, &mut errors);
        for state in &page.states {
            check_expr(&state.value, &mut errors);
        }
        for event in &page.events {
            check_actions(&event.body, &mut errors);
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn check_global_style(app: &App, errors: &mut Vec<ValidationError>) {
    for (key, _) in &app.global_style {
        if !crate::codegen::global_style::is_known_key(key) {
            errors.push(ValidationError {
                message: format!(
                    "Property \"{}\" is not valid in the global style block (giao_dien). Accepted: phong_chu, co, mau, mau_nen, hang, tai_phong, cuon_muot, cuon_lui.",
                    key
                ),
                line: app.pos.line,
                column: app.pos.column,
            });
        }
    }
}

fn check_duplicate_components(
    components: &[ComponentDef],
    errors: &mut Vec<ValidationError>,
) -> HashSet<String> {
    let mut seen: HashMap<&str, &ComponentDef> = HashMap::new();
    let mut names = HashSet::new();

    for def in components {
        names.insert(def.name.clone());
        if let Some(first) = seen.get(def.name.as_str()) {
            errors.push(ValidationError {
                message: format!("Component \"@the {}\" is defined more than once (first defined at line {}). Rename one definition or remove the duplicate.", def.name, first.pos.line),
                line: def.pos.line,
                column: def.pos.column,
            });
        } else {
            seen.insert(def.name.as_str(), def);
        }
    }

    names
}

fn check_children(
    children: &[Child],
    component_names: &HashSet<String>,
    errors: &mut Vec<ValidationError>,
) {
    for child in children {
        match child {
            Child::Element(el) => check_element(el, component_names, errors),
            Child::ComponentCall(call) => check_component_call(call, component_names, errors),
            Child::If(if_node) => check_if(if_node, component_names, errors),
            Child::Switch(switch_node) => check_switch(switch_node, component_names, errors),
            Child::Loop(loop_node) => check_loop(loop_node, component_names, errors),
            Child::StateDecl(decl) => check_expr(&decl.value, errors),
            Child::VarDecl(decl) => check_expr(&decl.value, errors),
            Child::PageEvent(event) => check_actions(&event.body, errors),
        }
    }
}

fn check_element(
    el: &Element,
    component_names: &HashSet<String>,
    errors: &mut Vec<ValidationError>,
) {
    let tag_name: &str = crate::locale::vi::tag_display_name_vi(el.tag);
    let flat_props = flatten_props_for_validation(el);
    check_color_props(tag_name, &flat_props, errors);
    check_props_map_exprs(&flat_props, errors);
    check_events(&el.events, errors);
    check_children(&el.children, component_names, errors);
}

fn flatten_props_for_validation(el: &Element) -> PropsMap {
    use vibao_ast::PropBlock::*;
    let mut flat: PropsMap = Vec::new();
    if let Some(content) = &el.content {
        flat.push(("noi_dung".to_string(), content.clone()));
    }
    if let Some(id) = &el.identity.id {
        flat.push(("id".to_string(), id.clone()));
    }
    if let Some(class) = &el.identity.class_binding {
        flat.push(("lop".to_string(), class.clone()));
    }
    for block in &el.blocks {
        let map: &PropsMap = match block {
            Mau(m) | KichThuoc(m) | KhoangCach(m) | Chu(m) | FlexBoCuc(m) | GridBoCuc(m)
            | ViTri(m) | Media(m) | NhapLieu(m) | LienKet(m) => m,
        };
        flat.extend(map.iter().cloned());
    }
    flat.extend(el.unknown_props.iter().cloned());
    flat
}

fn check_props_map_exprs(props: &PropsMap, errors: &mut Vec<ValidationError>) {
    for (_, expr) in props {
        check_expr(expr, errors);
    }
}

fn check_component_call(
    call: &ComponentCall,
    component_names: &HashSet<String>,
    errors: &mut Vec<ValidationError>,
) {
    if !component_names.contains(&call.name) {
        errors.push(ValidationError {
            message: format!("Component \"{}\" was used but no \"@the {}\" definition was found. Check the spelling or import it with `nhap {} tu \"...\"`.", call.name, call.name, call.name),
            line: call.pos.line,
            column: call.pos.column,
        });
    }
    check_color_props(&call.name, &call.props, errors);
    check_props_map_exprs(&call.props, errors);
    check_children(&call.children, component_names, errors);
}

fn check_if(
    if_node: &IfNode,
    component_names: &HashSet<String>,
    errors: &mut Vec<ValidationError>,
) {
    check_expr(&if_node.condition, errors);
    check_children(&if_node.consequent, component_names, errors);
    if let Some(alt) = &if_node.alternate {
        check_children(alt, component_names, errors);
    }
}

fn check_switch(
    switch_node: &SwitchNode,
    component_names: &HashSet<String>,
    errors: &mut Vec<ValidationError>,
) {
    check_expr(&switch_node.subject, errors);
    for case in &switch_node.cases {
        check_case(case, component_names, errors);
    }
    if let Some(default_body) = &switch_node.default_case {
        check_children(default_body, component_names, errors);
    }
}

fn check_case(
    case: &CaseNode,
    component_names: &HashSet<String>,
    errors: &mut Vec<ValidationError>,
) {
    check_expr(&case.value, errors);
    check_children(&case.body, component_names, errors);
}

fn check_loop(
    loop_node: &LoopNode,
    component_names: &HashSet<String>,
    errors: &mut Vec<ValidationError>,
) {
    match &loop_node.kind {
        LoopKind::Each { iterable, .. } => {
            check_expr(iterable, errors);
            check_children(&loop_node.body, component_names, errors);
        }
        LoopKind::Range { .. } => {
            check_children(&loop_node.body, component_names, errors);
        }
    }
}

fn check_color_props(tag_or_name: &str, props: &PropsMap, errors: &mut Vec<ValidationError>) {
    for (key, expr) in props {
        if !is_color_prop(key) {
            continue;
        }
        if let Expr::Literal(LiteralValue::Str(name), pos) = expr {
            if !color_map().contains_key(name.as_str()) {
                errors.push(ValidationError {
                    message: format!(
                        "Value \"{}\" for color prop \"{}\" on \"{}\" is invalid. Use a hex color, a supported ViBao color name, or a variable ($...).",
                        name, key, tag_or_name,
                    ),
                    line: pos.line,
                    column: pos.column,
                });
            }
        }
    }
}

fn check_events(events: &[EventNode], errors: &mut Vec<ValidationError>) {
    for event in events {
        check_actions(&event.body, errors);
    }
}

fn check_actions(actions: &[Action], errors: &mut Vec<ValidationError>) {
    for action in actions {
        match action {
            Action::FunctionCall {
                name, args, pos, ..
            } => {
                check_action_name(name, *pos, errors);
                check_array_mutation_args(name, args, *pos, errors);
                for arg in args {
                    check_expr(arg, errors);
                }
            }
            Action::ApiCall {
                endpoint,
                data,
                headers,
                on_success,
                on_failure,
                ..
            } => {
                check_expr(endpoint, errors);
                if let Some(d) = data {
                    check_expr(d, errors);
                }
                if let Some(h) = headers {
                    check_expr(h, errors);
                }
                if let Some(body) = on_success {
                    check_actions(body, errors);
                }
                if let Some(body) = on_failure {
                    check_actions(body, errors);
                }
            }
            Action::IfAction {
                condition,
                consequent,
                alternate,
                ..
            } => {
                check_expr(condition, errors);
                check_actions(consequent, errors);
                if let Some(alt) = alternate {
                    check_actions(alt, errors);
                }
            }
            Action::Assign { value, .. } => {
                check_expr(value, errors);
            }
        }
    }
}

fn check_expr(expr: &Expr, errors: &mut Vec<ValidationError>) {
    match expr {
        Expr::Call { callee, args, pos } => {
            check_function_name(callee, *pos, errors);
            for arg in args {
                check_expr(arg, errors);
            }
        }
        Expr::MemberAccess { object, .. } => {
            check_expr(object, errors);
        }
        Expr::Binary { left, right, .. } => {
            check_expr(left, errors);
            check_expr(right, errors);
        }
        Expr::Unary { operand, .. } => {
            check_expr(operand, errors);
        }
        Expr::ColorFunc { color, .. } => {
            check_expr(color, errors);
        }
        Expr::Array(items, _) => {
            for item in items {
                check_expr(item, errors);
            }
        }
        Expr::Object(fields, _) => {
            for (_, value) in fields {
                check_expr(value, errors);
            }
        }
        Expr::TemplateString(_, _) => {}
        Expr::Literal(_, _) | Expr::Variable(_, _) => {}
    }
}

fn check_function_name(name: &str, pos: vibao_ast::Pos, errors: &mut Vec<ValidationError>) {
    if crate::locale::resolve_function_name(name).is_some() {
        return;
    }
    errors.push(ValidationError {
        message: format!("Unknown function \"{}\". Check the spelling or use a supported ViBao expression function.", name),
        line: pos.line,
        column: pos.column,
    });
}

fn check_action_name(name: &str, pos: vibao_ast::Pos, errors: &mut Vec<ValidationError>) {
    if crate::locale::resolve_action_name(name).is_some() {
        return;
    }
    if crate::locale::action_vi::KNOWN_BUT_UNSUPPORTED_ACTIONS_VI.contains(&name) {
        errors.push(ValidationError {
            message: format!("Action \"{}\" is not supported by ViBao yet. The syntax is recognized, but there is no runtime behavior for it in this release.", name),
            line: pos.line,
            column: pos.column,
        });
        return;
    }
    errors.push(ValidationError {
        message: format!(
            "Unknown action \"{}\". Check the spelling or use an action supported by ViBao.",
            name
        ),
        line: pos.line,
        column: pos.column,
    });
}

fn check_array_mutation_args(
    name: &str,
    args: &[Expr],
    pos: vibao_ast::Pos,
    errors: &mut Vec<ValidationError>,
) {
    const ARRAY_MUTATION_ACTIONS: [&str; 3] = ["them_vao_mang", "xoa_theo_id", "cap_nhat_theo_id"];
    if !ARRAY_MUTATION_ACTIONS.contains(&name) {
        return;
    }
    let first_is_variable = matches!(args.first(), Some(Expr::Variable(_, _)));
    if !first_is_variable {
        errors.push(ValidationError {
            message: format!("Action \"{}\" requires its first argument to be a state variable (for example $tasks). The runtime cannot safely apply the mutation to another value shape.", name),
            line: pos.line,
            column: pos.column,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vibao_ast::{Page, Pos, Theme, VarDecl};

    fn p() -> Pos {
        Pos { line: 1, column: 1 }
    }

    fn empty_app() -> App {
        App {
            name: "Test".to_string(),
            lang: "vi".to_string(),
            imports: vec![],
            variables: vec![],
            themes: vec![],
            global_style: vec![],
            components: vec![],
            pages: vec![],
            pos: p(),
        }
    }

    fn element_with_color_prop(tag: vibao_ast::Tag, key: &str, value: &str) -> Element {
        Element {
            tag,
            content: None,
            identity: Default::default(),
            blocks: vec![],
            unknown_props: vec![(
                key.to_string(),
                Expr::Literal(LiteralValue::Str(value.to_string()), p()),
            )],
            children: vec![],
            events: vec![],
            responsive: vec![],
            animation: Default::default(),
            pos: p(),
        }
    }

    fn page_with(children: Vec<Child>) -> Page {
        Page {
            route: "/".to_string(),
            name: None,
            description: None,
            mau_nen: None,
            states: vec![],
            children,
            events: vec![],
            pos: p(),
        }
    }

    #[test]
    fn test_english_color_prop_uses_same_validation() {
        let app = {
            let mut app = empty_app();
            app.pages
                .push(page_with(vec![Child::Element(element_with_color_prop(
                    vibao_ast::Tag::Text,
                    "background_color",
                    "not_a_color",
                ))]));
            app
        };
        let result = validate(&app);
        assert!(
            result.is_err(),
            "an English color prop must go through the same color validator"
        );
        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("background_color")),
            "errors: {:?}",
            errors
        );
    }

    #[test]
    fn test_valid_color_name_passes() {
        let mut app = empty_app();
        app.pages
            .push(page_with(vec![Child::Element(element_with_color_prop(
                vibao_ast::Tag::Text,
                "mau",
                "xanh",
            ))]));
        assert!(validate(&app).is_ok());
    }

    #[test]
    fn test_unknown_color_name_is_rejected() {
        let mut app = empty_app();
        app.pages
            .push(page_with(vec![Child::Element(element_with_color_prop(
                vibao_ast::Tag::Text,
                "mau",
                "xanh_duong",
            ))]));
        let result = validate(&app);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("xanh_duong"));
    }

    #[test]
    fn test_non_color_prop_not_checked_as_color() {
        let mut app = empty_app();
        app.pages
            .push(page_with(vec![Child::Element(element_with_color_prop(
                vibao_ast::Tag::Khoi,
                "huong",
                "hang",
            ))]));
        assert!(validate(&app).is_ok());
    }

    #[test]
    fn test_undefined_component_call_is_rejected() {
        let mut app = empty_app();
        app.pages
            .push(page_with(vec![Child::ComponentCall(ComponentCall {
                name: "nut_bam".to_string(),
                props: vec![],
                identity: Default::default(),
                children: vec![],
                pos: p(),
            })]));
        let result = validate(&app);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.message.contains("nut_bam")));
    }

    #[test]
    fn test_defined_component_call_passes() {
        let mut app = empty_app();
        app.components.push(ComponentDef {
            name: "nut_bam".to_string(),
            params: vec![],
            children: vec![],
            pos: p(),
        });
        app.pages
            .push(page_with(vec![Child::ComponentCall(ComponentCall {
                name: "nut_bam".to_string(),
                props: vec![],
                identity: Default::default(),
                children: vec![],
                pos: p(),
            })]));
        assert!(validate(&app).is_ok());
    }

    #[test]
    fn test_duplicate_component_definition_is_rejected() {
        let mut app = empty_app();
        app.components.push(ComponentDef {
            name: "nut_bam".to_string(),
            params: vec![],
            children: vec![],
            pos: p(),
        });
        app.components.push(ComponentDef {
            name: "nut_bam".to_string(),
            params: vec![],
            children: vec![],
            pos: Pos {
                line: 20,
                column: 1,
            },
        });
        let result = validate(&app);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors
            .iter()
            .any(|e| e.message.contains("defined more than once")));
    }

    #[test]
    fn test_color_error_found_deep_inside_if_switch_loop() {
        let bad_element = Child::Element(element_with_color_prop(
            vibao_ast::Tag::Text,
            "mau",
            "mau_khong_ton_tai",
        ));

        let mut app = empty_app();
        app.pages
            .push(page_with(vec![Child::Loop(Box::new(LoopNode {
                kind: LoopKind::Range {
                    from: 0,
                    to: 3,
                    var_name: "i".to_string(),
                },
                body: vec![Child::If(Box::new(IfNode {
                    condition: Expr::Literal(LiteralValue::Bool(true), p()),
                    consequent: vec![bad_element],
                    alternate: None,
                    pos: p(),
                }))],
                pos: p(),
            }))]));

        let result = validate(&app);
        assert!(
            result.is_err(),
            "a color error nested deep inside a loop/if must still be detected"
        );
    }

    #[test]
    fn test_multiple_errors_all_collected_not_just_first() {
        let mut app = empty_app();
        app.pages.push(page_with(vec![
            Child::Element(element_with_color_prop(
                vibao_ast::Tag::Text,
                "mau",
                "mau_sai_1",
            )),
            Child::ComponentCall(ComponentCall {
                name: "component_chua_dinh_nghia".to_string(),
                props: vec![],
                identity: Default::default(),
                children: vec![],
                pos: p(),
            }),
        ]));

        let result = validate(&app);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(
            errors.len(),
            2,
            "must collect both errors, not stop at the first one"
        );
    }

    fn function_call_action(name: &str) -> Action {
        Action::FunctionCall {
            name: name.to_string(),
            args: vec![],
            opts: vec![],
            assign_to: None,
            pos: p(),
        }
    }

    fn element_with_event(body: Vec<Action>) -> Element {
        Element {
            tag: vibao_ast::Tag::Button,
            content: None,
            identity: Default::default(),
            blocks: vec![],
            unknown_props: vec![],
            children: vec![],
            events: vec![EventNode {
                name: vibao_ast::EventName::OnClick,
                body,
                pos: p(),
            }],
            responsive: vec![],
            animation: Default::default(),
            pos: p(),
        }
    }

    #[test]
    fn test_valid_action_name_passes() {
        let mut app = empty_app();
        app.pages
            .push(page_with(vec![Child::Element(element_with_event(vec![
                function_call_action("thong_bao"),
            ]))]));
        assert!(validate(&app).is_ok());
    }

    fn array_mutation_action(name: &str, array_var: &str) -> Action {
        Action::FunctionCall {
            name: name.to_string(),
            args: vec![Expr::Variable(array_var.to_string(), p())],
            opts: vec![],
            assign_to: None,
            pos: p(),
        }
    }

    #[test]
    fn test_all_14_function_call_action_names_pass() {
        let simple_names = [
            "thong_bao",
            "canh_bao",
            "dieu_huong",
            "mo_tab_moi",
            "mo_modal",
            "dong_modal",
            "cuon_den",
            "cuon_len_dau",
            "luu_du_lieu",
            "tai_du_lieu",
            "sao_chep",
        ];
        for name in simple_names {
            let mut app = empty_app();
            app.pages
                .push(page_with(vec![Child::Element(element_with_event(vec![
                    function_call_action(name),
                ]))]));
            assert!(validate(&app).is_ok(), "'{}' must be valid", name);
        }

        let array_mutation_names = ["them_vao_mang", "xoa_theo_id", "cap_nhat_theo_id"];
        for name in array_mutation_names {
            let mut app = empty_app();
            app.pages
                .push(page_with(vec![Child::Element(element_with_event(vec![
                    array_mutation_action(name, "tasks"),
                ]))]));
            assert!(
                validate(&app).is_ok(),
                "'{}' (with a correctly-shaped first argument) must be valid",
                name
            );
        }
    }

    #[test]
    fn test_goi_api_action_passes_validation() {
        let mut app = empty_app();
        app.pages
            .push(page_with(vec![Child::Element(element_with_event(vec![
                Action::ApiCall {
                    method: "GET".to_string(),
                    endpoint: Expr::literal_str("/api/x", p()),
                    data: None,
                    headers: None,
                    background: false,
                    assign_to: None,
                    on_success: None,
                    on_failure: None,
                    pos: p(),
                },
            ]))]));
        assert!(
            validate(&app).is_ok(),
            "'goi_api' (Action::ApiCall) must be valid"
        );
    }

    #[test]
    fn test_array_mutation_action_with_wrong_first_arg_is_rejected() {
        let mut app = empty_app();
        app.pages
            .push(page_with(vec![Child::Element(element_with_event(vec![
                Action::FunctionCall {
                    name: "them_vao_mang".to_string(),
                    args: vec![Expr::literal_str("tasks", p())],
                    opts: vec![],
                    assign_to: None,
                    pos: p(),
                },
            ]))]));
        let result = validate(&app);
        assert!(
            result.is_err(),
            "a wrongly-shaped first argument (a string instead of a variable) must be rejected"
        );
        assert!(result.unwrap_err()[0].message.contains("them_vao_mang"));
    }

    #[test]
    fn test_array_mutation_action_with_no_args_is_rejected() {
        let mut app = empty_app();
        app.pages
            .push(page_with(vec![Child::Element(element_with_event(vec![
                function_call_action("xoa_theo_id"),
            ]))]));
        assert!(
            validate(&app).is_err(),
            "thiếu hoàn toàn đối số phải bị từ chối"
        );
    }

    #[test]
    fn test_non_array_mutation_action_not_checked_for_array_shape() {
        let mut app = empty_app();
        app.pages
            .push(page_with(vec![Child::Element(element_with_event(vec![
                function_call_action("thong_bao"),
            ]))]));
        assert!(validate(&app).is_ok());
    }

    #[test]
    fn test_typo_action_name_is_rejected_as_unknown() {
        let mut app = empty_app();
        app.pages
            .push(page_with(vec![Child::Element(element_with_event(vec![
                function_call_action("thongbao"),
            ]))]));
        let result = validate(&app);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("thongbao"));
        assert!(errors[0].message.contains("Unknown action"));
        assert!(!errors[0].message.contains("not supported by ViBao"));
    }

    #[test]
    fn test_dang_xuat_is_rejected_with_distinct_message() {
        let mut app = empty_app();
        app.pages
            .push(page_with(vec![Child::Element(element_with_event(vec![
                function_call_action("dang_xuat"),
            ]))]));
        let result = validate(&app);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("dang_xuat"));
        assert!(errors[0].message.contains("not supported by ViBao"));
        assert!(!errors[0].message.contains("Unknown action"));
    }

    #[test]
    fn test_dang_xuat_end_to_end_through_real_lexer_and_parser() {
        let tokens = crate::lexer::tokenize("dang_xuat()")
            .expect("tokenize must not error for 'dang_xuat()'");
        let mut parser = crate::parser::Parser::new(tokens);
        let action = parser
            .parse_action()
            .expect("parse_action must not error for 'dang_xuat()'");

        let mut errors = Vec::new();
        check_actions(&[action], &mut errors);
        assert_eq!(
            errors.len(),
            1,
            "there must be exactly 1 validator error for 'dang_xuat'"
        );
        assert!(
            errors[0].message.contains("not supported by ViBao"),
            "message: {}",
            errors[0].message
        );
    }

    #[test]
    fn test_action_name_nested_in_if_action_is_checked() {
        let mut app = empty_app();
        app.pages
            .push(page_with(vec![Child::Element(element_with_event(vec![
                Action::IfAction {
                    condition: Expr::Literal(LiteralValue::Bool(true), p()),
                    consequent: vec![function_call_action("thongbao_sai")],
                    alternate: None,
                    pos: p(),
                },
            ]))]));
        let result = validate(&app);
        assert!(
            result.is_err(),
            "an action nested inside IfAction must still be checked"
        );
    }

    #[test]
    fn test_action_name_nested_in_api_call_callback_is_checked() {
        let mut app = empty_app();
        app.pages
            .push(page_with(vec![Child::Element(element_with_event(vec![
                Action::ApiCall {
                    method: "GET".to_string(),
                    endpoint: Expr::literal_str("/api/x", p()),
                    data: None,
                    headers: None,
                    background: false,
                    assign_to: None,
                    on_success: Some(vec![function_call_action("thongbao_sai")]),
                    on_failure: None,
                    pos: p(),
                },
            ]))]));
        let result = validate(&app);
        assert!(
            result.is_err(),
            "an action nested inside ApiCall's on_success must still be checked"
        );
    }

    #[test]
    fn test_assign_action_not_checked_as_action_name() {
        let mut app = empty_app();
        app.pages
            .push(page_with(vec![Child::Element(element_with_event(vec![
                Action::Assign {
                    target: "bien_bat_ky".to_string(),
                    value: Expr::literal_num(1.0, p()),
                    pos: p(),
                },
            ]))]));
        assert!(validate(&app).is_ok());
    }

    #[test]
    fn test_page_level_event_action_is_checked() {
        let mut app = empty_app();
        let mut page = page_with(vec![]);
        page.events.push(vibao_ast::PageEvent {
            name: vibao_ast::PageEventName::OnTai,
            body: vec![function_call_action("thongbao_sai")],
            pos: p(),
        });
        app.pages.push(page);
        let result = validate(&app);
        assert!(
            result.is_err(),
            "an unknown action in page.events (khi_tai) must be detected"
        );
    }

    #[test]
    fn test_page_level_event_with_valid_action_passes() {
        let mut app = empty_app();
        let mut page = page_with(vec![]);
        page.events.push(vibao_ast::PageEvent {
            name: vibao_ast::PageEventName::OnTai,
            body: vec![function_call_action("thong_bao")],
            pos: p(),
        });
        app.pages.push(page);
        assert!(validate(&app).is_ok());
    }

    fn call_expr(callee: &str, args: Vec<Expr>) -> Expr {
        Expr::Call {
            callee: callee.to_string(),
            args,
            pos: p(),
        }
    }

    #[test]
    fn test_valid_function_call_in_var_decl_passes() {
        let mut app = empty_app();
        app.variables.push(vibao_ast::VarDecl {
            name: "x".to_string(),
            value: call_expr("gia_tien", vec![Expr::literal_num(1000.0, p())]),
            pos: p(),
        });
        assert!(validate(&app).is_ok());
    }

    #[test]
    fn test_all_6_function_names_pass_in_var_decl() {
        let names = [
            "gia_tien",
            "ngay",
            "rut_gon",
            "hoa_chu",
            "phan_tram",
            "lam_tron",
        ];
        for name in names {
            let mut app = empty_app();
            app.variables.push(vibao_ast::VarDecl {
                name: "x".to_string(),
                value: call_expr(name, vec![]),
                pos: p(),
            });
            assert!(validate(&app).is_ok(), "'{}' must be valid", name);
        }
    }

    #[test]
    fn test_unknown_function_name_in_var_decl_is_rejected() {
        let mut app = empty_app();
        app.variables.push(vibao_ast::VarDecl {
            name: "x".to_string(),
            value: call_expr("gia_tien_sai", vec![]),
            pos: p(),
        });
        let result = validate(&app);
        assert!(result.is_err());
        assert!(result.unwrap_err()[0].message.contains("gia_tien_sai"));
    }

    #[test]
    fn test_unknown_function_call_nested_in_binary_is_detected() {
        let mut app = empty_app();
        app.variables.push(vibao_ast::VarDecl {
            name: "x".to_string(),
            value: Expr::Binary {
                op: vibao_ast::BinaryOp::Add,
                left: Box::new(Expr::Variable("a".to_string(), p())),
                right: Box::new(call_expr("gia_tien_sai", vec![])),
                pos: p(),
            },
            pos: p(),
        });
        let result = validate(&app);
        assert!(
            result.is_err(),
            "Call sai lồng trong Binary phải bị phát hiện"
        );
    }

    #[test]
    fn test_unknown_function_call_nested_in_array_is_detected() {
        let mut app = empty_app();
        app.variables.push(vibao_ast::VarDecl {
            name: "x".to_string(),
            value: Expr::Array(vec![call_expr("gia_tien_sai", vec![])], p()),
            pos: p(),
        });
        assert!(
            validate(&app).is_err(),
            "Call sai lồng trong Array phải bị phát hiện"
        );
    }

    #[test]
    fn test_unknown_function_call_nested_in_object_is_detected() {
        let mut app = empty_app();
        app.variables.push(vibao_ast::VarDecl {
            name: "x".to_string(),
            value: Expr::Object(
                vec![("field".to_string(), call_expr("gia_tien_sai", vec![]))],
                p(),
            ),
            pos: p(),
        });
        assert!(
            validate(&app).is_err(),
            "Call sai lồng trong Object phải bị phát hiện"
        );
    }

    #[test]
    fn test_unknown_function_call_nested_in_call_args_is_detected() {
        let mut app = empty_app();
        app.variables.push(vibao_ast::VarDecl {
            name: "x".to_string(),
            value: call_expr("gia_tien", vec![call_expr("rut_gon_sai", vec![])]),
            pos: p(),
        });
        assert!(
            validate(&app).is_err(),
            "Call sai lồng trong args của Call khác phải bị phát hiện"
        );
    }

    #[test]
    fn test_function_call_in_page_state_is_checked() {
        let mut app = empty_app();
        let mut page = page_with(vec![]);
        page.states.push(vibao_ast::StateDecl {
            name: "x".to_string(),
            value: call_expr("gia_tien_sai", vec![]),
            pos: p(),
        });
        app.pages.push(page);
        assert!(
            validate(&app).is_err(),
            "Call sai trong page.states phải bị phát hiện"
        );
    }

    #[test]
    fn test_function_call_in_component_param_default_is_checked() {
        let mut app = empty_app();
        app.components.push(vibao_ast::ComponentDef {
            name: "TheBao".to_string(),
            params: vec![vibao_ast::ParamDef {
                name: "gia".to_string(),
                data_type: vibao_ast::DataType::So,
                default_value: Some(call_expr("gia_tien_sai", vec![])),
                pos: p(),
            }],
            children: vec![],
            pos: p(),
        });
        assert!(
            validate(&app).is_err(),
            "Call sai trong default_value của ParamDef phải bị phát hiện"
        );
    }

    #[test]
    fn test_function_call_in_props_map_is_checked() {
        let el = element_with_color_prop(vibao_ast::Tag::Text, "khong_lien_quan", "trang");
        let mut el = el;
        el.content = Some(call_expr("gia_tien_sai", vec![]));
        let mut app = empty_app();
        app.pages.push(page_with(vec![Child::Element(el)]));
        assert!(
            validate(&app).is_err(),
            "Call sai trong props Element phải bị phát hiện"
        );
    }

    #[test]
    fn test_function_call_in_action_args_is_checked() {
        let mut app = empty_app();
        app.pages
            .push(page_with(vec![Child::Element(element_with_event(vec![
                Action::FunctionCall {
                    name: "thong_bao".to_string(),
                    args: vec![call_expr("gia_tien_sai", vec![])],
                    opts: vec![],
                    assign_to: None,
                    pos: p(),
                },
            ]))]));
        assert!(
            validate(&app).is_err(),
            "Call sai trong args của Action phải bị phát hiện"
        );
    }

    #[test]
    fn test_function_call_in_if_condition_is_checked() {
        let mut app = empty_app();
        app.pages.push(page_with(vec![Child::If(Box::new(IfNode {
            condition: call_expr("gia_tien_sai", vec![]),
            consequent: vec![],
            alternate: None,
            pos: p(),
        }))]));
        assert!(
            validate(&app).is_err(),
            "Call sai trong IfNode.condition phải bị phát hiện"
        );
    }

    #[test]
    fn test_function_call_in_action_if_condition_is_checked() {
        let mut app = empty_app();
        app.pages
            .push(page_with(vec![Child::Element(element_with_event(vec![
                Action::IfAction {
                    condition: call_expr("gia_tien_sai", vec![]),
                    consequent: vec![],
                    alternate: None,
                    pos: p(),
                },
            ]))]));
        assert!(
            validate(&app).is_err(),
            "Call sai trong Action::IfAction.condition phải bị phát hiện"
        );
    }

    #[test]
    fn test_function_call_in_assign_value_is_checked() {
        let mut app = empty_app();
        app.pages
            .push(page_with(vec![Child::Element(element_with_event(vec![
                Action::Assign {
                    target: "bien".to_string(),
                    value: call_expr("gia_tien_sai", vec![]),
                    pos: p(),
                },
            ]))]));
        assert!(
            validate(&app).is_err(),
            "Call sai trong Action::Assign.value phải bị phát hiện"
        );
    }

    #[test]
    fn test_function_call_in_api_call_endpoint_and_data_is_checked() {
        let mut app = empty_app();
        app.pages
            .push(page_with(vec![Child::Element(element_with_event(vec![
                Action::ApiCall {
                    method: "GET".to_string(),
                    endpoint: call_expr("gia_tien_sai", vec![]),
                    data: None,
                    headers: None,
                    background: false,
                    assign_to: None,
                    on_success: None,
                    on_failure: None,
                    pos: p(),
                },
            ]))]));
        assert!(
            validate(&app).is_err(),
            "Call sai trong ApiCall.endpoint phải bị phát hiện"
        );
    }

    #[test]
    fn test_function_call_in_child_var_decl_is_checked() {
        let mut app = empty_app();
        app.pages
            .push(page_with(vec![Child::VarDecl(vibao_ast::VarDecl {
                name: "y".to_string(),
                value: call_expr("gia_tien_sai", vec![]),
                pos: p(),
            })]));
        assert!(
            validate(&app).is_err(),
            "Call sai trong Child::VarDecl phải bị phát hiện"
        );
    }

    #[test]
    fn test_function_call_in_child_state_decl_is_checked() {
        let mut app = empty_app();
        app.pages
            .push(page_with(vec![Child::StateDecl(vibao_ast::StateDecl {
                name: "y".to_string(),
                value: call_expr("gia_tien_sai", vec![]),
                pos: p(),
            })]));
        assert!(
            validate(&app).is_err(),
            "Call sai trong Child::StateDecl phải bị phát hiện"
        );
    }

    #[test]
    fn test_valid_function_call_in_child_var_decl_passes() {
        let mut app = empty_app();
        app.pages
            .push(page_with(vec![Child::VarDecl(vibao_ast::VarDecl {
                name: "y".to_string(),
                value: call_expr("gia_tien", vec![Expr::literal_num(1000.0, p())]),
                pos: p(),
            })]));
        assert!(validate(&app).is_ok());
    }

    #[test]
    fn test_app_with_no_theme_passes() {
        let app = empty_app();
        assert!(validate(&app).is_ok());
    }

    fn theme_voi_bien(ten: &str, bien: &[&str]) -> Theme {
        Theme {
            name: ten.to_string(),
            variables: bien
                .iter()
                .map(|b| VarDecl {
                    name: b.to_string(),
                    value: Expr::Literal(LiteralValue::Color("#FFFFFF".to_string()), p()),
                    pos: p(),
                })
                .collect(),
            pos: p(),
        }
    }

    #[test]
    fn test_theme_hop_le_duoc_chap_nhan() {
        let mut app = empty_app();
        app.themes.push(theme_voi_bien("sang", &["nen"]));
        app.themes.push(theme_voi_bien("toi", &["nen"]));
        assert!(validate(&app).is_ok());
    }

    #[test]
    fn test_global_style_accepts_known_keys() {
        let mut app = empty_app();
        app.global_style = vec![
            ("phong_chu".to_string(), Expr::literal_str("Inter", p())),
            ("cuon_muot".to_string(), Expr::literal_str("true", p())),
        ];
        assert!(validate(&app).is_ok());
    }

    #[test]
    fn test_global_style_rejects_unknown_key() {
        let mut app = empty_app();
        app.global_style = vec![("phong_chuu".to_string(), Expr::literal_str("Inter", p()))];
        let errors = validate(&app).unwrap_err();
        assert_eq!(errors.len(), 1);
        assert!(
            errors[0].message.contains("phong_chuu"),
            "{}",
            errors[0].message
        );
        assert!(
            errors[0].message.contains("global style"),
            "{}",
            errors[0].message
        );
    }

    #[test]
    fn test_nhieu_loi_theme_deu_duoc_bao() {
        let mut app = empty_app();
        app.themes.push(theme_voi_bien("sang", &["nen"]));
        app.themes.push(theme_voi_bien("toi", &["chu"]));
        let errors = validate(&app).unwrap_err();
        assert_eq!(
            errors.len(),
            2,
            "mỗi theme thiếu một biến của theme kia, phải báo đủ cả hai"
        );
    }

    #[test]
    fn test_loi_theme_duoc_gom_cung_loi_khac() {
        let mut app = empty_app();
        app.themes.push(theme_voi_bien("sang", &[]));
        app.pages
            .push(page_with(vec![Child::Element(element_with_event(vec![
                function_call_action("thongbao_sai"),
            ]))]));
        let errors = validate(&app).unwrap_err();
        assert_eq!(
            errors.len(),
            2,
            "phải gom cả lỗi theme lẫn lỗi action, không dừng ở lỗi đầu"
        );
    }
}
