//! English surface names for `PropKey`.
//! English is the universal locale and is always accepted alongside the
//! active locale (currently Vietnamese).

use vibao_ast::PropKey;

pub fn prop_name_en(name: &str) -> Option<PropKey> {
    Some(match name {
        "background_color" => PropKey::BackgroundColor,
        "color" => PropKey::Color,
        "border_color" => PropKey::BorderColor,
        "width" => PropKey::Width,
        "height" => PropKey::Height,
        "max_width" => PropKey::MaxWidth,
        "min_width" => PropKey::MinWidth,
        "max_height" => PropKey::MaxHeight,
        "min_height" => PropKey::MinHeight,
        "radius" => PropKey::Radius,
        "padding" => PropKey::Padding,
        "margin" => PropKey::Margin,
        "border" => PropKey::Border,
        "border_style" => PropKey::BorderStyle,
        "shadow" => PropKey::Shadow,
        "overflow" => PropKey::Overflow,
        "z_index" => PropKey::ZIndex,
        "font_size" => PropKey::FontSize,
        "bold" => PropKey::Bold,
        "italic" => PropKey::Italic,
        "underline" => PropKey::Underline,
        "align" => PropKey::Align,
        "line_height" => PropKey::LineHeight,
        "letter_spacing" => PropKey::LetterSpacing,
        "text_transform" => PropKey::TextTransform,
        "font_family" => PropKey::FontFamily,
        "direction" => PropKey::Direction,
        "gap" => PropKey::Gap,
        "row_gap" => PropKey::RowGap,
        "column_gap" => PropKey::ColumnGap,
        "align_items" => PropKey::AlignItems,
        "wrap" => PropKey::Wrap,
        "columns" => PropKey::Columns,
        "rows" => PropKey::Rows,
        "translate_x" => PropKey::TranslateX,
        "translate_y" => PropKey::TranslateY,
        "position" => PropKey::Position,
        "offset" => PropKey::Offset,
        "hidden" => PropKey::Hidden,
        "fit" => PropKey::Fit,
        "source" => PropKey::Source,
        "alt" => PropKey::Alt,
        "lazy_load" => PropKey::LazyLoad,
        "type" => PropKey::Type,
        "placeholder" => PropKey::Placeholder,
        "value" => PropKey::Value,
        "required" => PropKey::Required,
        "disabled" => PropKey::Disabled,
        "class" => PropKey::ClassBinding,
        "to" => PropKey::To,
        "content" => PropKey::Content,
        "animation" => PropKey::Animation,
        "duration" => PropKey::Duration,
        "delay" => PropKey::Delay,
        "repeat" => PropKey::Repeat,
        "hover_animation" => PropKey::HoverAnimation,
        "scroll_animation" => PropKey::ScrollAnimation,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    const ALL: &[&str] = &[
        "background_color", "color", "border_color", "width", "height",
        "max_width", "min_width", "max_height", "min_height", "radius",
        "padding", "margin", "border", "border_style", "shadow", "overflow",
        "z_index", "font_size", "bold", "italic", "underline", "align",
        "line_height", "letter_spacing", "text_transform", "font_family",
        "direction", "gap", "row_gap", "column_gap", "align_items", "wrap",
        "columns", "rows", "translate_x", "translate_y", "position", "offset",
        "hidden", "fit", "source", "alt", "lazy_load", "type", "placeholder",
        "value", "required", "disabled", "class", "to", "content", "animation",
        "duration", "delay", "repeat", "hover_animation", "scroll_animation",
    ];

    #[test]
    fn all_names_resolve() {
        assert_eq!(ALL.len(), 57);
        assert!(ALL.iter().all(|name| prop_name_en(name).is_some()));
    }

    #[test]
    fn all_names_resolve_to_distinct_prop_keys() {
        let resolved: HashSet<PropKey> = ALL.iter().filter_map(|n| prop_name_en(n)).collect();
        assert_eq!(resolved.len(), 57);
    }

    #[test]
    fn vietnamese_names_do_not_resolve_here() {
        assert_eq!(prop_name_en("mau_nen"), None);
    }
}
