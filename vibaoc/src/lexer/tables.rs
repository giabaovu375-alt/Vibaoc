// ============================================================
// VIBAO COMPILER (Rust) — lexer/tables.rs
// KEYWORD & COLOR TABLES: lookup tables for keywords, components, color names
// ============================================================

use super::token::TokenKind;
use std::collections::HashMap;

// ════════════════════════════════════════════════════════════
// 2. KEYWORD & COLOR TABLES
// ════════════════════════════════════════════════════════════

/// The source of truth for this table is NOW
/// `crate::locale::{vi,en}::keyword_name_*` (see locale/vi.rs,
/// locale/en.rs) — THIS IS THE REAL WIRING into the lexer, per the
/// "ARCHITECTURAL DECISION... multi-locale model"
/// (ARCHITECTURE_PROPOSAL.md): the lexer ALWAYS checks BOTH the primary
/// locale (vi) AND the universal baseline locale (en) AT ONCE, never
/// choosing one or the other. This function used to maintain its own
/// separate, drifting table (Vietnamese only + 9 inconsistently mixed
/// English words) — RUNNING ALONGSIDE locale/vi.rs (which had those
/// exact same 21 words) without sharing it, making them 2 separate
/// sources of truth for the same concept. Now unified: keyword_map()
/// builds from exactly these 2 locale functions, with no second
/// hand-written list anymore — if locale/vi.rs or locale/en.rs gains a
/// new word, this function picks it up AUTOMATICALLY, with nothing to
/// change here.
pub(crate) fn keyword_map() -> HashMap<&'static str, TokenKind> {
    let mut m = HashMap::new();
    for name in ALL_KEYWORD_SURFACE_NAMES_VI.iter().copied() {
        if let Some(kind) = crate::locale::vi::keyword_name_vi(name) {
            m.insert(name, kind);
        }
    }
    for name in ALL_KEYWORD_SURFACE_NAMES_EN.iter().copied() {
        if let Some(kind) = crate::locale::en::keyword_name_en(name) {
            m.insert(name, kind);
        }
    }
    m
}

/// The 21 Vietnamese surface names — MUST match exactly the list that
/// `locale::vi::keyword_name_vi()` accepts (cross-checked by a test in
/// tables.rs::tests below). Needs to be listed explicitly here (instead
/// of derived backwards from TokenKind) because a HashMap key must be a
/// concrete &'static str, and keyword_name_vi() takes a &str parameter
/// rather than being able to enumerate its own domain.
const ALL_KEYWORD_SURFACE_NAMES_VI: &[&str] = &[
    "trang", "ung_dung", "chu_de", "trang_thai", "nhap", "tu",
    "neu", "khong_thi", "neu_nhieu", "truong_hop", "mac_dinh", "vong_lap",
    "khi_nhan", "khi_di_chuot", "khi_mat_focus", "khi_focus", "khi_doi",
    "khi_gui", "khi_cuon", "on_tai", "on_huy",
];

/// The 21 English surface names — same reasoning as above, matching
/// `locale::en::keyword_name_en()`.
const ALL_KEYWORD_SURFACE_NAMES_EN: &[&str] = &[
    "page", "app", "theme", "state", "import", "from",
    "if", "else", "else_if", "switch", "default", "loop",
    "on_click", "on_hover", "on_blur", "on_focus", "on_change",
    "on_submit", "on_scroll", "on_load", "on_unload",
];

