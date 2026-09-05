// ============================================================
// VIBAO COMPILER (Rust) — lexer/tests.rs
// UNIT TESTS — run with `cargo test`
// ============================================================

use super::tokenize;
use super::token::TokenKind;

#[test]
fn test_basic_app() {
    let src = r#"ung_dung("Test") { trang("/") { } }"#;
    let toks = tokenize(src).unwrap();
    assert_eq!(toks[0].kind, TokenKind::UngDung);
    assert_eq!(toks[1].kind, TokenKind::LParen);
    assert!(matches!(toks[2].kind, TokenKind::StringLit(ref s) if s == "Test"));
}

#[test]
fn test_pascalcase_identifier_keeps_original_casing() {
    // A test ISOLATED at the lexer layer (not going through the parser)
    // — if this test passes but the equivalent test in parser/app.rs
    // (test_component_param_name_matching_builtin_tag/
    // test_component_name_matching_color_name) still fails, the bug is
    // NOT in the lexer but somewhere in the parser — narrows down the
    // debugging scope clearly.
    //
    // CORRECTION (after actually running it): the original assertion
    // `toks.len() == 1` was WRONG — tokenize() always appends an Eof
    // token at the end, so the correct length is 2. The real data
    // confirmed Identifier("TheThe") DID keep its casing correctly — the
    // lexer was working correctly; the bug was only in the test.
    let toks = tokenize("TheThe").unwrap();
    assert_eq!(toks.len(), 2, "1 Identifier + 1 Eof: {:?}", toks);
    match &toks[0].kind {
        TokenKind::Identifier(s) => assert_eq!(s, "TheThe", "must PRESERVE casing, not lowercase to \"thethe\""),
        other => panic!("expected Identifier(\"TheThe\"), got {:?}", other),
    }
}

#[test]
fn test_uppercase_color_name_keeps_original_casing() {
    let toks = tokenize("Do").unwrap();
    assert_eq!(toks.len(), 2, "1 ColorName + 1 Eof: {:?}", toks);
    match &toks[0].kind {
        TokenKind::ColorName(s) => assert_eq!(s, "Do", "must PRESERVE casing, not lowercase to \"do\""),
        other => panic!("expected ColorName(\"Do\"), got {:?}", other),
    }
}

