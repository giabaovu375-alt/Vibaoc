use super::{ParseError, Parser};
use crate::lexer::TokenKind;
use vibao_ast::{Action, Expr};

impl Parser {
    pub(crate) fn parse_action(&mut self) -> Result<Action, ParseError> {
        let pos = self.current_pos();

        if self.check(&TokenKind::Neu) {
            self.advance();
            let condition = self.parse_value()?;
            self.consume(
                &TokenKind::LBrace,
                "Expected '{' to open the 'neu' action block",
            )?;
            let mut consequent = Vec::new();
            while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
                consequent.push(self.parse_action()?);
            }
            self.consume(
                &TokenKind::RBrace,
                "Expected '}' to close the 'neu' action block",
            )?;

            let mut alternate = None;
            if self.check(&TokenKind::KhongThi) {
                self.advance();
                self.consume(
                    &TokenKind::LBrace,
                    "Expected '{' to open the 'khong_thi' action block",
                )?;
                let mut alt_body = Vec::new();
                while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
                    alt_body.push(self.parse_action()?);
                }
                self.consume(
                    &TokenKind::RBrace,
                    "Expected '}' to close the 'khong_thi' action block",
                )?;
                alternate = Some(alt_body);
            }
            return Ok(Action::IfAction {
                condition,
                consequent,
                alternate,
                pos,
            });
        }

        let mut assign_to = None;
        if let TokenKind::Variable(name) = &self.current().kind {
            if self.check_at(1, &TokenKind::Equals) {
                let is_call = matches!(
                    self.peek(2).kind,
                    TokenKind::Identifier(_) | TokenKind::Component(_)
                ) && self.check_at(3, &TokenKind::LParen);

                let name = name.clone();
                if !is_call {
                    self.advance();
                    self.advance();
                    let value = self.parse_value()?;
                    self.skip_comma();
                    return Ok(Action::Assign {
                        target: name,
                        value,
                        pos,
                    });
                }
                self.advance();
                self.advance();
                assign_to = Some(name);
            }
        }

        let surface_name = self.expect_identifier_like()?;
        let name = crate::locale::resolve_action_name(&surface_name)
            .map(|action| action.runtime_name().to_string())
            .unwrap_or(surface_name);

        self.consume(
            &TokenKind::LParen,
            "Expected '(' before the action arguments",
        )?;

        let mut args = Vec::new();
        let mut opts = Vec::new();

        while !self.check(&TokenKind::RParen) && !self.is_at_end() {
            if let TokenKind::Identifier(k) = &self.current().kind {
                if self.check_at(1, &TokenKind::Colon) {
                    let key = crate::locale::canonical_action_option_key(k).to_string();
                    self.advance();
                    self.advance();
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

            let data = if args.len() > 2 {
                Some(args[2].clone())
            } else {
                None
            };
            let headers = opts
                .iter()
                .find(|(k, _)| k == "tieu_de")
                .map(|(_, v)| v.clone());
            let background = opts.iter().any(|(k, v)| {
                k == "chay_nen"
                    && matches!(v, Expr::Literal(vibao_ast::LiteralValue::Bool(true), _))
            });

            let mut on_success = None;
            let mut on_failure = None;

            if self.match_token(&TokenKind::LBrace) {
                while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
                    let cb_pos = self.current_pos();
                    let cb_name = self.expect_identifier_like()?;
                    if self.check(&TokenKind::Colon) {
                        self.advance();
                    }
                    self.consume(
                        &TokenKind::LBrace,
                        "Expected '{' to start the response handler body",
                    )?;
                    let mut cb_body = Vec::new();
                    while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
                        cb_body.push(self.parse_action()?);
                    }
                    self.consume(
                        &TokenKind::RBrace,
                        "Expected '}' to close the response handler body",
                    )?;

                    match crate::locale::resolve_response_handler(&cb_name) {
                        Some(crate::locale::ResponseHandler::Success) => on_success = Some(cb_body),
                        Some(crate::locale::ResponseHandler::Failure) => on_failure = Some(cb_body),
                        None => {
                            return Err(ParseError {
                                message: format!(
                                    "Unknown response handler '{}': expected 'thanh_cong' (or 'success') or 'that_bai' (or 'failure')",
                                    cb_name
                                ),
                                line: cb_pos.line,
                                column: cb_pos.column,
                            })
                        }
                    }
                }
                self.consume(&TokenKind::RBrace, "Expected '}' to close the API handler")?;
            }

            return Ok(Action::ApiCall {
                method,
                endpoint,
                data,
                headers,
                background,
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

    fn parse_action_source(source: &str) -> Result<Action, ParseError> {
        let tokens = tokenize(source).expect("source must tokenize");
        Parser::new(tokens).parse_action()
    }

    #[test]
    fn test_english_option_keys_are_normalized_to_runtime_keys() {
        let action = parse_action_source("notify(\"hi\", type: canh_bao, duration: 5000)").unwrap();
        match action {
            Action::FunctionCall { opts, .. } => {
                let keys: Vec<&str> = opts.iter().map(|(k, _)| k.as_str()).collect();
                assert_eq!(keys, vec!["kieu", "thoi_gian"]);
            }
            other => panic!("expected FunctionCall, got {:?}", other),
        }
    }

    #[test]
    fn test_vietnamese_option_keys_are_unchanged() {
        let action = parse_action_source("thong_bao(\"hi\", kieu: loi, thoi_gian: 100)").unwrap();
        match action {
            Action::FunctionCall { opts, .. } => {
                let keys: Vec<&str> = opts.iter().map(|(k, _)| k.as_str()).collect();
                assert_eq!(keys, vec!["kieu", "thoi_gian"]);
            }
            other => panic!("expected FunctionCall, got {:?}", other),
        }
    }

    #[test]
    fn test_api_call_accepts_english_response_handlers() {
        let source =
            "api_call(\"GET\", \"/x\") { success { notify(\"ok\") } failure { notify(\"bad\") } }";
        match parse_action_source(source).unwrap() {
            Action::ApiCall {
                on_success,
                on_failure,
                ..
            } => {
                assert!(on_success.is_some());
                assert!(on_failure.is_some());
            }
            other => panic!("expected ApiCall, got {:?}", other),
        }
    }

    #[test]
    fn test_api_call_vietnamese_response_handlers_still_work() {
        let source = "goi_api(\"GET\", \"/x\") { thanh_cong { thong_bao(\"ok\") } that_bai { thong_bao(\"bad\") } }";
        match parse_action_source(source).unwrap() {
            Action::ApiCall {
                on_success,
                on_failure,
                ..
            } => {
                assert!(on_success.is_some());
                assert!(on_failure.is_some());
            }
            other => panic!("expected ApiCall, got {:?}", other),
        }
    }

    #[test]
    fn test_api_call_reads_headers_option_in_both_languages() {
        for source in [
            "goi_api(\"POST\", \"/x\", {a: 1}, tieu_de: {Authorization: \"Bearer k\"})",
            "api_call(\"POST\", \"/x\", {a: 1}, headers: {Authorization: \"Bearer k\"})",
        ] {
            match parse_action_source(source).unwrap() {
                Action::ApiCall { headers, data, .. } => {
                    assert!(headers.is_some(), "{}", source);
                    assert!(data.is_some(), "{}", source);
                }
                other => panic!("expected ApiCall, got {:?}", other),
            }
        }
    }

    #[test]
    fn test_api_call_background_option_in_both_languages() {
        for source in [
            "goi_api(\"GET\", \"/x\", chay_nen: true)",
            "api_call(\"GET\", \"/x\", background: true)",
        ] {
            match parse_action_source(source).unwrap() {
                Action::ApiCall { background, .. } => assert!(background, "{}", source),
                other => panic!("expected ApiCall, got {:?}", other),
            }
        }
        match parse_action_source("goi_api(\"GET\", \"/x\")").unwrap() {
            Action::ApiCall { background, .. } => assert!(!background),
            other => panic!("expected ApiCall, got {:?}", other),
        }
    }

    #[test]
    fn test_api_call_rejects_unknown_response_handler() {
        let source = "api_call(\"GET\", \"/x\") { done { notify(\"ok\") } }";
        let err = parse_action_source(source).unwrap_err();
        assert!(
            err.message.starts_with("Unknown response handler 'done'"),
            "got: {}",
            err.message
        );
    }

    #[test]
    fn test_english_api_action_keeps_special_api_ast_path() {
        let tokens =
            tokenize("api_call(\"GET\", \"/health\")").expect("English API action must tokenize");
        let mut parser = Parser::new(tokens);
        let action = parser
            .parse_action()
            .expect("English API action must parse");
        assert!(
            matches!(action, Action::ApiCall { .. }),
            "api_call must be special-cased into ApiCall"
        );
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
