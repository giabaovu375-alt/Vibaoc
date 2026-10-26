use super::tag::Tag;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TagKind {
    Simple,
    Layout,
    Complex,
}

#[derive(Debug, Clone, Copy)]
pub struct TagSpec {
    pub tag: Tag,
    pub kind: TagKind,
    pub html_tag: &'static str,
}

pub fn tag_spec(tag: Tag) -> TagSpec {
    use TagKind::*;
    let (kind, html_tag) = match tag {
        Tag::Text => (Simple, "p"),
        Tag::H1 => (Simple, "h1"),
        Tag::H2 => (Simple, "h2"),
        Tag::H3 => (Simple, "h3"),
        Tag::P => (Simple, "p"),
        Tag::Nhan => (Simple, "span"),
        Tag::Image => (Simple, "img"),
        Tag::Video => (Simple, "video"),
        Tag::Icon => (Simple, "span"),
        Tag::Button => (Simple, "button"),
        Tag::Input => (Simple, "input"),
        Tag::Textarea => (Simple, "textarea"),
        Tag::Link => (Simple, "a"),
        Tag::Flex => (Layout, "div"),
        Tag::Grid => (Layout, "div"),
        Tag::Stack => (Layout, "div"),
        Tag::Khoi => (Layout, "div"),
        Tag::Cuon => (Layout, "div"),
        Tag::CanGiua => (Layout, "div"),
        Tag::Lop => (Layout, "div"),
        Tag::DinhDau => (Layout, "div"),
        Tag::DinhManHinh => (Layout, "div"),
        Tag::KhoangCach => (Simple, "div"),
        Tag::DuongKe => (Simple, "hr"),
        Tag::NhomInput => (Simple, "div"),
        Tag::ChonMot => (Simple, "input"),
        Tag::HopKiem => (Simple, "input"),
        Tag::LuaChon => (Simple, "select"),
        Tag::Form => (Simple, "form"),
        Tag::Modal => (Complex, "div"),
        Tag::Tabs => (Complex, "div"),
        Tag::GapMo => (Complex, "div"),
        Tag::BangChuyen => (Complex, "div"),
        Tag::XuongTrang => (Complex, "div"),
        Tag::Bang => (Complex, "div"),
        Tag::BieuDo => (Complex, "div"),
        Tag::BanDo => (Complex, "div"),
        Tag::TrinhSoanThao => (Complex, "div"),
        Tag::ThanhDieuHuong => (Simple, "nav"),
        Tag::VongQuay => (Simple, "div"),
        Tag::ThanhTienTrinh => (Simple, "div"),
    };
    TagSpec {
        tag,
        kind,
        html_tag,
    }
}

use super::prop::PropKey;

#[derive(Debug, Clone, Copy)]
pub struct PropSpec {
    pub key: PropKey,
    pub applies_to_simple: bool,
    pub applies_to_layout: bool,
    pub responsive_only: bool,
}

pub fn prop_spec(key: PropKey) -> PropSpec {
    use PropKey::*;
    let (applies_to_simple, applies_to_layout) = match key {
        BackgroundColor | Width | Height | MaxWidth | Radius | Padding | Margin | Border
        | Shadow | Overflow | ZIndex | FontSize | Direction | Gap | AlignItems | Wrap | Align
        | HtmlId => (true, true),

        Color | BorderColor | BorderStyle | Bold | Italic | Underline | PreserveLines
        | LineHeight | LetterSpacing | TextTransform | FontFamily | Fit | Source | Alt
        | LazyLoad | Type | Placeholder | Value | OptionValue | Required | Disabled
        | ClassBinding | To | Target | Rel | Content | Animation | Duration | Delay | Repeat
        | HoverAnimation | ScrollAnimation => (true, false),

        MinWidth | MaxHeight | MinHeight | RowGap | ColumnGap | Columns | Rows | TranslateX
        | TranslateY | Position | Offset | Hidden => (false, true),
    };
    let responsive_only = matches!(key, Hidden);
    PropSpec {
        key,
        applies_to_simple,
        applies_to_layout,
        responsive_only,
    }
}

#[cfg(test)]
mod prop_tests {
    use super::*;

    #[test]
    fn test_prop_spec_covers_every_variant() {
        assert!(prop_spec(PropKey::BackgroundColor).applies_to_simple);
        assert!(prop_spec(PropKey::BackgroundColor).applies_to_layout);
        assert!(prop_spec(PropKey::Color).applies_to_simple);
        assert!(!prop_spec(PropKey::Color).applies_to_layout);
        assert!(!prop_spec(PropKey::MinWidth).applies_to_simple);
        assert!(prop_spec(PropKey::MinWidth).applies_to_layout);
    }

    #[test]
    fn test_hidden_is_responsive_only_and_only_hidden_is() {
        assert!(
            prop_spec(PropKey::Hidden).responsive_only,
            "Hidden (\"an\") MUST have responsive_only = true"
        );
        let responsive_only_count = ALL_PROP_KEYS_FOR_TEST
            .iter()
            .filter(|k| prop_spec(**k).responsive_only)
            .count();
        assert_eq!(
            responsive_only_count, 1,
            "currently exactly 1 prop should be responsive_only (Hidden)"
        );
    }

