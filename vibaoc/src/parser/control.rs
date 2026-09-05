// ============================================================
// VIBAO COMPILER (Rust) — parser/control.rs
// Handles the display-flow control structures (Control Flow) such as
// the 'neu' / 'khong_thi' conditional and the 'vong_lap' loop.
// ============================================================

use super::{ParseError, Parser};
use vibao_ast::{CaseNode, Child, IfNode, LoopKind, LoopNode, SwitchNode};
use crate::lexer::TokenKind;

impl Parser {
    /// Parses a conditional: neu bieu_thuc { ... } khong_thi { ... }
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
            self.advance(); // consume 'khong_thi'
            if self.check(&TokenKind::Neu) {
                // Matches the khong_thi neu form (a nested Else If)
                let nested_if = self.parse_if_node()?;
                alternate = Some(vec![Child::If(Box::new(nested_if))]);
            } else {
                self.consume(&TokenKind::LBrace, "Expected '{' to start the 'khong_thi' block")?;
                let mut alt_body = Vec::new();
                while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
                    alt_body.push(self.parse_child()?);
                }
                self.consume(&TokenKind::RBrace, "Expected '}' to close the 'khong_thi' block")?;
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

    /// Parses a loop: vong_lap $item trong $mang, or a numeric ascending range loop
    pub(crate) fn parse_loop_node(&mut self) -> Result<LoopNode, ParseError> {
        let pos = self.current_pos();
        self.consume(&TokenKind::VongLap, "Expected the 'vong_lap' keyword")?;

        // BUG ALREADY FIXED: both loop forms start with a variable -
        // "vong_lap $item trong $ds { }" (Each) AND "vong_lap $i tu 1
        // den 3 { }" (Range with a counter variable). The OLD code
        // always treated a leading Variable as the Each form, then
        // immediately called parse_value() on whatever token followed
        // (even when that was "tu"/"from" - which should have been
        // recognized as Range). Before "tu" became its own keyword
        // TokenKind::Tu (for the `nhap` syntax), this bug SILENTLY
        // turned "tu" into a meaningless string value for the iterable -
        // no crash, just a wrong result. Now that "tu" is a distinct
        // keyword, the error surfaces clearly right away ("Could not
        // parse a value") instead of failing silently - THIS WAS THE
        // OPPORTUNITY TO FIX THE ROOT CAUSE: the token RIGHT AFTER the
        // first variable needs to be inspected to distinguish the two
        // forms, instead of guessing.
        let kind = if let TokenKind::Variable(first_var) = self.current().kind.clone() {
            self.advance();

            // NEW FEATURE (not a bug fix - an addition per a feature
            // request): "vong_lap $item, $idx trong $ds { ... }" - also
            // captures the INDEX of each item, needed to correctly
            // update/remove a single element within a dynamic array (e.g.
            // cap_nhat_theo_chi_so($ds, $idx, ...) - see action.rs). The
            // AST (LoopKind::Each::index_var), codegen (CompiledLoop,
            // LoopSpec JSON), AND the runtime
            // (LoopFrame::index_var/index_value, state.rs::scope_resolve)
            // were ALREADY FULLY READY beforehand (with tests already
            // confirming correct behavior) - ONLY THE PARSER had no
            // syntax for the USER to actually declare index_var (it was
            // always hardcoded to None) - this was the ONE missing link
            // in the entire pipeline.
            //
            // Syntax: a comma RIGHT AFTER item_var, before "trong".
            // No other English name is used - kept consistent as "$item,
            // $idx" (a comma separating the 2 variables, with no
            // dedicated "index" keyword - simple, similar to familiar
            // destructuring).
            let index_var = if self.check(&TokenKind::Comma) {
                self.advance(); // consume ','
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
                // The Each form: vong_lap $item trong $ds { ... }
                // or:            vong_lap $item, $idx trong $ds { ... }
                self.advance();
                let iterable = self.parse_value()?;
                LoopKind::Each {
                    iterable,
                    item_var: first_var,
                    index_var,
                }
            } else if self.check(&TokenKind::Tu) {
                // The Range form with a counter variable: vong_lap $i tu
                // 1 den 3 { ... }. The variable name is kept in the AST so
                // codegen/runtime use the exact name the dev declared.
                self.advance();
                let from_val = match self.advance().kind {
                    TokenKind::NumberLit(v, _) => v as i64,
                    other => return Err(self.error(format!("Expected a valid range start, received {}", other))),
                };
                if self.check_ident_or_color_name("den") {
                    self.advance();
                }
                let to_val = match self.advance().kind {
                    TokenKind::NumberLit(v, _) => v as i64,
                    other => return Err(self.error(format!("Expected a valid range end, received {}", other))),
                };
                if from_val > to_val {
                    return Err(self.error(
                        "Range 'tu N1 den N2' requires N1 <= N2; descending ranges are not supported in 0.1.0".to_string(),
                    ));
                }
                // BUG ALREADY FIXED: `first_var` (the counter variable
                // name the dev declared, e.g. "dem" in "$dem tu 1 den 3")
                // used to be parsed correctly and then DISCARDED
                // (`let _ = first_var;`) - LoopKind::Range had nowhere to
                // store it, so the loop always used the default name "i"
                // at runtime regardless of what the dev named it. It's
                // now kept, with the leading "$" stripped to match
                // convention (item_var in the Each branch also has no
                // "$", see compile_loop_node in codegen).
                LoopKind::Range { from: from_val, to: to_val, var_name: first_var.replace('$', "") }
            } else {
                return Err(self.error(format!(
                    "After variable '${}' in 'vong_lap', expected 'trong' (iterate a list) or 'tu' (iterate a numeric range), received {}",
                    first_var, self.current().kind,
                )));
            }
        } else if self.check(&TokenKind::Tu) {
            // The Range form with NO explicit counter variable: vong_lap tu 1 den 3
            self.advance();
            let from_val = match self.advance().kind {
                TokenKind::NumberLit(v, _) => v as i64,
                other => return Err(self.error(format!("Expected a valid range start, received {}", other))),
            };
            if self.check_ident_or_color_name("den") {
                self.advance();
            }
            let to_val = match self.advance().kind {
                TokenKind::NumberLit(v, _) => v as i64,
                other => return Err(self.error(format!("Expected a valid range end, received {}", other))),
            };
            if from_val > to_val {
                return Err(self.error(
                    "Range 'tu N1 den N2' requires N1 <= N2; descending ranges are not supported in 0.1.0".to_string(),
                ));
            }
            // No explicit counter variable declared - defaults to "i".
            LoopKind::Range { from: from_val, to: to_val, var_name: "i".to_string() }
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

    /// Parses a multi-branch conditional:
    ///   truong_hop $bien {
    ///       gia_tri_1 { ... }
    ///       gia_tri_2 { ... }
    ///       mac_dinh { ... }
    ///   }
    ///
    /// DID NOT EXIST BEFORE: even though the lexer already had the
    /// TruongHop/MacDinh tokens and the AST already had
    /// SwitchNode/CaseNode, no parser function ever actually constructed
    /// this node (confirmed by a "never constructed" warning during a
    /// build) - this is the first time this syntax actually works.
    ///
    /// Each "case" is an expression (the value to match against, parsed
    /// through parse_value() to reuse the existing expression logic -
    /// allowing a case to be a literal, a variable, or a more complex
    /// expression) followed by a `{...}` block - DISTINGUISHED from
    /// element syntax (`tag(props) {...}`) by the absence of `(...)`
    /// right after it, with `{` appearing directly after the case
    /// expression instead. `mac_dinh { ... }` (if present) should be the
    /// LAST branch - like switch/default in most other languages,
    /// though the compiler doesn't strictly enforce ordering (mac_dinh
    /// appearing in the middle is still accepted, just unusual style).
    pub(crate) fn parse_switch_node(&mut self) -> Result<SwitchNode, ParseError> {
        let pos = self.current_pos();
        self.consume(&TokenKind::TruongHop, "Expected the 'truong_hop' keyword")?;
        let subject = self.parse_value()?;

        self.consume(&TokenKind::LBrace, "Expected '{' to start the 'truong_hop' block")?;

        let mut cases = Vec::new();
        let mut default_case = None;

        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            if self.check(&TokenKind::MacDinh) {
                self.advance(); // consume 'mac_dinh'
                self.consume(&TokenKind::LBrace, "Expected '{' to start the 'mac_dinh' block")?;
                let mut body = Vec::new();
                while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
                    body.push(self.parse_child()?);
                }
                self.consume(&TokenKind::RBrace, "Expected '}' to close the 'mac_dinh' block")?;
                if default_case.is_some() {
                    return Err(self.error(
                        "Only one 'mac_dinh' block is allowed in a 'truong_hop'",
                    ));
                }
                default_case = Some(body);
            } else {
                let case_pos = self.current_pos();
                let value = self.parse_value()?;
                self.consume(&TokenKind::LBrace, "Expected '{' to start the case body in 'truong_hop'")?;
                let mut body = Vec::new();
                while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
                    body.push(self.parse_child()?);
                }
                self.consume(&TokenKind::RBrace, "Expected '}' to close the case body in 'truong_hop'")?;
                cases.push(CaseNode {
                    value,
                    body,
                    pos: case_pos,
                });
            }
        }

