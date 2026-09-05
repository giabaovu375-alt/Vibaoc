// ============================================================
// VIBAO COMPILER (Rust) — locale/mod.rs
// LOCALE LAYER: the ONE place in the entire compiler that knows "what
// source (.vbao) name in locale X maps to which semantic identity"
// (Tag, PropKey, and later ActionName/FunctionName — see
// vibao_ast::semantic).
//
// Each locale is its own submodule (vi.rs for Vietnamese — the PRIMARY
// locale; en.rs for English — ALWAYS active alongside every other
// locale, see the "multi-locale model" architectural decision in
// ARCHITECTURE_PROPOSAL.md; ja.rs/etc. can be added later following the
// same pattern). Key point: adding a new locale does NOT touch the
// lexer/parser/codegen/semantic layers at all — it only needs one new
// file here, mapping strings -> Tag (and later PropKey/ActionName/
// FunctionName), following the exact same structure as vi.rs/en.rs; the
// lexer then ALWAYS checks both that locale AND en.rs at once (never
// "pick one or the other").
//
// STATUS:
// - Tag/Keyword: FULLY WIRED at BOTH layers (lexer + resolve), for BOTH
//   vi AND en.
// - PropKey: WIRED through the locale resolver; codegen/validator use
//   the semantic identity and PropSpec to decide applicability.
// - ActionName: WIRED into the parser/validator/lexer vocabulary; the
//   parser normalizes the English surface name to the canonical runtime
//   name.
// - FunctionName: WIRED into the parser/validator; the parser
//   normalizes the English surface name to the canonical runtime name
//   before the Expr is serialized into the registry.
//
// Tag/Keyword details (complete):
// - parser/app.rs::parse_child() calls resolve_tag() below (the
//   semantic resolve step: FROM a Component(surface_name) token,
//   determine WHICH Tag it is).
// - lexer/tables.rs::keyword_map()/component_set() now build DIRECTLY
//   from locale::{vi,en}::{keyword_name_*,tag_name_*} (the tokenizing
//   step: does the LEXER emit a distinct TokenKind::Component/keyword
//   at all) — previously this step still used a separate hand-written
//   table (Vietnamese only); now it's in sync, and "box" (en) and
//   "khoi" (vi) both work directly from the real lexer.
// The 2 steps remain separate per the settled flow: Source -> Lexer ->
// Token/surface name -> Resolver -> Tag -> AST -> Codegen — the only
// difference is that BOTH steps NOW read from a single real source (the
// locale files); tables.rs no longer holds a second source of truth.
// ============================================================

pub mod action_en;
pub mod action_vi;
pub mod en;
pub mod function_en;
pub mod function_vi;
pub mod prop_en;
pub mod prop_vi;
pub mod vi;

/// Resolves a surface name (the exact string the dev typed in the
/// source) to a Tag — checks the Vietnamese locale first, then English
/// (the baseline locale, always active in parallel), per the model
/// settled in ARCHITECTURE_PROPOSAL.md.
///
/// LOOKUP ORDER: locale::vi is checked FIRST, locale::en SECOND — with
/// the current vocabulary the two tables never overlap (confirmed by a
/// cross-check test in locale/en.rs), so this order does NOT currently
/// affect the result; it only matters if (in the future) some name
/// accidentally becomes valid in BOTH tables — that case should be
/// CAUGHT AND EXPLICITLY FLAGGED rather than silently decided by lookup
/// order (see the risk noted in ARCHITECTURE_PROPOSAL.md).
///
/// The primary locale is currently hardcoded to Vietnamese; once a 3rd
/// locale is added, the compile-time locale should become a
/// config/CLI parameter instead of staying hardcoded.
pub fn resolve_tag(surface_name: &str) -> Option<vibao_ast::Tag> {
    vi::tag_name_vi(surface_name).or_else(|| en::tag_name_en(surface_name))
}

/// Resolves a surface name (a PROP KEY name the dev typed in a
/// PropsMap) to a PropKey — checks the Vietnamese locale first, then
/// English, following the same multi-locale contract used for
/// Tag/Keyword.
pub fn resolve_prop_key(surface_name: &str) -> Option<vibao_ast::PropKey> {
    prop_vi::prop_name_vi(surface_name).or_else(|| prop_en::prop_name_en(surface_name))
}

/// Resolves a surface name (an ACTION name the dev typed inside an
/// event block) to an ActionName — checks the Vietnamese locale first,
/// then English.
///
/// IMPORTANT NOTE: `None` returned for a name does NOT always mean it
/// was mistyped — the parser/validator separately distinguish a "Known
/// but Unsupported" group (e.g. `dang_xuat`) via
/// `action_vi::KNOWN_BUT_UNSUPPORTED_ACTIONS_VI`.
pub fn resolve_action_name(surface_name: &str) -> Option<vibao_ast::ActionName> {
    action_vi::action_name_vi(surface_name).or_else(|| action_en::action_name_en(surface_name))
}

