// ============================================================
// VIBAO COMPILER (Rust) — parser/action.rs
// Handles the executable statements (Action) inside an event block,
// including state assignment, ordinary function calls, and API calls.
// ============================================================

use super::{ParseError, Parser};
use vibao_ast::{Action, Expr};
use crate::lexer::TokenKind;

impl Parser {
    /// Parses a single complete action statement inside an event handler
    pub(crate) fn parse_action(&mut self) -> Result<Action, ParseError> {
        let pos = self.current_pos();

        // Handles a local branch inside an event block: neu dieu_kien { ... }
        if self.check(&TokenKind::Neu) {
            self.advance();
            let condition = self.parse_value()?;
            self.consume(&TokenKind::LBrace, "Expected '{' to open the 'neu' action block")?;
            let mut consequent = Vec::new();
            while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
                consequent.push(self.parse_action()?);
            }
            self.consume(&TokenKind::RBrace, "Expected '}' to close the 'neu' action block")?;

            let mut alternate = None;
            if self.check(&TokenKind::KhongThi) {
                self.advance();
                self.consume(&TokenKind::LBrace, "Expected '{' to open the 'khong_thi' action block")?;
                let mut alt_body = Vec::new();
                while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
                    alt_body.push(self.parse_action()?);
                }
                self.consume(&TokenKind::RBrace, "Expected '}' to close the 'khong_thi' action block")?;
                alternate = Some(alt_body);
            }
            return Ok(Action::IfAction { condition, consequent, alternate, pos });
        }

        // Recognizes capturing a function/api return value into a
        // variable: $res = ...
        // Must lookahead BEFORE consuming any tokens, to correctly
        // distinguish:
        //   $x = 5          -> Assign
        //   $x = ham(...)   -> FunctionCall/ApiCall with assign_to = Some("x")
        // (An old bug: advance() consumed $var and '=' before checking
        // check_at(1, LParen), so the lookahead position was off by 2,
        // always falling through to Assign even when the right-hand side
        // was a function/api call.)
        let mut assign_to = None;
        if let TokenKind::Variable(name) = &self.current().kind {
            if self.check_at(1, &TokenKind::Equals) {
                // The token right after '=' sits at offset 2; this is a
                // function/api call when offset 2 is Identifier/Component
                // AND offset 3 is LParen.
                let is_call = matches!(
                    self.peek(2).kind,
                    TokenKind::Identifier(_) | TokenKind::Component(_)
                ) && self.check_at(3, &TokenKind::LParen);

                let name = name.clone();
                if !is_call {
                    self.advance(); // consume the variable
                    self.advance(); // consume '='
                    let value = self.parse_value()?;
                    self.skip_comma();
                    return Ok(Action::Assign { target: name, value, pos });
                }
                // This is a function/api call: consume $var and '=',
                // keeping the variable name to attach to assign_to on the
                // FunctionCall/ApiCall below.
                self.advance(); // consume the variable
                self.advance(); // consume '='
                assign_to = Some(name);
            }
        }

        // The name of a utility function or API call task — uses
        // expect_identifier_like() (accepts Identifier/Component/
        // ColorName) instead of just Identifier/Component, for the same
        // reason already fixed in parse_component_def/parse_import
        // (app.rs): a function name that HAPPENS to collide with one of
        // the 14 color names (e.g. a custom function named "xanh") is
        // still a valid identity in the FUNCTION NAME position, even if
        // rare.
        let surface_name = self.expect_identifier_like()?;
        // Normalizes a recognized action to its canonical runtime name
        // right at the compiler boundary. The runtime only ever receives
        // an already-normalized semantic identity and doesn't need to
        // know anything about the locale surface. An unknown name is
        // left as-is so the validator can report the exact name the user
        // actually typed.
        let name = crate::locale::resolve_action_name(&surface_name)
            .map(|action| action.runtime_name().to_string())
            .unwrap_or(surface_name);

        self.consume(&TokenKind::LParen, "Expected '(' before the action arguments")?;
        
        let mut args = Vec::new();
        let mut opts = Vec::new();

        while !self.check(&TokenKind::RParen) && !self.is_at_end() {
            // Recognizes a Named Option in key:value form (e.g. kieu: thanh_cong)
            if let TokenKind::Identifier(k) = &self.current().kind {
                if self.check_at(1, &TokenKind::Colon) {
                    let key = k.clone();
                    self.advance(); // consume the identifier
                    self.advance(); // consume ':'
                    let val = self.parse_value()?;
                    opts.push((key, val));
                    self.skip_comma();
                    continue;
                }
            }

            args.push(self.parse_value()?);
            self.skip_comma();
        }
        self.consume(&TokenKind::RParen, "Expected ')' to close the arguments")?;

        // Special-cases the network task "goi_api" -> converts it into its own dedicated ApiCall AST node
        if name == "goi_api" {
            let method = if !args.is_empty() {
                match &args[0] {
                    Expr::Literal(vibao_ast::LiteralValue::Str(m), _) => m.clone(),
                    _ => "GET".to_string(),
                }
            } else {
                "GET".to_string()
            };

            let endpoint = if args.len() > 1 {
                args[1].clone()
            } else {
                Expr::literal_str("", pos)
            };

            let data = if args.len() > 2 { Some(args[2].clone()) } else { None };

            let mut on_success = None;
            let mut on_failure = None;

            // Checks for and unpacks the nested callback blocks in the form `{ thanh_cong: { ... } }`
            if self.match_token(&TokenKind::LBrace) {
                while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
                    let cb_name = self.expect_identifier_like()?;
                    if self.check(&TokenKind::Colon) { self.advance(); }
                    self.consume(&TokenKind::LBrace, "Expected '{' to start the response handler body")?;
                    let mut cb_body = Vec::new();
                    while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
                        cb_body.push(self.parse_action()?);
                    }
                    self.consume(&TokenKind::RBrace, "Expected '}' to close the response handler body")?;
                    
                    if cb_name == "thanh_cong" {
                        on_success = Some(cb_body);
                    } else if cb_name == "that_bai" {
                        on_failure = Some(cb_body);
                    }
                }
                self.consume(&TokenKind::RBrace, "Expected '}' to close the API handler")?;
            }

            return Ok(Action::ApiCall {
                method,
                endpoint,
                data,
                assign_to,
                on_success,
                on_failure,
                pos,
            });
        }

        self.skip_comma();

        Ok(Action::FunctionCall {
            name,
            args,
            opts,
            assign_to,
            pos,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::tokenize;

    #[test]
    fn test_english_action_is_normalized_to_canonical_runtime_name() {
        let tokens = tokenize("notify(\"hello\")").expect("English action must tokenize");
        let mut parser = Parser::new(tokens);
        let action = parser.parse_action().expect("English action must parse");
        match action {
            Action::FunctionCall { name, .. } => assert_eq!(name, "thong_bao"),
            other => panic!("expected FunctionCall, got {:?}", other),
        }
    }

    #[test]
    fn test_english_api_action_keeps_special_api_ast_path() {
        let tokens = tokenize("api_call(\"GET\", \"/health\")").expect("English API action must tokenize");
        let mut parser = Parser::new(tokens);
        let action = parser.parse_action().expect("English API action must parse");
        assert!(matches!(action, Action::ApiCall { .. }), "api_call must be special-cased into ApiCall");
    }

    #[test]
    fn test_english_array_action_is_normalized_before_shape_validation() {
        let tokens = tokenize("array_push(\"tasks\", 1)").expect("English action must tokenize");
        let mut parser = Parser::new(tokens);
        let action = parser.parse_action().expect("English action must parse");
        match action {
            Action::FunctionCall { name, .. } => assert_eq!(name, "them_vao_mang"),
            other => panic!("expected FunctionCall, got {:?}", other),
        }
    }
}
