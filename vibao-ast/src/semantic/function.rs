use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FunctionName {
    FormatPrice,
    FormatDate,
    Truncate,
    Uppercase,
    FormatPercent,
    Round,
}

impl FunctionName {
    pub const fn runtime_name(self) -> &'static str {
        match self {
            Self::FormatPrice => "gia_tien",
            Self::FormatDate => "ngay",
            Self::Truncate => "rut_gon",
            Self::Uppercase => "hoa_chu",
            Self::FormatPercent => "phan_tram",
            Self::Round => "lam_tron",
        }
    }
}
