use super::{ParseError, Parser};
use crate::lexer::TokenKind;
use vibao_ast::{
    AnimationProps, Breakpoint, ComponentCall, Element, EventName, EventNode, Expr, LiteralValue,
    PropKey, PropsMap, ResponsiveNode,
};

fn tag_requires_blocks(tag: vibao_ast::Tag) -> bool {
    use vibao_ast::Tag;
    matches!(
        tag,
        Tag::Khoi
            | Tag::Flex
            | Tag::Grid
            | Tag::Input
            | Tag::Textarea
            | Tag::ChonMot
            | Tag::HopKiem
            | Tag::LuaChon
    )
}

fn prop_block_kind(name: &str) -> Option<fn(PropsMap) -> vibao_ast::PropBlock> {
    use vibao_ast::PropBlock;
    match name {
        "mau" | "color" => Some(PropBlock::Mau),
        "kich_thuoc" | "size" => Some(PropBlock::KichThuoc),
        "khoang_cach" | "spacing" => Some(PropBlock::KhoangCach),
        "chu" | "typography" => Some(PropBlock::Chu),
        "flex_bo_cuc" | "flex_layout" => Some(PropBlock::FlexBoCuc),
        "grid_bo_cuc" | "grid_layout" => Some(PropBlock::GridBoCuc),
        "vi_tri" | "position" => Some(PropBlock::ViTri),
        "media" => Some(PropBlock::Media),
        "nhap_lieu" | "input" => Some(PropBlock::NhapLieu),
        "lien_ket" | "link" => Some(PropBlock::LienKet),
        _ => None,
    }
}

fn block_kind_for_prop_key(
    key: PropKey,
    tag: vibao_ast::Tag,
) -> Option<fn(PropsMap) -> vibao_ast::PropBlock> {
    use vibao_ast::{PropBlock, Tag};
    match key {
        PropKey::BackgroundColor | PropKey::Color | PropKey::BorderColor => Some(PropBlock::Mau),

        PropKey::Width
        | PropKey::Height
        | PropKey::MaxWidth
        | PropKey::MinWidth
        | PropKey::MaxHeight
        | PropKey::MinHeight => Some(PropBlock::KichThuoc),

        PropKey::Radius
        | PropKey::Padding
        | PropKey::Margin
        | PropKey::Border
        | PropKey::BorderStyle
        | PropKey::Shadow
        | PropKey::Overflow
        | PropKey::ZIndex => Some(PropBlock::KhoangCach),

        PropKey::FontSize
        | PropKey::Bold
        | PropKey::Italic
        | PropKey::Underline
        | PropKey::PreserveLines
        | PropKey::Align
        | PropKey::LineHeight
        | PropKey::LetterSpacing
        | PropKey::TextTransform
        | PropKey::FontFamily => Some(PropBlock::Chu),

        PropKey::Columns | PropKey::Rows => Some(PropBlock::GridBoCuc),
        PropKey::Direction | PropKey::AlignItems | PropKey::Wrap => Some(PropBlock::FlexBoCuc),
        PropKey::Gap | PropKey::RowGap | PropKey::ColumnGap => {
            if tag == Tag::Grid {
                Some(PropBlock::GridBoCuc)
            } else {
                Some(PropBlock::FlexBoCuc)
            }
        }

        PropKey::TranslateX | PropKey::TranslateY | PropKey::Position | PropKey::Offset => {
            Some(PropBlock::ViTri)
        }

        PropKey::Fit | PropKey::Source | PropKey::Alt | PropKey::LazyLoad => Some(PropBlock::Media),

        PropKey::Type
        | PropKey::Placeholder
        | PropKey::Value
        | PropKey::OptionValue
        | PropKey::Required
        | PropKey::Disabled => Some(PropBlock::NhapLieu),

        PropKey::To | PropKey::Target | PropKey::Rel => Some(PropBlock::LienKet),

        PropKey::HtmlId
        | PropKey::ClassBinding
        | PropKey::Content
        | PropKey::Hidden
        | PropKey::Animation
        | PropKey::Duration
        | PropKey::Delay
        | PropKey::Repeat
        | PropKey::HoverAnimation
        | PropKey::ScrollAnimation => None,
    }
}

const ALLOWED_HTML_TAGS: [&str; 10] = [
    "section", "header", "footer", "main", "article", "aside", "nav", "figure", "ul", "li",
];

fn prop_block_names(block: &vibao_ast::PropBlock) -> (&'static str, &'static str) {
    use vibao_ast::PropBlock;
    match block {
        PropBlock::Mau(_) => ("mau", "color"),
        PropBlock::KichThuoc(_) => ("kich_thuoc", "size"),
        PropBlock::KhoangCach(_) => ("khoang_cach", "spacing"),
        PropBlock::Chu(_) => ("chu", "typography"),
        PropBlock::FlexBoCuc(_) => ("flex_bo_cuc", "flex_layout"),
        PropBlock::GridBoCuc(_) => ("grid_bo_cuc", "grid_layout"),
        PropBlock::ViTri(_) => ("vi_tri", "position"),
        PropBlock::Media(_) => ("media", "media"),
        PropBlock::NhapLieu(_) => ("nhap_lieu", "input"),
        PropBlock::LienKet(_) => ("lien_ket", "link"),
    }
}

fn block_name_in_style(block: &vibao_ast::PropBlock, english: bool) -> &'static str {
    let (vi, en) = prop_block_names(block);
    if english {
        en
    } else {
        vi
    }
}

fn identity_block_name(english: bool) -> &'static str {
    if english {
        "identity"
    } else {
        "dinh_danh"
    }
}

fn is_english_block_name(name: &str) -> bool {
    matches!(
        name,
        "color"
            | "size"
            | "spacing"
            | "typography"
            | "flex_layout"
            | "grid_layout"
            | "position"
            | "input"
            | "link"
            | "identity"
    )
}

fn home_block_name(key: &str, tag: vibao_ast::Tag, english: bool) -> Option<&'static str> {
    let resolved = crate::locale::resolve_prop_key(key)?;
    match resolved {
        PropKey::HtmlId | PropKey::ClassBinding => Some(identity_block_name(english)),
        other => block_kind_for_prop_key(other, tag)
            .map(|make| block_name_in_style(&make(Vec::new()), english)),
    }
}