        self.consume(&TokenKind::RBrace, "Expected '}' to close the 'truong_hop' block")?;

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
        // BUG ALREADY FIXED: "vong_lap $i tu 1 den 3" used to be
        // misread as the Each form (since it starts with a Variable),
        // then tried to parse_value() right on the "tu" token - silently
        // wrong (or, after "tu" became the keyword TokenKind::Tu, a
        // clear error: "Could not parse a value: keyword 'tu'"). It must
        // now be correctly distinguished: a Variable followed by
        // "tu"/"from" means a Range WITH a counter variable, not Each.
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
        // BUG ALREADY FIXED (a separate case from the test above - that
        // one happened to use the exact name "i" so it couldn't
        // self-detect this bug): the counter variable name the dev
        // declared used to be parsed CORRECTLY and then IMMEDIATELY
        // DISCARDED (`let _ = first_var;`), with LoopKind::Range having
        // nowhere to store it. A name OTHER THAN "i" is used here to make
        // sure the (fixed) bug can't silently pass by coincidence.
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
                // Not explicitly declared -> defaults to "i".
                assert_eq!(var_name, "i");
            }
            other => panic!("expected LoopKind::Range, got {:?}", other),
        }
    }

    #[test]
    fn test_each_loop_with_trong_keyword() {
        let node = parse_loop_from_source("vong_lap $item trong $ds { }");
        match node.kind {
            LoopKind::Each { item_var, index_var, .. } => {
                assert_eq!(item_var, "item");
                assert!(index_var.is_none());
            }
            other => panic!("expected LoopKind::Each, got {:?}", other),
        }
    }

    #[test]
    fn test_variable_followed_by_neither_trong_nor_tu_is_clear_error() {
        // A leading variable followed by something other than
        // "trong"/"tu" must produce a clear error, no guessing or
        // silent failure.
        let tokens = tokenize("vong_lap $i xyz { }").unwrap();
        let mut p = Parser::new(tokens);
        let result = p.parse_loop_node();
        assert!(result.is_err());
    }

    // ── Tests for the NEW syntax: vong_lap $item, $idx trong $ds ──
    // NEW FEATURE (not a bug fix) - the AST/codegen/runtime were already
    // fully ready beforehand (see CompiledLoop in codegen/control.rs and
    // LoopFrame in vibao-runtime/src/runtime/state.rs); only the parser
    // was missing this syntax until this round of fixes.

    #[test]
    fn test_each_loop_with_index_variable() {
        let node = parse_loop_from_source("vong_lap $task, $idx trong $tasks { }");
        match node.kind {
            LoopKind::Each { item_var, index_var, .. } => {
                assert_eq!(item_var, "task");
                assert_eq!(index_var, Some("idx".to_string()));
            }
            other => panic!("expected LoopKind::Each, got {:?}", other),
        }
    }

    #[test]
    fn test_each_loop_without_index_variable_still_none() {
        // Regression: the OLD syntax (without ", $idx") must continue to
        // work exactly as before - index_var is still None, unaffected
        // by adding the new branch.
        let node = parse_loop_from_source("vong_lap $item trong $ds { }");
        match node.kind {
            LoopKind::Each { index_var, .. } => assert!(index_var.is_none()),
            other => panic!("expected LoopKind::Each, got {:?}", other),
        }
    }

    #[test]
    fn test_each_loop_index_variable_missing_after_comma_is_clear_error() {
        // A ',' is present but NOT followed by a variable (e.g. a number
        // or an unexpected keyword) - must produce a clear error, no
        // panic and no guessing.
        let tokens = tokenize("vong_lap $item, 5 trong $ds { }").unwrap();
        let mut p = Parser::new(tokens);
        let result = p.parse_loop_node();
        assert!(result.is_err(), "must error when ',' is not followed by a valid variable");
    }

    #[test]
    fn test_descending_range_is_rejected_at_parse_time() {
        let tokens = tokenize("vong_lap $i tu 5 den 1 { }").unwrap();
        let mut p = Parser::new(tokens);
        let result = p.parse_loop_node();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("descending ranges are not supported"));
    }

    #[test]
    fn test_range_loop_unaffected_by_index_var_syntax() {
        // Regression: a Range loop (which doesn't use "trong") must not
        // be affected at all by the newly added branch -
        // "vong_lap $i tu 1 den 3" (no comma) must parse exactly as
        // before.
        let node = parse_loop_from_source("vong_lap $i tu 1 den 3 { }");
        assert!(matches!(node.kind, LoopKind::Range { from: 1, to: 3, .. }));
    }
}
