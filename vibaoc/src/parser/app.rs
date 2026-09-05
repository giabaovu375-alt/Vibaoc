// ============================================================
// VIBAO COMPILER (Rust) — parser/app.rs
// Handles parsing the top-level structures: the application (app),
// themes, global variables, pages, and component definitions (@the).
// ============================================================

use super::{ParseError, Parser};
use vibao_ast::{App, ComponentDef, Page, Theme, VarDecl, StateDecl, ParamDef, DataType, Child, ColorValue};
use crate::lexer::TokenKind;

/// The intermediate result of `parse_module_body` — bundles every kind
/// of module/app-level declaration into one struct instead of returning
/// a 5-element tuple (easier to read at the call site, and easier to
/// extend with a new declaration kind later without changing the
/// function signature).
pub(crate) struct ModuleBody {
    pub imports: Vec<vibao_ast::ImportDecl>,
    pub variables: Vec<VarDecl>,
    pub themes: Vec<Theme>,
    pub components: Vec<ComponentDef>,
    pub pages: Vec<Page>,
}

impl Parser {
    /// Checks whether the current token is "@the" (2 tokens: At, then
    /// an Identifier whose VALUE is exactly "the").
    ///
    /// BUG ALREADY FIXED: this condition used to be
    /// `check_at(1, &TokenKind::Identifier("the".to_string()))` — but
    /// `check`/`check_at` only compare std::mem::discriminant (the enum
    /// variant's KIND), NOT the String value inside it. The result: ANY
    /// identifier after "@" (e.g. "@banner", "@item", "@x") was
    /// misrecognized as "@the", causing parse_component_def() to
    /// accidentally swallow a token without ever checking whether it
    /// was actually "the" - silently misparsing the structure, with no
    /// error raised. This function correctly checks the actual string
    /// value inside the Identifier token.
    fn is_at_symbol_the(&self) -> bool {
        if !self.check(&TokenKind::At) {
            return false;
        }
        matches!(&self.peek(1).kind, TokenKind::Identifier(s) if s == "the")
    }

    /// Parses the application's main entry point: ung_dung("Ten") { ... }
    pub(crate) fn parse_app(&mut self) -> Result<App, ParseError> {
        let pos = self.current_pos();
        self.consume(&TokenKind::UngDung, "Expected the 'ung_dung' keyword")?;
        self.consume(&TokenKind::LParen, "Expected '(' after the 'ung_dung' keyword")?;
        
        let name = match self.advance().kind {
            TokenKind::StringLit(s) => s,
            other => return Err(self.error(format!("Expected an application name string, received {}", other))),
        };
        
        self.consume(&TokenKind::RParen, "Expected ')' after the application name")?;
        self.consume(&TokenKind::LBrace, "Expected '{' to start the application body")?;

        let body = self.parse_module_body(&TokenKind::RBrace)?;

        self.consume(&TokenKind::RBrace, "Expected '}' to close the application body")?;

        Ok(App {
            name,
            imports: body.imports,
            variables: body.variables,
            themes: body.themes,
            components: body.components,
            pages: body.pages,
            pos,
        })
    }


    /// Parses the shared BODY between 2 contexts:
    ///   1. Inside `ung_dung("...") { <here> }` - used by parse_app.
    ///   2. An entire file brought in via `nhap` (with NO `ung_dung`
    ///      wrapper) - used by parse_module_file (see below).
    ///
    /// Split out into its own function so these 2 contexts do NOT
    /// duplicate/drift from each other when someone adds a new
    /// declaration kind (e.g. adding `hang_so` (constant) later - only
    /// needs to change HERE, and both contexts pick it up
    /// automatically).
    ///
    /// `stop_at`: the token signaling the end of the loop - `RBrace` in
    /// context (1), `Eof` in context (2) (a file has no closing
    /// delimiter, so end-of-file = end-of-body).
    pub(crate) fn parse_module_body(&mut self, stop_at: &TokenKind) -> Result<ModuleBody, ParseError> {
        let mut imports = Vec::new();
        let mut variables = Vec::new();
        let mut themes = Vec::new();
        let mut components = Vec::new();
        let mut pages = Vec::new();

        while !self.check(stop_at) && !self.is_at_end() {
            if self.check_ident("lang") {
                return Err(self.error("The 'lang' declaration must appear at the top of the file"));
            } else if self.check(&TokenKind::Nhap) {
                imports.push(self.parse_import()?);
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
                return Err(self.error(format!("Invalid declaration inside the application block: {}", self.current().kind)));
            }
        }

        Ok(ModuleBody { imports, variables, themes, components, pages })
    }

