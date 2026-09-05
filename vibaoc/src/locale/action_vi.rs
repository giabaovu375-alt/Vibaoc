// ============================================================
// VIBAO COMPILER (Rust) — locale/action_vi.rs
// LOCALE: Vietnamese — maps an ACTION name written in the source
// (.vbao) to an ActionName (the semantic identity, defined in
// vibao_ast::semantic::action). Kept SEPARATE from vi.rs/prop_vi.rs —
// the Action domain is completely independent from Tag/Keyword/Prop.
//
// SOURCES used to cross-check every name (matched 1:1 against code that
// is ACTUALLY RUNNING, not inferred/invented — same principle applied
// to prop_vi.rs):
// 1. `vibao-runtime/src/runtime/action.rs::dispatch_function_call()` —
//    11 names (group A, side effects via a String match).
// 2. `vibao-runtime/src/runtime/action.rs::dispatch_one()` — 1 name
//    ("goi_api", special-cased by the PARSER into `Action::ApiCall`,
//    see `parser/action.rs` around line 107) + 3 names (group C,
//    matched separately BEFORE the general FunctionCall branch, around
//    lines 83-87).
//
// SCOPE AS SETTLED (see ARCHITECTURE_PROPOSAL.md, section "UPDATE —
// ActionName/FunctionName investigation"): "dang_xuat" (log out) is NOT
// here — confirmed to have no runtime handler under any name, and no
// auth/session/token concept exists in the state model. Clear decision:
// do NOT create ActionName::DangXuat with a no-op/made-up handler — it
// would create a "false semantic promise". "dang_xuat" temporarily
// remains a string in `component_set()`/`vocabulary.rs` on the
// lexer/tables.rs side, but does NOT resolve through
// `action_name_vi()` — the validator distinguishes "Unknown" (not an
// action at all) from "Known but Unsupported" (like "dang_xuat" —
// recognized as a legacy action name, but not yet implemented) — these
// must NOT be collapsed into a single error type.
//
// STATUS: FULL 15/15 ActionName coverage; this resolver is used by the
// parser, the validator, and the lexer vocabulary. The parser
// normalizes the surface locale name to the canonical runtime name
// before an action enters the registry.
// ============================================================

use vibao_ast::ActionName;

/// (Vietnamese source name) -> ActionName table — FULL 15/15 coverage,
/// matches 1:1 with the real `dispatch_function_call()`/`dispatch_one()`.
pub fn action_name_vi(name: &str) -> Option<ActionName> {
    Some(match name {
        // ── Group A: dispatch_function_call() (11) ──
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

        // ── Group B: special-cased into Action::ApiCall by the parser (1) ──
        "goi_api" => ActionName::ApiCall,

        // ── Group C: array CRUD, matched separately in dispatch_one() (3) ──
        "them_vao_mang" => ActionName::ArrayPush,
        "xoa_theo_id" => ActionName::ArrayRemoveById,
        "cap_nhat_theo_id" => ActionName::ArrayUpdateById,

        // "dang_xuat" is intentionally absent here — see the full
        // explanation in the doc-comment at the top of the file.
        _ => return None,
    })
}

/// Names that exist in the vocabulary but have no semantic/runtime
/// implementation yet. The validator uses this list to raise a distinct
/// diagnostic instead of treating them as a plain typo.
pub const KNOWN_BUT_UNSUPPORTED_ACTIONS_VI: &[&str] = &["dang_xuat"];

#[cfg(test)]
mod tests {
    use super::*;

    /// Matches EXACTLY the 15 source names — hand-copied once from the
    /// table built in ARCHITECTURE_PROPOSAL.md, NOT derived backwards
    /// from ActionName (to avoid a self-confirming loop).
    const ALL_ACTION_SURFACE_NAMES_VI: &[&str] = &[
        "thong_bao", "canh_bao", "dieu_huong", "mo_tab_moi",
        "mo_modal", "dong_modal", "cuon_den", "cuon_len_dau",
        "luu_du_lieu", "tai_du_lieu", "sao_chep",
        "goi_api",
        "them_vao_mang", "xoa_theo_id", "cap_nhat_theo_id",
    ];

    #[test]
    fn test_all_15_source_names_resolve() {
        assert_eq!(ALL_ACTION_SURFACE_NAMES_VI.len(), 15);
        for name in ALL_ACTION_SURFACE_NAMES_VI.iter().copied() {
            assert!(
                action_name_vi(name).is_some(),
                "'{}' must resolve to an ActionName", name
            );
        }
    }

    #[test]
    fn test_covers_exactly_15_distinct_action_name() {
        use std::collections::HashSet;
        let resolved: HashSet<ActionName> = ALL_ACTION_SURFACE_NAMES_VI
            .iter()
            .filter_map(|n| action_name_vi(n))
            .collect();
        assert_eq!(resolved.len(), 15, "15 source names must resolve to 15 distinct ActionName values (no aliasing)");
    }

    #[test]
    fn test_dang_xuat_does_not_resolve() {
        // Settled decision: "dang_xuat" is NOT a valid ActionName (no
        // runtime handler exists, and there's no auth/session concept).
        assert_eq!(action_name_vi("dang_xuat"), None);
    }

    #[test]
    fn test_dang_xuat_is_tracked_as_known_but_unsupported() {
        // "dang_xuat" must appear in the known-but-unsupported list (so
        // it gets a CLEAR diagnostic rather than falling into a generic
        // "unknown" bucket) — this test guards against someone
        // accidentally removing the name from the tracking list without
        // meaning to.
        assert!(KNOWN_BUT_UNSUPPORTED_ACTIONS_VI.contains(&"dang_xuat"));
    }

    #[test]
    fn test_unknown_name_returns_none() {
        assert_eq!(action_name_vi("khong_ton_tai_xyz"), None);
    }
}
