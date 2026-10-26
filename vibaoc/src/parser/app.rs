use super::{ParseError, Parser};
use crate::lexer::TokenKind;
use vibao_ast::{
    App, Child, ColorValue, ComponentDef, DataType, Page, ParamDef, StateDecl, Theme, VarDecl,
};

pub(crate) struct ModuleBody {
    pub imports: Vec<vibao_ast::ImportDecl>,
    pub variables: Vec<VarDecl>,
    pub themes: Vec<Theme>,
    pub global_style: vibao_ast::PropsMap,
    pub components: Vec<ComponentDef>,
    pub pages: Vec<Page>,
}

impl Parser {
    fn is_at_symbol_the(&self) -> bool {
        if !self.check(&TokenKind::At) {
            return false;
        }
        matches!(&self.peek(1).kind, TokenKind::Identifier(s) if s == "the")
    }

    pub(crate) fn parse_app(&mut self) -> Result<App, ParseError> {
        let pos = self.current_pos();
        self.consume(&TokenKind::UngDung, "Expected the 'ung_dung' keyword")?;
        self.consume(
            &TokenKind::LParen,
            "Expected '(' after the 'ung_dung' keyword",
        )?;

        let name = match self.advance().kind {
            TokenKind::StringLit(s) => s,
            other => {
                return Err(self.error(format!(
                    "Expected an application name string, received {}",
                    other
                )))
            }
        };

        self.consume(
            &TokenKind::RParen,
            "Expected ')' after the application name",
        )?;
        self.consume(
            &TokenKind::LBrace,
            "Expected '{' to start the application body",
        )?;

        let body = self.parse_module_body(&TokenKind::RBrace)?;

        self.consume(
            &TokenKind::RBrace,
            "Expected '}' to close the application body",
        )?;

        Ok(App {
            name,
            lang: self.locale.code().to_string(),
            imports: body.imports,
            variables: body.variables,
            themes: body.themes,
            global_style: body.global_style,
            components: body.components,
            pages: body.pages,
            pos,
        })
    }

    pub(crate) fn parse_module_body(
        &mut self,
        stop_at: &TokenKind,
    ) -> Result<ModuleBody, ParseError> {
        let mut imports = Vec::new();
        let mut variables = Vec::new();
        let mut themes = Vec::new();
        let mut global_style: vibao_ast::PropsMap = Vec::new();
        let mut global_style_declared = false;
        let mut components = Vec::new();
        let mut pages = Vec::new();

        while !self.check(stop_at) && !self.is_at_end() {
            if self.check_ident("lang") {
                return Err(self.error("The 'lang' declaration must appear at the top of the file"));
            } else if self.check(&TokenKind::Nhap) {
                imports.push(self.parse_import()?);
            } else if (self.check_ident("giao_dien") || self.check_ident("global_style"))
                && self.check_at(1, &TokenKind::LBrace)
            {
                if global_style_declared {
                    return Err(self.error("The global style block is already declared: merge it into a single 'giao_dien' block"));
                }
                global_style_declared = true;
                self.advance();
                global_style = self.parse_named_props_block()?;
            } else if self.check(&TokenKind::Theme) {
                themes.push(self.parse_theme()?);
            } else if self.check(&TokenKind::Trang) {
                pages.push(self.parse_page()?);
            } else if self.check(&TokenKind::State) {
                variables.push(self.parse_var_decl()?);
            } else if self.is_at_symbol_the() {
                components.push(self.parse_component_def()?);
            } else if let TokenKind::Variable(_) = &self.current().kind {
                variables.push(self.parse_var_decl()?);
            } else {
                return Err(self.error(format!(
                    "Invalid declaration inside the application block: {}",
                    self.current().kind
                )));
            }
        }

        Ok(ModuleBody {
            imports,
            variables,
            themes,
            global_style,
            components,
            pages,
        })
    }

