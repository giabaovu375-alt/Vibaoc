use vibao_ast::Tag;

pub fn tag_name_en(name: &str) -> Option<Tag> {
    Some(match name {
        "text" => Tag::Text,
        "h1" => Tag::H1,
        "h2" => Tag::H2,
        "h3" => Tag::H3,
        "p" => Tag::P,
        "label" => Tag::Nhan,
        "image" => Tag::Image,
        "video" => Tag::Video,
        "icon" => Tag::Icon,
        "button" => Tag::Button,
        "input" => Tag::Input,
        "textarea" => Tag::Textarea,
        "link" => Tag::Link,
        "flex" => Tag::Flex,
        "grid" => Tag::Grid,
        "stack" => Tag::Stack,
        "box" => Tag::Khoi,
        "scroll" => Tag::Cuon,
        "container" => Tag::CanGiua,
        "layer" => Tag::Lop,
        "sticky" => Tag::DinhDau,
        "fixed" => Tag::DinhManHinh,
        "spacer" => Tag::KhoangCach,
        "divider" => Tag::DuongKe,
        "form" => Tag::Form,
        "input_group" => Tag::NhomInput,
        "radio" => Tag::ChonMot,
        "checkbox" => Tag::HopKiem,
        "select" => Tag::LuaChon,
        "modal" => Tag::Modal,
        "tabs" => Tag::Tabs,
        "accordion" => Tag::GapMo,
        "carousel" => Tag::BangChuyen,
        "pagination" => Tag::XuongTrang,
        "spinner" => Tag::VongQuay,
        "progress" => Tag::ThanhTienTrinh,
        "table" => Tag::Bang,
        "chart" => Tag::BieuDo,
        "map" => Tag::BanDo,
        "nav" => Tag::ThanhDieuHuong,
        "editor" => Tag::TrinhSoanThao,
        _ => return None,
    })
}

#[cfg(test)]
pub fn tag_display_name_en(tag: Tag) -> &'static str {
    match tag {
        Tag::Text => "text",
        Tag::H1 => "h1",
        Tag::H2 => "h2",
        Tag::H3 => "h3",
        Tag::P => "p",
        Tag::Nhan => "label",
        Tag::Image => "image",
        Tag::Video => "video",
        Tag::Icon => "icon",
        Tag::Button => "button",
        Tag::Input => "input",
        Tag::Textarea => "textarea",
        Tag::Link => "link",
        Tag::Flex => "flex",
        Tag::Grid => "grid",
        Tag::Stack => "stack",
        Tag::Khoi => "box",
        Tag::Cuon => "scroll",
        Tag::CanGiua => "container",
        Tag::Lop => "layer",
        Tag::DinhDau => "sticky",
        Tag::DinhManHinh => "fixed",
        Tag::KhoangCach => "spacer",
        Tag::DuongKe => "divider",
        Tag::Form => "form",
        Tag::NhomInput => "input_group",
        Tag::ChonMot => "radio",
        Tag::HopKiem => "checkbox",
        Tag::LuaChon => "select",
        Tag::Modal => "modal",
        Tag::Tabs => "tabs",
        Tag::GapMo => "accordion",
        Tag::BangChuyen => "carousel",
        Tag::XuongTrang => "pagination",
        Tag::VongQuay => "spinner",
        Tag::ThanhTienTrinh => "progress",
        Tag::Bang => "table",
        Tag::BieuDo => "chart",
        Tag::BanDo => "map",
        Tag::ThanhDieuHuong => "nav",
        Tag::TrinhSoanThao => "editor",
    }
}

use crate::lexer::TokenKind;