/// The source of truth is now `crate::locale::{vi,en}::tag_name_*` —
/// same reasoning noted for keyword_map() above. This function lists 41
/// Vietnamese tag names + 40 English tag names, so "box" and "khoi"
/// BOTH emit TokenKind::Component right at the real lexer stage. The
/// action/function group (thong_bao/goi_api/gia_tien/...) NOW ALSO has
/// its own locale layer (`locale::action_vi`/`locale::function_vi`,
/// completed during the ActionName/FunctionName round) — no longer a
/// separate hand-written Vietnamese list like before, see the rest of
/// this function.
pub(crate) fn component_set() -> Vec<&'static str> {
    let mut v: Vec<&'static str> = Vec::new();
    for name in ALL_TAG_SURFACE_NAMES_VI.iter().copied() {
        if crate::locale::vi::tag_name_vi(name).is_some() {
            v.push(name);
        }
    }
    for name in ALL_TAG_SURFACE_NAMES_EN.iter().copied() {
        if crate::locale::en::tag_name_en(name).is_some() {
            v.push(name);
        }
    }
    // The source of truth is now `crate::locale::{resolve_action_name,
    // resolve_function_name}` (same principle applied to Tag/Keyword
    // above) — this group used to hand-list 19 fixed names (including
    // "dang_xuat"), RUNNING ALONGSIDE `locale::action_vi`/
    // `locale::function_vi` (which already existed from the
    // ActionName/FunctionName round) without sharing them — another
    // case of "2 sources of truth for one concept", the same kind of
    // risk already fixed for Tag/Keyword/Prop. Now unified: "dang_xuat"
    // NO LONGER appears here (since `resolve_action_name("dang_xuat")`
    // returns `None`, per the SETTLED decision: dang_xuat has no
    // runtime handler and must not create a "false semantic promise" —
    // see AUDIT.md/ARCHITECTURE_PROPOSAL.md, ActionName section). The
    // lexer NO LONGER emits TokenKind::Component for "dang_xuat" — it
    // falls into the final branch of classify_identifier() and becomes
    // an ORDINARY TokenKind::Identifier (NOT a lexer error —
    // classify_identifier() has no concept of "error"; any name that
    // doesn't match a keyword/component/color is simply a valid
    // Identifier).
    //
    // CORRECTED A WRONG NOTE (team review,
    // VIBAOC_LEXER_COMPONENT_SET_BUGS.md, item 1): an earlier version of
    // this comment claimed "a dev writing dang_xuat(...) will get an
    // unknown identifier error AT THE PARSER LAYER" — RE-VERIFIED AND
    // CONFIRMED WRONG. `parser/action.rs::parse_action()` reads the name
    // via `expect_identifier_like()` (which accepts Identifier,
    // Component, OR ColorName — no distinction made), so
    // `dang_xuat(...)` STILL parses successfully into
    // `Action::FunctionCall { name: "dang_xuat", ... }` — there is NO
    // syntax error whatsoever. `check_action_name()` (validator.rs)
    // operates on the already-parsed `name: String`, and does NOT
    // depend on the original lexer TokenKind — so the "not yet
    // supported by ViBao..." message STILL fires EXACTLY AS DESIGNED,
    // nothing is lost. There is NO real "tradeoff" happening here — a
    // real cargo build/test is still needed for final confirmation
    // (reading the code gives a very clear answer, but hasn't been
    // proven 100% by an actual build yet).
    for name in ALL_ACTION_SURFACE_NAMES_VI.iter().chain(ALL_ACTION_SURFACE_NAMES_EN.iter()).copied() {
        if crate::locale::resolve_action_name(name).is_some() {
            v.push(name);
        }
    }
    for name in ALL_FUNCTION_SURFACE_NAMES_VI.iter().chain(ALL_FUNCTION_SURFACE_NAMES_EN.iter()).copied() {
        if crate::locale::resolve_function_name(name).is_some() {
            v.push(name);
        }
    }
    v
}

/// The 15 Vietnamese surface names for ActionName — MUST match the
/// domain that `locale::action_vi::action_name_vi()` accepts
/// (cross-checked by a test below). DELIBERATELY does NOT include
/// "dang_xuat" — see the explanation in `component_set()`.
const ALL_ACTION_SURFACE_NAMES_VI: &[&str] = &[
    "thong_bao", "canh_bao", "dieu_huong", "mo_tab_moi",
    "mo_modal", "dong_modal", "cuon_den", "cuon_len_dau",
    "luu_du_lieu", "tai_du_lieu", "sao_chep", "goi_api",
    "them_vao_mang", "xoa_theo_id", "cap_nhat_theo_id",
];

