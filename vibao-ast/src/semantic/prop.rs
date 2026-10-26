use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PropCategory {
    Color,
    Size,
    SpacingBorderEffects,
    Typography,
    FlexGridLayout,
    PositionTransform,
    ResponsiveOnly,
    MediaObject,
    FormInput,
    Link,
    Content,
    Animation,
}

impl PropCategory {
    pub fn display_name_vi(self) -> &'static str {
        match self {
            PropCategory::Color => "màu sắc",
            PropCategory::Size => "kích thước",
            PropCategory::SpacingBorderEffects => "khoảng cách/viền/hiệu ứng",
            PropCategory::Typography => "kiểu chữ",
            PropCategory::FlexGridLayout => "bố cục flex/grid",
            PropCategory::PositionTransform => "vị trí/biến đổi",
            PropCategory::ResponsiveOnly => "chỉ dùng trong khối responsive",
            PropCategory::MediaObject => "media/đối tượng",
            PropCategory::FormInput => "form/input",
            PropCategory::Link => "liên kết",
            PropCategory::Content => "nội dung",
            PropCategory::Animation => "hiệu ứng chuyển động",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PropKey {
    BackgroundColor,
    Color,
    BorderColor,

    Width,
    Height,
    MaxWidth,
    MinWidth,
    MaxHeight,
    MinHeight,

    Radius,
    Padding,
    Margin,
    Border,
    BorderStyle,
    Shadow,
    Overflow,
    ZIndex,

    FontSize,
    Bold,
    Italic,
    Underline,
    PreserveLines,
    Align,
    LineHeight,
    LetterSpacing,
    TextTransform,
    FontFamily,

    Direction,
    Gap,
    RowGap,
    ColumnGap,
    AlignItems,
    Wrap,
    Columns,
    Rows,

    TranslateX,
    TranslateY,
    Position,
    Offset,

    Hidden,

    Fit,
    Source,
    Alt,
    LazyLoad,

    Type,
    Placeholder,
    Value,
    OptionValue,
    Required,
    Disabled,
    ClassBinding,
    HtmlId,

    To,
    Target,
    Rel,

    Content,

    Animation,
    Duration,
    Delay,
    Repeat,
    HoverAnimation,
    ScrollAnimation,
}

impl PropKey {
    pub fn category(self) -> PropCategory {
        match self {
            PropKey::BackgroundColor | PropKey::Color | PropKey::BorderColor => PropCategory::Color,

            PropKey::Width | PropKey::Height | PropKey::MaxWidth
            | PropKey::MinWidth | PropKey::MaxHeight | PropKey::MinHeight => PropCategory::Size,

            PropKey::Radius | PropKey::Padding | PropKey::Margin | PropKey::Border
            | PropKey::BorderStyle | PropKey::Shadow | PropKey::Overflow
            | PropKey::ZIndex => PropCategory::SpacingBorderEffects,

            PropKey::FontSize | PropKey::Bold | PropKey::Italic | PropKey::Underline
            | PropKey::PreserveLines | PropKey::Align | PropKey::LineHeight | PropKey::LetterSpacing
            | PropKey::TextTransform | PropKey::FontFamily => PropCategory::Typography,

            PropKey::Direction | PropKey::Gap | PropKey::RowGap | PropKey::ColumnGap
            | PropKey::AlignItems | PropKey::Wrap | PropKey::Columns
            | PropKey::Rows => PropCategory::FlexGridLayout,

            PropKey::TranslateX | PropKey::TranslateY | PropKey::Position
            | PropKey::Offset => PropCategory::PositionTransform,

            PropKey::Hidden => PropCategory::ResponsiveOnly,

            PropKey::Fit | PropKey::Source | PropKey::Alt
            | PropKey::LazyLoad => PropCategory::MediaObject,

            PropKey::Type | PropKey::Placeholder | PropKey::Value | PropKey::OptionValue
            | PropKey::Required | PropKey::Disabled | PropKey::ClassBinding
            | PropKey::HtmlId => PropCategory::FormInput,

            PropKey::To | PropKey::Target | PropKey::Rel => PropCategory::Link,

            PropKey::Content => PropCategory::Content,

            PropKey::Animation | PropKey::Duration | PropKey::Delay | PropKey::Repeat
            | PropKey::HoverAnimation | PropKey::ScrollAnimation => PropCategory::Animation,
        }
    }
}
