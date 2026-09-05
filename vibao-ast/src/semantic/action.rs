// ============================================================
// VIBAO — vibao-ast/src/semantic/action.rs
// ActionName — SEMANTIC IDENTITY for every ViBao builtin action (used
// inside event blocks: khi_nhan/khi_doi/on_tai/...).
//
// SCOPE AS DECIDED (after investigation + discussion with the user, see
// ARCHITECTURE_PROPOSAL.md, section "UPDATE — ActionName/FunctionName
// investigation"): EXACTLY 15 actions — ONLY actions that ALREADY HAVE a
// real runtime handler (`vibao-runtime/src/runtime/action.rs`), NOT
// including "dang_xuat" (log out) (confirmed: no runtime handler exists
// under any name, and no auth/session/token concept exists anywhere in
// the state model to "minimally implement" — clear decision: DO NOT
// create `ActionName::DangXuat` with a no-op or made-up handler, since
// that would create a "false semantic promise" — the compiler claiming
// ViBao understands dang_xuat while the runtime has no concept of
// logout whatsoever).
//
// PRINCIPLE SETTLED for ALL future actions: "only add an action to
// ActionName once ViBao has both a semantic contract AND a minimal
// runtime behavior defined." Any action that doesn't meet this bar MUST
// be reported by the compiler as "Known + Unsupported" (name is known,
// recognized as an action, but NOT YET implemented) — it must NOT fall
// into "Unknown" (treated as a typo/nonexistent) AND must NOT be
// silently treated as valid as if a handler existed. See
// `vibaoc::locale::action_vi::KNOWN_BUT_UNSUPPORTED_ACTIONS_VI` (in the
// `vibaoc` crate, DIFFERENT from this one — `vibao-ast` does not depend
// on `vibaoc` so it can't `use` it directly; only the fully-qualified
// name is mentioned here for reference) — where names like "dang_xuat"
// are listed. This is STILL ONLY DATA prepared in advance — no logic
// yet consumes it to actually produce the 3 distinct states at
// parse/validate time; see the TODO in `action_vi.rs` for the exact
// current status).
//
// SOURCE OF TRUTH (checked 1:1, no invented names): 11 side-effect
// actions via `action.rs::dispatch_function_call()` + 1 special action
// `goi_api` (already has its own AST struct `Action::ApiCall`, but still
// needs an identity because it is READ as a function name at the same
// syntax position as every other action — see
// `parser/action.rs::parse_action()` around line 80-107) + 3 array CRUD
// functions (`them_vao_mang`/`xoa_theo_id`/`cap_nhat_theo_id`, matched
// separately BEFORE the general FunctionCall branch in
// `action.rs::dispatch_one()` because they need the RAW variable NAME,
// not an already-evaluated value).
//
// LOCALE: similar to PropKey — the English variant names exist ONLY to
// keep the Rust code readable, not as a translation layer in the ViBao
// compilation pipeline. The Vietnamese locale (and any future locale)
// is a data table mapping surface names -> ActionName, just like
// `locale::prop_vi`.
// ============================================================

use serde::{Deserialize, Serialize};

/// The FULL list of 15 ViBao builtin actions that ALREADY HAVE a real
/// runtime handler — matches 1:1 with the classification table built in
/// ARCHITECTURE_PROPOSAL.md (section "UPDATE — ActionName/FunctionName
/// investigation").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ActionName {
    // ── Group A: side-effect actions via dispatch_function_call() (11) ──
    /// Source: "thong_bao" — shows a temporary toast.
    Notify,
    /// Source: "canh_bao" — shows an alert (different visual severity
    /// from Notify, see `dom.rs`).
    Alert,
    /// Source: "dieu_huong" — navigates a route via the SPA router.
    Navigate,
    /// Source: "mo_tab_moi" (open new tab).
    OpenNewTab,
    /// Source: "mo_modal" (open modal).
    OpenModal,
    /// Source: "dong_modal" (close modal).
    CloseModal,
    /// Source: "cuon_den" (scroll to).
    ScrollTo,
    /// Source: "cuon_len_dau" (scroll to top).
    ScrollToTop,
    /// Source: "luu_du_lieu" (save data) — POST via `api::call` (NOT
    /// sessionStorage as the name might suggest — confirmed by reading
    /// the actual code; this is a real network call, differing from the
    /// design of the old TS version this was ported from).
    SaveData,
    /// Source: "tai_du_lieu" (load data) — GET via `api::call`.
    LoadData,
    /// Source: "sao_chep" (copy) — copies text to the clipboard.
    CopyToClipboard,

    // ── Group B: special action with its own AST struct (1) ──────
    /// Source: "goi_api" (call api) — the parser special-cases this INTO
    /// `Action::ApiCall` (NOT `Action::FunctionCall`) RIGHT AFTER reading
    /// the name (see `parser/action.rs` around line 107) — but it's still
    /// an ActionName because it is read at the same syntax position as
    /// "action name", exactly like the other 14 names; the dev still
    /// needs to know this is a valid action name.
    ApiCall,

    // ── Group C: array CRUD, matched separately BEFORE the general
    // FunctionCall branch (3) ──────
    /// Source: "them_vao_mang" (push to array) — appends one element to
    /// the end of an array in state. The first argument MUST be a bare
    /// Expr::Variable (the array variable's own name), not a more
    /// complex expression.
    ArrayPush,
    /// Source: "xoa_theo_id" (remove by id) — removes the array element
    /// matching an id.
    ArrayRemoveById,
    /// Source: "cap_nhat_theo_id" (update by id) — updates the array
    /// element matching an id.
    ArrayUpdateById,
}


impl ActionName {
    /// The canonical name the AST/runtime currently uses when
    /// dispatching. The surface locale (Vietnamese/English) is
    /// normalized before an action enters the registry/runtime, so the
    /// runtime never needs to know which locale was used in the source.
    pub const fn runtime_name(self) -> &'static str {
        match self {
            Self::Notify => "thong_bao",
            Self::Alert => "canh_bao",
            Self::Navigate => "dieu_huong",
            Self::OpenNewTab => "mo_tab_moi",
            Self::OpenModal => "mo_modal",
            Self::CloseModal => "dong_modal",
            Self::ScrollTo => "cuon_den",
            Self::ScrollToTop => "cuon_len_dau",
            Self::SaveData => "luu_du_lieu",
            Self::LoadData => "tai_du_lieu",
            Self::CopyToClipboard => "sao_chep",
            Self::ApiCall => "goi_api",
            Self::ArrayPush => "them_vao_mang",
            Self::ArrayRemoveById => "xoa_theo_id",
            Self::ArrayUpdateById => "cap_nhat_theo_id",
        }
    }
}
