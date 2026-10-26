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
    let toks_dau = tokenize(r#"text("x", màu: xanh)"#).unwrap();
    let toks_khong_dau = tokenize(r#"text("x", mau: xanh)"#).unwrap();
    assert_eq!(toks_dau.len(), toks_khong_dau.len());
    for (a, b) in toks_dau.iter().zip(toks_khong_dau.iter()) {
        assert_eq!(a.kind, b.kind, "the with-diacritics and without-diacritics tokens must match");
    }
}

#[test]
fn test_quoted_multiword_identifier_matches_underscore_form() {
    let toks_quoted = tokenize("text(\"x\", 'màu chữ': xanh)").unwrap();
    let toks_underscore = tokenize(r#"text("x", mau_chu: xanh)"#).unwrap();
    assert_eq!(toks_quoted.len(), toks_underscore.len());
    for (a, b) in toks_quoted.iter().zip(toks_underscore.iter()) {
        assert_eq!(a.kind, b.kind, "the single-quoted and underscore tokens must match");
    }
}

#[test]
fn test_string_literal_diacritics_untouched() {
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
    let toks = tokenize("'trang'(\"/\") { }").unwrap();
    assert_eq!(toks[0].kind, TokenKind::Trang);
}

#[test]
fn test_trang_as_keyword_vs_color() {
    let toks1 = tokenize(r#"trang("/")"#).unwrap();
    assert_eq!(toks1[0].kind, TokenKind::Trang);

    let toks2 = tokenize(r#"color:trang"#).unwrap();
    assert!(matches!(toks2[2].kind, TokenKind::ColorName(ref s) if s == "trang"));
}

#[test]
fn test_minus_as_operator_vs_negative() {
    let toks1 = tokenize(r#"$n - 1"#).unwrap();
    assert!(matches!(toks1[0].kind, TokenKind::Variable(ref s) if s == "n"));
    assert_eq!(toks1[1].kind, TokenKind::Minus);
    assert!(matches!(toks1[2].kind, TokenKind::NumberLit(v, _) if v == 1.0));

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
    let toks = tokenize(r#"!$da_dang_nhap"#).unwrap();
    assert_eq!(toks[0].kind, TokenKind::Bang);
    assert!(matches!(toks[1].kind, TokenKind::Variable(ref s) if s == "da_dang_nhap"));

    let toks2 = tokenize(r#"$a != $b"#).unwrap();
    assert!(matches!(toks2[0].kind, TokenKind::Variable(_)));
    assert_eq!(toks2[1].kind, TokenKind::Neq);
}

#[test]
fn test_percent_operator_vs_unit() {
    let toks1 = tokenize(r#"50%"#).unwrap();
    assert!(matches!(toks1[0].kind, TokenKind::NumberLit(v, ref raw) if v == 50.0 && raw == "50%"));

    let toks2 = tokenize(r#"$n % 2"#).unwrap();
    assert!(matches!(toks2[0].kind, TokenKind::Variable(ref s) if s == "n"));
    assert_eq!(toks2[1].kind, TokenKind::Percent);
    assert!(matches!(toks2[2].kind, TokenKind::NumberLit(v, _) if v == 2.0));

    let toks3 = tokenize(r#"$n%2"#).unwrap();
    assert!(matches!(toks3[0].kind, TokenKind::Variable(ref s) if s == "n"));
    assert_eq!(toks3[1].kind, TokenKind::Percent);
    assert!(matches!(toks3[2].kind, TokenKind::NumberLit(v, _) if v == 2.0));
}

#[test]
fn test_all_event_keywords() {
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

#[test]
fn test_variable_with_diacritics_is_normalized() {
    let toks = tokenize("$đường_dẫn $Kỹ_năng $nền").unwrap();
    assert!(matches!(toks[0].kind, TokenKind::Variable(ref s) if s == "duong_dan"));
    assert!(matches!(toks[1].kind, TokenKind::Variable(ref s) if s == "Ky_nang"));
    assert!(matches!(toks[2].kind, TokenKind::Variable(ref s) if s == "nen"));
}