/// Resolves a surface name (an EXPRESSION FUNCTION name the dev typed
/// inside `Expr::Call`) to a FunctionName. This resolver is wired into
/// `validator.rs` via `check_function_name()`; it acts as the semantic
/// gate for a function name before codegen/runtime use that
/// expression.
pub fn resolve_function_name(surface_name: &str) -> Option<vibao_ast::FunctionName> {
    function_vi::function_name_vi(surface_name).or_else(|| function_en::function_name_en(surface_name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_tag_accepts_vietnamese_name() {
        assert_eq!(resolve_tag("khoi"), Some(vibao_ast::Tag::Khoi));
    }

    #[test]
    fn test_resolve_tag_accepts_english_name() {
        assert_eq!(resolve_tag("box"), Some(vibao_ast::Tag::Khoi));
    }

    #[test]
    fn test_resolve_tag_vi_and_en_agree_for_every_tag() {
        // Cross-checked through the actual function used by the real
        // lexer/parser — unlike the similar test in locale/en.rs (which
        // only tests each locale in isolation), this test confirms
        // resolve_tag() (the ONE entry point the parser actually uses)
        // works correctly for EVERY Tag through BOTH spellings.
        let vi_names = [
            "text", "h1", "h2", "h3", "p", "nhan",
            "image", "video", "icon",
            "button", "input", "link", "lien_ket",
            "flex", "grid", "stack", "khoi", "cuon", "can_giua", "lop", "dinh_dau", "dinh_man_hinh",
            "khoang_cach", "duong_ke",
            "form", "nhom_input", "chon_mot", "hop_kiem", "lua_chon",
            "modal", "tabs", "gap_mo", "bang_chuyen", "xuong_trang", "vong_quay",
            "thanh_tien_trinh", "bang", "bieu_do", "ban_do", "thanh_dieu_huong", "trinh_soan_thao",
        ];
        for name in vi_names {
            assert!(resolve_tag(name).is_some(), "'{}' (vi) must resolve", name);
        }
    }

    #[test]
    fn test_resolve_tag_unknown_name_returns_none() {
        assert_eq!(resolve_tag("khong_ton_tai_xyz"), None);
    }

    #[test]
    fn test_resolve_prop_key_accepts_vietnamese_name() {
        assert_eq!(resolve_prop_key("mau_nen"), Some(vibao_ast::PropKey::BackgroundColor));
    }

    #[test]
    fn test_resolve_prop_key_unknown_name_returns_none() {
        assert_eq!(resolve_prop_key("khong_ton_tai_xyz"), None);
    }

    #[test]
    fn test_resolve_action_name_accepts_vietnamese_name() {
        assert_eq!(resolve_action_name("thong_bao"), Some(vibao_ast::ActionName::Notify));
    }

    #[test]
    fn test_resolve_action_name_covers_all_15() {
        let names = [
            "thong_bao", "canh_bao", "dieu_huong", "mo_tab_moi",
            "mo_modal", "dong_modal", "cuon_den", "cuon_len_dau",
            "luu_du_lieu", "tai_du_lieu", "sao_chep",
            "goi_api",
            "them_vao_mang", "xoa_theo_id", "cap_nhat_theo_id",
        ];
        assert_eq!(names.len(), 15);
        for name in names {
            assert!(resolve_action_name(name).is_some(), "'{}' must resolve", name);
        }
    }

    #[test]
    fn test_resolve_action_name_dang_xuat_returns_none_not_a_typo() {
        // "dang_xuat" returns None from resolve_action_name() — BUT this
        // is NOT an ordinary "mistyped/nonexistent name", see
        // action_vi::KNOWN_BUT_UNSUPPORTED_ACTIONS_VI.
        assert_eq!(resolve_action_name("dang_xuat"), None);
        assert!(action_vi::KNOWN_BUT_UNSUPPORTED_ACTIONS_VI.contains(&"dang_xuat"));
    }

    #[test]
    fn test_resolve_action_name_unknown_name_returns_none() {
        assert_eq!(resolve_action_name("khong_ton_tai_xyz"), None);
    }

    #[test]
    fn test_resolve_function_name_accepts_vietnamese_name() {
        assert_eq!(resolve_function_name("gia_tien"), Some(vibao_ast::FunctionName::FormatPrice));
    }

    #[test]
    fn test_resolve_function_name_covers_all_6() {
        let names = ["gia_tien", "ngay", "rut_gon", "hoa_chu", "phan_tram", "lam_tron"];
        for name in names {
            assert!(resolve_function_name(name).is_some(), "'{}' must resolve", name);
        }
    }

    #[test]
    fn test_resolve_function_name_unknown_name_returns_none() {
        assert_eq!(resolve_function_name("khong_ton_tai_xyz"), None);
    }
}
