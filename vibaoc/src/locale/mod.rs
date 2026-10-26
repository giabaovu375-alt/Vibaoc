pub mod action_en;
pub mod action_vi;
pub mod en;
pub mod function_en;
pub mod function_vi;
pub mod prop_en;
pub mod prop_vi;
pub mod vi;

pub fn resolve_tag(surface_name: &str) -> Option<vibao_ast::Tag> {
    vi::tag_name_vi(surface_name).or_else(|| en::tag_name_en(surface_name))
}

pub fn resolve_prop_key(surface_name: &str) -> Option<vibao_ast::PropKey> {
    prop_vi::prop_name_vi(surface_name).or_else(|| prop_en::prop_name_en(surface_name))
}

pub fn resolve_action_name(surface_name: &str) -> Option<vibao_ast::ActionName> {
    action_vi::action_name_vi(surface_name).or_else(|| action_en::action_name_en(surface_name))
}

pub fn resolve_function_name(surface_name: &str) -> Option<vibao_ast::FunctionName> {
    function_vi::function_name_vi(surface_name)
        .or_else(|| function_en::function_name_en(surface_name))
}

pub fn canonical_action_option_key(name: &str) -> &str {
    match name {
        "type" => "kieu",
        "duration" => "thoi_gian",
        "headers" => "tieu_de",
        "background" => "chay_nen",
        other => other,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponseHandler {
    Success,
    Failure,
}

pub fn resolve_response_handler(name: &str) -> Option<ResponseHandler> {
    match name {
        "thanh_cong" | "success" => Some(ResponseHandler::Success),
        "that_bai" | "failure" => Some(ResponseHandler::Failure),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_action_option_key_english_maps_to_runtime_key() {
        assert_eq!(canonical_action_option_key("type"), "kieu");
        assert_eq!(canonical_action_option_key("duration"), "thoi_gian");
        assert_eq!(canonical_action_option_key("headers"), "tieu_de");
        assert_eq!(canonical_action_option_key("background"), "chay_nen");
        assert_eq!(canonical_action_option_key("kieu"), "kieu");
        assert_eq!(canonical_action_option_key("thoi_gian"), "thoi_gian");
        assert_eq!(canonical_action_option_key("khac"), "khac");
    }

    #[test]
    fn test_response_handler_accepts_both_languages() {
        assert_eq!(
            resolve_response_handler("thanh_cong"),
            Some(ResponseHandler::Success)
        );
        assert_eq!(
            resolve_response_handler("success"),
            Some(ResponseHandler::Success)
        );
        assert_eq!(
            resolve_response_handler("that_bai"),
            Some(ResponseHandler::Failure)
        );
        assert_eq!(
            resolve_response_handler("failure"),
            Some(ResponseHandler::Failure)
        );
        assert_eq!(resolve_response_handler("error"), None);
    }

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
        let vi_names = [
            "text",
            "h1",
            "h2",
            "h3",
            "p",
            "nhan",
            "image",
            "video",
            "icon",
            "button",
            "input",
            "link",
            "lien_ket",
            "flex",
            "grid",
            "stack",
            "khoi",
            "cuon",
            "can_giua",
            "lop",
            "dinh_dau",
            "dinh_man_hinh",
            "khoang_cach",
            "duong_ke",
            "form",
            "nhom_input",
            "chon_mot",
            "hop_kiem",
            "lua_chon",
            "modal",
            "tabs",
            "gap_mo",
            "bang_chuyen",
            "xuong_trang",
            "vong_quay",
            "thanh_tien_trinh",
            "bang",
            "bieu_do",
            "ban_do",
            "thanh_dieu_huong",
            "trinh_soan_thao",
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
        assert_eq!(
            resolve_prop_key("mau_nen"),
            Some(vibao_ast::PropKey::BackgroundColor)
        );
    }

    #[test]
    fn test_resolve_prop_key_unknown_name_returns_none() {
        assert_eq!(resolve_prop_key("khong_ton_tai_xyz"), None);
    }

    #[test]
    fn test_resolve_action_name_accepts_vietnamese_name() {
        assert_eq!(
            resolve_action_name("thong_bao"),
            Some(vibao_ast::ActionName::Notify)
        );
    }

    #[test]
    fn test_resolve_action_name_covers_all_16() {
        let names = [
            "thong_bao",
            "canh_bao",
            "dieu_huong",
            "mo_tab_moi",
            "mo_modal",
            "dong_modal",
            "cuon_den",
            "cuon_len_dau",
            "luu_du_lieu",
            "tai_du_lieu",
            "sao_chep",
            "dat_chu_de",
            "goi_api",
            "them_vao_mang",
            "xoa_theo_id",
            "cap_nhat_theo_id",
        ];
        assert_eq!(names.len(), 16);
        for name in names {
            assert!(
                resolve_action_name(name).is_some(),
                "'{}' must resolve",
                name
            );
        }
    }

    #[test]
    fn test_resolve_action_name_dang_xuat_returns_none_not_a_typo() {
        assert_eq!(resolve_action_name("dang_xuat"), None);
        assert!(action_vi::KNOWN_BUT_UNSUPPORTED_ACTIONS_VI.contains(&"dang_xuat"));
    }

    #[test]
    fn test_resolve_action_name_unknown_name_returns_none() {
        assert_eq!(resolve_action_name("khong_ton_tai_xyz"), None);
    }

    #[test]
    fn test_resolve_function_name_accepts_vietnamese_name() {
        assert_eq!(
            resolve_function_name("gia_tien"),
            Some(vibao_ast::FunctionName::FormatPrice)
        );
    }

    #[test]
    fn test_resolve_function_name_covers_all_6() {
        let names = [
            "gia_tien",
            "ngay",
            "rut_gon",
            "hoa_chu",
            "phan_tram",
            "lam_tron",
        ];
        for name in names {
            assert!(
                resolve_function_name(name).is_some(),
                "'{}' must resolve",
                name
            );
        }
    }

    #[test]
    fn test_resolve_function_name_unknown_name_returns_none() {
        assert_eq!(resolve_function_name("khong_ton_tai_xyz"), None);
    }
}