    #[test]
    fn test_responsive_only_props_still_have_applies_to_layout_true() {
        assert!(prop_spec(PropKey::Hidden).applies_to_layout);
    }

    #[test]
    fn test_no_prop_applies_to_neither() {
        for key in ALL_PROP_KEYS_FOR_TEST {
            let spec = prop_spec(key);
            assert!(
                spec.applies_to_simple || spec.applies_to_layout,
                "{:?} does not apply to Simple OR Layout — check its classification",
                key
            );
        }
    }

    #[test]
    fn test_simple_applicability_count() {
        let count = ALL_PROP_KEYS_FOR_TEST
            .iter()
            .filter(|k| prop_spec(**k).applies_to_simple)
            .count();
        assert_eq!(count, 50, "must match 18 (both) + 32 (simple-only) = 50");
    }

    #[test]
    fn test_layout_group_matches_known_layout_prop_count() {
        let count = ALL_PROP_KEYS_FOR_TEST
            .iter()
            .filter(|k| prop_spec(**k).applies_to_layout)
            .count();
        assert_eq!(count, 30, "must match 18 (both) + 12 (layout-only) = 30");
    }

    #[test]
    fn test_total_prop_key_count_is_62() {
        assert_eq!(ALL_PROP_KEYS_FOR_TEST.len(), 62);
    }

    const ALL_PROP_KEYS_FOR_TEST: [PropKey; 62] = [
        PropKey::BackgroundColor,
        PropKey::Color,
        PropKey::BorderColor,
        PropKey::Width,
        PropKey::Height,
        PropKey::MaxWidth,
        PropKey::MinWidth,
        PropKey::MaxHeight,
        PropKey::MinHeight,
        PropKey::Radius,
        PropKey::Padding,
        PropKey::Margin,
        PropKey::Border,
        PropKey::BorderStyle,
        PropKey::Shadow,
        PropKey::Overflow,
        PropKey::ZIndex,
        PropKey::FontSize,
        PropKey::Bold,
        PropKey::Italic,
        PropKey::Underline,
        PropKey::PreserveLines,
        PropKey::Align,
        PropKey::LineHeight,
        PropKey::LetterSpacing,
        PropKey::TextTransform,
        PropKey::FontFamily,
        PropKey::Direction,
        PropKey::Gap,
        PropKey::RowGap,
        PropKey::ColumnGap,
        PropKey::AlignItems,
        PropKey::Wrap,
        PropKey::Columns,
        PropKey::Rows,
        PropKey::TranslateX,
        PropKey::TranslateY,
        PropKey::Position,
        PropKey::Offset,
        PropKey::Hidden,
        PropKey::Fit,
        PropKey::Source,
        PropKey::Alt,
        PropKey::LazyLoad,
        PropKey::Type,
        PropKey::Placeholder,
        PropKey::Value,
        PropKey::OptionValue,
        PropKey::Required,
        PropKey::Disabled,
        PropKey::ClassBinding,
        PropKey::HtmlId,
        PropKey::To,
        PropKey::Target,
        PropKey::Rel,
        PropKey::Content,
        PropKey::Animation,
        PropKey::Duration,
        PropKey::Delay,
        PropKey::Repeat,
        PropKey::HoverAnimation,
        PropKey::ScrollAnimation,
    ];
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tag_spec_covers_every_variant() {
        assert_eq!(tag_spec(Tag::Khoi).kind, TagKind::Layout);
        assert_eq!(tag_spec(Tag::Text).kind, TagKind::Simple);
        assert_eq!(tag_spec(Tag::Modal).kind, TagKind::Complex);
    }

    #[test]
    fn test_layout_group_has_exactly_9_members() {
        let count = [
            Tag::Flex,
            Tag::Grid,
            Tag::Stack,
            Tag::Khoi,
            Tag::Cuon,
            Tag::CanGiua,
            Tag::Lop,
            Tag::DinhDau,
            Tag::DinhManHinh,
        ]
        .iter()
        .filter(|t| tag_spec(**t).kind == TagKind::Layout)
        .count();
        assert_eq!(count, 9);
    }

    #[test]
    fn test_complex_group_matches_old_builtin_complex_exactly() {
        let complex_tags = [
            Tag::Modal,
            Tag::Tabs,
            Tag::GapMo,
            Tag::BangChuyen,
            Tag::XuongTrang,
            Tag::Bang,
            Tag::BieuDo,
            Tag::BanDo,
            Tag::TrinhSoanThao,
        ];
        assert_eq!(
            complex_tags.len(),
            9,
            "must match BUILTIN_COMPLEX's 9 elements exactly (form moved to Simple)"
        );
        let count = complex_tags
            .iter()
            .filter(|t| tag_spec(**t).kind == TagKind::Complex)
            .count();
        assert_eq!(count, complex_tags.len());

        assert_eq!(tag_spec(Tag::ThanhDieuHuong).kind, TagKind::Simple);
    }
}