    /// Parses an import declaration: `nhap ten tu "duong_dan"` or
    /// `nhap { ten_a, ten_b } tu "duong_dan"`.
    ///
    /// The syntax ONLY records the name list + raw path - whether that
    /// file actually exists, or actually defines the names listed, is
    /// the responsibility of the module resolver (which runs AFTER the
    /// parser), not this function. This preserves a clean layer
    /// boundary: the parser only knows syntax, and knows nothing about
    /// the filesystem.
    fn parse_import(&mut self) -> Result<vibao_ast::ImportDecl, ParseError> {
        let pos = self.current_pos();
        self.advance(); // tiêu thụ 'nhap'

        let mut names = Vec::new();
        if self.check(&TokenKind::LBrace) {
            // The multi-name form: nhap { a, b, c } tu "..."
            self.advance(); // consume '{'
            loop {
                // The same bug class as the @the parameter (see the full
                // explanation in parse_component_def) - an import name
                // can collide with an existing component/tag or one of
                // the 14 color names, causing the lexer to emit
                // Component/ColorName instead of a plain Identifier.
                // Uses expect_identifier_like() (already exists) instead
                // of only accepting Identifier.
                let name = self.expect_identifier_like()?;
                names.push(name);
                if self.check(&TokenKind::Comma) {
                    self.advance();
                    continue;
                }
                break;
            }
            self.consume(&TokenKind::RBrace, "Expected '}' to close the import name list")?;
        } else {
            // The single-name form: nhap ten tu "..."
            // Same reasoning as the multi-name branch above - uses
            // expect_identifier_like() instead of only accepting a
            // plain Identifier.
            let name = self.expect_identifier_like()?;
            names.push(name);
        }

        self.consume(&TokenKind::Tu, "Expected the 'tu' keyword after the import name list")?;

        let path = match self.advance().kind {
            TokenKind::StringLit(s) => s,
            other => return Err(self.error(format!(
                "Expected an import path string after 'tu', received {}", other
            ))),
        };

        Ok(vibao_ast::ImportDecl { names, path, pos })
    }

    /// The entry point for a file brought in via `nhap` by another file
    /// - has NO `ung_dung(...)` wrapper, and is just a flat list of
    /// declarations (usually just 1-2 `@the` component definitions, but
    /// syntactically allows every declaration kind that parse_module_body
    /// supports, e.g. a file can also `nhap` another file itself - see
    /// the module resolver for how circular imports are detected).
    pub(crate) fn parse_module_file(&mut self) -> Result<ModuleBody, ParseError> {
        self.parse_language_header()?;
        let body = self.parse_module_body(&TokenKind::Eof)?;
        self.expect(&TokenKind::Eof)?;
        Ok(body)
    }


    /// Parses a Theme block: theme TenTheme { $bien = gia_tri }
    fn parse_theme(&mut self) -> Result<Theme, ParseError> {
        let pos = self.current_pos();
        self.advance(); // consume 'theme'
        
        let name = match self.advance().kind {
            TokenKind::Identifier(s) | TokenKind::StringLit(s) => s,
            other => return Err(self.error(format!("Expected a theme identifier, received {}", other))),
        };

        self.consume(&TokenKind::LBrace, "Expected '{' after the theme name")?;
        let mut variables = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            variables.push(self.parse_var_decl()?);
        }
        self.consume(&TokenKind::RBrace, "Expected '}' to close the theme")?;

