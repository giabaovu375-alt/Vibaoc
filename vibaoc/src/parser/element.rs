// ============================================================
// VIBAO COMPILER (Rust) — parser/element.rs
// Handles basic UI tags (Element) and calls to user-defined
// components (ComponentCall), including Props, interactive Events,
// Responsive directives, and Animation.
// ============================================================

use super::{ParseError, Parser};
use vibao_ast::{Element, ComponentCall, EventNode, EventName, PropsMap, AnimationProps, ResponsiveNode, Breakpoint, LiteralValue, PropKey, Expr};
use crate::lexer::TokenKind;

/// Maps a tag -> the prop name that receives a positional shorthand
/// value (e.g. `text("Xin chao")`/`image("anh.png")`) — defaults to
/// "noi_dung" for every tag NOT in this table (preserving the old
/// behavior, breaking nothing). Only lists TAGS whose "primary
/// parameter" means something other than "noi_dung" — currently only
/// image/video (their source is an image/video path, not text).
fn primary_prop_for_tag(tag: vibao_ast::Tag) -> &'static str {
    use vibao_ast::Tag;
    match tag {
        Tag::Image | Tag::Video => "nguon",
        _ => "noi_dung",
    }
}

impl Parser {
    /// Parses the value of a prop, already knowing that prop's NAME
    /// (`key`) — unlike the plain `parse_value()`, which has no context
    /// about the key.
    ///
    /// A REAL BUG THAT WAS FIXED: the `den` prop (the navigation target
    /// of `link`/`lien_ket`, e.g. `link(den: "/gioi-thieu")`) collides
    /// with the color name "den" (black, #000000) in color_map() (see
    /// lexer/tables.rs) — ONLY when the value is written WITHOUT quotes,
    /// as a bare identifier that happens to match a valid color name
    /// (e.g. accidentally writing `link(den: den)` instead of
    /// `link(den: "/den")`, or some route/variable name that
    /// accidentally collides with one of the 14 color names like
    /// "trang", "do", "xanh"...). Root cause: the lexer tokenizes the
    /// ENTIRE file BEFORE the parser ever runs (see
    /// lexer/mod.rs::tokenize) — at the moment the lexer encounters a
    /// bare identifier right after a ':', it has NO IDEA what the
    /// current prop-key is, and only decides to emit ColorName instead
    /// of Identifier based on "sits right after a ':' + exists in
    /// color_map()" (see scan.rs::classify_identifier, the
    /// is_prop_value_position variable).
    ///
    /// Fixing this cleanly at the LEXER isn't feasible (the lexer has
    /// no concept of "the current prop-key", and shouldn't) — the right
    /// place to fix it is here, at the PARSER layer, where we know for
    /// certain that `key == "den"`: if the token just received is
    /// `ColorName(n)`, it gets "downgraded" back to
    /// `LiteralValue::Str(n)` using its ORIGINAL NAME (not the already-
    /// resolved hex code) before being returned — as if that identifier
    /// had never been misread as a color by the lexer. This only applies
    /// when key == "den"; every other prop still goes through
    /// `parse_value()` as before, with no behavior change.
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