fn requires_block_syntax_message(tag_name: &str, key: &str, tag: vibao_ast::Tag) -> String {
    let english = crate::locale::prop_en::prop_name_en(key).is_some()
        && crate::locale::prop_vi::prop_name_vi(key).is_none();
    match home_block_name(key, tag, english) {
        Some(block) => format!(
            "Tag '{}' requires block syntax for property '{}': write {} {{ @{} {{ {}: ... }} }} instead of using '(...)'",
            tag_name, key, tag_name, block, key
        ),
        None => format!(
            "Tag '{}' requires block syntax for property '{}': declare it inside an @block {{ ... }} in the tag body instead of using '(...)'",
            tag_name, key
        ),
    }
}

fn misplaced_prop_message(
    key: &str,
    declared_name: &str,
    declared_make: fn(PropsMap) -> vibao_ast::PropBlock,
    tag: vibao_ast::Tag,
) -> Option<String> {
    use vibao_ast::PropBlock;
    let resolved = crate::locale::resolve_prop_key(key)?;
    let english = is_english_block_name(declared_name);
    let declared = declared_make(Vec::new());
    match resolved {
        PropKey::HtmlId | PropKey::ClassBinding => {
            let target = identity_block_name(english);
            Some(format!(
                "Property '{}' belongs to block @{}, not @{}: move it to @{} {{ {}: ... }}",
                key, target, declared_name, target, key
            ))
        }
        PropKey::Gap | PropKey::RowGap | PropKey::ColumnGap
            if matches!(declared, PropBlock::FlexBoCuc(_) | PropBlock::GridBoCuc(_)) =>
        {
            None
        }
        PropKey::HoverAnimation | PropKey::ScrollAnimation => Some(format!(
            "Property '{}' cannot be used inside block @{}: animation properties are written in '(...)' of the tag",
            key, declared_name
        )),
        other => match block_kind_for_prop_key(other, tag) {
            Some(home_make) => {
                let home = home_make(Vec::new());
                if std::mem::discriminant(&home) == std::mem::discriminant(&declared) {
                    None
                } else {
                    let target = block_name_in_style(&home, english);
                    Some(format!(
                        "Property '{}' belongs to block @{}, not @{}: move it to @{} {{ {}: ... }}",
                        key, target, declared_name, target, key
                    ))
                }
            }
            None => Some(format!(
                "Property '{}' cannot be used inside block @{}",
                key, declared_name
            )),
        },
    }
}

fn duplicate_block_message(name: &str) -> String {
    format!(
        "Block '@{}' is already declared on this tag and cannot be repeated: merge it into a single '@{}' block",
        name, name
    )
}

fn push_into_block(
    blocks: &mut Vec<vibao_ast::PropBlock>,
    make_block: fn(PropsMap) -> vibao_ast::PropBlock,
    key: String,
    value: Expr,
) {
    let probe = make_block(Vec::new());
    let existing = blocks
        .iter_mut()
        .find(|b| std::mem::discriminant(*b) == std::mem::discriminant(&probe));
    match existing {
        Some(vibao_ast::PropBlock::Mau(m))
        | Some(vibao_ast::PropBlock::KichThuoc(m))
        | Some(vibao_ast::PropBlock::KhoangCach(m))
        | Some(vibao_ast::PropBlock::Chu(m))
        | Some(vibao_ast::PropBlock::FlexBoCuc(m))
        | Some(vibao_ast::PropBlock::GridBoCuc(m))
        | Some(vibao_ast::PropBlock::ViTri(m))
        | Some(vibao_ast::PropBlock::Media(m))
        | Some(vibao_ast::PropBlock::NhapLieu(m))
        | Some(vibao_ast::PropBlock::LienKet(m)) => m.push((key, value)),
        None => blocks.push(make_block(vec![(key, value)])),
    }
}

impl Parser {
    fn tag_surface_name(&self, tag: vibao_ast::Tag) -> String {
        if let Some(prev) = self.pos.checked_sub(1).and_then(|i| self.tokens.get(i)) {
            if let TokenKind::Component(name) = &prev.kind {
                if crate::locale::resolve_tag(name) == Some(tag) {
                    return name.clone();
                }
            }
        }
        crate::locale::vi::tag_display_name_vi(tag).to_string()
    }

    pub(crate) fn parse_prop_value(&mut self, key: &str) -> Result<vibao_ast::Expr, ParseError> {
        if key == "den" {
            if let TokenKind::ColorName(n) = self.current().kind.clone() {
                let pos = self.current_pos();
                self.advance();
                return Ok(vibao_ast::Expr::Literal(LiteralValue::Str(n), pos));
            }
        }
        self.parse_value()
    }

    fn animation_name_from_expr(&self, expr: &Expr, key: &str) -> Result<String, ParseError> {
        let name = match expr {
            Expr::Literal(LiteralValue::Str(name), _) if !name.trim().is_empty() => name,
            _ => {
                return Err(self.error(format!(
                "The value of prop '{}' must be a static effect-name string, e.g.: \"phong_to\"",
                key
            )))
            }
        };

        let allowed = match crate::locale::resolve_prop_key(key) {
            Some(PropKey::HoverAnimation) => &["phong_to", "lam_sang", "nang_len"][..],
            Some(PropKey::ScrollAnimation) => {
                &["fade_in", "truot_len", "truot_xuong", "phong_to", "rung"][..]
            }
            _ => &[][..],
        };

        if !allowed.contains(&name.as_str()) {
            return Err(self.error(format!(
                "Animation '{}' is not valid for prop '{}'. Supported animations: {}",
                name,
                key,
                allowed.join(", ")
            )));
        }

        Ok(name.clone())
    }