#[test]
fn test_identifier_with_diacritics_normalizes_like_ascii() {
    // "mau" (with diacritics, no spaces) must produce the SAME token as "mau" (without diacritics).
    let toks_dau = tokenize(r#"text("x", màu: xanh)"#).unwrap();
    let toks_khong_dau = tokenize(r#"text("x", mau: xanh)"#).unwrap();
    assert_eq!(toks_dau.len(), toks_khong_dau.len());
    for (a, b) in toks_dau.iter().zip(toks_khong_dau.iter()) {
        assert_eq!(a.kind, b.kind, "the with-diacritics and without-diacritics tokens must match");
    }
}

#[test]
fn test_quoted_multiword_identifier_matches_underscore_form() {
    // 'mau chu' (single quotes, with a space) must produce the SAME
    // token as "mau_chu" (written without spaces, no diacritics) - both
    // spellings must be exactly equivalent since they go through the
    // same classify_identifier(). Switched from backticks to single
    // quotes - a single quote is easier to type on a mobile virtual
    // keyboard (sits on the first symbol layer, instead of requiring a
    // switch to the "123"->"#+=" layer like a backtick does).
    let toks_quoted = tokenize("text(\"x\", 'màu chữ': xanh)").unwrap();
    let toks_underscore = tokenize(r#"text("x", mau_chu: xanh)"#).unwrap();
    assert_eq!(toks_quoted.len(), toks_underscore.len());
    for (a, b) in toks_quoted.iter().zip(toks_underscore.iter()) {
        assert_eq!(a.kind, b.kind, "the single-quoted and underscore tokens must match");
    }
}

#[test]
fn test_string_literal_diacritics_untouched() {
    // "Xin chao" (with diacritics, the displayed content) must NOT be
    // normalized - its diacritics must be preserved, since this is
    // text displayed to the end user, completely different from an
    // identifier (a variable/prop name).
    let toks = tokenize(r#"text("Xin chào")"#).unwrap();
    let has_original_string = toks.iter().any(|t| {
        matches!(&t.kind, TokenKind::StringLit(s) if s == "Xin chào")
    });
    assert!(has_original_string, "a displayed string must keep its Vietnamese diacritics");
}

#[test]
fn test_unclosed_quoted_identifier_errors() {
    let result = tokenize("text('mau chu: xanh)");
    assert!(result.is_err());
}

#[test]
fn test_empty_quoted_identifier_errors() {
    let result = tokenize("text('': xanh)");
    assert!(result.is_err());
}

#[test]
fn test_quoted_identifier_can_be_a_keyword() {
    // 'trang' (single-quoted, even though it's just one word) must
    // still be classified correctly as the TRANG keyword when it starts
    // a statement - confirms classify_identifier() shares the same
    // logic, with no separate branch missing for the single-quote
    // case.
    let toks = tokenize("'trang'(\"/\") { }").unwrap();
    assert_eq!(toks[0].kind, TokenKind::Trang);
}


#[test]
fn test_trang_as_keyword_vs_color() {
    // "trang" at the start of a statement -> the TRANG keyword
    let toks1 = tokenize(r#"trang("/")"#).unwrap();
    assert_eq!(toks1[0].kind, TokenKind::Trang);

    // "trang" after a ':' -> the color white (COLOR_NAME)
    let toks2 = tokenize(r#"color:trang"#).unwrap();
    assert!(matches!(toks2[2].kind, TokenKind::ColorName(ref s) if s == "trang"));
}

#[test]
fn test_minus_as_operator_vs_negative() {
    // "$n - 1" (spaced apart) -> variable, subtraction operator, positive number
    let toks1 = tokenize(r#"$n - 1"#).unwrap();
    assert!(matches!(toks1[0].kind, TokenKind::Variable(ref s) if s == "n"));
    assert_eq!(toks1[1].kind, TokenKind::Minus);
    assert!(matches!(toks1[2].kind, TokenKind::NumberLit(v, _) if v == 1.0));

    // "-5" (at the start of an expression, no space) -> negative number
    let toks2 = tokenize(r#"-5"#).unwrap();
    assert!(matches!(toks2[0].kind, TokenKind::NumberLit(v, _) if v == -5.0));
}

#[test]
fn test_number_with_unit() {
    let toks = tokenize(r#"50%"#).unwrap();
    assert!(matches!(toks[0].kind, TokenKind::NumberLit(v, ref raw) if v == 50.0 && raw == "50%"));
}

#[test]
fn test_string_with_vietnamese() {
    let toks = tokenize(r#""Xin chào ViBao! 🐧""#).unwrap();
    assert!(matches!(toks[0].kind, TokenKind::StringLit(ref s) if s == "Xin chào ViBao! 🐧"));
}

#[test]
fn test_unclosed_string_errors() {
    let result = tokenize(r#""chưa đóng"#);
    assert!(result.is_err());
}

#[test]
fn test_bang_operator_standalone() {
    // A standalone '!' must produce Bang, not get confused with '!=' (Neq)
    let toks = tokenize(r#"!$da_dang_nhap"#).unwrap();
    assert_eq!(toks[0].kind, TokenKind::Bang);
    assert!(matches!(toks[1].kind, TokenKind::Variable(ref s) if s == "da_dang_nhap"));

    let toks2 = tokenize(r#"$a != $b"#).unwrap();
    assert!(matches!(toks2[0].kind, TokenKind::Variable(_)));
    assert_eq!(toks2[1].kind, TokenKind::Neq);
}

#[test]
fn test_percent_operator_vs_unit() {
    // "50%" directly attached to a digit -> a CSS unit suffix, still a single NumberLit
    let toks1 = tokenize(r#"50%"#).unwrap();
    assert!(matches!(toks1[0].kind, TokenKind::NumberLit(v, ref raw) if v == 50.0 && raw == "50%"));

    // "$n % 2" (spaced apart) -> a standalone modulo operator
    let toks2 = tokenize(r#"$n % 2"#).unwrap();
    assert!(matches!(toks2[0].kind, TokenKind::Variable(ref s) if s == "n"));
    assert_eq!(toks2[1].kind, TokenKind::Percent);
    assert!(matches!(toks2[2].kind, TokenKind::NumberLit(v, _) if v == 2.0));

    // "$n%2" (no spaces, not following a digit) -> must still be modulo,
    // must not be mistaken for a number since '%' here directly follows
    // a Variable, not a digit
    let toks3 = tokenize(r#"$n%2"#).unwrap();
    assert!(matches!(toks3[0].kind, TokenKind::Variable(ref s) if s == "n"));
    assert_eq!(toks3[1].kind, TokenKind::Percent);
    assert!(matches!(toks3[2].kind, TokenKind::NumberLit(v, _) if v == 2.0));
}

#[test]
fn test_all_event_keywords() {
    // Confirms all 7 events in EventName (ast.rs) have a corresponding
    // lexer keyword - on_blur/on_focus/on_scroll were previously
    // missing.
    assert_eq!(tokenize("on_click").unwrap()[0].kind, TokenKind::OnClick);
    assert_eq!(tokenize("on_hover").unwrap()[0].kind, TokenKind::OnHover);
    assert_eq!(tokenize("on_blur").unwrap()[0].kind, TokenKind::OnBlur);
    assert_eq!(tokenize("on_focus").unwrap()[0].kind, TokenKind::OnFocus);
    assert_eq!(tokenize("on_change").unwrap()[0].kind, TokenKind::OnChange);
    assert_eq!(tokenize("on_submit").unwrap()[0].kind, TokenKind::OnSubmit);
    assert_eq!(tokenize("on_scroll").unwrap()[0].kind, TokenKind::OnScroll);
}

#[test]
fn test_language_header_tokens_include_semicolon() {
    let tokens = crate::lexer::tokenize("lang = \"vi\";").unwrap();
    assert!(matches!(tokens[0].kind, TokenKind::Identifier(ref name) if name == "lang"));
    assert!(matches!(tokens[1].kind, TokenKind::Equals));
    assert!(matches!(tokens[2].kind, TokenKind::StringLit(ref value) if value == "vi"));
    assert!(matches!(tokens[3].kind, TokenKind::Semicolon));
}
