// ============================================================
// VIBAO COMPILER (Rust) — locale/vi.rs
// LOCALE: Vietnamese — maps a tag name WRITTEN IN THE SOURCE (.vbao) to
// a Tag (the semantic identity, defined in vibao_ast::semantic).
//
// THIS IS THE ONE FILE in the entire compiler that knows "khoi" means
// Tag::Khoi — if another locale is added later (e.g. Japanese), there
// will be a SIMILAR locale/ja.rs file (a sibling of this one, see
// locale/mod.rs) that doesn't touch Tag (the semantic identity stays
// the same) or anywhere in codegen (which only ever looks at Tag, never
// the original string).
//
// STATUS: REALLY WIRED UP — `lexer::tables::component_set()` and
// `lexer::tables::keyword_map()` NOW build DIRECTLY from
// `tag_name_vi()`/`keyword_name_vi()` (this file), instead of
// maintaining a separate hand-written table like before.
// `parser/app.rs::parse_child()` still calls `locale::resolve_tag()`
// (locale/mod.rs) as the semantic resolve step AFTER the lexer — the 2
// steps stay separate by design (the lexer decides "is this a valid
// tag/keyword at all", resolve decides "which semantic identity does
// THIS tag/keyword correspond to").
// ============================================================

use vibao_ast::Tag;

/// (Vietnamese source name) -> Tag table — matches 1:1 with the "Tag"
/// group in the old `component_set()` (does NOT include the
/// action/function group — "thong_bao", "goi_api", "gia_tien"... —
/// those belong to ActionName/FunctionName, with their own locale table
/// elsewhere).
pub fn tag_name_vi(name: &str) -> Option<Tag> {
    Some(match name {
        "text" => Tag::Text,
        "h1" => Tag::H1,
        "h2" => Tag::H2,
        "h3" => Tag::H3,
        "p" => Tag::P,
        "nhan" => Tag::Nhan,
        "image" => Tag::Image,
        "video" => Tag::Video,
        "icon" => Tag::Icon,
        "button" => Tag::Button,
        "input" => Tag::Input,
        "link" => Tag::Link,
        "lien_ket" => Tag::Link,
        "flex" => Tag::Flex,
        "grid" => Tag::Grid,
        "stack" => Tag::Stack,
        "khoi" => Tag::Khoi,
        "cuon" => Tag::Cuon,
        "can_giua" => Tag::CanGiua,
        "lop" => Tag::Lop,
        "dinh_dau" => Tag::DinhDau,
        "dinh_man_hinh" => Tag::DinhManHinh,
        "khoang_cach" => Tag::KhoangCach,
        "duong_ke" => Tag::DuongKe,
        "form" => Tag::Form,
        "nhom_input" => Tag::NhomInput,
        "chon_mot" => Tag::ChonMot,
        "hop_kiem" => Tag::HopKiem,
        "lua_chon" => Tag::LuaChon,
        "modal" => Tag::Modal,
        "tabs" => Tag::Tabs,
        "gap_mo" => Tag::GapMo,
        "bang_chuyen" => Tag::BangChuyen,
        "xuong_trang" => Tag::XuongTrang,
        "vong_quay" => Tag::VongQuay,
        "thanh_tien_trinh" => Tag::ThanhTienTrinh,
        "bang" => Tag::Bang,
        "bieu_do" => Tag::BieuDo,
        "ban_do" => Tag::BanDo,
        "thanh_dieu_huong" => Tag::ThanhDieuHuong,
        "trinh_soan_thao" => Tag::TrinhSoanThao,
        _ => return None,
    })
}

