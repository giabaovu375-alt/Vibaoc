use super::{ParseError, Parser};
use crate::lexer::TokenKind;
use vibao_ast::{CaseNode, Child, IfNode, LoopKind, LoopNode, SwitchNode};

impl Parser {
    pub(crate) fn parse_if_node(&mut self) -> Result<IfNode, ParseError> {
        let pos = self.current_pos();
        self.consume(&TokenKind::Neu, "Expected the 'neu' keyword")?;
        let condition = self.parse_value()?;

        self.consume(&TokenKind::LBrace, "Expected '{' to start the 'neu' block")?;
        let mut consequent = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            consequent.push(self.parse_child()?);
        }
        self.consume(&TokenKind::RBrace, "Expected '}' to close the 'neu' block")?;

        let mut alternate = None;
        if self.check(&TokenKind::KhongThi) {
            self.advance();
            if self.check(&TokenKind::Neu) {
                let nested_if = self.parse_if_node()?;
                alternate = Some(vec![Child::If(Box::new(nested_if))]);
            } else {
                self.consume(
                    &TokenKind::LBrace,
                    "Expected '{' to start the 'khong_thi' block",
                )?;
                let mut alt_body = Vec::new();
                while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
                    alt_body.push(self.parse_child()?);
                }
                self.consume(
                    &TokenKind::RBrace,
                    "Expected '}' to close the 'khong_thi' block",
                )?;
                alternate = Some(alt_body);
            }
        }

        Ok(IfNode {
            condition,
            consequent,
            alternate,
            pos,
        })
    }

    pub(crate) fn parse_loop_node(&mut self) -> Result<LoopNode, ParseError> {
        let pos = self.current_pos();
        self.consume(&TokenKind::VongLap, "Expected the 'vong_lap' keyword")?;

        let kind = if let TokenKind::Variable(first_var) = self.current().kind.clone() {
            self.advance();

            let index_var = if self.check(&TokenKind::Comma) {
                self.advance();
                match self.current().kind.clone() {
                    TokenKind::Variable(idx_name) => {
                        self.advance();
                        Some(idx_name)
                    }
                    other => {
                        return Err(self.error(format!(
                            "Expected an index variable (for example $idx) after ',' in 'vong_lap', received {}",
                            other
                        )));
                    }
                }
            } else {
                None
            };

            if self.check_ident("trong") {
                self.advance();
                let iterable = self.parse_value()?;
                LoopKind::Each {
                    iterable,
                    item_var: first_var,
                    index_var,
                }
            } else if self.check(&TokenKind::Tu) {
                self.advance();
                let from_val = match self.advance().kind {
                    TokenKind::NumberLit(v, _) => v as i64,
                    other => {
                        return Err(
                            self.error(format!("Expected a valid range start, received {}", other))
                        )
                    }
                };
                if self.check_ident_or_color_name("den") {
                    self.advance();
                }
                let to_val = match self.advance().kind {
                    TokenKind::NumberLit(v, _) => v as i64,
                    other => {
                        return Err(
                            self.error(format!("Expected a valid range end, received {}", other))
                        )
                    }
                };
                if from_val > to_val {
                    return Err(self.error(
                        "Range 'tu N1 den N2' requires N1 <= N2; descending ranges are not supported".to_string(),
                    ));
                }
                LoopKind::Range {
                    from: from_val,
                    to: to_val,
                    var_name: first_var.replace('$', ""),
                }
            } else {
                return Err(self.error(format!(
                    "After variable '${}' in 'vong_lap', expected 'trong' (iterate a list) or 'tu' (iterate a numeric range), received {}",
                    first_var, self.current().kind,
                )));
            }
        } else if self.check(&TokenKind::Tu) {
            self.advance();
            let from_val = match self.advance().kind {
                TokenKind::NumberLit(v, _) => v as i64,
                other => {
                    return Err(
                        self.error(format!("Expected a valid range start, received {}", other))
                    )
                }
            };
            if self.check_ident_or_color_name("den") {
                self.advance();
            }
            let to_val = match self.advance().kind {
                TokenKind::NumberLit(v, _) => v as i64,
                other => {
                    return Err(
                        self.error(format!("Expected a valid range end, received {}", other))
                    )
                }
            };
            if from_val > to_val {
                return Err(self.error(
                    "Range 'tu N1 den N2' requires N1 <= N2; descending ranges are not supported"
                        .to_string(),
                ));
            }
            LoopKind::Range {
                from: from_val,
                to: to_val,
                var_name: "i".to_string(),
            }
        } else {
            return Err(self.error("Unsupported or malformed loop syntax"));
        };

        self.consume(&TokenKind::LBrace, "Expected '{' to start the loop body")?;
        let mut body = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            body.push(self.parse_child()?);
        }
        self.consume(&TokenKind::RBrace, "Expected '}' to close the loop body")?;

        Ok(LoopNode { kind, body, pos })
    }

    pub(crate) fn parse_switch_node(&mut self) -> Result<SwitchNode, ParseError> {
        let pos = self.current_pos();
        self.consume(&TokenKind::TruongHop, "Expected the 'truong_hop' keyword")?;
        let subject = self.parse_value()?;

        self.consume(
            &TokenKind::LBrace,
            "Expected '{' to start the 'truong_hop' block",
        )?;

        let mut cases = Vec::new();
        let mut default_case = None;

        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            if self.check(&TokenKind::MacDinh) {
                self.advance();
                self.consume(
                    &TokenKind::LBrace,
                    "Expected '{' to start the 'mac_dinh' block",
                )?;
                let mut body = Vec::new();
                while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
                    body.push(self.parse_child()?);
                }
                self.consume(
                    &TokenKind::RBrace,
                    "Expected '}' to close the 'mac_dinh' block",
                )?;
                if default_case.is_some() {
                    return Err(
                        self.error("Only one 'mac_dinh' block is allowed in a 'truong_hop'")
                    );
                }
                default_case = Some(body);
            } else {
                let case_pos = self.current_pos();
                let value = self.parse_value()?;
                self.consume(
                    &TokenKind::LBrace,
                    "Expected '{' to start the case body in 'truong_hop'",
                )?;
                let mut body = Vec::new();
                while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
                    body.push(self.parse_child()?);
                }
                self.consume(
                    &TokenKind::RBrace,
                    "Expected '}' to close the case body in 'truong_hop'",
                )?;
                cases.push(CaseNode {
                    value,
                    body,
                    pos: case_pos,
                });
            }
        }

        self.consume(
            &TokenKind::RBrace,
            "Expected '}' to close the 'truong_hop' block",
        )?;

        Ok(SwitchNode {
            subject,
            cases,
            default_case,
            pos,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::tokenize;

    fn parse_loop_from_source(src: &str) -> LoopNode {
        let tokens = tokenize(src).unwrap();
        let mut p = Parser::new(tokens);
        p.parse_loop_node().unwrap()
    }

    #[test]
    fn test_range_loop_with_counter_variable() {
        let node = parse_loop_from_source("vong_lap $i tu 1 den 3 { }");
        match node.kind {
            LoopKind::Range { from, to, var_name } => {
                assert_eq!(from, 1);
                assert_eq!(to, 3);
                assert_eq!(var_name, "i");
            }
            other => panic!("expected LoopKind::Range, got {:?}", other),
        }
    }

    #[test]
    fn test_range_loop_custom_counter_variable_name_is_kept() {
        let node = parse_loop_from_source("vong_lap $dem tu 1 den 3 { }");
        match node.kind {
            LoopKind::Range { var_name, .. } => {
                assert_eq!(var_name, "dem");
            }
            other => panic!("expected LoopKind::Range, got {:?}", other),
        }
    }

    #[test]
    fn test_range_loop_without_counter_variable() {
        let node = parse_loop_from_source("vong_lap tu 1 den 3 { }");
        match node.kind {
            LoopKind::Range { from, to, var_name } => {
                assert_eq!(from, 1);
                assert_eq!(to, 3);
                assert_eq!(var_name, "i");
            }
            other => panic!("expected LoopKind::Range, got {:?}", other),
        }
    }

    #[test]
    fn test_each_loop_with_trong_keyword() {
        let node = parse_loop_from_source("vong_lap $item trong $ds { }");
        match node.kind {
            LoopKind::Each {
                item_var,
                index_var,
                ..
            } => {
                assert_eq!(item_var, "item");
                assert!(index_var.is_none());
            }
            other => panic!("expected LoopKind::Each, got {:?}", other),
        }
    }

    #[test]
    fn test_variable_followed_by_neither_trong_nor_tu_is_clear_error() {
        let tokens = tokenize("vong_lap $i xyz { }").unwrap();
        let mut p = Parser::new(tokens);
        let result = p.parse_loop_node();
        assert!(result.is_err());
    }

    #[test]
    fn test_each_loop_with_index_variable() {
        let node = parse_loop_from_source("vong_lap $task, $idx trong $tasks { }");
        match node.kind {
            LoopKind::Each {
                item_var,
                index_var,
                ..
            } => {
                assert_eq!(item_var, "task");
                assert_eq!(index_var, Some("idx".to_string()));
            }
            other => panic!("expected LoopKind::Each, got {:?}", other),
        }
    }

    #[test]
    fn test_each_loop_without_index_variable_still_none() {
        let node = parse_loop_from_source("vong_lap $item trong $ds { }");
        match node.kind {
            LoopKind::Each { index_var, .. } => assert!(index_var.is_none()),
            other => panic!("expected LoopKind::Each, got {:?}", other),
        }
    }

    #[test]
    fn test_each_loop_index_variable_missing_after_comma_is_clear_error() {
        let tokens = tokenize("vong_lap $item, 5 trong $ds { }").unwrap();
        let mut p = Parser::new(tokens);
        let result = p.parse_loop_node();
        assert!(
            result.is_err(),
            "must error when ',' is not followed by a valid variable"
        );
    }

    #[test]
    fn test_descending_range_is_rejected_at_parse_time() {
        let tokens = tokenize("vong_lap $i tu 5 den 1 { }").unwrap();
        let mut p = Parser::new(tokens);
        let result = p.parse_loop_node();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err
            .to_string()
            .contains("descending ranges are not supported"));
    }

    #[test]
    fn test_range_loop_unaffected_by_index_var_syntax() {
        let node = parse_loop_from_source("vong_lap $i tu 1 den 3 { }");
        assert!(matches!(node.kind, LoopKind::Range { from: 1, to: 3, .. }));
    }
}
