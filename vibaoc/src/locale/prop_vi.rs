use vibao_ast::PropKey;

pub fn prop_name_vi(name: &str) -> Option<PropKey> {
    Some(match name {
        "mau_nen" => PropKey::BackgroundColor,
        "mau" => PropKey::Color,
        "mau_chu" => PropKey::Color,
        "mau_vien" => PropKey::BorderColor,

        "rong" => PropKey::Width,
        "cao" => PropKey::Height,
        "max_rong" => PropKey::MaxWidth,
        "min_rong" => PropKey::MinWidth,
        "max_cao" => PropKey::MaxHeight,
        "min_cao" => PropKey::MinHeight,

        "radius" => PropKey::Radius,
        "dem" => PropKey::Padding,
        "le" => PropKey::Margin,
        "vien" => PropKey::Border,
        "kieu_vien" => PropKey::BorderStyle,
        "bong" => PropKey::Shadow,
        "cuon_tran" => PropKey::Overflow,
        "tang_z" => PropKey::ZIndex,

        "co" => PropKey::FontSize,
        "dam" => PropKey::Bold,
        "nghieng" => PropKey::Italic,
        "gach_chan" => PropKey::Underline,
        "giu_dong" => PropKey::PreserveLines,
        "can" => PropKey::Align,
        "hang" => PropKey::LineHeight,
        "khoang_chu" => PropKey::LetterSpacing,
        "bien_doi" => PropKey::TextTransform,
        "phong_chu" => PropKey::FontFamily,

        "huong" => PropKey::Direction,
        "gap" => PropKey::Gap,
        "gap_doc" => PropKey::RowGap,
        "gap_ngang" => PropKey::ColumnGap,
        "doc" => PropKey::AlignItems,
        "boc" => PropKey::Wrap,
        "cot" => PropKey::Columns,
        "hang_luoi" => PropKey::Rows,

        "tran_x" => PropKey::TranslateX,
        "tran_y" => PropKey::TranslateY,
        "vi_tri" => PropKey::Position,
        "offset" => PropKey::Offset,

        "an" => PropKey::Hidden,

        "khop" => PropKey::Fit,
        "nguon" => PropKey::Source,
        "mo_ta_anh" => PropKey::Alt,
        "tai_cham" => PropKey::LazyLoad,

        "loai" => PropKey::Type,
        "chu_tro" => PropKey::Placeholder,
        "gia_tri" => PropKey::Value,
        "chon" => PropKey::OptionValue,
        "bat_buoc" => PropKey::Required,
        "vo_hieu" => PropKey::Disabled,
        "lop" => PropKey::ClassBinding,
        "id" => PropKey::HtmlId,

        "den" => PropKey::To,
        "muc_tieu" => PropKey::Target,
        "quan_he" => PropKey::Rel,

        "noi_dung" => PropKey::Content,

        "hieu_ung" => PropKey::Animation,
        "thoi_gian" => PropKey::Duration,
        "tre" => PropKey::Delay,
        "lap" => PropKey::Repeat,
        "hieu_ung_hover" => PropKey::HoverAnimation,
        "hieu_ung_cuon" => PropKey::ScrollAnimation,

        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL_PROP_SURFACE_NAMES_VI: &[&str] = &[
        "mau_nen", "mau", "mau_chu", "mau_vien",
        "rong", "cao", "max_rong", "min_rong", "max_cao", "min_cao",
        "radius", "dem", "le", "vien", "kieu_vien", "bong", "cuon_tran", "tang_z",
        "co", "dam", "nghieng", "gach_chan", "giu_dong", "can", "hang", "khoang_chu",
        "bien_doi", "phong_chu",
        "huong", "gap", "gap_doc", "gap_ngang", "doc", "boc", "cot", "hang_luoi",
        "tran_x", "tran_y", "vi_tri", "offset",
        "an",
        "khop", "nguon", "mo_ta_anh", "tai_cham",
        "loai", "chu_tro", "gia_tri", "chon", "bat_buoc", "vo_hieu", "lop", "id",
        "den", "muc_tieu", "quan_he",
        "noi_dung",
        "hieu_ung", "thoi_gian", "tre", "lap", "hieu_ung_hover", "hieu_ung_cuon",
    ];

    #[test]
    fn test_all_63_source_names_resolve() {
        assert_eq!(ALL_PROP_SURFACE_NAMES_VI.len(), 63);
        for name in ALL_PROP_SURFACE_NAMES_VI.iter().copied() {
            assert!(
                prop_name_vi(name).is_some(),
                "'{}' must resolve to a PropKey", name
            );
        }
    }

    #[test]
    fn test_mau_and_mau_chu_are_same_propkey() {
        assert_eq!(prop_name_vi("mau"), prop_name_vi("mau_chu"));
        assert_eq!(prop_name_vi("mau"), Some(PropKey::Color));
    }

    #[test]
    fn test_unknown_name_returns_none() {
        assert_eq!(prop_name_vi("khong_ton_tai_xyz"), None);
    }

    #[test]
    fn test_covers_exactly_62_distinct_propkey() {
        use std::collections::HashSet;
        let resolved: HashSet<PropKey> = ALL_PROP_SURFACE_NAMES_VI
            .iter()
            .filter_map(|n| prop_name_vi(n))
            .collect();
        assert_eq!(resolved.len(), 62, "63 source names must resolve to exactly 62 PropKeys (mau/mau_chu merge into 1)");
    }
}