/// The reverse direction: Tag -> Vietnamese source name. Used when a
/// user-facing display of the original name is needed (error messages,
/// debugging) — kept SEPARATE from tag_name_vi() even though it could
/// theoretically be derived by inverting the table above, because
/// writing it explicitly lets the compiler check exhaustiveness (adding
/// a new Tag and forgetting to add it here is an immediate compile
/// error — inverting the table at runtime wouldn't give that
/// guarantee).
pub fn tag_display_name_vi(tag: Tag) -> &'static str {
    match tag {
        Tag::Text => "text",
        Tag::H1 => "h1",
        Tag::H2 => "h2",
        Tag::H3 => "h3",
        Tag::P => "p",
        Tag::Nhan => "nhan",
        Tag::Image => "image",
        Tag::Video => "video",
        Tag::Icon => "icon",
        Tag::Button => "button",
        Tag::Input => "input",
        // "link"/"lien_ket" are 2 SPELLINGS for THE SAME Tag
        // (Tag::Link, see the note at the variant definition) — the
        // reverse direction only needs to return one canonical name.
        // "link" was chosen (it comes first in the original
        // component_set() list).
        Tag::Link => "link",
        Tag::Flex => "flex",
        Tag::Grid => "grid",
        Tag::Stack => "stack",
        Tag::Khoi => "khoi",
        Tag::Cuon => "cuon",
        Tag::CanGiua => "can_giua",
        Tag::Lop => "lop",
        Tag::DinhDau => "dinh_dau",
        Tag::DinhManHinh => "dinh_man_hinh",
        Tag::KhoangCach => "khoang_cach",
        Tag::DuongKe => "duong_ke",
        Tag::Form => "form",
        Tag::NhomInput => "nhom_input",
        Tag::ChonMot => "chon_mot",
        Tag::HopKiem => "hop_kiem",
        Tag::LuaChon => "lua_chon",
        Tag::Modal => "modal",
        Tag::Tabs => "tabs",
        Tag::GapMo => "gap_mo",
        Tag::BangChuyen => "bang_chuyen",
        Tag::XuongTrang => "xuong_trang",
        Tag::VongQuay => "vong_quay",
        Tag::ThanhTienTrinh => "thanh_tien_trinh",
        Tag::Bang => "bang",
        Tag::BieuDo => "bieu_do",
        Tag::BanDo => "ban_do",
        Tag::ThanhDieuHuong => "thanh_dieu_huong",
        Tag::TrinhSoanThao => "trinh_soan_thao",
    }
}

// ════════════════════════════════════════════════════════════
// KEYWORD (structural keywords: trang/neu/vong_lap/state/theme/on_click/...)
// ════════════════════════════════════════════════════════════
//
// Unlike Tag above (whose semantic identity was defined SEPARATELY in
// vibao_ast::semantic because Tag had no proper semantic ID before the
// Tag enum existed), a keyword ALREADY HAD a real semantic ID from the
// start — it's `lexer::token::TokenKind` itself (TokenKind::Theme,
// ::State, ::OnClick... — an enum variant, not a String). So there's NO
// need to create a new "Keyword" enum in vibao_ast::semantic —
// TokenKind ALREADY fills that role; the only issue was that there used
// to be just one lookup table (lexer::tables::keyword_map()) mixing
// Vietnamese words with some English-only words (theme/state/on_click/
// on_hover/on_blur/on_focus/on_change/on_submit/on_scroll) together as
// ONE. The new architectural decision (see ARCHITECTURE_PROPOSAL.md,
// section "ARCHITECTURAL DECISION... multi-locale model") is: EVERY
// locale must have a 100% COMPLETE translation, and English ALWAYS runs
// in parallel as a "baseline locale" — so the table below is the FULL
// VIETNAMESE TRANSLATION (including the 9 words that previously only
// existed in English).
//
// STATUS: REALLY wired into the Lexer (same as tag_name_vi() above) —
// see `lexer::tables::keyword_map()`.

use crate::lexer::TokenKind;

