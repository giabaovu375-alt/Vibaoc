//! English surface names for `FunctionName`.

use vibao_ast::FunctionName;

pub fn function_name_en(name: &str) -> Option<FunctionName> {
    Some(match name {
        "format_price" => FunctionName::FormatPrice,
        "format_date" => FunctionName::FormatDate,
        "truncate" => FunctionName::Truncate,
        "uppercase" => FunctionName::Uppercase,
        "format_percent" => FunctionName::FormatPercent,
        "round" => FunctionName::Round,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    const ALL: &[&str] = &[
        "format_price", "format_date", "truncate", "uppercase", "format_percent", "round",
    ];

    #[test]
    fn all_names_resolve() {
        assert_eq!(ALL.len(), 6);
        assert!(ALL.iter().all(|name| function_name_en(name).is_some()));
    }

    #[test]
    fn all_names_resolve_to_distinct_function_names() {
        let resolved: HashSet<FunctionName> = ALL.iter().filter_map(|n| function_name_en(n)).collect();
        assert_eq!(resolved.len(), 6);
    }

    #[test]
    fn vietnamese_names_do_not_resolve_here() {
        assert_eq!(function_name_en("gia_tien"), None);
    }
}
