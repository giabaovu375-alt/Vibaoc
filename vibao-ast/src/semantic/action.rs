use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ActionName {
    Notify,
    Alert,
    Navigate,
    OpenNewTab,
    OpenModal,
    CloseModal,
    ScrollTo,
    ScrollToTop,
    SaveData,
    LoadData,
    CopyToClipboard,
    SetTheme,

    ApiCall,

    ArrayPush,
    ArrayRemoveById,
    ArrayUpdateById,
}

impl ActionName {
    pub const fn runtime_name(self) -> &'static str {
        match self {
            Self::Notify => "thong_bao",
            Self::Alert => "canh_bao",
            Self::Navigate => "dieu_huong",
            Self::OpenNewTab => "mo_tab_moi",
            Self::OpenModal => "mo_modal",
            Self::CloseModal => "dong_modal",
            Self::ScrollTo => "cuon_den",
            Self::ScrollToTop => "cuon_len_dau",
            Self::SaveData => "luu_du_lieu",
            Self::LoadData => "tai_du_lieu",
            Self::CopyToClipboard => "sao_chep",
            Self::SetTheme => "dat_chu_de",
            Self::ApiCall => "goi_api",
            Self::ArrayPush => "them_vao_mang",
            Self::ArrayRemoveById => "xoa_theo_id",
            Self::ArrayUpdateById => "cap_nhat_theo_id",
        }
    }
}