/// The 6 Vietnamese surface names for FunctionName — matches the domain
/// of `locale::function_vi::function_name_vi()`.
const ALL_FUNCTION_SURFACE_NAMES_VI: &[&str] = &[
    "gia_tien", "ngay", "rut_gon", "hoa_chu", "phan_tram", "lam_tron",
];

/// English action/function names — same semantic identity, only the surface differs.
const ALL_ACTION_SURFACE_NAMES_EN: &[&str] = &[
    "notify", "alert", "navigate", "open_new_tab", "open_modal", "close_modal",
    "scroll_to", "scroll_to_top", "save_data", "load_data", "copy_to_clipboard",
    "api_call", "array_push", "array_remove_by_id", "array_update_by_id",
];

const ALL_FUNCTION_SURFACE_NAMES_EN: &[&str] = &[
    "format_price", "format_date", "truncate", "uppercase", "format_percent", "round",
];

/// The 41 Vietnamese tag names (including "link"/"lien_ket" — 2
/// spellings for the same Tag) — matches `locale::vi::tag_name_vi()`.
const ALL_TAG_SURFACE_NAMES_VI: &[&str] = &[
    "text", "h1", "h2", "h3", "p", "nhan",
    "image", "video", "icon",
    "button", "input", "link", "lien_ket",
    "flex", "grid", "stack", "khoi", "cuon", "can_giua", "lop",
    "dinh_dau", "dinh_man_hinh",
    "khoang_cach", "duong_ke",
    "form", "nhom_input", "chon_mot", "hop_kiem", "lua_chon",
    "modal", "tabs", "gap_mo", "bang_chuyen", "xuong_trang",
    "vong_quay", "thanh_tien_trinh", "bang", "bieu_do", "ban_do",
    "thanh_dieu_huong", "trinh_soan_thao",
];

/// The 40 English tag names — matches `locale::en::tag_name_en()`.
const ALL_TAG_SURFACE_NAMES_EN: &[&str] = &[
    "text", "h1", "h2", "h3", "p", "label",
    "image", "video", "icon",
    "button", "input", "link",
    "flex", "grid", "stack", "box", "scroll", "container", "layer", "sticky", "fixed",
    "spacer", "divider",
    "form", "input_group", "radio", "checkbox", "select",
    "modal", "tabs", "accordion", "carousel", "pagination", "spinner",
    "progress", "table", "chart", "map", "nav", "editor",
];

/// The action/utility-function NAME group (the SECOND group in
/// component_set() above, marked "Feedback / actions") — kept SEPARATE
/// from Tag (the rest of component_set()) so it's possible to tell
/// whether a TokenKind::Component(surface_name) that
/// `locale::resolve_tag()` fails to resolve is failing BECAUSE it's a
/// valid action/function name used in the WRONG POSITION (e.g. writing
/// "thong_bao(...)" in a page body instead of inside an event block),
/// versus some genuinely different error (which shouldn't happen if the
/// lexer/locale invariant is maintained correctly).
///
/// The source of truth is now `crate::locale::{resolve_action_name,
/// resolve_function_name}` — this function used to HAND-COPY 19
/// elements from the "Feedback / actions" group (including
/// "dang_xuat"), kept separate from `component_set()` despite meaning
/// the same thing — RUNNING ALONGSIDE 2 other real sources
/// (`locale::action_vi`/`locale::function_vi`) without sharing them,
/// another instance of "2 sources for one concept" like what was
/// already fixed for Tag/Keyword/Prop. Now unified: uses EXACTLY these
/// 2 resolve functions — "dang_xuat" NOW returns `false` (no longer "a
/// valid action used in the wrong position" — it isn't a valid action
/// AT ALL anymore, see the settled decision in
/// AUDIT.md/ARCHITECTURE_PROPOSAL.md, ActionName section). A
/// cross-check test ensures this list never drifts from the real
/// `component_set()` (see tests.rs at the bottom of this file).
pub(crate) fn is_known_action_or_function_name(name: &str) -> bool {
    crate::locale::resolve_action_name(name).is_some()
        || crate::locale::resolve_function_name(name).is_some()
}