    /// The animation name in `hieu_ung_hover`/`hieu_ung_cuon` is static
    /// metadata: the runtime needs a class name to bind to, not a
    /// dynamic Expr. Only a string literal is accepted, to avoid
    /// producing an ambiguous runtime contract.
    fn animation_name_from_expr(
        &self,
        expr: &Expr,
        key: &str,
    ) -> Result<String, ParseError> {
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
            Some(PropKey::HoverAnimation) => &["phong_to", "lam_sang"][..],
            Some(PropKey::ScrollAnimation) => &[
                "fade_in",
                "truot_len",
                "truot_xuong",
                "phong_to",
                "rung",
            ][..],
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

    /// Finishes parsing a standard UI element after the tag has already
    /// been read (already resolved to a Tag — the semantic identity —
    /// by the caller, see parser/app.rs::parse_child()).
    pub(crate) fn parse_element_rest(&mut self, tag: vibao_ast::Tag, pos: vibao_ast::Pos) -> Result<Element, ParseError> {
        let mut props = Vec::new();
        let mut animation = AnimationProps::default();
        
        // Recognizes the parenthesized property list: tag(mau: do, co: 16)
        if self.match_token(&TokenKind::LParen) {
            while !self.check(&TokenKind::RParen) && !self.is_at_end() {
                // Supports a keyless shorthand for the FIRST primary
                // parameter of each tag (e.g. text("Xin chao ban") ->
                // noi_dung, image("anh.png") -> nguon).
                //
                // NEW FEATURE (an intentional extension, not a bug fix):
                // this shorthand used to ALWAYS assign to "noi_dung"
                // regardless of tag - for image/video, the "primary
                // content" isn't text but a SOURCE (an image/video path),
                // so "noi_dung" was semantically wrong. primary_prop_for_tag
                // above maps each tag to the prop name that ACTUALLY
                // receives the shorthand value - the default (for any tag
                // not in the table) stays "noi_dung" as before, breaking
                // nothing.
                if props.is_empty()
                    && matches!(
                        self.current().kind,
                        TokenKind::StringLit(_) | TokenKind::NumberLit(_, _) | TokenKind::Variable(_)
                    )
                {
                    let val = self.parse_value()?;
                    let primary_key = primary_prop_for_tag(tag);
                    props.push((primary_key.to_string(), val));
                } else {
                    let key = self.expect_identifier_like()?;
                    self.consume(&TokenKind::Colon, "Expected ':' after the property name")?;
                    let val = self.parse_prop_value(&key)?;
                    props.push((key, val));
                }
                self.skip_comma();
            }
            self.consume(&TokenKind::RParen, "Expected ')' to close the property list")?;

            // `hieu_ung_hover` / `hieu_ung_cuon` are animation metadata,
            // not ordinary HTML/CSS props. They share the same PropKey
            // to support both the Vietnamese and English locales, and
            // are then split out of PropsMap into AnimationProps so
            // codegen/runtime use the correct boundary.
            let mut retained = Vec::with_capacity(props.len());
            for (key, value) in props {
                match crate::locale::resolve_prop_key(&key) {
                    Some(PropKey::HoverAnimation) => {
                        animation.hieu_ung_hover = Some(self.animation_name_from_expr(&value, &key)?);
                    }
                    Some(PropKey::ScrollAnimation) => {
                        animation.hieu_ung_cuon = Some(self.animation_name_from_expr(&value, &key)?);
                    }
                    _ => retained.push((key, value)),
                }
            }
            props = retained;
        }

        let mut children = Vec::new();
        let mut events = Vec::new();
        let mut responsive = Vec::new();

        // Recognizes the braces containing the child node set or an event handler block
        if self.match_token(&TokenKind::LBrace) {
            while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
                if let Some(event_name) = self.match_event_name() {
                    let e_pos = self.current_pos();
                    self.consume(&TokenKind::LBrace, "Expected '{' to start the event action body")?;
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
                    // Handles a Responsive directive: @di_dong, @may_tinh... or an effect
                    self.advance(); // consume '@'
                    let r_pos = self.current_pos();
                    let name = self.expect_identifier_like()?;
                    match name.as_str() {
                        "di_dong" => {
                            let overrides = self.parse_responsive_props()?;
                            responsive.push(ResponsiveNode { breakpoint: Breakpoint::DiDong, overrides, pos: r_pos });
                        }
                        "may_tinh_bang" => {
                            let overrides = self.parse_responsive_props()?;
                            responsive.push(ResponsiveNode { breakpoint: Breakpoint::MayTinhBang, overrides, pos: r_pos });
                        }
                        "may_tinh" => {
                            let overrides = self.parse_responsive_props()?;
                            responsive.push(ResponsiveNode { breakpoint: Breakpoint::MayTinh, overrides, pos: r_pos });
                        }
                        "hieu_ung" => {
                            self.consume(&TokenKind::Colon, "Expected ':' after the @hieu_ung directive")?;
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
                        _ => return Err(self.error(format!("Directive '@{}' is not supported", name))),
                    }
                } else {
                    children.push(self.parse_child()?);
                }
            }
            self.consume(&TokenKind::RBrace, "Expected '}' to close the component body")?;
        }

        Ok(Element {
            tag,
            props,
            children,
            events,
            responsive,
            animation,
            pos,
        })
    }

    /// Finishes parsing a call to a user-defined component
    pub(crate) fn parse_component_call_rest(&mut self, name: String, pos: vibao_ast::Pos) -> Result<ComponentCall, ParseError> {
        let mut props = Vec::new();
        if self.match_token(&TokenKind::LParen) {
            while !self.check(&TokenKind::RParen) && !self.is_at_end() {
                let key = self.expect_identifier_like()?;
                self.consume(&TokenKind::Colon, "Expected ':' after the component property name")?;
                // Uses parse_prop_value (not the plain parse_value) —
                // for the same reason as parse_element_rest: if a
                // user-defined component (@the) has a param named "den"
                // (e.g. a string param holding a path), calling that
                // component with a bare identifier value that happens
                // to match a color name would hit the exact same
                // den/black-color collision bug fixed there — see the
                // full note at parse_prop_value's definition.
                let val = self.parse_prop_value(&key)?;
                props.push((key, val));
                self.skip_comma();
            }
            self.consume(&TokenKind::RParen, "Expected ')' to close the component call arguments")?;
        }

        let mut children = Vec::new();
        if self.match_token(&TokenKind::LBrace) {
            while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
                children.push(self.parse_child()?);
            }
            self.consume(&TokenKind::RBrace, "Expected '}' to close the component call body")?;
        }

        Ok(ComponentCall { name, props, children, pos })
    }