    fn parse_import(&mut self) -> Result<vibao_ast::ImportDecl, ParseError> {
        let pos = self.current_pos();
        self.advance();

        let mut names = Vec::new();
        if self.check(&TokenKind::LBrace) {
            self.advance();
            loop {
                let name = self.expect_identifier_like()?;
                names.push(name);
                if self.check(&TokenKind::Comma) {
                    self.advance();
                    continue;
                }
                break;
            }
            self.consume(
                &TokenKind::RBrace,
                "Expected '}' to close the import name list",
            )?;
        } else {
            let name = self.expect_identifier_like()?;
            names.push(name);
        }

        self.consume(
            &TokenKind::Tu,
            "Expected the 'tu' keyword after the import name list",
        )?;

        let path = match self.advance().kind {
            TokenKind::StringLit(s) => s,
            other => {
                return Err(self.error(format!(
                    "Expected an import path string after 'tu', received {}",
                    other
                )))
            }
        };

        Ok(vibao_ast::ImportDecl { names, path, pos })
    }

    pub(crate) fn parse_module_file(&mut self) -> Result<ModuleBody, ParseError> {
        self.parse_language_header()?;
        let body = self.parse_module_body(&TokenKind::Eof)?;
        self.expect(&TokenKind::Eof)?;
        Ok(body)
    }

    fn parse_theme(&mut self) -> Result<Theme, ParseError> {
        let pos = self.current_pos();
        self.advance();

        let name = match self.advance().kind {
            TokenKind::Identifier(s) | TokenKind::StringLit(s) => s,
            other => {
                return Err(self.error(format!("Expected a theme identifier, received {}", other)))
            }
        };

        self.consume(&TokenKind::LBrace, "Expected '{' after the theme name")?;
        let mut variables = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            variables.push(self.parse_theme_var()?);
        }
        self.consume(&TokenKind::RBrace, "Expected '}' to close the theme")?;