pub(crate) fn color_map() -> HashMap<&'static str, &'static str> {
    let mut m = HashMap::new();
    m.insert("trang", "#FFFFFF");
    m.insert("den", "#000000");
    m.insert("do", "#E53E3E");
    m.insert("xanh", "#3182CE");
    m.insert("xanh_la", "#38A169");
    m.insert("vang", "#F59E0B");
    m.insert("hong", "#D53F8C");
    m.insert("tim", "#805AD5");
    m.insert("cam", "#DD6B20");
    m.insert("xam", "#718096");
    m.insert("xam_nhat", "#F7FAFC");
    m.insert("xam_dam", "#2D3748");
    m.insert("luc", "#25855A");
    m.insert("nau", "#7B341E");
    m
}

/// The names of the 3 built-in color functions (used in expressions,
/// e.g. `trong_suot(do, 50)`) — the SINGLE SOURCE OF TRUTH, unifying 2
/// places that used to be defined independently
/// (parser/expr.rs::parse_color_func recognizes the function name to
/// start parsing; codegen/mod.rs::color_func_name generates that same
/// name back out into CSS). Any drift between the two used to go
/// undetected (the parser could parse it but codegen generated the
/// wrong name, or vice versa) because there was no shared table to
/// check against — now both sides read from here.
pub(crate) fn color_func_map() -> [(vibao_ast::ColorFuncKind, &'static str); 3] {
    use vibao_ast::ColorFuncKind;
    [
        (ColorFuncKind::TrongSuot, "trong_suot"),
        (ColorFuncKind::LamSang, "lam_sang"),
        (ColorFuncKind::LamToi, "lam_toi"),
    ]
}

/// Color function name (string) -> ColorFuncKind, used by the parser
/// when it recognizes the start of a color function call.
pub(crate) fn resolve_color_func_name(name: &str) -> Option<vibao_ast::ColorFuncKind> {
    color_func_map().iter().find(|(_, n)| *n == name).map(|(k, _)| *k)
}

/// ColorFuncKind -> function name (string), used by codegen when generating CSS
/// (vd `trong_suot(#E53E3E, 50)`).
pub fn color_func_name(func: vibao_ast::ColorFuncKind) -> &'static str {
    color_func_map()
        .iter()
        .find(|(k, _)| *k == func)
        .map(|(_, n)| *n)
        .expect("color_func_map() must list every ColorFuncKind variant")
}