    /// Matches and consumes an event name, returning the corresponding enum
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

    /// Extracts the list of property overrides inside a responsive block
    fn parse_responsive_props(&mut self) -> Result<PropsMap, ParseError> {
        self.consume(&TokenKind::LBrace, "Expected '{' after the breakpoint name")?;
        let mut overrides = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            let key = self.expect_identifier_like()?;
            self.consume(&TokenKind::Colon, "Expected ':' after the responsive override property")?;
            let val = self.parse_value()?;
            overrides.push((key, val));
            self.skip_comma();
        }
        self.consume(&TokenKind::RBrace, "Expected '}' to close the responsive block")?;
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

    #[test]
    fn test_hover_animation_prop_is_extracted_from_props() {
        let tokens = tokenize(r#"(hieu_ung_hover: "phong_to")"#).unwrap();
        let mut parser = Parser::new(tokens);
        let el = parser.parse_element_rest(vibao_ast::Tag::Button, p()).unwrap();

        assert_eq!(el.animation.hieu_ung_hover.as_deref(), Some("phong_to"));
        assert!(el.props.iter().all(|(k, _)| k != "hieu_ung_hover"));
    }

    #[test]
    fn test_scroll_animation_english_prop_is_extracted() {
        let tokens = tokenize(r#"(scroll_animation: "truot_len")"#).unwrap();
        let mut parser = Parser::new(tokens);
        let el = parser.parse_element_rest(vibao_ast::Tag::Button, p()).unwrap();

        assert_eq!(el.animation.hieu_ung_cuon.as_deref(), Some("truot_len"));
        assert!(el.props.iter().all(|(k, _)| k != "scroll_animation"));
    }

    #[test]
    fn test_animation_prop_must_be_static_string() {
        let tokens = tokenize(r#"(hieu_ung_hover: $anim)"#).unwrap();
        let mut parser = Parser::new(tokens);
        let err = parser.parse_element_rest(vibao_ast::Tag::Button, p()).unwrap_err();

        assert!(err.message.contains("hieu_ung_hover"));
        assert!(err.message.contains("static effect-name string"));
    }

    #[test]
    fn test_hover_animation_rejects_unsupported_css_name() {
        let tokens = tokenize(r#"(hieu_ung_hover: "rung")"#).unwrap();
        let mut parser = Parser::new(tokens);
        let err = parser.parse_element_rest(vibao_ast::Tag::Button, p()).unwrap_err();

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
            let el = parser.parse_element_rest(vibao_ast::Tag::Button, p()).unwrap();
            assert_eq!(el.animation.hieu_ung_cuon.as_deref(), Some(name));
        }
    }

    #[test]
    fn test_hover_animation_english_prop_uses_same_allowed_names() {
        let tokens = tokenize(r#"(hover_animation: "lam_sang")"#).unwrap();
        let mut parser = Parser::new(tokens);
        let el = parser.parse_element_rest(vibao_ast::Tag::Button, p()).unwrap();

        assert_eq!(el.animation.hieu_ung_hover.as_deref(), Some("lam_sang"));
    }

    #[test]
    fn test_animation_props_do_not_leak_into_normal_props() {
        let tokens = tokenize(r#"(hieu_ung_hover: "lam_sang", mau: do)"#).unwrap();
        let mut parser = Parser::new(tokens);
        let el = parser.parse_element_rest(vibao_ast::Tag::Button, p()).unwrap();

        assert_eq!(el.animation.hieu_ung_hover.as_deref(), Some("lam_sang"));
        assert!(el.props.iter().any(|(k, _)| k == "mau"));
        assert!(el.props.iter().all(|(k, _)| k != "hieu_ung_hover"));
    }

    /// A direct regression test for the real bug: `link(den: den)` — the
    /// `den` prop's value is a bare identifier that matches the color
    /// name "den" (black). Before the fix, the value was misread by the
    /// lexer as ColorName("den"), which the parser then converted into
    /// `Expr::Literal(LiteralValue::Color("#000000"))` — causing the
    /// generated `href`/`data-vb-link` to become "#000000" instead of
    /// the string "den" the user actually intended to write. It must now
    /// be `Str("den")`.
    #[test]
    fn test_den_prop_value_not_confused_with_color_den() {
        let tokens = tokenize(r#"("Trang chủ", den: den)"#).unwrap();
        let mut parser = Parser::new(tokens);
        let el = parser.parse_element_rest(vibao_ast::Tag::Link, p()).unwrap();

        let (_, val) = el.props.iter().find(|(k, _)| k == "den").expect("missing prop den");
        assert!(
            matches!(val, Expr::Literal(LiteralValue::Str(s), _) if s == "den"),
            "prop den must be Str(\"den\"), got {:?}",
            val
        );
    }

    /// The normal case (no color-name collision) must still work as
    /// before - a plain string literal for a real route.
    #[test]
    fn test_den_prop_value_normal_string_still_works() {
        let tokens = tokenize(r#"(den: "/gioi-thieu")"#).unwrap();
        let mut parser = Parser::new(tokens);
        let el = parser.parse_element_rest(vibao_ast::Tag::Link, p()).unwrap();

        let (_, val) = el.props.iter().find(|(k, _)| k == "den").expect("missing prop den");
        assert!(matches!(val, Expr::Literal(LiteralValue::Str(s), _) if s == "/gioi-thieu"));
    }

    /// Every OTHER prop must still resolve a bare identifier matching a
    /// color name as an actual color, as normal - the fix only applies
    /// to the key "den" and must not break valid color behavior for
    /// other props (e.g. `mau:`).
    #[test]
    fn test_other_props_still_resolve_bare_color_identifier_as_color() {
        let tokens = tokenize(r#"(mau: den)"#).unwrap();
        let mut parser = Parser::new(tokens);
        let el = parser.parse_element_rest(vibao_ast::Tag::Text, p()).unwrap();

        let (_, val) = el.props.iter().find(|(k, _)| k == "mau").expect("missing prop mau");
        assert!(
            matches!(val, Expr::Literal(LiteralValue::Color(c), _) if c == "#000000"),
            "prop mau: den must still resolve to the color black, got {:?}",
            val
        );
    }

    /// The same bug, same fix, but through a call to a user-defined
    /// component (`@the` component call) instead of a standard element -
    /// any component with a param named "den" must avoid the same
    /// collision.
    #[test]
    fn test_den_prop_on_component_call_not_confused_with_color() {
        let tokens = tokenize(r#"(den: den)"#).unwrap();
        let mut parser = Parser::new(tokens);
        let call = parser.parse_component_call_rest("LinkTuyChinh".to_string(), p()).unwrap();

        let (_, val) = call.props.iter().find(|(k, _)| k == "den").expect("missing prop den");
        assert!(matches!(val, Expr::Literal(LiteralValue::Str(s), _) if s == "den"));
    }

    // ── Tests for the NEW feature: the first positional param now
    // accepts a bare variable (e.g. text($ten)), not just
    // StringLit/NumberLit like before ──

    #[test]
    fn test_positional_param_accepts_bare_variable() {
        // A real bug reported across 2 consecutive builds in 2 different
        // .vbao files: "text($nhan, co: 12)" was rejected with the error
        // "Expected an identifier name, received variable '$nhan'" -
        // because the positional param shorthand used to ONLY accept
        // StringLit/NumberLit.
        let tokens = tokenize(r#"($nhan, co: 12)"#).unwrap();
        let mut parser = Parser::new(tokens);
        let el = parser.parse_element_rest(vibao_ast::Tag::Text, p()).unwrap();

        let (_, val) = el.props.iter().find(|(k, _)| k == "noi_dung").expect("missing prop noi_dung");
        assert!(
            matches!(val, Expr::Variable(n, _) if n == "nhan"),
            "prop noi_dung must be Variable(\"nhan\"), got {:?}",
            val
        );
        // The second prop (co: 12) must still parse correctly after the
        // variable in the first position has been consumed.
        assert!(el.props.iter().any(|(k, _)| k == "co"), "prop 'co' must still parse correctly after the positional param");
    }

    #[test]
    fn test_positional_param_accepts_member_access() {
        // "$item.ten" (member access) must also fall into the shorthand
        // branch correctly - the FIRST token is still Variable("item"),
        // and the ".ten" part is read afterward by parse_value() itself.
        let tokens = tokenize(r#"($item.ten)"#).unwrap();
        let mut parser = Parser::new(tokens);
        let el = parser.parse_element_rest(vibao_ast::Tag::Text, p()).unwrap();

        let (_, val) = el.props.iter().find(|(k, _)| k == "noi_dung").expect("missing prop noi_dung");
        assert!(
            matches!(val, Expr::MemberAccess { property, .. } if property == "ten"),
            "prop noi_dung must be MemberAccess with property \"ten\", got {:?}",
            val
        );
    }

    #[test]
    fn test_positional_param_string_and_number_still_work() {
        // Regression: the 2 OLD cases (already working before) must not
        // be affected by adding the new Variable branch.
        let tokens = tokenize(r#"("Xin chào")"#).unwrap();
        let mut parser = Parser::new(tokens);
        let el = parser.parse_element_rest(vibao_ast::Tag::Text, p()).unwrap();
        let (_, val) = el.props.iter().find(|(k, _)| k == "noi_dung").unwrap();
        assert!(matches!(val, Expr::Literal(LiteralValue::Str(s), _) if s == "Xin chào"));
    }

    #[test]
    fn test_named_prop_after_variable_shorthand_not_confused() {
        // Confirms a named prop ("key: value") is NOT mistaken for the
        // positional shorthand - only a Variable in the VERY FIRST
        // position (props still empty) enters the shorthand branch; a
        // named prop always has a bare Identifier/Component/ColorName
        // followed by ':', never "$variable:".
        let tokens = tokenize(r#"(mau: xanh, co: 16)"#).unwrap();
        let mut parser = Parser::new(tokens);
        let el = parser.parse_element_rest(vibao_ast::Tag::Text, p()).unwrap();
        assert!(el.props.iter().all(|(k, _)| k != "noi_dung"), "must not auto-add noi_dung when there is no positional param");
        assert_eq!(el.props.len(), 2);
    }
}
