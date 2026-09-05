// ============================================================
// VIBAO COMPILER (Rust) — locale/function_vi.rs
// LOCALE: Vietnamese — maps an EXPRESSION FUNCTION name written in the
// source (.vbao) to a FunctionName (the semantic identity, defined in
// vibao_ast::semantic::function).
//
// STATUS: the vocabulary + locale mapping is wired into both the parser
// and the validator. The parser normalizes a resolved name to its
// canonical runtime name; the validator uses the same semantic resolver
// to catch invalid function names.
//
// SOURCE OF TRUTH, matched 1:1: `vibao-runtime/src/runtime/expr_eval.rs::
// eval_call()` — 6 names.
// ============================================================

use vibao_ast::FunctionName;

/// (Vietnamese source name) -> FunctionName table — FULL 6/6 coverage.
pub fn function_name_vi(name: &str) -> Option<FunctionName> {
    Some(match name {
        "gia_tien" => FunctionName::FormatPrice,
        "ngay" => FunctionName::FormatDate,
        "rut_gon" => FunctionName::Truncate,
        "hoa_chu" => FunctionName::Uppercase,
        "phan_tram" => FunctionName::FormatPercent,
        "lam_tron" => FunctionName::Round,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL_FUNCTION_SURFACE_NAMES_VI: &[&str] = &[
        "gia_tien", "ngay", "rut_gon", "hoa_chu", "phan_tram", "lam_tron",
    ];

    #[test]
    fn test_all_6_source_names_resolve() {
        assert_eq!(ALL_FUNCTION_SURFACE_NAMES_VI.len(), 6);
        for name in ALL_FUNCTION_SURFACE_NAMES_VI.iter().copied() {
            assert!(function_name_vi(name).is_some(), "'{}' must resolve", name);
        }
    }

    #[test]
    fn test_covers_exactly_6_distinct_function_name() {
        use std::collections::HashSet;
        let resolved: HashSet<FunctionName> = ALL_FUNCTION_SURFACE_NAMES_VI
            .iter()
            .filter_map(|n| function_name_vi(n))
            .collect();
        assert_eq!(resolved.len(), 6);
    }

    #[test]
    fn test_unknown_name_returns_none() {
        assert_eq!(function_name_vi("khong_ton_tai_xyz"), None);
    }
}
