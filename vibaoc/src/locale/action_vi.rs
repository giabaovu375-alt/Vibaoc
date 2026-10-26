use vibao_ast::ActionName;

pub fn action_name_vi(name: &str) -> Option<ActionName> {
    Some(match name {
        "thong_bao" => ActionName::Notify,
        "canh_bao" => ActionName::Alert,
        "dieu_huong" => ActionName::Navigate,
        "mo_tab_moi" => ActionName::OpenNewTab,
        "mo_modal" => ActionName::OpenModal,
        "dong_modal" => ActionName::CloseModal,
        "cuon_den" => ActionName::ScrollTo,
        "cuon_len_dau" => ActionName::ScrollToTop,
        "luu_du_lieu" => ActionName::SaveData,
        "tai_du_lieu" => ActionName::LoadData,
        "sao_chep" => ActionName::CopyToClipboard,
        "dat_chu_de" => ActionName::SetTheme,

        "goi_api" => ActionName::ApiCall,

        "them_vao_mang" => ActionName::ArrayPush,
        "xoa_theo_id" => ActionName::ArrayRemoveById,
        "cap_nhat_theo_id" => ActionName::ArrayUpdateById,

        _ => return None,
    })
}

pub const KNOWN_BUT_UNSUPPORTED_ACTIONS_VI: &[&str] = &["dang_xuat"];

#[cfg(test)]
mod tests {
    use super::*;

    const ALL_ACTION_SURFACE_NAMES_VI: &[&str] = &[
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

    #[test]
    fn test_all_16_source_names_resolve() {
        assert_eq!(ALL_ACTION_SURFACE_NAMES_VI.len(), 16);
        for name in ALL_ACTION_SURFACE_NAMES_VI.iter().copied() {
            assert!(
                action_name_vi(name).is_some(),
                "'{}' must resolve to an ActionName",
                name
            );
        }
    }

    #[test]
    fn test_covers_exactly_16_distinct_action_name() {
        use std::collections::HashSet;
        let resolved: HashSet<ActionName> = ALL_ACTION_SURFACE_NAMES_VI
            .iter()
            .filter_map(|n| action_name_vi(n))
            .collect();
        assert_eq!(
            resolved.len(),
            16,
            "16 source names must resolve to 16 distinct ActionName values (no aliasing)"
        );
    }

    #[test]
    fn test_dang_xuat_does_not_resolve() {
        assert_eq!(action_name_vi("dang_xuat"), None);
    }

    #[test]
    fn test_dang_xuat_is_tracked_as_known_but_unsupported() {
        assert!(KNOWN_BUT_UNSUPPORTED_ACTIONS_VI.contains(&"dang_xuat"));
    }

    #[test]
    fn test_unknown_name_returns_none() {
        assert_eq!(action_name_vi("khong_ton_tai_xyz"), None);
    }
}
