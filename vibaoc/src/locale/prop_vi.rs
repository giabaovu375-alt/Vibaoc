// ============================================================
// VIBAO COMPILER (Rust) — locale/prop_vi.rs
// LOCALE: Vietnamese — maps a PROP KEY name written in the source
// (.vbao) to a PropKey (the semantic identity, defined in
// vibao_ast::semantic::prop). Kept SEPARATE from vi.rs (unlike
// Tag/Keyword, which share one file) — for easier review/diffing given
// how much larger the prop vocabulary is (57 versus 41 Tag + 21
// keyword), and because the Prop domain is completely independent from
// the Tag/Keyword domain (no technical reason to combine them into one
// file).
//
// SOURCES used to cross-check each name (most authoritative first,
// following the exact same principle used when writing
// EN_LOCALE_PROPOSAL.md for Tag):
// 1. `vibaoc::codegen::props::expand_props()` — the PropKey semantic
//    branches actually running in Simple Element.
// 2. Individual `match key.as_str()` branches across the 9 `resolve_*`
//    functions in `vibaoc::codegen::layout` — for the 12 names NOT
//    present in (1), which ARE ACTUALLY RUNNING (Layout Element) even
//    without a constant table.
// No name in this file is inferred or invented — all 57 semantic names
// match 1:1 against String literals that ACTUALLY EXIST AND RUN in
// the two files above (see the full cross-reference table in
// ARCHITECTURE_PROPOSAL.md, section 3.3).
//
// STATUS: FULL 57/57 PropKey semantic coverage (58 source names, with 1
// alias). This resolver is used directly by the parser/codegen/
// validator; the English surface goes through the same semantic
// identity.
// ============================================================

use vibao_ast::PropKey;

/// (Vietnamese source name) -> PropKey table — FULL 57/57 coverage,
/// matching the semantic handling of Simple/Layout and the 12 names
/// specific to layout.rs (not duplicated in props.rs) — 58 source names
/// TOTAL, merged down to 57 PropKeys since "mau"/"mau_chu" alias to the
/// same PropKey.
pub fn prop_name_vi(name: &str) -> Option<PropKey> {
    Some(match name {
        // ── Color ──
        "mau_nen" => PropKey::BackgroundColor,
        "mau" => PropKey::Color,
        "mau_chu" => PropKey::Color,
        "mau_vien" => PropKey::BorderColor,

        // ── Size ──
        "rong" => PropKey::Width,
        "cao" => PropKey::Height,
        "max_rong" => PropKey::MaxWidth,
        "min_rong" => PropKey::MinWidth,
        "max_cao" => PropKey::MaxHeight,
        "min_cao" => PropKey::MinHeight,

        // ── Spacing / Border / Effects ──
        "radius" => PropKey::Radius,
        "dem" => PropKey::Padding,
        "le" => PropKey::Margin,
        "vien" => PropKey::Border,
        "kieu_vien" => PropKey::BorderStyle,
        "bong" => PropKey::Shadow,
        "cuon_tran" => PropKey::Overflow,
        "tang_z" => PropKey::ZIndex,

        // ── Typography ──
        "co" => PropKey::FontSize,
        "dam" => PropKey::Bold,
        "nghieng" => PropKey::Italic,
        "gach_chan" => PropKey::Underline,
        "can" => PropKey::Align,
        "hang" => PropKey::LineHeight,
        "khoang_chu" => PropKey::LetterSpacing,
        "bien_doi" => PropKey::TextTransform,
        "phong_chu" => PropKey::FontFamily,

        // ── Flex / Grid layout ──
        "huong" => PropKey::Direction,
        "gap" => PropKey::Gap,
        "gap_doc" => PropKey::RowGap,
        "gap_ngang" => PropKey::ColumnGap,
        "doc" => PropKey::AlignItems,
        "boc" => PropKey::Wrap,
        "cot" => PropKey::Columns,
        "hang_luoi" => PropKey::Rows,

        // ── Position / Transform ──
        "tran_x" => PropKey::TranslateX,
        "tran_y" => PropKey::TranslateY,
        "vi_tri" => PropKey::Position,
        "offset" => PropKey::Offset,

        // ── Responsive-only ──
        "an" => PropKey::Hidden,

        // ── Media / Object ──
        "khop" => PropKey::Fit,
        "nguon" => PropKey::Source,
        "mo_ta_anh" => PropKey::Alt,
        "tai_cham" => PropKey::LazyLoad,

        // ── Form / Input ──
        "loai" => PropKey::Type,
        "chu_tro" => PropKey::Placeholder,
        "gia_tri" => PropKey::Value,
        "bat_buoc" => PropKey::Required,
        "vo_hieu" => PropKey::Disabled,
        "lop" => PropKey::ClassBinding,

        // ── Link ──
        "den" => PropKey::To,

        // ── Content ──
        "noi_dung" => PropKey::Content,

        // ── Animation ──
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

    /// Matches EXACTLY the 58 source names (57 PropKeys, with
    /// "mau"/"mau_chu" both pointing to the same PropKey) — hand-copied
    /// once from the full cross-reference table
    /// (ARCHITECTURE_PROPOSAL.md, section 3.3), NOT derived backwards
    /// from PropKey (to avoid a self-confirming loop — the reference
    /// list MUST be independent from the match table above).
    const ALL_PROP_SURFACE_NAMES_VI: &[&str] = &[
        "mau_nen", "mau", "mau_chu", "mau_vien",
        "rong", "cao", "max_rong", "min_rong", "max_cao", "min_cao",
        "radius", "dem", "le", "vien", "kieu_vien", "bong", "cuon_tran", "tang_z",
        "co", "dam", "nghieng", "gach_chan", "can", "hang", "khoang_chu",
        "bien_doi", "phong_chu",
        "huong", "gap", "gap_doc", "gap_ngang", "doc", "boc", "cot", "hang_luoi",
        "tran_x", "tran_y", "vi_tri", "offset",
        "an",
        "khop", "nguon", "mo_ta_anh", "tai_cham",
        "loai", "chu_tro", "gia_tri", "bat_buoc", "vo_hieu", "lop",
        "den",
        "noi_dung",
        "hieu_ung", "thoi_gian", "tre", "lap", "hieu_ung_hover", "hieu_ung_cuon",
    ];

    #[test]
    fn test_all_58_source_names_resolve() {
        assert_eq!(ALL_PROP_SURFACE_NAMES_VI.len(), 58);
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
    fn test_covers_exactly_57_distinct_propkey() {
        use std::collections::HashSet;
        let resolved: HashSet<PropKey> = ALL_PROP_SURFACE_NAMES_VI
            .iter()
            .filter_map(|n| prop_name_vi(n))
            .collect();
        assert_eq!(resolved.len(), 57, "58 source names must resolve to exactly 57 PropKeys (mau/mau_chu merge into 1)");
    }
}