        Ok(Theme { name, variables, pos })
    }

    /// Parses a variable/state declaration: $ten_bien = gia_tri, or state $ten = gia_tri
    pub(crate) fn parse_var_decl(&mut self) -> Result<VarDecl, ParseError> {
        let pos = self.current_pos();
        if self.check(&TokenKind::State) {
            self.advance();
        }
        
        let name = match self.advance().kind {
            TokenKind::Variable(n) => n,
            other => return Err(self.error(format!("Expected a variable name starting with $, received {}", other))),
        };

        self.consume(&TokenKind::Equals, "Expected '=' after the variable name")?;
        let value = self.parse_value()?;
        self.skip_comma();

        Ok(VarDecl { name, value, pos })
    }

    /// Parses a color value (used for the named option "mau_nen: ..."):
    /// accepts a hex code (#RRGGBB, already lexed as ColorHex), a
    /// Vietnamese color name (ColorName - the lexer only checked that
    /// the raw name is valid; the token itself still keeps the ORIGINAL
    /// name, not a hex code, so it's still wrapped in ColorValue::Name so
    /// codegen resolves the actual hex value when it generates the CSS),
    /// or a variable "$ten".
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

    /// Parses a Page: trang("/route", "Ten Trang", mau_nen: xanh) { ... }
    fn parse_page(&mut self) -> Result<Page, ParseError> {
        let pos = self.current_pos();
        self.advance(); // consume 'trang'
        self.consume(&TokenKind::LParen, "Expected '(' after the 'trang' keyword")?;

        let route = match self.advance().kind {
            TokenKind::StringLit(s) => s,
            other => return Err(self.error(format!("Expected a route string, received {}", other))),
        };

        let mut name = None;
        let mut mau_nen = None;

        // After the route, there can be: a page-name string
        // (positional, unnamed), and/or the named option "mau_nen: ..." -
        // order isn't enforced, reading repeatedly across commas until
        // ')' is reached.
        while self.match_token(&TokenKind::Comma) {
            // A named option in "key: value" form (currently only mau_nen).
            if let TokenKind::Identifier(k) = &self.current().kind {
                if crate::locale::resolve_prop_key(k) == Some(vibao_ast::PropKey::BackgroundColor) {
                    if self.check_at(1, &TokenKind::Colon) {
                        self.advance(); // consume "mau_nen"
                        self.advance(); // consume ':'
                        mau_nen = Some(self.parse_color_value()?);
                        continue;
                    }
                    // BUG ALREADY FIXED: if ':' was missing after
                    // "mau_nen", the code used to fall through to the
                    // branch below and silently interpret "mau_nen" as
                    // the POSITIONAL page name - a syntax mistake
                    // swallowed with no error. "mau_nen" isn't a
                    // reasonable page name (it's a named-option name), so
                    // this now raises a clear error the moment ':' is
                    // missing, instead of guessing.
                    return Err(self.error(
                        "Expected ':' after \"mau_nen\" (a named option needs the form \"mau_nen: <color>\")".to_string()
                    ));
                }
            }
            // Not a named option -> treated as the positional page name (as before).
            name = match self.advance().kind {
                TokenKind::StringLit(s) | TokenKind::Identifier(s) => Some(s),
                other => return Err(self.error(format!("Expected a valid page name, received {}", other))),
            };
        }

        self.consume(&TokenKind::RParen, "Expected ')' after the page declaration")?;
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
            mau_nen,
            states,
            events,
            children,
            pos,
        })
    }

    /// Parses a page lifecycle event: on_tai { ... } or on_huy { ... }
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

    /// Parses a custom component definition: @the TenThanhPhan($param: kieu) { ... }
    fn parse_component_def(&mut self) -> Result<ComponentDef, ParseError> {
        let pos = self.current_pos();
        // This function is only ever called once is_at_symbol_the() has
        // confirmed exactly 2 consecutive tokens: TokenKind::At then
        // TokenKind::Identifier("the"). The lexer never emits a single
        // combined "the" token, so this always consumes exactly these 2
        // tokens.
        self.advance(); // consume @
        self.advance(); // consume 'the'

        let name = match self.advance().kind {
            TokenKind::Identifier(s) | TokenKind::Component(s) => s,
            // The same bug class as the component's parameter names
            // (see the explanation in the params section of
            // parse_component_def below) - a component's OWN name can
            // also collide with one of the 14 color names (e.g.
            // "@the Do(...)" - though the component-naming convention of
            // capitalizing the first letter makes this rare in practice,
            // it's still handled consistently so there's no hard-to-
            // predict "forbidden zone").
            TokenKind::ColorName(s) => s,
            other => return Err(self.error(format!("Expected a component name, received {}", other))),
        };

        let mut params = Vec::new();
        if self.match_token(&TokenKind::LParen) {
            while !self.check(&TokenKind::RParen) && !self.is_at_end() {
                let p_pos = self.current_pos();
                // A REAL BUG THAT WAS FIXED (the same bug class as "den"
                // colliding with a color name, fixed earlier, but this
                // time it's a PARAMETER NAME colliding with a BUILT-IN
                // COMPONENT/TAG - e.g. "nhan" is in the text-like tag
                // list ["text","h1","h2","h3","p","nhan"] in
                // lexer/tables.rs): this used to only accept
                // TokenKind::Variable/Identifier as a parameter name - if
                // a dev declared "@the TheThe(nhan: chuoi, ...)", the
                // lexer had already emitted "nhan" as
                // TokenKind::Component("nhan") (exactly like
                // "text"/"button"...) BEFORE the parser could know this
                // was a PARAMETER NAME position (not a tag-call
                // position) - the old parser rejected it immediately,
                // even though the syntax was fully valid per the spec
                // (docs/VIBAO_SPEC.md, section 8: a parameter name is a
                // plain identifier, with nothing forbidding it from
                // colliding with a tag name).
                //
                // Fix: uses expect_identifier_like() (ALREADY EXISTS,
                // used for exactly this purpose elsewhere - see its
                // definition site for the full history of the "den" bug
                // fix) - accepts Identifier/Component/ColorName as a
                // name, treating it as if the lexer had "never"
                // misclassified it. Variable is STILL tried FIRST (the
                // old branch is kept) so as not to break the case of
                // someone declaring a parameter with a leading "$"
                // (though the spec doesn't require this, it also doesn't
                // clearly forbid it - keeping backward compatibility,
                // not narrowing what syntax was previously accepted).
                let p_name = if let TokenKind::Variable(n) = &self.current().kind {
                    let n = n.clone();
                    self.advance();
                    n
                } else {
                    self.expect_identifier_like()?
                };

                let mut data_type = DataType::Any;
                if self.match_token(&TokenKind::Colon) {
                    // BUG ALREADY FIXED: a token that didn't match one of
                    // the 7 valid type names (including a typo, or NOT
                    // being an Identifier at all - e.g. accidentally
                    // typing a type name that collides with a
                    // color/component) used to SILENTLY fall back to
                    // DataType::Any - the build raised no error, and the
                    // parameter's type was silently misread (accepting
                    // any value at all). This now raises a clear error
                    // RIGHT HERE (at the exact position the bad input
                    // occurred, calling self.error() BEFORE advance() so
                    // the reported position points at the actual offending
                    // token, not the next one by mistake), instead of
                    // letting the error surface later in codegen or the
                    // runtime.
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
            self.consume(&TokenKind::RParen, "Expected ')' to close the parameter list")?;
        }

        self.consume(&TokenKind::LBrace, "Expected '{' to start the component body")?;
        let mut children = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            children.push(self.parse_child()?);
        }
        self.consume(&TokenKind::RBrace, "Expected '}' to close the component body")?;

        Ok(ComponentDef {
            name,
            params,
            children,
            pos,
        })
    }

    /// Parses any child node within the UI layout tree
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
            Ok(Child::StateDecl(StateDecl { name: var.name, value: var.value, pos: var.pos }))
        } else if let TokenKind::Variable(_) = &self.current().kind {
            let var = self.parse_var_decl()?;
            Ok(Child::VarDecl(var))
        } else {
            if let TokenKind::Component(surface_name) = &self.current().kind {
                // Wiring up the real Tag (BUG-25 context + the settled
                // flow: a user writes "box"/"khoi" -> Lexer -> Token/
                // surface name -> Resolver -> Tag::Khoi -> AST). HERE is
                // the Resolver step: TokenKind::Component(surface_name)
                // has already been confirmed by the LEXER to have
                // surface_name present in component_set() (the old
                // built-in table) - but that table only has Vietnamese
                // names, so once real locales (vi/en) were added, this
                // needed to resolve through BOTH locale tables (not just
                // the old one) so "box" and "khoi" both produce
                // Tag::Khoi. resolve_tag() below does exactly that -
                // checking locale::vi FIRST (the primary locale), then
                // locale::en (the universal baseline locale), per the
                // model settled in ARCHITECTURE_PROPOSAL.md.
                let surface_name = surface_name.clone();
                let pos_tag = self.current_pos();
                self.advance();
                // BUG ALREADY FIXED (found through review - thanks): the
                // None branch here used to ALWAYS report a "compiler
                // bug" - but component_set() (lexer/tables.rs) contains
                // BOTH Tag names AND action/function names (the
                // "Feedback / actions" group, e.g.
                // "thong_bao"/"goi_api"/"gia_tien"...) - locale::resolve_tag()
                // ONLY covers the Tag group (correctly, on purpose). If a
                // dev writes an action/function name in a TAG POSITION
                // (e.g. "thong_bao(...)" as a child of a page/box,
                // instead of the correct place inside an event block
                // like "khi_nhan { thong_bao(...) }" - see the events
                // section of docs/VIBAO_SPEC.md), the lexer STILL emits
                // TokenKind::Component("thong_bao") normally (correctly,
                // since "thong_bao" IS in component_set()) - but this is
                // an ORDINARY SYNTAX ERROR made by the user (wrong
                // position), NOT a compiler consistency bug between 2
                // sources. The 2 cases are distinguished using
                // is_known_action_or_function_name() (backed by the same
                // data source as component_set(), with a cross-check
                // test - see lexer/tables.rs) to produce the right
                // message.
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
                        // This case ONLY happens if
                        // component_set()/is_known_action_or_function_name()/
                        // locale::resolve_tag() have actually drifted
                        // apart (a cross-check test in lexer/tables.rs
                        // already guards against this) - if it still
                        // happens, it really is a compiler bug.
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
                if self.check(&TokenKind::LParen) || self.check(&TokenKind::LBrace) || self.check(&TokenKind::Colon) {
                    let call = self.parse_component_call_rest(name, pos)?;
                    Ok(Child::ComponentCall(call))
                } else {
                    // BUG-25 ALREADY FIXED (see AUDIT.md): this branch
                    // used to SILENTLY create a Child::Element with
                    // `tag` set to an arbitrary string (name), with no
                    // validation at all - since `Element.tag` is NOW a
                    // Tag (no longer a String), there's no longer any way
                    // to "silently" accept an invalid name (Rust requires
                    // a real Tag to exist). The spec/tests were checked:
                    // there is NO evidence that "calling a component
                    // without parentheses" was intended to be valid
                    // syntax (every example in docs/VIBAO_SPEC.md calls a
                    // component with parentheses) - so this is now always
                    // treated as a SYNTAX ERROR, reported clearly instead
                    // of guessing the user's intent.
                    Err(self.error(format!(
                        "'{}' is not a built-in tag, and is not followed by '(' or '{{' to be a valid custom component call - if this is a component name defined via @the, it needs parentheses (e.g. '{}(...)' or '{}{{ }}'), and if this is meant to be a built-in tag, check the spelling",
                        name, name, name
                    )))
                }
            } else {
                Err(self.error(format!("Invalid UI tree structure: {}", self.current().kind)))
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
        assert_eq!(import.names, vec!["nut_bam".to_string(), "the_card".to_string()]);
        assert_eq!(import.path, "./components/ui.vbao");
    }

    #[test]
    fn test_parse_import_missing_tu_is_error() {
        // A missing 'tu' keyword must produce a clear error, no guessing.
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
        // A file brought in via `nhap` by another file has NO ung_dung
        // wrapper - it's just one (or more) bare @the definitions.
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
        // A file brought in via nhap is itself allowed to nhap another
        // file (the module resolver at the layer above is responsible
        // for detecting circular imports - the parser only needs to
        // allow this syntax).
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
        // BUG ALREADY FIXED: "mau_nen" missing its ':' used to be
        // silently misread as the positional page name - no error
        // raised. It must now return a clear error.
        let tokens = tokenize(r#"trang("/", mau_nen) { }"#).unwrap();
        let mut p = Parser::new(tokens);
        let result = p.parse_page();
        assert!(result.is_err(), "expected a parse error when 'mau_nen' is missing its ':', but parsing succeeded");
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
        // Order isn't enforced: mau_nen first, page name second.
        let page = parse_page_from_source(r#"trang("/", mau_nen: do, "Trang chủ") { }"#);
        assert_eq!(page.name, Some("Trang chủ".to_string()));
        match page.mau_nen {
            Some(ColorValue::Name(n)) => assert_eq!(n, "do"),
            other => panic!("expected ColorValue::Name, got {:?}", other),
        }
    }

    // ── Tests for a real bug: an @the component's parameter/name
    // colliding with an existing component/tag or one of the 14 color
    // names ──

    fn parse_component_def_from_source(src: &str) -> ComponentDef {
        let tokens = tokenize(src).unwrap();
        let mut p = Parser::new(tokens);
        p.parse_component_def().unwrap()
    }

    #[test]
    fn test_component_param_name_matching_builtin_tag() {
        // A REAL BUG THAT WAS FIXED (found through an actual test
        // build): "nhan" is a built-in tag (text-like, see
        // lexer/tables.rs) - the parser used to reject a parameter named
        // "nhan" because the lexer emits TokenKind::Component instead of
        // Identifier, and parse_component_def() only accepted
        // Variable/Identifier as a parameter name.
        let def = parse_component_def_from_source("@the TheThe(nhan: chuoi, mau_nen: mau) { }");
        assert_eq!(def.name, "TheThe");
        assert_eq!(def.params.len(), 2);
        assert_eq!(def.params[0].name, "nhan");
        assert_eq!(def.params[1].name, "mau_nen");
    }

    #[test]
    fn test_component_param_name_matching_color_name() {
        // The same bug class, but with a parameter name colliding with
        // one of the 14 color names (e.g. "do") instead of a tag.
        let def = parse_component_def_from_source("@the TheBao(do: so) { }");
        assert_eq!(def.params[0].name, "do");
    }

    #[test]
    fn test_component_name_matching_color_name() {
        // The component's OWN name (not a parameter) colliding with a
        // color name.
        let def = parse_component_def_from_source("@the Do(x: so) { }");
        assert_eq!(def.name, "Do");
    }

    // ── BUG ALREADY FIXED: an action/function used in the wrong
    // position (a child, not inside an event block) used to WRONGLY
    // report a "compiler bug" instead of an ordinary syntax error
    // ─────────────────────────────────────────────────

    #[test]
    fn test_action_name_used_as_child_reports_wrong_position_error_not_compiler_bug() {
        // "thong_bao" is in component_set() (the action/function group)
        // so the lexer still normally emits
        // TokenKind::Component("thong_bao") - but it isn't a valid Tag,
        // and this is not a compiler consistency bug, but a dev putting
        // it in the wrong position (missing the surrounding event
        // block).
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

    // ── DataType - a clear error instead of silently falling back ─────────
    // (BUG ALREADY FIXED: see the note at the DataType-parsing site in
    // parse_component_def(). An invalid data type - a typo, or not an
    // Identifier at all - used to silently become DataType::Any, with
    // the build raising no error.)

    #[test]
    fn test_component_param_invalid_datatype_typo_is_parse_error() {
        // "choui" (a typo of "chuoi") must be rejected, NOT silently
        // become DataType::Any.
        let tokens = tokenize("@the TheBao(ten: choui) { }").unwrap();
        let mut p = Parser::new(tokens);
        let result = p.parse_component_def();
        assert!(result.is_err(), "a typo'd data type must produce a parse error, not silently become Any");
    }

    #[test]
    fn test_component_param_datatype_matching_color_name_is_parse_error() {
        // "do" (a color name, NOT one of the 7 valid type names) in the
        // data-type position - this used to fall into the non-
        // Identifier-token case (ColorName), silently falling back to
        // Any. It must now produce a clear error.
        let tokens = tokenize("@the TheBao(ten: do) { }").unwrap();
        let mut p = Parser::new(tokens);
        let result = p.parse_component_def();
        assert!(result.is_err(), "a color name in the data-type position must produce a parse error, not silently become Any");
    }

    #[test]
    fn test_component_param_all_7_valid_datatypes_still_work() {
        // Confirms the fix does NOT break the 7 already-valid types -
        // each must continue to parse correctly, with no false
        // failures.
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
            assert_eq!(def.params[0].data_type, expected, "type '{}' must parse correctly", src_type);
        }
    }

    #[test]
    fn test_component_param_no_type_annotation_still_defaults_to_any() {
        // Not writing ':' after a parameter name - is NOT an error, it's
        // still valid and defaults to Any (unlike the case of HAVING a
        // ':' but with a wrong type).
        let def = parse_component_def_from_source("@the TheBao(ten) { }");
        assert_eq!(def.params[0].data_type, DataType::Any);
    }

    #[test]
    fn test_member_access_field_matching_color_name() {
        // The same bug class in a DIFFERENT position: a field after '.'
        // (member access) colliding with a color name, e.g. $task.do (a
        // field named "do", with NO relation to the color red in this
        // context).
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
        // An import name colliding with a color name (e.g. "vang" - one
        // of the 14 color names).
        let tokens = tokenize(r#"nhap vang tu "./components/vang.vbao""#).unwrap();
        let mut p = Parser::new(tokens);
        let import = p.parse_import().unwrap();
        assert_eq!(import.names, vec!["vang".to_string()]);
    }
}
