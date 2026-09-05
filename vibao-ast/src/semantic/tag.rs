// ============================================================
// VIBAO — vibao-ast/src/semantic/tag.rs
// Tag — SEMANTIC IDENTITY for every ViBao built-in element/tag.
//
// PRINCIPLE (per the design requirement): "one meaning should have one
// source of truth". Before this enum existed, "what tag is this" only
// existed as a raw String ("khoi", "text"...) matched INDEPENDENTLY AT
// EACH CALL SITE (~190 string-match sites counted during the audit,
// spread across vibaoc/src/codegen/{element,layout,props}.rs) — nothing
// enforced that those sites stayed in sync with each other, and any
// future locale would have needed its own separate string table, easy
// to drift out of sync.
//
// This enum is ONLY an identity — it carries no display name, no
// "corresponding HTML tag", no "is this a layout tag". That information
// is METADATA and belongs to `semantic::registry`, NOT redefined here
// (following the principle "enum = identity, registry = information
// about the identity" — avoiding the old mistake where PropsMap/
// TokenKind::Component were both an identity and a carrier of behavior
// at the same time).
//
// LOCALE: this enum carries NO Vietnamese or English names — the Rust
// variant names (`Box`, `Text`...) exist only to make the code
// readable for humans, NOT as an "English IR". Mapping "khoi"
// (Vietnamese) or "box" (English, if a locale is added later) onto
// Tag::Box is the LEXER's job (data, one table per locale) and is not
// duplicated anywhere else.
// ============================================================

use serde::{Deserialize, Serialize};

/// The FULL list of 67 ViBao built-in tags currently defined — maps 1:1
/// with `vibaoc::lexer::tables::component_set()` (that list will shrink
/// over time as component_set() is properly split into separate
/// Tag/ActionName/FunctionName tables — see the note in registry.rs).
///
/// Grouping order matches the original group comments in
/// component_set() to make cross-referencing easier during migration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Tag {
    // ── Text ─────────────────────────────────────────────────
    Text,
    H1,
    H2,
    H3,
    P,
    Nhan,

    // ── Media ────────────────────────────────────────────────
    Image,
    Video,
    Icon,

    // ── Interactive ──────────────────────────────────────────
    Button,
    Input,
    /// Source alias: "link" OR "lien_ket" — both map to THE SAME Tag
    /// variant (cross-checked against the old
    /// codegen/element.rs::tag_to_html(): `"link" | "lien_ket" => "a"` —
    /// no code path ever distinguished behavior between the two names,
    /// confirming this really is an alias and not two different
    /// concepts). Mapping both source spellings onto the SAME identity
    /// is the locale's job (see lexer::locale_vi); Tag only needs one
    /// variant.
    Link,

    // ── Layout (cross-checked with LAYOUT_TAGS in codegen/layout.rs) ──────
    Flex,
    Grid,
    Stack,
    Khoi,
    Cuon,
    CanGiua,
    Lop,
    DinhDau,
    DinhManHinh,

    // ── Spacing (handled as a Simple tag, NOT Layout — see
    // tag_to_html() in codegen/element.rs: "khoang_cach"=>"div",
    // "duong_ke"=>"hr") ──────────────────────────────────────
    KhoangCach,
    DuongKe,

    // ── Form (Simple — NOT part of BUILTIN_COMPLEX, even though it's
    // in the same "Form" group in the original comment; only "Form"
    // itself is Complex) ──
    NhomInput,
    ChonMot,
    HopKiem,
    LuaChon,

    // ── Complex UI (cross-checked with BUILTIN_COMPLEX in codegen/element.rs
    // — NOTE: VongQuay/ThanhTienTrinh/ThanhDieuHuong are NOT part of
    // BUILTIN_COMPLEX despite being in the same original comment group —
    // they are Simple tags) ──
    Form,
    Modal,
    Tabs,
    GapMo,
    BangChuyen,
    XuongTrang,
    VongQuay,
    ThanhTienTrinh,
    Bang,
    BieuDo,
    BanDo,
    ThanhDieuHuong,
    TrinhSoanThao,
}
