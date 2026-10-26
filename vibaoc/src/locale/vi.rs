use vibao_ast::Tag;

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
        "vung_van_ban" => Tag::Textarea,
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
        Tag::Textarea => "vung_van_ban",
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

use crate::lexer::TokenKind;

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
        assert_eq!(OLD_COMPONENT_SET_TAG_GROUP.len(), 41);
    }

    #[test]
    fn test_tag_name_vi_and_display_name_are_consistent_round_trip() {
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
        assert_eq!(tag_name_vi("thong_bao"), None);
        assert_eq!(tag_name_vi("goi_api"), None);
        assert_eq!(tag_name_vi("gia_tien"), None);
    }

    #[test]
    fn test_tag_kind_lookup_works_through_locale_resolution() {
        let tag = tag_name_vi("khoi").unwrap();
        assert_eq!(tag_spec(tag).kind, vibao_ast::semantic::TagKind::Layout);
        assert_eq!(tag_spec(tag).html_tag, "div");
    }
}