pub fn keyword_name_en(name: &str) -> Option<TokenKind> {
    Some(match name {
        "page" => TokenKind::Trang,
        "app" => TokenKind::UngDung,
        "theme" => TokenKind::Theme,
        "state" => TokenKind::State,
        "import" => TokenKind::Nhap,
        "from" => TokenKind::Tu,

        "if" => TokenKind::Neu,
        "else" => TokenKind::KhongThi,
        "else_if" => TokenKind::NeuNhieu,
        "switch" => TokenKind::TruongHop,
        "default" => TokenKind::MacDinh,
        "loop" => TokenKind::VongLap,

        "on_click" => TokenKind::OnClick,
        "on_hover" => TokenKind::OnHover,
        "on_blur" => TokenKind::OnBlur,
        "on_focus" => TokenKind::OnFocus,
        "on_change" => TokenKind::OnChange,
        "on_submit" => TokenKind::OnSubmit,
        "on_scroll" => TokenKind::OnScroll,
        "on_load" => TokenKind::OnTai,
        "on_unload" => TokenKind::OnHuy,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use vibao_ast::semantic::tag_spec;

    const ALL_TAG_NAMES_EN: &[&str] = &[
        "text",
        "h1",
        "h2",
        "h3",
        "p",
        "label",
        "image",
        "video",
        "icon",
        "button",
        "input",
        "textarea",
        "link",
        "flex",
        "grid",
        "stack",
        "box",
        "scroll",
        "container",
        "layer",
        "sticky",
        "fixed",
        "spacer",
        "divider",
        "form",
        "input_group",
        "radio",
        "checkbox",
        "select",
        "modal",
        "tabs",
        "accordion",
        "carousel",
        "pagination",
        "spinner",
        "progress",
        "table",
        "chart",
        "map",
        "nav",
        "editor",
    ];

    #[test]
    fn test_tag_count_matches_41() {
        assert_eq!(ALL_TAG_NAMES_EN.len(), 41);
    }

    #[test]
    fn test_every_tag_name_resolves() {
        for name in ALL_TAG_NAMES_EN {
            assert!(
                tag_name_en(name).is_some(),
                "name '{}' does not resolve to any Tag",
                name
            );
        }
    }

    #[test]
    fn test_round_trip_name_to_tag_to_name() {
        for name in ALL_TAG_NAMES_EN {
            let tag = tag_name_en(name).unwrap();
            assert_eq!(
                tag_display_name_en(tag),
                *name,
                "round-trip mismatch for '{}'",
                name
            );
        }
    }

    #[test]
    fn test_unknown_name_resolves_to_none() {
        assert_eq!(tag_name_en("khong_ton_tai"), None);
        assert_eq!(tag_name_en("khoi"), None);
    }

    #[test]
    fn test_tag_kind_lookup_works_through_en_locale_resolution() {
        let tag = tag_name_en("box").unwrap();
        assert_eq!(tag_spec(tag).kind, vibao_ast::semantic::TagKind::Layout);
        assert_eq!(tag_spec(tag).html_tag, "div");
    }

    #[test]
    fn test_vi_and_en_locale_agree_on_every_tag_via_registry() {
        for name in ALL_TAG_NAMES_EN {
            let tag_en = tag_name_en(name).unwrap();
            let name_vi = super::super::vi::tag_display_name_vi(tag_en);
            let tag_vi = super::super::vi::tag_name_vi(name_vi).unwrap();
            assert_eq!(
                tag_en, tag_vi,
                "Tag mismatch when round-tripping through both locales for '{}'",
                name
            );
            assert_eq!(
                tag_spec(tag_en).kind,
                tag_spec(tag_vi).kind,
                "TagKind mismatch between the two locales for the Tag of '{}'",
                name
            );
        }
    }

    #[test]
    fn test_keyword_name_en_covers_all_21() {
        for name in [
            "page",
            "app",
            "theme",
            "state",
            "import",
            "from",
            "if",
            "else",
            "else_if",
            "switch",
            "default",
            "loop",
            "on_click",
            "on_hover",
            "on_blur",
            "on_focus",
            "on_change",
            "on_submit",
            "on_scroll",
            "on_load",
            "on_unload",
        ] {
            assert!(keyword_name_en(name).is_some(), "'{}' must resolve", name);
        }
    }

    #[test]
    fn test_vi_and_en_keyword_locale_agree_for_all_21_words() {
        let pairs = [
            ("trang", "page"),
            ("ung_dung", "app"),
            ("chu_de", "theme"),
            ("trang_thai", "state"),
            ("nhap", "import"),
            ("tu", "from"),
            ("neu", "if"),
            ("khong_thi", "else"),
            ("neu_nhieu", "else_if"),
            ("truong_hop", "switch"),
            ("mac_dinh", "default"),
            ("vong_lap", "loop"),
            ("khi_nhan", "on_click"),
            ("khi_di_chuot", "on_hover"),
            ("khi_mat_focus", "on_blur"),
            ("khi_focus", "on_focus"),
            ("khi_doi", "on_change"),
            ("khi_gui", "on_submit"),
            ("khi_cuon", "on_scroll"),
            ("on_tai", "on_load"),
            ("on_huy", "on_unload"),
        ];
        assert_eq!(
            pairs.len(),
            21,
            "total of 9 original keywords + 12 newly proposed keywords = 21"
        );
        for (vi_name, en_name) in pairs {
            let from_vi = super::super::vi::keyword_name_vi(vi_name);
            let from_en = keyword_name_en(en_name);
            assert_eq!(
                from_vi, from_en,
                "TokenKind mismatch between locale vi ('{}') and en ('{}')",
                vi_name, en_name
            );
            assert!(from_vi.is_some(), "'{}' (vi) must resolve", vi_name);
        }
    }
}