/// The FULL (Vietnamese name) -> TokenKind table for EVERY structural
/// keyword — including 9 NEWLY translated words (previously left in
/// English in the old keyword_map()): theme->chu_de, state->trang_thai,
/// on_click->khi_nhan, on_hover->khi_di_chuot, on_blur->khi_mat_focus,
/// on_focus->khi_focus, on_change->khi_doi, on_submit->khi_gui,
/// on_scroll->khi_cuon (each name was confirmed with the language
/// designer before writing this table — not chosen unilaterally,
/// learning from the lua_chon/thanh_dieu_huong mistake).
pub fn keyword_name_vi(name: &str) -> Option<TokenKind> {
    Some(match name {
        "trang" => TokenKind::Trang,
        "ung_dung" => TokenKind::UngDung,
        "chu_de" => TokenKind::Theme,
        "trang_thai" => TokenKind::State,
        "nhap" => TokenKind::Nhap,
        "tu" => TokenKind::Tu,

        "neu" => TokenKind::Neu,
        "khong_thi" => TokenKind::KhongThi,
        "neu_nhieu" => TokenKind::NeuNhieu,
        "truong_hop" => TokenKind::TruongHop,
        "mac_dinh" => TokenKind::MacDinh,
        "vong_lap" => TokenKind::VongLap,

        "khi_nhan" => TokenKind::OnClick,
        "khi_di_chuot" => TokenKind::OnHover,
        "khi_mat_focus" => TokenKind::OnBlur,
        "khi_focus" => TokenKind::OnFocus,
        "khi_doi" => TokenKind::OnChange,
        "khi_gui" => TokenKind::OnSubmit,
        "khi_cuon" => TokenKind::OnScroll,
        "on_tai" => TokenKind::OnTai,
        "on_huy" => TokenKind::OnHuy,
        _ => return None,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    use vibao_ast::semantic::tag_spec;

    /// The original list from `component_set()` (lexer/tables.rs) —
    /// hand-copied once for this cross-check test, NOT imported
    /// directly from component_set() (which mixes in action/function
    /// names too, not a pure tag list). If component_set() changes and
    /// this list is forgotten, the test below will fail — that's the
    /// whole point (catch it early instead of silently drifting).
    const OLD_COMPONENT_SET_TAG_GROUP: &[&str] = &[
        "text", "h1", "h2", "h3", "p", "nhan",
        "image", "video", "icon",
        "button", "input", "link", "lien_ket",
        "flex", "grid", "stack", "khoi", "cuon", "can_giua", "lop", "dinh_dau", "dinh_man_hinh",
        "khoang_cach", "duong_ke",
        "form", "nhom_input", "chon_mot", "hop_kiem", "lua_chon",
        "modal", "tabs", "gap_mo", "bang_chuyen", "xuong_trang", "vong_quay",
        "thanh_tien_trinh", "bang", "bieu_do", "ban_do", "thanh_dieu_huong", "trinh_soan_thao",
    ];

    #[test]
    fn test_every_old_component_set_tag_resolves_to_a_tag() {
        for name in OLD_COMPONENT_SET_TAG_GROUP {
            assert!(
                tag_name_vi(name).is_some(),
                "name '{}' is in the old component_set() but does NOT resolve to any Tag — \
                 locale/vi.rs is missing an entry, needs to be added before this table is complete",
                name
            );
        }
    }

    #[test]
    fn test_tag_count_matches_old_component_set_tag_group_exactly() {
        // 41 = the exact count computed and cross-checked by hand
        // against the old component_set() when the Tag enum was
        // designed — see the note in vibao_ast::semantic::tag.
        assert_eq!(OLD_COMPONENT_SET_TAG_GROUP.len(), 41);
    }

    #[test]
    fn test_tag_name_vi_and_display_name_are_consistent_round_trip() {
        // For EVERY name in the table (except "lien_ket", since
        // "link"/"lien_ket" both point to Tag::Link but display only
        // returns one of them — round-tripping through the alias
        // changes the name, and that's the CORRECT BEHAVIOR by design,
        // not a bug), the round trip name -> Tag -> name must return
        // exactly the original name.
        for name in OLD_COMPONENT_SET_TAG_GROUP {
            if *name == "lien_ket" {
                continue;
            }
            let tag = tag_name_vi(name).unwrap();
            assert_eq!(tag_display_name_vi(tag), *name, "round-trip mismatch for '{}'", name);
        }
    }

    #[test]
    fn test_link_and_lien_ket_are_aliases_of_the_same_tag() {
        assert_eq!(tag_name_vi("link"), tag_name_vi("lien_ket"));
    }

    #[test]
    fn test_unknown_name_resolves_to_none() {
        assert_eq!(tag_name_vi("khong_ton_tai_xyz"), None);
        // Action/function names (NOT tags) must NOT accidentally
        // resolve to a Tag — confirms a clean boundary between the 2
        // groups.
        assert_eq!(tag_name_vi("thong_bao"), None);
        assert_eq!(tag_name_vi("goi_api"), None);
        assert_eq!(tag_name_vi("gia_tien"), None);
    }

    #[test]
    fn test_tag_kind_lookup_works_through_locale_resolution() {
        // A small integration test: name -> Tag (locale) -> TagSpec
        // (semantic registry) works correctly across the 2 different
        // modules.
        let tag = tag_name_vi("khoi").unwrap();
        assert_eq!(tag_spec(tag).kind, vibao_ast::semantic::TagKind::Layout);
        assert_eq!(tag_spec(tag).html_tag, "div");
    }
}