/// Returns the hex code for a known ViBao color name, or `None` if the
/// name is NOT in the color table.
///
/// BUG ALREADY FIXED: this function used to return a plain `String`
/// using `unwrap_or_else(|| name.to_string())` — meaning a NONEXISTENT
/// color name (e.g. "xanh_duong", "do_tuoi" — not among the 12 names in
/// color_map()) was silently used VERBATIM as the CSS value
/// (`color: xanh_duong;`); this invalid CSS was then completely ignored
/// by the browser with no warning at all — the dev had no idea the
/// compiler had "swallowed" their color-name typo. Switching to
/// `Option<String>` forces EVERY call site to explicitly decide how to
/// handle the "not found" case (raising a clear error), instead of one
/// silently-wrong fallback being repeated at multiple call sites.
pub fn resolve_color_name(name: &str) -> Option<String> {
    color_map().get(name).map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use vibao_ast::ColorFuncKind;

    #[test]
    fn test_color_func_name_and_resolve_are_inverse() {
        // Round-trip: every ColorFuncKind variant -> name -> back to the
        // same variant — ensures color_func_name()/resolve_color_func_name()
        // never drift apart (this is exactly the bug that was fixed:
        // the parser and codegen used to each keep their own hand-written
        // table, with nothing guaranteeing they matched).
        for (kind, _) in color_func_map() {
            let name = color_func_name(kind);
            assert_eq!(resolve_color_func_name(name), Some(kind));
        }
    }

    #[test]
    fn test_resolve_color_func_name_unknown_is_none() {
        assert_eq!(resolve_color_func_name("khong_ton_tai"), None);
    }

    #[test]
    fn test_color_func_name_known_values() {
        assert_eq!(color_func_name(ColorFuncKind::TrongSuot), "trong_suot");
        assert_eq!(color_func_name(ColorFuncKind::LamSang), "lam_sang");
        assert_eq!(color_func_name(ColorFuncKind::LamToi), "lam_toi");
    }

    // ── is_known_action_or_function_name() cross-checked against
    // component_set() (bug already fixed: parser/app.rs used to
    // WRONGLY report a "compiler bug" for an ordinary user syntax
    // mistake — e.g. writing "thong_bao(...)" in the wrong position —
    // because resolve_tag() didn't cover the action/function group. The
    // test below ensures is_known_action_or_function_name() (a
    // 19-element hand-copied list) never drifts from the real
    // component_set(), following the same pattern used for every other
    // cross-check table in the project.)

    #[test]
    fn test_action_function_names_are_exactly_component_set_minus_tags() {
        // "A valid Tag" here is checked via locale::resolve_tag() — the
        // real source of truth for Tag, not a hand-copied list of 41
        // names (avoiding having 3 sources of truth at once). Every name
        // in component_set() that does NOT resolve to a Tag MUST be
        // recognized by is_known_action_or_function_name(), and vice
        // versa — otherwise one of the two tables has drifted.
        for name in component_set() {
            let is_tag = crate::locale::resolve_tag(name).is_some();
            let is_action = is_known_action_or_function_name(name);
            assert!(
                is_tag != is_action,
                "'{}' must belong to EXACTLY 1 of the 2 groups (Tag XOR action/function), \
                 currently: is_tag={}, is_action={}",
                name, is_tag, is_action
            );
        }
    }

    #[test]
    fn test_is_known_action_or_function_name_rejects_tag_names() {
        // Symmetric check: a Tag name must NOT leak into is_known_action_or_function_name().
        assert!(!is_known_action_or_function_name("khoi"));
        assert!(!is_known_action_or_function_name("text"));
    }

    #[test]
    fn test_is_known_action_or_function_name_accepts_every_action() {
        // UPDATED: "dang_xuat" removed from this list (per the settled
        // decision — dang_xuat is not a valid action, see AUDIT.md,
        // "Removing dang_xuat from component_set()"). The list now has 21
        // names (15 actions + 6 functions — CORRECTING an earlier wrong
        // count in this comment, per team review
        // VIBAOC_LEXER_COMPONENT_SET_BUGS.md, item 2: a previous version
        // incorrectly said "18 names (14 actions + 4...)"). See the
        // separate test_dang_xuat_is_no_longer_recognized below for the
        // excluded case.
        for name in [
            "thong_bao", "canh_bao", "dieu_huong", "mo_tab_moi",
            "mo_modal", "dong_modal", "cuon_den", "cuon_len_dau",
            "luu_du_lieu", "tai_du_lieu", "sao_chep", "goi_api",
            "them_vao_mang", "xoa_theo_id", "cap_nhat_theo_id",
            "gia_tien", "ngay", "rut_gon", "hoa_chu", "phan_tram",
            "lam_tron",
        ] {
            assert!(is_known_action_or_function_name(name), "'{}' must be recognized", name);
        }
    }

    #[test]
    fn test_dang_xuat_is_no_longer_recognized() {
        // Per the settled decision (AUDIT.md/ARCHITECTURE_PROPOSAL.md,
        // ActionName section): "dang_xuat" has no runtime handler, so no
        // ActionName was created for it — it's NO LONGER recognized as a
        // valid action by the lexer/is_known_action_or_function_name().
        assert!(!is_known_action_or_function_name("dang_xuat"));
        assert!(!component_set().contains(&"dang_xuat"));
    }

    #[test]
    fn test_action_function_surface_names_do_not_overlap_with_tag_keyword() {
        // A proactive check (already cross-verified with a script before
        // writing this code, confirmed 100% no overlap) — guards against
        // someone later accidentally choosing an action/function name
        // that collides with an existing tag/keyword, which would create
        // ambiguity at the lexer level.
        let mut all_others: std::collections::HashSet<&str> = std::collections::HashSet::new();
        all_others.extend(ALL_TAG_SURFACE_NAMES_VI.iter().copied());
        all_others.extend(ALL_TAG_SURFACE_NAMES_EN.iter().copied());
        all_others.extend(ALL_KEYWORD_SURFACE_NAMES_VI.iter().copied());
        all_others.extend(ALL_KEYWORD_SURFACE_NAMES_EN.iter().copied());
        for name in ALL_ACTION_SURFACE_NAMES_VI.iter().chain(ALL_ACTION_SURFACE_NAMES_EN.iter()).copied() {
            assert!(!all_others.contains(name), "'{}' (action) collides with an existing tag/keyword", name);
        }
        for name in ALL_FUNCTION_SURFACE_NAMES_VI.iter().chain(ALL_FUNCTION_SURFACE_NAMES_EN.iter()).copied() {
            assert!(!all_others.contains(name), "'{}' (function) collides with an existing tag/keyword", name);
        }
        // Action and Function must also not collide with each other.
        for name in ALL_ACTION_SURFACE_NAMES_VI.iter().copied() {
            assert!(
                !ALL_FUNCTION_SURFACE_NAMES_VI.contains(&name),
                "'{}' appears in BOTH action AND function — creates ambiguity", name
            );
        }
    }

    #[test]
    fn test_component_set_reflects_every_actionname_and_functionname_variant() {
        // BUG FIX (VIBAOC_LEXER_COMPONENT_SET_BUGS.md, item 3, team
        // review, highest priority) — BEFORE this test, every
        // cross-check test went in the SAME direction: starting from the
        // Vietnamese name lists (ALL_ACTION_SURFACE_NAMES_VI/
        // ALL_FUNCTION_SURFACE_NAMES_VI), then checking that they
        // resolve — if someone added a NEW variant to
        // `ActionName`/`FunctionName` (vibao-ast) and to
        // `action_vi.rs`/`function_vi.rs`, but FORGOT to add the name to
        // these 2 const lists in tables.rs, NONE of the tests above
        // would catch it — because both the const lists AND
        // component_set() would have "forgotten" the exact same thing
        // together, with nothing to detect the drift.
        //
        // This test goes in the OPPOSITE direction: starting from the
        // `ActionName`/`FunctionName` ENUM ITSELF (the most authoritative
        // source — Rust's match exhaustiveness guarantees at compile
        // time that no variant is missed), mapping BACKWARDS to a
        // Vietnamese surface name through a hand-copied table RIGHT HERE
        // (NOT reusing ALL_ACTION_SURFACE_NAMES_VI — reusing it would
        // make the test self-confirming, losing all its bug-catching
        // value), then confirming that EXACT name is present in the real
        // `component_set()`. If someone adds a new enum variant and
        // FORGETS both of the other two places, the NON-EXHAUSTIVE MATCH
        // below will be a COMPILE ERROR immediately (no need to even run
        // the test to find out) — this is exactly the core benefit the
        // team review proposed.
        fn action_name_to_vi(a: vibao_ast::ActionName) -> &'static str {
            use vibao_ast::ActionName::*;
            match a {
                Notify => "thong_bao",
                Alert => "canh_bao",
                Navigate => "dieu_huong",
                OpenNewTab => "mo_tab_moi",
                OpenModal => "mo_modal",
                CloseModal => "dong_modal",
                ScrollTo => "cuon_den",
                ScrollToTop => "cuon_len_dau",
                SaveData => "luu_du_lieu",
                LoadData => "tai_du_lieu",
                CopyToClipboard => "sao_chep",
                ApiCall => "goi_api",
                ArrayPush => "them_vao_mang",
                ArrayRemoveById => "xoa_theo_id",
                ArrayUpdateById => "cap_nhat_theo_id",
            }
        }
        fn function_name_to_vi(f: vibao_ast::FunctionName) -> &'static str {
            use vibao_ast::FunctionName::*;
            match f {
                FormatPrice => "gia_tien",
                FormatDate => "ngay",
                Truncate => "rut_gon",
                Uppercase => "hoa_chu",
                FormatPercent => "phan_tram",
                Round => "lam_tron",
            }
        }
        const ALL_ACTIONS: [vibao_ast::ActionName; 15] = {
            use vibao_ast::ActionName::*;
            [
                Notify, Alert, Navigate, OpenNewTab, OpenModal, CloseModal,
                ScrollTo, ScrollToTop, SaveData, LoadData, CopyToClipboard,
                ApiCall, ArrayPush, ArrayRemoveById, ArrayUpdateById,
            ]
        };
        const ALL_FUNCTIONS: [vibao_ast::FunctionName; 6] = {
            use vibao_ast::FunctionName::*;
            [FormatPrice, FormatDate, Truncate, Uppercase, FormatPercent, Round]
        };

        let cs = component_set();
        for a in ALL_ACTIONS {
            let vi_name = action_name_to_vi(a);
            assert!(
                cs.contains(&vi_name),
                "ActionName::{:?} (Vietnamese name '{}') MUST be present in the real component_set() — \
                 if this fails, the 2 sources (the real enum vs component_set()) have DRIFTED APART",
                a, vi_name
            );
        }
        for f in ALL_FUNCTIONS {
            let vi_name = function_name_to_vi(f);
            assert!(
                cs.contains(&vi_name),
                "FunctionName::{:?} (Vietnamese name '{}') MUST be present in the real component_set()",
                f, vi_name
            );
        }
        for name in ALL_ACTION_SURFACE_NAMES_EN {
            assert!(cs.contains(name), "English action '{}' MUST be present in component_set()", name);
        }
        for name in ALL_FUNCTION_SURFACE_NAMES_EN {
            assert!(cs.contains(name), "English function '{}' MUST be present in component_set()", name);
        }
    }

    // ── NEW tests: confirm keyword_map()/component_set() are NOW
    // REALLY wired into the locale layer (vi + en at once), per the
    // "ARCHITECTURAL DECISION... multi-locale model" — this is the
    // empirical proof that the REAL lexer (not just resolve_tag() at a
    // later step) now accepts both spellings for EVERY tag/keyword, not
    // just tags like before.

    #[test]
    fn test_keyword_map_accepts_both_vi_and_en_for_every_keyword() {
        let km = keyword_map();
        let pairs = [
            ("trang", "page"), ("ung_dung", "app"), ("chu_de", "theme"),
            ("trang_thai", "state"), ("nhap", "import"), ("tu", "from"),
            ("neu", "if"), ("khong_thi", "else"), ("neu_nhieu", "else_if"),
            ("truong_hop", "switch"), ("mac_dinh", "default"), ("vong_lap", "loop"),
            ("khi_nhan", "on_click"), ("khi_di_chuot", "on_hover"),
            ("khi_mat_focus", "on_blur"), ("khi_focus", "on_focus"),
            ("khi_doi", "on_change"), ("khi_gui", "on_submit"),
            ("khi_cuon", "on_scroll"), ("on_tai", "on_load"), ("on_huy", "on_unload"),
        ];
        assert_eq!(pairs.len(), 21);
        for (vi_name, en_name) in pairs {
            assert!(km.contains_key(vi_name), "keyword_map() is missing '{}' (vi)", vi_name);
            assert!(km.contains_key(en_name), "keyword_map() is missing '{}' (en)", en_name);
            assert_eq!(
                km.get(vi_name), km.get(en_name),
                "'{}' (vi) and '{}' (en) must map to the same TokenKind in the real keyword_map()",
                vi_name, en_name
            );
        }
        // No more separate, drifting hand-written list — keyword_map()
        // used to have EXACTLY 21 entries (9 pure-Vietnamese keywords +
        // 9 mixed-in English words + 2 on_tai/on_huy = 20, see git
        // history); it now has 42 entries (21 vi + 21 en, with no name
        // colliding between the two groups — confirmed by
        // test_keyword_vi_en_surface_names_do_not_overlap below).
        assert_eq!(km.len(), 42, "keyword_map() must have exactly 21 (vi) + 21 (en) = 42 entries");
    }

    #[test]
    fn test_component_set_accepts_both_vi_and_en_for_every_tag() {
        let cs = component_set();
        // "box" (en) and "khoi" (vi) now BOTH emit TokenKind::Component
        // in the real lexer — previously only "khoi" worked (see
        // AUDIT.md).
        assert!(cs.contains(&"box"), "component_set() must accept 'box' (en)");
        assert!(cs.contains(&"khoi"), "component_set() must still accept 'khoi' (vi)");
        assert!(cs.contains(&"label"), "component_set() must accept 'label' (en)");
        assert!(cs.contains(&"nhan"), "component_set() must still accept 'nhan' (vi)");
        // 41 (vi tags, including the lien_ket alias) + 40 (en tags) + 15
        // (vi actions) + 15 (en actions) + 6 (vi functions) + 6 (en
        // functions) = 123. (Corrected a stale comment here that still
        // said "= 100" from before the action/function locale layer was
        // wired in — the assertion below was already correct at 123,
        // only the explanatory comment above it was out of date.)
        assert_eq!(cs.len(), 123, "component_set() must have exactly 41 (vi tag) + 40 (en tag) + 15 (vi action) + 15 (en action) + 6 (vi function) + 6 (en function) = 123 entries");
    }

    #[test]
    fn test_keyword_vi_en_surface_names_do_not_overlap() {
        // If a word were ever accidentally valid in BOTH tables at once,
        // that's the exact risk documented in
        // ARCHITECTURE_PROPOSAL.md ("when 2 tables are checked at the
        // same time, there is a theoretical chance of a name collision")
        // — this test checks PROACTIVELY, instead of waiting to discover
        // it as a real bug.
        for vi_name in ALL_KEYWORD_SURFACE_NAMES_VI {
            assert!(
                !ALL_KEYWORD_SURFACE_NAMES_EN.contains(vi_name),
                "'{}' appears in BOTH the vi and en keyword tables — creates ambiguity", vi_name
            );
        }
    }

    #[test]
    fn test_tag_vi_en_surface_names_overlap_only_on_intentional_english_loanwords() {
        // Tags that ALREADY borrow their English name in locale/vi.rs
        // (text/h1../button/flex/grid/stack/form/modal/tabs/image/
        // video/icon/input/link — see vocabulary.rs, deliberately left
        // untranslated) WILL collide by name between the two tables —
        // this is INTENTIONAL, not a risk. This test confirms the
        // overlap set exactly matches the known list, with no
        // UNINTENDED collision slipping in from anywhere else.
        let expected_overlap: std::collections::HashSet<&str> = [
            "text", "h1", "h2", "h3", "p", "image", "video", "icon",
            "button", "input", "link", "flex", "grid", "stack", "form",
            "modal", "tabs",
        ].into_iter().collect();
        let actual_overlap: std::collections::HashSet<&str> = ALL_TAG_SURFACE_NAMES_VI
            .iter()
            .copied()
            .filter(|n| ALL_TAG_SURFACE_NAMES_EN.iter().any(|e| e == n))
            .collect();
        assert_eq!(
            actual_overlap, expected_overlap,
            "the set of tags colliding between vi/en has changed from the known list of intentional English loanwords"
        );
    }
}