    pub(crate) fn parse_element_rest(
        &mut self,
        tag: vibao_ast::Tag,
        pos: vibao_ast::Pos,
    ) -> Result<Element, ParseError> {
        let mut content: Option<Expr> = None;
        let mut flat_props: PropsMap = Vec::new();
        let mut animation = AnimationProps::default();
        let requires_blocks = tag_requires_blocks(tag);
        let tag_name = self.tag_surface_name(tag);

        if self.match_token(&TokenKind::LParen) {
            while !self.check(&TokenKind::RParen) && !self.is_at_end() {
                if content.is_none()
                    && flat_props.is_empty()
                    && matches!(
                        self.current().kind,
                        TokenKind::StringLit(_)
                            | TokenKind::NumberLit(_, _)
                            | TokenKind::Variable(_)
                    )
                {
                    content = Some(self.parse_value()?);
                } else {
                    let key = self.expect_identifier_like()?;
                    self.consume(&TokenKind::Colon, "Expected ':' after the property name")?;
                    let val = self.parse_prop_value(&key)?;
                    let is_animation_key = matches!(
                        crate::locale::resolve_prop_key(&key),
                        Some(PropKey::HoverAnimation) | Some(PropKey::ScrollAnimation)
                    );
                    if requires_blocks && !is_animation_key {
                        return Err(self.error(requires_block_syntax_message(&tag_name, &key, tag)));
                    }
                    flat_props.push((key, val));
                }
                self.skip_comma();
            }
            self.consume(
                &TokenKind::RParen,
                "Expected ')' to close the property list",
            )?;

            let mut retained = Vec::with_capacity(flat_props.len());
            for (key, value) in flat_props {
                match crate::locale::resolve_prop_key(&key) {
                    Some(PropKey::HoverAnimation) => {
                        animation.hieu_ung_hover =
                            Some(self.animation_name_from_expr(&value, &key)?);
                    }
                    Some(PropKey::ScrollAnimation) => {
                        animation.hieu_ung_cuon =
                            Some(self.animation_name_from_expr(&value, &key)?);
                    }
                    _ => retained.push((key, value)),
                }
            }
            flat_props = retained;
        }

        let mut identity = vibao_ast::IdentityProps::default();
        let mut identity_declared = false;
        let mut blocks: Vec<vibao_ast::PropBlock> = Vec::new();
        let mut unknown_props: PropsMap = Vec::new();
        let mut explicit_blocks_seen: std::collections::HashSet<String> =
            std::collections::HashSet::new();
        for (key, value) in flat_props {
            match crate::locale::resolve_prop_key(&key) {
                Some(PropKey::HtmlId) => identity.id = Some(value),
                Some(PropKey::ClassBinding) => identity.class_binding = Some(value),
                Some(resolved_key) => {
                    if let Some(make_block) = block_kind_for_prop_key(resolved_key, tag) {
                        push_into_block(&mut blocks, make_block, key, value);
                    } else {
                        unknown_props.push((key, value));
                    }
                }
                None => unknown_props.push((key, value)),
            }
        }

        let mut children = Vec::new();
        let mut events = Vec::new();
        let mut responsive = Vec::new();

        if self.match_token(&TokenKind::LBrace) {
            while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
                if let Some(event_name) = self.match_event_name() {
                    let e_pos = self.current_pos();
                    self.consume(
                        &TokenKind::LBrace,
                        "Expected '{' to start the event action body",
                    )?;
                    let mut body = Vec::new();
                    while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
                        body.push(self.parse_action()?);
                    }
                    self.consume(&TokenKind::RBrace, "Expected '}' to close the event")?;
                    events.push(EventNode {
                        name: event_name,
                        body,
                        pos: e_pos,
                    });
                } else if self.check(&TokenKind::At) {
                    self.advance();
                    let r_pos = self.current_pos();
                    let name = self.expect_identifier_like()?;
                    match name.as_str() {
                        "di_dong" => {
                            let overrides = self.parse_named_props_block()?;
                            responsive.push(ResponsiveNode {
                                breakpoint: Breakpoint::DiDong,
                                overrides,
                                pos: r_pos,
                            });
                        }
                        "may_tinh_bang" => {
                            let overrides = self.parse_named_props_block()?;
                            responsive.push(ResponsiveNode {
                                breakpoint: Breakpoint::MayTinhBang,
                                overrides,
                                pos: r_pos,
                            });
                        }
                        "may_tinh" => {
                            let overrides = self.parse_named_props_block()?;
                            responsive.push(ResponsiveNode {
                                breakpoint: Breakpoint::MayTinh,
                                overrides,
                                pos: r_pos,
                            });
                        }
                        "hieu_ung" => {
                            self.consume(
                                &TokenKind::Colon,
                                "Expected ':' after the @hieu_ung directive",
                            )?;
                            match &self.current().kind {
                                TokenKind::Identifier(act) => {
                                    animation.hieu_ung = Some(act.clone());
                                    self.advance();
                                }
                                other => {
                                    return Err(self.error(format!(
                                        "Expected an animation identifier after '@hieu_ung:', received {}",
                                        other
                                    )))
                                }
                            }
                        }
                        "dinh_danh" | "identity" => {
                            if identity_declared {
                                return Err(self.error(duplicate_block_message(&name)));
                            }
                            identity_declared = true;
                            let props = self.parse_named_props_block()?;
                            self.fill_identity_props(&mut identity, props, &name, true)?;
                        }
                        other_name => {
                            if let Some(make_block) = prop_block_kind(other_name) {
                                let block_key = prop_block_names(&make_block(Vec::new())).0;
                                if !explicit_blocks_seen.insert(block_key.to_string()) {
                                    return Err(self.error(duplicate_block_message(other_name)));
                                }
                                let props_pos = self.current_pos();
                                let props = self.parse_named_props_block()?;
                                for (k, _) in &props {
                                    if let Some(message) =
                                        misplaced_prop_message(k, other_name, make_block, tag)
                                    {
                                        return Err(ParseError {
                                            message,
                                            line: props_pos.line,
                                            column: props_pos.column,
                                        });
                                    }
                                }
                                for (k, v) in props {
                                    push_into_block(&mut blocks, make_block, k, v);
                                }
                            } else {
                                return Err(self.error(format!(
                                    "Directive '@{}' is not supported",
                                    other_name
                                )));
                            }
                        }
                    }
                } else {
                    children.push(self.parse_child()?);
                }
            }
            self.consume(
                &TokenKind::RBrace,
                "Expected '}' to close the component body",
            )?;
        }

        Ok(Element {
            tag,
            content,
            identity,
            blocks,
            unknown_props,
            children,
            events,
            responsive,
            animation,
            pos,
        })
    }

    fn fill_identity_props(
        &self,
        identity: &mut vibao_ast::IdentityProps,
        props: PropsMap,
        block_name: &str,
        allow_html_tag: bool,
    ) -> Result<(), ParseError> {
        for (key, value) in props {
            match key.as_str() {
                "id" => identity.id = Some(value),
                "lop" | "class" => identity.class_binding = Some(value),
                "the_html" | "html_tag" if allow_html_tag => {
                    let tag = match &value {
                        Expr::Literal(vibao_ast::LiteralValue::Str(s), _) => s.clone(),
                        _ => {
                            return Err(self.error(format!(
                                "'{}' must be a static string, for example \"section\"",
                                key
                            )))
                        }
                    };
                    if !ALLOWED_HTML_TAGS.contains(&tag.as_str()) {
                        return Err(self.error(format!(
                            "'{}' does not accept \"{}\": use one of {}",
                            key,
                            tag,
                            ALLOWED_HTML_TAGS.join(", ")
                        )));
                    }
                    identity.html_tag = Some(tag);
                }
                other => {
                    let accepted = if allow_html_tag {
                        "only 'id', 'lop' and 'the_html' are accepted"
                    } else {
                        "only 'id' and 'lop' are accepted"
                    };
                    return Err(self.error(format!(
                        "'{}' is not valid in block @{}: {}",
                        other, block_name, accepted
                    )));
                }
            }
        }
        Ok(())
    }

    pub(crate) fn parse_component_call_rest(
        &mut self,
        name: String,
        pos: vibao_ast::Pos,
    ) -> Result<ComponentCall, ParseError> {
        let mut props = Vec::new();
        if self.match_token(&TokenKind::LParen) {
            while !self.check(&TokenKind::RParen) && !self.is_at_end() {
                let key = self.expect_identifier_like()?;
                self.consume(
                    &TokenKind::Colon,
                    "Expected ':' after the component property name",
                )?;
                let val = self.parse_prop_value(&key)?;
                props.push((key, val));
                self.skip_comma();
            }
            self.consume(
                &TokenKind::RParen,
                "Expected ')' to close the component call arguments",
            )?;
        }

        let mut identity = vibao_ast::IdentityProps::default();
        let mut identity_declared = false;
        let mut children = Vec::new();
        if self.match_token(&TokenKind::LBrace) {
            while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
                if self.check(&TokenKind::At) {
                    self.advance();
                    let directive = self.expect_identifier_like()?;
                    if directive == "dinh_danh" || directive == "identity" {
                        if identity_declared {
                            return Err(self.error(duplicate_block_message(&directive)));
                        }
                        identity_declared = true;
                        let id_props = self.parse_named_props_block()?;
                        self.fill_identity_props(&mut identity, id_props, &directive, false)?;
                    } else if prop_block_kind(&directive).is_some() {
                        return Err(self.error(format!(
                            "'@{}' is a semantic block of Element and does not apply to component calls: component parameters are free-named, write them directly in the '(...)' of {}(...)",
                            directive, name
                        )));
                    } else {
                        return Err(self.error(format!(
                            "Directive '@{}' is not supported on a component call",
                            directive
                        )));
                    }
                } else {
                    children.push(self.parse_child()?);
                }
            }
            self.consume(
                &TokenKind::RBrace,
                "Expected '}' to close the component call body",
            )?;
        }

        Ok(ComponentCall {
            name,
            props,
            identity,
            children,
            pos,
        })
    }

    fn match_event_name(&mut self) -> Option<EventName> {
        let name = match &self.current().kind {
            TokenKind::OnClick => Some(EventName::OnClick),
            TokenKind::OnHover => Some(EventName::OnHover),
            TokenKind::OnBlur => Some(EventName::OnBlur),
            TokenKind::OnFocus => Some(EventName::OnFocus),
            TokenKind::OnChange => Some(EventName::OnChange),
            TokenKind::OnSubmit => Some(EventName::OnSubmit),
            TokenKind::OnScroll => Some(EventName::OnScroll),
            _ => None,
        };
        if name.is_some() {
            self.advance();
        }
        name
    }

    pub(crate) fn parse_named_props_block(&mut self) -> Result<PropsMap, ParseError> {
        self.consume(&TokenKind::LBrace, "Expected '{' after the breakpoint name")?;
        let mut overrides = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            let key = self.expect_identifier_like()?;
            self.consume(
                &TokenKind::Colon,
                "Expected ':' after the responsive override property",
            )?;
            let val = self.parse_prop_value(&key)?;
            overrides.push((key, val));
            self.skip_comma();
        }
        self.consume(
            &TokenKind::RBrace,
            "Expected '}' to close the responsive block",
        )?;
        Ok(overrides)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::tokenize;
    use vibao_ast::{Expr, Pos};

    fn p() -> Pos {
        Pos { line: 1, column: 1 }
    }

    fn find_in_blocks<'a>(blocks: &'a [vibao_ast::PropBlock], key: &str) -> Option<&'a Expr> {
        use vibao_ast::PropBlock::*;
        for block in blocks {
            let map: &PropsMap = match block {
                Mau(m) | KichThuoc(m) | KhoangCach(m) | Chu(m) | FlexBoCuc(m) | GridBoCuc(m)
                | ViTri(m) | Media(m) | NhapLieu(m) | LienKet(m) => m,
            };
            if let Some((_, v)) = map.iter().find(|(k, _)| k == key) {
                return Some(v);
            }
        }
        None
    }

    #[test]
    fn test_hover_animation_prop_is_extracted_from_props() {
        let tokens = tokenize(r#"(hieu_ung_hover: "phong_to")"#).unwrap();
        let mut parser = Parser::new(tokens);
        let el = parser
            .parse_element_rest(vibao_ast::Tag::Button, p())
            .unwrap();

        assert_eq!(el.animation.hieu_ung_hover.as_deref(), Some("phong_to"));
        assert!(find_in_blocks(&el.blocks, "hieu_ung_hover").is_none());
    }

    #[test]
    fn test_scroll_animation_english_prop_is_extracted() {
        let tokens = tokenize(r#"(scroll_animation: "truot_len")"#).unwrap();
        let mut parser = Parser::new(tokens);
        let el = parser
            .parse_element_rest(vibao_ast::Tag::Button, p())
            .unwrap();

        assert_eq!(el.animation.hieu_ung_cuon.as_deref(), Some("truot_len"));
        assert!(find_in_blocks(&el.blocks, "scroll_animation").is_none());
    }

    #[test]
    fn test_animation_prop_must_be_static_string() {
        let tokens = tokenize(r#"(hieu_ung_hover: $anim)"#).unwrap();
        let mut parser = Parser::new(tokens);
        let err = parser
            .parse_element_rest(vibao_ast::Tag::Button, p())
            .unwrap_err();

        assert!(err.message.contains("hieu_ung_hover"));
        assert!(err.message.contains("static effect-name string"));
    }

    #[test]
    fn test_hover_animation_rejects_unsupported_css_name() {
        let tokens = tokenize(r#"(hieu_ung_hover: "rung")"#).unwrap();
        let mut parser = Parser::new(tokens);
        let err = parser
            .parse_element_rest(vibao_ast::Tag::Button, p())
            .unwrap_err();

        assert!(err.message.contains("rung"));
        assert!(err.message.contains("hieu_ung_hover"));
        assert!(err.message.contains("phong_to"));
        assert!(err.message.contains("lam_sang"));
    }

    #[test]
    fn test_scroll_animation_accepts_all_builtin_animation_names() {
        for name in ["fade_in", "truot_len", "truot_xuong", "phong_to", "rung"] {
            let source = format!(r#"(hieu_ung_cuon: "{}")"#, name);
            let tokens = tokenize(&source).unwrap();
            let mut parser = Parser::new(tokens);
            let el = parser
                .parse_element_rest(vibao_ast::Tag::Button, p())
                .unwrap();
            assert_eq!(el.animation.hieu_ung_cuon.as_deref(), Some(name));
        }
    }

    #[test]
    fn test_hover_animation_english_prop_uses_same_allowed_names() {
        let tokens = tokenize(r#"(hover_animation: "lam_sang")"#).unwrap();
        let mut parser = Parser::new(tokens);
        let el = parser
            .parse_element_rest(vibao_ast::Tag::Button, p())
            .unwrap();

        assert_eq!(el.animation.hieu_ung_hover.as_deref(), Some("lam_sang"));
    }

    #[test]
    fn test_animation_props_do_not_leak_into_normal_props() {
        let tokens = tokenize(r#"(hieu_ung_hover: "lam_sang", mau: do)"#).unwrap();
        let mut parser = Parser::new(tokens);
        let el = parser
            .parse_element_rest(vibao_ast::Tag::Button, p())
            .unwrap();

        assert_eq!(el.animation.hieu_ung_hover.as_deref(), Some("lam_sang"));
        assert!(
            matches!(find_in_blocks(&el.blocks, "mau"), Some(Expr::Literal(LiteralValue::Color(c), _)) if c == "#E53E3E"),
            "prop mau: do (đỏ, #E53E3E) phải còn nguyên trong blocks sau khi tách animation"
        );
        assert!(find_in_blocks(&el.blocks, "hieu_ung_hover").is_none());
    }

    #[test]
    fn test_den_prop_value_not_confused_with_color_den() {
        let tokens = tokenize(r#"("Trang chủ", den: den)"#).unwrap();
        let mut parser = Parser::new(tokens);
        let el = parser
            .parse_element_rest(vibao_ast::Tag::Link, p())
            .unwrap();

        let val = find_in_blocks(&el.blocks, "den").expect("missing prop den");
        assert!(
            matches!(val, Expr::Literal(LiteralValue::Str(s), _) if s == "den"),
            "prop den must be Str(\"den\"), got {:?}",
            val
        );
    }

    #[test]
    fn test_den_prop_value_normal_string_still_works() {
        let tokens = tokenize(r#"(den: "/gioi-thieu")"#).unwrap();
        let mut parser = Parser::new(tokens);
        let el = parser
            .parse_element_rest(vibao_ast::Tag::Link, p())
            .unwrap();

        let val = find_in_blocks(&el.blocks, "den").expect("missing prop den");
        assert!(matches!(val, Expr::Literal(LiteralValue::Str(s), _) if s == "/gioi-thieu"));
    }

    #[test]
    fn test_other_props_still_resolve_bare_color_identifier_as_color() {
        let tokens = tokenize(r#"(mau: den)"#).unwrap();
        let mut parser = Parser::new(tokens);
        let el = parser
            .parse_element_rest(vibao_ast::Tag::Text, p())
            .unwrap();

        let val = find_in_blocks(&el.blocks, "mau").expect("missing prop mau");
        assert!(
            matches!(val, Expr::Literal(LiteralValue::Color(c), _) if c == "#000000"),
            "prop mau: den must still resolve to the color black, got {:?}",
            val
        );
    }

    #[test]
    fn test_den_prop_on_component_call_not_confused_with_color() {
        let tokens = tokenize(r#"(den: den)"#).unwrap();
        let mut parser = Parser::new(tokens);
        let call = parser
            .parse_component_call_rest("LinkTuyChinh".to_string(), p())
            .unwrap();

        let (_, val) = call
            .props
            .iter()
            .find(|(k, _)| k == "den")
            .expect("missing prop den");
        assert!(matches!(val, Expr::Literal(LiteralValue::Str(s), _) if s == "den"));
    }

    #[test]
    fn test_positional_param_accepts_bare_variable() {
        let tokens = tokenize(r#"($nhan, co: 12)"#).unwrap();
        let mut parser = Parser::new(tokens);
        let el = parser
            .parse_element_rest(vibao_ast::Tag::Text, p())
            .unwrap();

        let val = el.content.as_ref().expect("missing content");
        assert!(
            matches!(val, Expr::Variable(n, _) if n == "nhan"),
            "content must be Variable(\"nhan\"), got {:?}",
            val
        );
        assert!(
            find_in_blocks(&el.blocks, "co").is_some(),
            "prop 'co' must still parse correctly after the positional param"
        );
    }

    #[test]
    fn test_positional_param_accepts_member_access() {
        let tokens = tokenize(r#"($item.ten)"#).unwrap();
        let mut parser = Parser::new(tokens);
        let el = parser
            .parse_element_rest(vibao_ast::Tag::Text, p())
            .unwrap();

        let val = el.content.as_ref().expect("missing content");
        assert!(
            matches!(val, Expr::MemberAccess { property, .. } if property == "ten"),
            "content must be MemberAccess with property \"ten\", got {:?}",
            val
        );
    }

    #[test]
    fn test_positional_param_string_and_number_still_work() {
        let tokens = tokenize(r#"("Xin chào")"#).unwrap();
        let mut parser = Parser::new(tokens);
        let el = parser
            .parse_element_rest(vibao_ast::Tag::Text, p())
            .unwrap();
        let val = el.content.as_ref().unwrap();
        assert!(matches!(val, Expr::Literal(LiteralValue::Str(s), _) if s == "Xin chào"));
    }

    #[test]
    fn test_named_prop_after_variable_shorthand_not_confused() {
        let tokens = tokenize(r#"(mau: xanh, co: 16)"#).unwrap();
        let mut parser = Parser::new(tokens);
        let el = parser
            .parse_element_rest(vibao_ast::Tag::Text, p())
            .unwrap();
        assert!(
            el.content.is_none(),
            "must not auto-add content when there is no positional param"
        );
        assert_eq!(
            find_in_blocks(&el.blocks, "mau").is_some() as u8
                + find_in_blocks(&el.blocks, "co").is_some() as u8,
            2,
            "both mau and co must have parsed into blocks"
        );
    }

    #[test]
    fn test_den_regression_fix_applies_inside_explicit_block() {
        let tokens = tokenize(r#"{ @lien_ket { den: den } }"#).unwrap();
        let mut parser = Parser::new(tokens);
        let el = parser
            .parse_element_rest(vibao_ast::Tag::Link, p())
            .unwrap();
        let val = find_in_blocks(&el.blocks, "den").expect("missing prop den");
        assert!(
            matches!(val, Expr::Literal(LiteralValue::Str(s), _) if s == "den"),
            "prop den trong khối @lien_ket phải là Str(\"den\"), got {:?}",
            val
        );
    }

    #[test]
    fn test_component_call_rejects_semantic_block_with_clear_message() {
        let tokens = tokenize(r#"{ @mau { nen: trang } }"#).unwrap();
        let mut parser = Parser::new(tokens);
        let err = parser
            .parse_component_call_rest("The_Card".to_string(), p())
            .unwrap_err();
        assert!(
            err.message.contains("component parameters are free-named"),
            "message phải giải thích lý do, got: {}",
            err.message
        );
    }

    #[test]
    fn test_unknown_prop_does_not_vanish_from_ast() {
        let tokens = tokenize(r#"("Xin chào", ten_prop_khong_ton_tai: 1)"#).unwrap();
        let mut parser = Parser::new(tokens);
        let el = parser
            .parse_element_rest(vibao_ast::Tag::Text, p())
            .unwrap();
        assert_eq!(
            el.unknown_props.len(),
            1,
            "prop lạ phải nằm trong unknown_props, không biến mất"
        );
        assert_eq!(el.unknown_props[0].0, "ten_prop_khong_ton_tai");
    }

    #[test]
    fn test_dinh_danh_block_cannot_be_declared_twice() {
        let tokens = tokenize(r#"{ @dinh_danh { id: "a" } @dinh_danh { id: "b" } }"#).unwrap();
        let mut parser = Parser::new(tokens);
        let err = parser
            .parse_element_rest(vibao_ast::Tag::Khoi, p())
            .unwrap_err();
        assert!(
            err.message.contains("dinh_danh") && err.message.contains("already declared"),
            "phải báo lỗi khai báo trùng @dinh_danh, got: {}",
            err.message
        );
    }

    #[test]
    fn test_dinh_danh_block_duplicate_also_rejected_on_component_call() {
        let tokens = tokenize(r#"{ @dinh_danh { id: "a" } @dinh_danh { id: "b" } }"#).unwrap();
        let mut parser = Parser::new(tokens);
        let err = parser
            .parse_component_call_rest("The_Card".to_string(), p())
            .unwrap_err();
        assert!(
            err.message.contains("dinh_danh") && err.message.contains("already declared"),
            "phải báo lỗi khai báo trùng @dinh_danh trên component call, got: {}",
            err.message
        );
    }

    #[test]
    fn test_gap_on_flex_tag_goes_to_flex_bo_cuc_block() {
        let tokens = tokenize(r#"{ @flex_bo_cuc { gap: 10 } }"#).unwrap();
        let mut parser = Parser::new(tokens);
        let el = parser
            .parse_element_rest(vibao_ast::Tag::Flex, p())
            .unwrap();
        assert!(
            el.blocks.iter().any(|b| matches!(b, vibao_ast::PropBlock::FlexBoCuc(m) if m.iter().any(|(k, _)| k == "gap"))),
            "gap trên tag flex phải rơi vào FlexBoCuc, blocks thực tế: {:?}",
            el.blocks
        );
        assert!(
            !el.blocks
                .iter()
                .any(|b| matches!(b, vibao_ast::PropBlock::GridBoCuc(_))),
            "không được có GridBoCuc nào sinh ra cho tag flex"
        );
    }

    #[test]
    fn test_gap_on_grid_tag_goes_to_grid_bo_cuc_block() {
        let tokens = tokenize(r#"{ @grid_bo_cuc { gap: 10 } }"#).unwrap();
        let mut parser = Parser::new(tokens);
        let el = parser
            .parse_element_rest(vibao_ast::Tag::Grid, p())
            .unwrap();
        assert!(
            el.blocks.iter().any(|b| matches!(b, vibao_ast::PropBlock::GridBoCuc(m) if m.iter().any(|(k, _)| k == "gap"))),
            "gap trên tag grid phải rơi vào GridBoCuc, blocks thực tế: {:?}",
            el.blocks
        );
        assert!(
            !el.blocks
                .iter()
                .any(|b| matches!(b, vibao_ast::PropBlock::FlexBoCuc(_))),
            "không được có FlexBoCuc nào sinh ra cho tag grid"
        );
    }

    #[test]
    fn test_columns_always_goes_to_grid_bo_cuc_regardless_of_tag() {
        let tokens = tokenize(r#"{ @grid_bo_cuc { cot: "1fr 1fr" } }"#).unwrap();
        let mut parser = Parser::new(tokens);
        let el = parser
            .parse_element_rest(vibao_ast::Tag::Grid, p())
            .unwrap();
        assert!(
            el.blocks.iter().any(|b| matches!(b, vibao_ast::PropBlock::GridBoCuc(m) if m.iter().any(|(k, _)| k == "cot"))),
            "cot (Columns) phải luôn vào GridBoCuc, blocks thực tế: {:?}",
            el.blocks
        );
    }

    #[test]
    fn test_direction_always_goes_to_flex_bo_cuc_regardless_of_tag() {
        let tokens = tokenize(r#"{ @flex_bo_cuc { huong: "doc" } }"#).unwrap();
        let mut parser = Parser::new(tokens);
        let el = parser
            .parse_element_rest(vibao_ast::Tag::Flex, p())
            .unwrap();
        assert!(
            el.blocks.iter().any(|b| matches!(b, vibao_ast::PropBlock::FlexBoCuc(m) if m.iter().any(|(k, _)| k == "huong"))),
            "huong (Direction) phải luôn vào FlexBoCuc, blocks thực tế: {:?}",
            el.blocks
        );
    }

    #[test]
    fn test_gap_flat_syntax_defaults_to_flex_bo_cuc_on_exempt_tag() {
        let tokens = tokenize(r#"(gap: 10)"#).unwrap();
        let mut parser = Parser::new(tokens);
        let el = parser
            .parse_element_rest(vibao_ast::Tag::Stack, p())
            .unwrap();
        assert!(
            el.blocks.iter().any(|b| matches!(b, vibao_ast::PropBlock::FlexBoCuc(m) if m.iter().any(|(k, _)| k == "gap"))),
            "gap trên tag không phải flex/grid phải mặc định FlexBoCuc, blocks thực tế: {:?}",
            el.blocks
        );
    }

    #[test]
    fn test_khoi_rejects_flat_named_args_syntax() {
        let tokens = tokenize(r#"(dem: 20)"#).unwrap();
        let mut parser = Parser::new(tokens);
        let err = parser
            .parse_element_rest(vibao_ast::Tag::Khoi, p())
            .unwrap_err();
        assert!(
            err.message.contains("requires block syntax") && err.message.contains("@khoang_cach"),
            "lỗi phải gợi ý đúng khối @khoang_cach, got: {}",
            err.message
        );
    }

    #[test]
    fn test_flex_rejects_flat_named_args_syntax() {
        let tokens = tokenize(r#"(huong: "doc")"#).unwrap();
        let mut parser = Parser::new(tokens);
        assert!(parser
            .parse_element_rest(vibao_ast::Tag::Flex, p())
            .is_err());
    }

    #[test]
    fn test_input_rejects_flat_named_args_syntax() {
        let tokens = tokenize(r#"(chu_tro: "Nhập tên")"#).unwrap();
        let mut parser = Parser::new(tokens);
        assert!(parser
            .parse_element_rest(vibao_ast::Tag::Input, p())
            .is_err());
    }

    #[test]
    fn test_khoi_accepts_explicit_block_syntax() {
        let tokens = tokenize(r#"{ @khoang_cach { dem: 20 } }"#).unwrap();
        let mut parser = Parser::new(tokens);
        let el = parser
            .parse_element_rest(vibao_ast::Tag::Khoi, p())
            .unwrap();
        assert!(
            el.blocks.iter().any(|b| matches!(b, vibao_ast::PropBlock::KhoangCach(m) if m.iter().any(|(k, _)| k == "dem"))),
            "blocks thực tế: {:?}",
            el.blocks
        );
    }

    #[test]
    fn test_exempt_tag_simple_and_block_syntax_produce_same_result() {
        let simple_tokens = tokenize(r#"("Xin chào", co: 16)"#).unwrap();
        let mut p1 = Parser::new(simple_tokens);
        let el_simple = p1.parse_element_rest(vibao_ast::Tag::Text, p()).unwrap();

        let block_tokens = tokenize(r#"("Xin chào") { @chu { co: 16 } }"#).unwrap();
        let mut p2 = Parser::new(block_tokens);
        let el_block = p2.parse_element_rest(vibao_ast::Tag::Text, p()).unwrap();

        let get_co = |el: &vibao_ast::Element| -> bool {
            el.blocks.iter().any(
                |b| matches!(b, vibao_ast::PropBlock::Chu(m) if m.iter().any(|(k, _)| k == "co")),
            )
        };
        assert!(
            get_co(&el_simple),
            "cách viết đơn giản: blocks = {:?}",
            el_simple.blocks
        );
        assert!(
            get_co(&el_block),
            "cách viết khối: blocks = {:?}",
            el_block.blocks
        );
    }

    #[test]
    fn test_english_block_names_resolve_same_as_vietnamese() {
        let tokens = tokenize(r#"{ @spacing { padding: 16 } }"#).unwrap();
        let mut parser = Parser::new(tokens);
        let el = parser
            .parse_element_rest(vibao_ast::Tag::Khoi, p())
            .unwrap();
        assert!(
            el.blocks.iter().any(|b| matches!(b, vibao_ast::PropBlock::KhoangCach(m) if m.iter().any(|(k, _)| k == "padding"))),
            "blocks: {:?}", el.blocks
        );
    }

    #[test]
    fn test_english_identity_block_name() {
        let tokens = tokenize(r#"{ @identity { id: "x" } }"#).unwrap();
        let mut parser = Parser::new(tokens);
        let el = parser
            .parse_element_rest(vibao_ast::Tag::Khoi, p())
            .unwrap();
        assert!(el.identity.id.is_some());
    }

    #[test]
    fn test_animation_key_in_parens_allowed_on_tag_that_requires_blocks() {
        let tokens = tokenize(r#"(hieu_ung_cuon: "fade_in")"#).unwrap();
        let mut parser = Parser::new(tokens);
        let el = parser
            .parse_element_rest(vibao_ast::Tag::Khoi, p())
            .unwrap();
        assert_eq!(el.animation.hieu_ung_cuon.as_deref(), Some("fade_in"));
    }

    #[test]
    fn test_hover_animation_nang_len_is_accepted() {
        let tokens = tokenize(r#"(hieu_ung_hover: "nang_len")"#).unwrap();
        let mut parser = Parser::new(tokens);
        let el = parser
            .parse_element_rest(vibao_ast::Tag::Button, p())
            .unwrap();
        assert_eq!(el.animation.hieu_ung_hover.as_deref(), Some("nang_len"));
    }

    #[test]
    fn test_non_animation_key_still_rejected_alongside_animation_key() {
        let tokens = tokenize(r#"(hieu_ung_cuon: "fade_in", dem: 10)"#).unwrap();
        let mut parser = Parser::new(tokens);
        assert!(parser
            .parse_element_rest(vibao_ast::Tag::Khoi, p())
            .is_err());
    }

    fn parse_err(tag: vibao_ast::Tag, source: &str) -> String {
        let tokens = tokenize(source).unwrap();
        let mut parser = Parser::new(tokens);
        parser.parse_element_rest(tag, p()).unwrap_err().message
    }

    #[test]
    fn test_requires_block_error_names_the_target_block() {
        let msg = parse_err(vibao_ast::Tag::Khoi, "(dem: 20)");
        assert_eq!(
            msg,
            "Tag 'khoi' requires block syntax for property 'dem': write khoi { @khoang_cach { dem: ... } } instead of using '(...)'"
        );
    }

    #[test]
    fn test_requires_block_error_uses_english_block_name_for_english_prop() {
        let msg = parse_err(vibao_ast::Tag::Khoi, "(padding: 20)");
        assert!(msg.contains("@spacing { padding: ... }"), "got: {}", msg);
    }

    #[test]
    fn test_requires_block_error_points_to_identity_block_for_id() {
        let msg = parse_err(vibao_ast::Tag::Khoi, r#"(id: "a")"#);
        assert!(msg.contains("@dinh_danh { id: ... }"), "got: {}", msg);
    }

    #[test]
    fn test_requires_block_error_for_gap_follows_the_tag() {
        let flex = parse_err(vibao_ast::Tag::Flex, "(gap: 8)");
        assert!(flex.contains("@flex_bo_cuc { gap: ... }"), "got: {}", flex);
        let grid = parse_err(vibao_ast::Tag::Grid, "(gap: 8)");
        assert!(grid.contains("@grid_bo_cuc { gap: ... }"), "got: {}", grid);
    }

    #[test]
    fn test_requires_block_error_for_unknown_prop_has_generic_hint() {
        let msg = parse_err(vibao_ast::Tag::Khoi, "(ten_khong_ton_tai: 1)");
        assert!(msg.contains("declare it inside an @block"), "got: {}", msg);
    }

    #[test]
    fn test_requires_block_error_uses_tag_name_as_written() {
        let tokens = tokenize("box(padding: 16)").unwrap();
        let mut parser = Parser::new(tokens);
        parser.advance();
        let err = parser
            .parse_element_rest(vibao_ast::Tag::Khoi, p())
            .unwrap_err();
        assert!(
            err.message.starts_with("Tag 'box' requires"),
            "got: {}",
            err.message
        );
    }

    #[test]
    fn test_prop_in_wrong_block_is_rejected_with_target_block() {
        let msg = parse_err(vibao_ast::Tag::Khoi, "{ @mau { co: 16 } }");
        assert_eq!(
            msg,
            "Property 'co' belongs to block @chu, not @mau: move it to @chu { co: ... }"
        );
    }

    #[test]
    fn test_prop_in_wrong_block_uses_english_names_when_block_is_english() {
        let msg = parse_err(vibao_ast::Tag::Khoi, "{ @color { font_size: 16 } }");
        assert!(
            msg.contains("belongs to block @typography, not @color"),
            "got: {}",
            msg
        );
    }

    #[test]
    fn test_align_and_align_items_live_in_different_blocks() {
        let msg = parse_err(vibao_ast::Tag::Flex, r#"{ @flex_bo_cuc { can: "giua" } }"#);
        assert!(
            msg.contains("belongs to block @chu, not @flex_bo_cuc"),
            "got: {}",
            msg
        );
        let msg = parse_err(vibao_ast::Tag::Flex, r#"{ @chu { doc: "giua" } }"#);
        assert!(
            msg.contains("belongs to block @flex_bo_cuc, not @chu"),
            "got: {}",
            msg
        );
    }

    #[test]
    fn test_identity_props_inside_semantic_block_point_to_identity_block() {
        let msg = parse_err(vibao_ast::Tag::Khoi, r#"{ @mau { id: "a" } }"#);
        assert!(
            msg.contains("belongs to block @dinh_danh, not @mau"),
            "got: {}",
            msg
        );
    }

    #[test]
    fn test_prop_without_block_cannot_be_used_inside_any_block() {
        let msg = parse_err(vibao_ast::Tag::Khoi, "{ @mau { an: true } }");
        assert_eq!(msg, "Property 'an' cannot be used inside block @mau");
    }

    #[test]
    fn test_animation_prop_inside_block_points_back_to_parens() {
        let msg = parse_err(
            vibao_ast::Tag::Khoi,
            r#"{ @mau { hieu_ung_cuon: "fade_in" } }"#,
        );
        assert!(
            msg.contains("animation properties are written in '(...)'"),
            "got: {}",
            msg
        );
    }

    #[test]
    fn test_gap_is_accepted_in_both_layout_blocks_on_any_tag() {
        for source in [
            "{ @flex_bo_cuc { gap: 10 } }",
            "{ @grid_bo_cuc { gap: 10 } }",
        ] {
            for tag in [
                vibao_ast::Tag::Flex,
                vibao_ast::Tag::Grid,
                vibao_ast::Tag::Khoi,
            ] {
                let tokens = tokenize(source).unwrap();
                let mut parser = Parser::new(tokens);
                assert!(
                    parser.parse_element_rest(tag, p()).is_ok(),
                    "gap trong {} trên {:?} phải hợp lệ",
                    source,
                    tag
                );
            }
        }
    }

    #[test]
    fn test_unknown_prop_inside_block_is_not_a_parse_error() {
        let tokens = tokenize("{ @mau { ten_khong_ton_tai: 1 } }").unwrap();
        let mut parser = Parser::new(tokens);
        let el = parser
            .parse_element_rest(vibao_ast::Tag::Khoi, p())
            .unwrap();
        assert!(find_in_blocks(&el.blocks, "ten_khong_ton_tai").is_some());
    }

    #[test]
    fn test_same_block_in_vietnamese_and_english_counts_as_duplicate() {
        let msg = parse_err(
            vibao_ast::Tag::Khoi,
            "{ @mau { mau_nen: trang } @color { mau_vien: den } }",
        );
        assert_eq!(
            msg,
            "Block '@color' is already declared on this tag and cannot be repeated: merge it into a single '@color' block"
        );
    }

    #[test]
    fn test_identity_block_rejects_unknown_key_with_block_name() {
        let msg = parse_err(vibao_ast::Tag::Khoi, "{ @identity { dem: 1 } }");
        assert_eq!(
            msg,
            "'dem' is not valid in block @identity: only 'id', 'lop' and 'the_html' are accepted"
        );
    }

    #[test]
    fn test_identity_block_accepts_semantic_html_tag() {
        for key in ["the_html", "html_tag"] {
            let source = format!("{{ @dinh_danh {{ {}: \"section\" }} }}", key);
            let tokens = tokenize(&source).unwrap();
            let mut parser = Parser::new(tokens);
            let el = parser
                .parse_element_rest(vibao_ast::Tag::Khoi, p())
                .unwrap();
            assert_eq!(el.identity.html_tag.as_deref(), Some("section"), "{}", key);
        }
    }

    #[test]
    fn test_identity_html_tag_must_be_in_the_allowed_list() {
        let msg = parse_err(
            vibao_ast::Tag::Khoi,
            r#"{ @dinh_danh { the_html: "script" } }"#,
        );
        assert!(
            msg.starts_with("'the_html' does not accept \"script\": use one of section,"),
            "got: {}",
            msg
        );
    }

    #[test]
    fn test_identity_html_tag_must_be_a_static_string() {
        let msg = parse_err(vibao_ast::Tag::Khoi, "{ @dinh_danh { the_html: $x } }");
        assert!(msg.contains("must be a static string"), "got: {}", msg);
    }

    #[test]
    fn test_component_call_identity_does_not_accept_html_tag() {
        let tokens = tokenize(r#"{ @dinh_danh { the_html: "section" } }"#).unwrap();
        let mut parser = Parser::new(tokens);
        let err = parser
            .parse_component_call_rest("The_Card".to_string(), p())
            .unwrap_err();
        assert_eq!(
            err.message,
            "'the_html' is not valid in block @dinh_danh: only 'id' and 'lop' are accepted"
        );
    }

    #[test]
    fn test_component_call_rejects_semantic_block_naming_the_component() {
        let tokens = tokenize("{ @mau { mau_nen: trang } }").unwrap();
        let mut parser = Parser::new(tokens);
        let err = parser
            .parse_component_call_rest("The_Card".to_string(), p())
            .unwrap_err();
        assert!(
            err.message.ends_with("of The_Card(...)"),
            "got: {}",
            err.message
        );
    }
}