        Ok(Theme {
            name,
            variables,
            pos,
        })
    }

    fn parse_theme_var(&mut self) -> Result<VarDecl, ParseError> {
        let pos = self.current_pos();
        let name = match self.advance().kind {
            TokenKind::Variable(n) => n,
            other => {
                return Err(self.error(format!(
                    "Expected a theme variable name starting with $, received {}",
                    other
                )))
            }
        };

        if self.check(&TokenKind::Equals) || self.check(&TokenKind::Colon) {
            self.advance();
        } else {
            return Err(self.error("Expected '=' or ':' after the theme variable name"));
        }

        let value_pos = self.current_pos();
        let color_name = match &self.current().kind {
            TokenKind::Trang => Some("trang".to_string()),
            TokenKind::Identifier(n) | TokenKind::Component(n) => Some(n.clone()),
            _ => None,
        };
        let value = match color_name.and_then(|n| crate::lexer::resolve_color_name(&n)) {
            Some(hex) => {
                self.advance();
                vibao_ast::Expr::Literal(vibao_ast::LiteralValue::Color(hex), value_pos)
            }
            None => self.parse_value()?,
        };
        self.skip_comma();

        Ok(VarDecl { name, value, pos })
    }

    pub(crate) fn parse_var_decl(&mut self) -> Result<VarDecl, ParseError> {
        let pos = self.current_pos();
        if self.check(&TokenKind::State) {
            self.advance();
        }

        let name = match self.advance().kind {
            TokenKind::Variable(n) => n,
            other => {
                return Err(self.error(format!(
                    "Expected a variable name starting with $, received {}",
                    other
                )))
            }
        };

        self.consume(&TokenKind::Equals, "Expected '=' after the variable name")?;
        let value = self.parse_value()?;
        self.skip_comma();

        Ok(VarDecl { name, value, pos })
    }

    fn parse_color_value(&mut self) -> Result<ColorValue, ParseError> {
        match self.advance().kind {
            TokenKind::ColorHex(hex) => Ok(ColorValue::Hex(hex)),
            TokenKind::ColorName(name) => Ok(ColorValue::Name(name)),
            TokenKind::Variable(name) => Ok(ColorValue::Variable(name)),
            other => Err(self.error(format!(
                "Expected a color value (hex code, color name, or variable) for \"mau_nen\", received {}",
                other
            ))),
        }
    }

    fn parse_page(&mut self) -> Result<Page, ParseError> {
        let pos = self.current_pos();
        self.advance();
        self.consume(&TokenKind::LParen, "Expected '(' after the 'trang' keyword")?;

        let route = match self.advance().kind {
            TokenKind::StringLit(s) => s,
            other => return Err(self.error(format!("Expected a route string, received {}", other))),
        };

        let mut name = None;
        let mut mau_nen = None;
        let mut description = None;

        while self.match_token(&TokenKind::Comma) {
            if let TokenKind::Identifier(k) = &self.current().kind {
                if crate::locale::resolve_prop_key(k) == Some(vibao_ast::PropKey::BackgroundColor) {
                    if self.check_at(1, &TokenKind::Colon) {
                        self.advance();
                        self.advance();
                        mau_nen = Some(self.parse_color_value()?);
                        continue;
                    }
                    return Err(self.error(
                        "Expected ':' after \"mau_nen\" (a named option needs the form \"mau_nen: <color>\")".to_string()
                    ));
                }
                if k == "mo_ta" || k == "description" {
                    if self.check_at(1, &TokenKind::Colon) {
                        self.advance();
                        self.advance();
                        description = match self.advance().kind {
                            TokenKind::StringLit(s) => Some(s),
                            other => {
                                return Err(self.error(format!(
                                    "Expected a string after \"mo_ta:\", received {}",
                                    other
                                )))
                            }
                        };
                        continue;
                    }
                    return Err(self.error(
                        "Expected ':' after \"mo_ta\" (a named option needs the form \"mo_ta: \\\"...\\\"\")".to_string()
                    ));
                }
            }
            name = match self.advance().kind {
                TokenKind::StringLit(s) | TokenKind::Identifier(s) => Some(s),
                other => {
                    return Err(
                        self.error(format!("Expected a valid page name, received {}", other))
                    )
                }
            };
        }

        self.consume(
            &TokenKind::RParen,
            "Expected ')' after the page declaration",
        )?;
        self.consume(&TokenKind::LBrace, "Expected '{' to start the page body")?;

        let mut states = Vec::new();
        let mut events = Vec::new();
        let mut children = Vec::new();

        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            if self.check(&TokenKind::State) {
                let var = self.parse_var_decl()?;
                states.push(StateDecl {
                    name: var.name,
                    value: var.value,
                    pos: var.pos,
                });
            } else if self.check(&TokenKind::OnTai) || self.check(&TokenKind::OnHuy) {
                events.push(self.parse_page_event()?);
            } else {
                children.push(self.parse_child()?);
            }
        }

        self.consume(&TokenKind::RBrace, "Expected '}' to close the page body")?;

        Ok(Page {
            route,
            name,
            description,
            mau_nen,
            states,
            events,
            children,
            pos,
        })
    }

    fn parse_page_event(&mut self) -> Result<vibao_ast::PageEvent, ParseError> {
        let pos = self.current_pos();
        let tok = self.advance();
        let name = match tok.kind {
            TokenKind::OnTai => vibao_ast::PageEventName::OnTai,
            TokenKind::OnHuy => vibao_ast::PageEventName::OnHuy,
            _ => unreachable!(),
        };

        self.consume(&TokenKind::LBrace, "Expected '{' after the page event name")?;
        let mut body = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            body.push(self.parse_action()?);
        }
        self.consume(&TokenKind::RBrace, "Expected '}' to close the page event")?;

        Ok(vibao_ast::PageEvent { name, body, pos })
    }

    fn parse_component_def(&mut self) -> Result<ComponentDef, ParseError> {
        let pos = self.current_pos();
        self.advance();
        self.advance();

        let name = match self.advance().kind {
            TokenKind::Identifier(s) | TokenKind::Component(s) => s,
            TokenKind::ColorName(s) => s,
            other => {
                return Err(self.error(format!("Expected a component name, received {}", other)))
            }
        };

        let mut params = Vec::new();
        if self.match_token(&TokenKind::LParen) {
            while !self.check(&TokenKind::RParen) && !self.is_at_end() {
                let p_pos = self.current_pos();
                let p_name = if let TokenKind::Variable(n) = &self.current().kind {
                    let n = n.clone();
                    self.advance();
                    n
                } else {
                    self.expect_identifier_like()?
                };

                let mut data_type = DataType::Any;
                if self.match_token(&TokenKind::Colon) {
                    let unexpected = self.error(format!(
                        "Invalid data type '{}' — expected one of: chuoi, so, mau, bool, mang, doi_tuong, hanh_dong",
                        self.current().kind
                    ));
                    data_type = match self.advance().kind {
                        TokenKind::Identifier(s) => match s.as_str() {
                            "chuoi" => DataType::Chuoi,
                            "so" => DataType::So,
                            "mau" => DataType::Mau,
                            "bool" => DataType::Bool,
                            "mang" => DataType::Mang,
                            "doi_tuong" => DataType::DoiTuong,
                            "hanh_dong" => DataType::HanhDong,
                            _ => return Err(unexpected),
                        },
                        _ => return Err(unexpected),
                    };
                }

                let mut default_value = None;
                if self.match_token(&TokenKind::Equals) {
                    default_value = Some(self.parse_value()?);
                }

                params.push(ParamDef {
                    name: p_name,
                    data_type,
                    default_value,
                    pos: p_pos,
                });
                self.skip_comma();
            }
            self.consume(
                &TokenKind::RParen,
                "Expected ')' to close the parameter list",
            )?;
        }

        self.consume(
            &TokenKind::LBrace,
            "Expected '{' to start the component body",
        )?;
        let mut children = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            children.push(self.parse_child()?);
        }
        self.consume(
            &TokenKind::RBrace,
            "Expected '}' to close the component body",
        )?;

        Ok(ComponentDef {
            name,
            params,
            children,
            pos,
        })
    }

    pub(crate) fn parse_child(&mut self) -> Result<Child, ParseError> {
        let pos = self.current_pos();
        if self.check(&TokenKind::Neu) {
            Ok(Child::If(Box::new(self.parse_if_node()?)))
        } else if self.check(&TokenKind::VongLap) {
            Ok(Child::Loop(Box::new(self.parse_loop_node()?)))
        } else if self.check(&TokenKind::TruongHop) {
            Ok(Child::Switch(Box::new(self.parse_switch_node()?)))
        } else if self.check(&TokenKind::State) {
            let var = self.parse_var_decl()?;
            Ok(Child::StateDecl(StateDecl {
                name: var.name,
                value: var.value,
                pos: var.pos,
            }))
        } else if let TokenKind::Variable(_) = &self.current().kind {
            let var = self.parse_var_decl()?;
            Ok(Child::VarDecl(var))
        } else {
            if let TokenKind::Component(surface_name) = &self.current().kind {
                let surface_name = surface_name.clone();
                let pos_tag = self.current_pos();
                self.advance();
                let tag = crate::locale::resolve_tag(&surface_name).ok_or_else(|| {
                    if crate::lexer::is_known_action_or_function_name(&surface_name) {
                        ParseError {
                            message: format!(
                                "'{}' is an action/function, not a UI tag. Actions/functions can only be used inside an event body (for example 'khi_nhan {{ {}(...) }}').",
                                surface_name, surface_name
                            ),
                            line: pos_tag.line,
                            column: pos_tag.column,
                        }
                    } else {
                        ParseError {
                            message: format!(
                                "'{}' was recognized as a valid tag by the lexer but could not be resolved by the locale registry. This is an internal compiler consistency error; report it to ViBao maintainers.",
                                surface_name
                            ),
                            line: pos_tag.line,
                            column: pos_tag.column,
                        }
                    }
                })?;
                let el = self.parse_element_rest(tag, pos)?;
                Ok(Child::Element(el))
            } else if let TokenKind::Identifier(name) = &self.current().kind {
                let name = name.clone();
                self.advance();
                if self.check(&TokenKind::LParen)
                    || self.check(&TokenKind::LBrace)
                    || self.check(&TokenKind::Colon)
                {
                    let call = self.parse_component_call_rest(name, pos)?;
                    Ok(Child::ComponentCall(call))
                } else {
                    Err(self.error(format!(
                        "'{}' is not a built-in tag, and is not followed by '(' or '{{' to be a valid custom component call - if this is a component name defined via @the, it needs parentheses (e.g. '{}(...)' or '{}{{ }}'), and if this is meant to be a built-in tag, check the spelling",
                        name, name, name
                    )))
                }
            } else {
                Err(self.error(format!(
                    "Invalid UI tree structure: {}",
                    self.current().kind
                )))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::tokenize;

    fn parse_page_from_source(src: &str) -> Page {
        let tokens = tokenize(src).unwrap();
        let mut p = Parser::new(tokens);
        p.parse_page().unwrap()
    }

    #[test]
    fn test_parse_page_background_color_accepts_english_prop_name() {
        let page = parse_page_from_source(r#"trang("/", background_color: xanh) { }"#);
        assert!(matches!(
            page.mau_nen,
            Some(ColorValue::Name(ref value)) if value == "xanh"
        ));
    }

    #[test]
    fn test_parse_single_import() {
        let tokens = tokenize(r#"nhap nut_bam tu "./components/nut_bam.vbao""#).unwrap();
        let mut p = Parser::new(tokens);
        let import = p.parse_import().unwrap();
        assert_eq!(import.names, vec!["nut_bam".to_string()]);
        assert_eq!(import.path, "./components/nut_bam.vbao");
    }

    #[test]
    fn test_parse_multi_name_import() {
        let tokens = tokenize(r#"nhap { nut_bam, the_card } tu "./components/ui.vbao""#).unwrap();
        let mut p = Parser::new(tokens);
        let import = p.parse_import().unwrap();
        assert_eq!(
            import.names,
            vec!["nut_bam".to_string(), "the_card".to_string()]
        );
        assert_eq!(import.path, "./components/ui.vbao");
    }

    #[test]
    fn test_parse_import_missing_tu_is_error() {
        let tokens = tokenize(r#"nhap nut_bam "./components/nut_bam.vbao""#).unwrap();
        let mut p = Parser::new(tokens);
        assert!(p.parse_import().is_err());
    }

    #[test]
    fn test_import_inside_ung_dung_block() {
        let src = r#"
            ung_dung("Test") {
                nhap nut_bam tu "./components/nut_bam.vbao"
                trang("/") { }
            }
        "#;
        let tokens = tokenize(src).unwrap();
        let mut p = Parser::new(tokens);
        let app = p.parse_app().unwrap();
        assert_eq!(app.imports.len(), 1);
        assert_eq!(app.imports[0].names, vec!["nut_bam".to_string()]);
        assert_eq!(app.pages.len(), 1);
    }

    #[test]
    fn test_parse_module_file_without_ung_dung_wrapper() {
        let src = r#"
            @the nut_bam(label) {
                button(noi_dung: $label)
            }
        "#;
        let tokens = tokenize(src).unwrap();
        let mut p = Parser::new(tokens);
        let body = p.parse_module_file().unwrap();
        assert_eq!(body.components.len(), 1);
        assert_eq!(body.components[0].name, "nut_bam");
    }

    #[test]
    fn test_parse_module_file_can_itself_import() {
        let src = r#"
            nhap the_icon tu "./icon.vbao"
            @the nut_bam(label) {
                button(noi_dung: $label)
            }
        "#;
        let tokens = tokenize(src).unwrap();
        let mut p = Parser::new(tokens);
        let body = p.parse_module_file().unwrap();
        assert_eq!(body.imports.len(), 1);
        assert_eq!(body.components.len(), 1);
    }

    #[test]
    fn test_parse_page_without_mau_nen_stays_none() {
        let page = parse_page_from_source(r#"trang("/") { }"#);
        assert!(page.mau_nen.is_none());
    }

    #[test]
    fn test_parse_page_with_name_only_still_works() {
        let page = parse_page_from_source(r#"trang("/", "Trang chủ") { }"#);
        assert_eq!(page.name, Some("Trang chủ".to_string()));
        assert!(page.mau_nen.is_none());
    }

    #[test]
    fn test_parse_page_with_mau_nen_color_name() {
        let page = parse_page_from_source(r#"trang("/", mau_nen: xanh) { }"#);
        match page.mau_nen {
            Some(ColorValue::Name(n)) => assert_eq!(n, "xanh"),
            other => panic!("expected ColorValue::Name(\"xanh\"), got {:?}", other),
        }
    }

    #[test]
    fn test_parse_page_with_mau_nen_hex() {
        let page = parse_page_from_source(r#"trang("/", mau_nen: #FF0000) { }"#);
        match page.mau_nen {
            Some(ColorValue::Hex(h)) => assert_eq!(h, "#FF0000"),
            other => panic!("expected ColorValue::Hex, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_page_mau_nen_missing_colon_is_error() {
        let tokens = tokenize(r#"trang("/", mau_nen) { }"#).unwrap();
        let mut p = Parser::new(tokens);
        let result = p.parse_page();
        assert!(
            result.is_err(),
            "expected a parse error when 'mau_nen' is missing its ':', but parsing succeeded"
        );
    }

    #[test]
    fn test_parse_page_with_name_and_mau_nen_together() {
        let page = parse_page_from_source(r#"trang("/", "Trang chủ", mau_nen: xam_nhat) { }"#);
        assert_eq!(page.name, Some("Trang chủ".to_string()));
        match page.mau_nen {
            Some(ColorValue::Name(n)) => assert_eq!(n, "xam_nhat"),
            other => panic!("expected ColorValue::Name, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_page_mau_nen_before_name_also_works() {
        let page = parse_page_from_source(r#"trang("/", mau_nen: do, "Trang chủ") { }"#);
        assert_eq!(page.name, Some("Trang chủ".to_string()));
        match page.mau_nen {
            Some(ColorValue::Name(n)) => assert_eq!(n, "do"),
            other => panic!("expected ColorValue::Name, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_page_with_mo_ta() {
        let page =
            parse_page_from_source(r#"trang("/", "Trang chủ", mo_ta: "Giới thiệu bản thân") { }"#);
        assert_eq!(page.name, Some("Trang chủ".to_string()));
        assert_eq!(page.description, Some("Giới thiệu bản thân".to_string()));
    }

    #[test]
    fn test_parse_page_english_description_option_also_works() {
        let page = parse_page_from_source(r#"trang("/", description: "About me") { }"#);
        assert_eq!(page.description, Some("About me".to_string()));
    }

    #[test]
    fn test_parse_page_without_mo_ta_stays_none() {
        let page = parse_page_from_source(r#"trang("/") { }"#);
        assert_eq!(page.description, None);
    }

    #[test]
    fn test_parse_page_mo_ta_order_independent_from_mau_nen() {
        let page =
            parse_page_from_source(r#"trang("/", mo_ta: "Mo ta", mau_nen: xanh, "Trang chủ") { }"#);
        assert_eq!(page.name, Some("Trang chủ".to_string()));
        assert_eq!(page.description, Some("Mo ta".to_string()));
        assert!(page.mau_nen.is_some());
    }

    #[test]
    fn test_parse_page_name_before_mo_ta() {
        let page = parse_page_from_source(r#"trang("/", "Trang chủ", mo_ta: "Mo ta") { }"#);
        assert_eq!(page.name, Some("Trang chủ".to_string()));
        assert_eq!(page.description, Some("Mo ta".to_string()));
    }

    #[test]
    fn test_parse_page_mo_ta_missing_colon_is_error() {
        let tokens = tokenize(r#"trang("/", mo_ta) { }"#).unwrap();
        let mut p = Parser::new(tokens);
        let result = p.parse_page();
        assert!(
            result.is_err(),
            "expected a parse error when 'mo_ta' is missing its ':'"
        );
    }

    #[test]
    fn test_parse_page_mo_ta_requires_string_literal() {
        let tokens = tokenize(r#"trang("/", mo_ta: 123) { }"#).unwrap();
        let mut p = Parser::new(tokens);
        let result = p.parse_page();
        assert!(
            result.is_err(),
            "expected a parse error when 'mo_ta' value isn't a string literal"
        );
    }

    fn parse_component_def_from_source(src: &str) -> ComponentDef {
        let tokens = tokenize(src).unwrap();
        let mut p = Parser::new(tokens);
        p.parse_component_def().unwrap()
    }

    #[test]
    fn test_component_param_name_matching_builtin_tag() {
        let def = parse_component_def_from_source("@the TheThe(nhan: chuoi, mau_nen: mau) { }");
        assert_eq!(def.name, "TheThe");
        assert_eq!(def.params.len(), 2);
        assert_eq!(def.params[0].name, "nhan");
        assert_eq!(def.params[1].name, "mau_nen");
    }

    #[test]
    fn test_component_param_name_matching_color_name() {
        let def = parse_component_def_from_source("@the TheBao(do: so) { }");
        assert_eq!(def.params[0].name, "do");
    }

    #[test]
    fn test_component_name_matching_color_name() {
        let def = parse_component_def_from_source("@the Do(x: so) { }");
        assert_eq!(def.name, "Do");
    }

    #[test]
    fn test_action_name_used_as_child_reports_wrong_position_error_not_compiler_bug() {
        let tokens = tokenize("thong_bao(\"Xin chao\")").unwrap();
        let mut p = Parser::new(tokens);
        let result = p.parse_child();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            !err.message.contains("bug compiler"),
            "must NOT be wrongly reported as a compiler bug for an ordinary syntax error: {}",
            err.message
        );
        assert!(
            err.message.contains("khi_nhan") || err.message.contains("event body"),
            "the message must point to the correct usage (an event block): {}",
            err.message
        );
    }

    #[test]
    fn test_goi_api_used_as_child_reports_wrong_position_error() {
        let tokens = tokenize("goi_api(\"GET\", \"/api\")").unwrap();
        let mut p = Parser::new(tokens);
        let result = p.parse_child();
        assert!(result.is_err());
        assert!(!result.unwrap_err().message.contains("bug compiler"));
    }

    #[test]
    fn test_component_param_invalid_datatype_typo_is_parse_error() {
        let tokens = tokenize("@the TheBao(ten: choui) { }").unwrap();
        let mut p = Parser::new(tokens);
        let result = p.parse_component_def();
        assert!(
            result.is_err(),
            "a typo'd data type must produce a parse error, not silently become Any"
        );
    }

    #[test]
    fn test_component_param_datatype_matching_color_name_is_parse_error() {
        let tokens = tokenize("@the TheBao(ten: do) { }").unwrap();
        let mut p = Parser::new(tokens);
        let result = p.parse_component_def();
        assert!(result.is_err(), "a color name in the data-type position must produce a parse error, not silently become Any");
    }

    #[test]
    fn test_component_param_all_7_valid_datatypes_still_work() {
        for (src_type, expected) in [
            ("chuoi", DataType::Chuoi),
            ("so", DataType::So),
            ("mau", DataType::Mau),
            ("bool", DataType::Bool),
            ("mang", DataType::Mang),
            ("doi_tuong", DataType::DoiTuong),
            ("hanh_dong", DataType::HanhDong),
        ] {
            let src = format!("@the TheBao(ten: {}) {{ }}", src_type);
            let def = parse_component_def_from_source(&src);
            assert_eq!(
                def.params[0].data_type, expected,
                "type '{}' must parse correctly",
                src_type
            );
        }
    }

    #[test]
    fn test_component_param_no_type_annotation_still_defaults_to_any() {
        let def = parse_component_def_from_source("@the TheBao(ten) { }");
        assert_eq!(def.params[0].data_type, DataType::Any);
    }

    #[test]
    fn test_member_access_field_matching_color_name() {
        let tokens = tokenize("$task.do").unwrap();
        let mut p = Parser::new(tokens);
        let expr = p.parse_value().unwrap();
        match expr {
            vibao_ast::Expr::MemberAccess { property, .. } => assert_eq!(property, "do"),
            other => panic!("expected MemberAccess, got {:?}", other),
        }
    }

    #[test]
    fn test_import_name_matching_color_name() {
        let tokens = tokenize(r#"nhap vang tu "./components/vang.vbao""#).unwrap();
        let mut p = Parser::new(tokens);
        let import = p.parse_import().unwrap();
        assert_eq!(import.names, vec!["vang".to_string()]);
    }
}
