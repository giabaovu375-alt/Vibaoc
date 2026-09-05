// ============================================================
// VIBAO — vibao-ast/src/semantic/prop.rs
// PropKey — SEMANTIC IDENTITY for every ViBao built-in prop key
// (applies to BOTH Simple Element AND Layout Element — this is the
// core difference versus before, see the "assessment" below).
//
// PRINCIPLE (inheriting the same principle applied to Tag in tag.rs):
// "one meaning should have one source of truth". Before this enum,
// "what prop is this" existed as a raw String matched in 2 INDEPENDENT
// PLACES:
//   - `vibaoc::codegen::props::KNOWN_PROP_KEYS` (45 names — used only
//     for Simple Element, via `expand_props()`)
//   - scattered across 9 `resolve_*` functions in
//     `vibaoc::codegen::layout` (NO equivalent constant table — only
//     implicit in each match branch, used for Layout Element)
// A full cross-reference table was built (see ARCHITECTURE_PROPOSAL.md,
// section 3.3) and found: 57 UNIQUE prop keys total (after merging the
// "mau"/"mau_chu" alias), of which 17 props shared the SAME MEANING in
// both places (nothing ENFORCED that they stayed in sync besides the
// author's discipline — the root cause of the historical BUG-16:
// map_justify once drifted apart between the two versions), and 12
// props existed ONLY in layout.rs, completely ABSENT from
// KNOWN_PROP_KEYS — the root cause of BUG-25 (a typo'd prop name on a
// layout element was never flagged, unlike on a Simple Element).
//
// This enum is ONLY an identity — it carries no CSS property name, no
// "does this prop apply to Simple or Layout or both", and no value
// mapping (e.g. "giua" -> "center"). That information is METADATA,
// belonging to `semantic::registry` (PropSpec, added in this same
// round) — not redefined here, following the same "enum = identity,
// registry = information about the identity" principle applied to Tag.
//
// LOCALE: this enum carries NO Vietnamese or English names — the Rust
// variant names (`BackgroundColor`, `Width`...) exist only to make the
// Rust code readable for humans (named after the MEANING/CSS property
// it produces, NOT a mechanical word-for-word translation of the
// original Vietnamese name — e.g. "mau_nen" literally means "background
// color" and the variant is `BackgroundColor`, matching the actual CSS
// property, not "ColorBackground" or some other name). Mapping
// "mau_nen" (Vietnamese) or "background_color"/the equivalent English
// name (if ViBao supports English prop keys later) onto
// PropKey::BackgroundColor is the LEXER/locale layer's job (data, one
// table per locale), NOT duplicated anywhere else — following the same
// model already applied to Tag.
//
// ARCHITECTURAL DECISION SETTLED (see ARCHITECTURE_PROPOSAL.md, section
// "DECISION SETTLED... Direction 2"): variant names use ENGLISH — this
// decision came AFTER `Tag` (tag.rs) was written, so `Tag` STILL keeps
// its old Vietnamese names (Khoi/CanGiua/LuaChon/...) — NOT retroactively
// synced to `Tag`, per the explicit direction "leave the old tags alone
// for now" (not expanding scope beyond what was asked). `PropKey` is the
// FIRST enum to follow the new English naming convention.
// ============================================================

use serde::{Deserialize, Serialize};

/// The FULL list of 57 ViBao built-in prop keys currently defined —
/// matches 1:1 with the table built in ARCHITECTURE_PROPOSAL.md, section
/// 3.3 (58 source names, merged down to 57 variants after unifying the
/// "mau"/"mau_chu" alias into one identity — the same kind of alias
/// `Tag::Link` already does for "link"/"lien_ket").
///
/// Grouping order follows MEANING (Color/Size/Spacing/Typography/...),
/// NOT the old source file (props.rs vs layout.rs) — since both sources
/// are now unified into a single list, there's no longer a reason to
/// keep that distinction at the identity level (distinguishing "which
/// props apply to Simple/Layout/both" is PropSpec's job in
/// registry.rs).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PropKey {
    // ── Color ────────────────────────────────────────────────
    /// Source: "mau_nen".
    BackgroundColor,
    /// Source: "mau" OR "mau_chu" — 2 Vietnamese names, THE SAME
    /// identity (cross-checked against the old props.rs:
    /// `"mau" | "mau_chu" => ... "color"` — both always matched the
    /// same branch, no code path ever distinguished behavior between
    /// them, confirming this really is an alias, not two different
    /// concepts).
    Color,
    /// Source: "mau_vien".
    BorderColor,

    // ── Size ─────────────────────────────────────────────────
    /// Source: "rong" (width).
    Width,
    /// Source: "cao" (height).
    Height,
    /// Source: "max_rong" (max width).
    MaxWidth,
    /// Source: "min_rong" (min width) — was ONLY in the old layout.rs
    /// (resolve_box), NOT part of the old KNOWN_PROP_KEYS (see BUG-25).
    MinWidth,
    /// Source: "max_cao" (max height) — was ONLY in the old layout.rs
    /// (resolve_box, BUG-25).
    MaxHeight,
    /// Source: "min_cao" (min height) — was ONLY in the old layout.rs
    /// (resolve_box, BUG-25).
    MinHeight,

    // ── Spacing / Border / Effects ───────────────────────────
    /// Source: "radius".
    Radius,
    /// Source: "dem" (padding).
    Padding,
    /// Source: "le" (margin).
    Margin,
    /// Source: "vien" (border) — border thickness (defaults to
    /// borderStyle: solid on the old Simple Element; see PropSpec/
    /// registry for the exact behavior).
    Border,
    /// Source: "kieu_vien" (border style).
    BorderStyle,
    /// Source: "bong" (shadow).
    Shadow,
    /// Source: "cuon_tran" (overflow).
    Overflow,
    /// Source: "tang_z" (z-index).
    ZIndex,

    // ── Typography ───────────────────────────────────────────
    /// Source: "co" (font size). Also one of the 6 valid overrides for
    /// the old `@di_dong`/`@may_tinh_bang`/`@may_tinh` (responsive)
    /// blocks.
    FontSize,
    /// Source: "dam" (bold) — boolean, maps to fontWeight bold/normal.
    Bold,
    /// Source: "nghieng" (italic) — boolean, maps to fontStyle italic.
    Italic,
    /// Source: "gach_chan" (underline) — boolean, maps to
    /// textDecoration underline.
    Underline,
    /// Source: "can" (align) — maps a Vietnamese value
    /// (trai/phai/giua/deu = left/right/center/justify) to a CSS
    /// keyword; the CSS meaning differs by tag (textAlign for
    /// text-bearing tags, justifyContent/justifyItems for layout tags)
    /// — see PropSpec/the corresponding value-mapping function. This is
    /// NOT two different PropKeys (it's the same prop the user types;
    /// only the CSS target differs based on tag context, exactly like
    /// Tag itself isn't split by usage context).
    Align,
    /// Source: "hang" (line height).
    LineHeight,
    /// Source: "khoang_chu" (letter spacing).
    LetterSpacing,
    /// Source: "bien_doi" (text transform).
    TextTransform,
    /// Source: "phong_chu" (font family).
    FontFamily,

    // ── Flex / Grid layout ───────────────────────────────────
    /// Source: "huong" (direction, flex-direction row/column). ALSO used
    /// for "cuon" (scroll) (the old resolve_scroll also read "huong" but
    /// with a DIFFERENT value domain — "ngang"/"doc" (horizontal/
    /// vertical) instead of "row"/"column" — see PropSpec/registry notes
    /// for the per-tag distinction, same pattern as "can"/Align above).
    Direction,
    /// Source: "gap".
    Gap,
    /// Source: "gap_doc" (row gap) — was ONLY in the old layout.rs (BUG-25).
    RowGap,
    /// Source: "gap_ngang" (column gap) — was ONLY in the old layout.rs (BUG-25).
    ColumnGap,
    /// Source: "doc" (align-items).
    AlignItems,
    /// Source: "boc" (wrap) — boolean, maps to flexWrap wrap.
    Wrap,
    /// Source: "cot" (columns, grid-template-columns) — was ONLY in the
    /// old layout.rs, specific to "grid" (BUG-25).
    Columns,
    /// Source: "hang_luoi" (grid rows, grid-template-rows) — was ONLY in
    /// the old layout.rs, specific to "grid" (BUG-25).
    Rows,

    // ── Position / Transform (old layout.rs: resolve_box/resolve_fixed
    // /resolve_sticky_top) ───────────────────────────────────
    /// Source: "tran_x" (transform translateX) — ONLY in layout.rs (BUG-25).
    TranslateX,
    /// Source: "tran_y" (transform translateY) — ONLY in layout.rs (BUG-25).
    TranslateY,
    /// Source: "vi_tri" (position) — fixed edge (tren/duoi/trai/phai =
    /// top/bottom/left/right) for "dinh_man_hinh" — ONLY in layout.rs
    /// (BUG-25).
    Position,
    /// Source: "offset" — sticky offset distance for "dinh_dau" — ONLY
    /// in layout.rs (BUG-25).
    Offset,

    // ── Responsive-only (only valid inside the old @di_dong/
    // @may_tinh_bang/@may_tinh blocks — resolve_responsive_css) ──
    /// Source: "an" (hidden) — boolean, maps to display:none. Only valid
    /// inside a responsive block (BUG-25 — was never part of
    /// KNOWN_PROP_KEYS).
    Hidden,

    // ── Media / Object ───────────────────────────────────────
    /// Source: "khop" (fit, object-fit).
    Fit,
    /// Source: "nguon" (source) — maps to the HTML "src" attribute.
    Source,
    /// Source: "mo_ta_anh" (image description) — maps to the HTML "alt"
    /// attribute.
    Alt,
    /// Source: "tai_cham" (lazy load) — boolean, maps to the
    /// loading="lazy" attribute.
    LazyLoad,

    // ── Form / Input ─────────────────────────────────────────
    /// Source: "loai" (type) — maps to the HTML "type" attribute.
    Type,
    /// Source: "chu_tro" (placeholder) — maps to the HTML "placeholder"
    /// attribute.
    Placeholder,
    /// Source: "gia_tri" (value) — maps to the HTML "value" attribute.
    Value,
    /// Source: "bat_buoc" (required) — boolean, maps to the "required"
    /// attribute.
    Required,
    /// Source: "vo_hieu" (disabled) — boolean, maps to the "disabled"
    /// attribute.
    Disabled,
    /// Source: "lop" (class) — toggles CSS classes based on the
    /// truthiness of an expression.
    ClassBinding,

    // ── Link ─────────────────────────────────────────────────
    /// Source: "den" (to) — the navigation target; generates BOTH the
    /// "href" attribute AND "data-vb-link" (a flag for SPA router
    /// interception) — see PropSpec/registry for the exact limitation
    /// (static targets only).
    To,

    // ── Content ──────────────────────────────────────────────
    /// Source: "noi_dung" (content) — CONFIRMED NOT A BUG (BUG-17, see
    /// AUDIT.md): handled separately on purpose, generates data-vb-text,
    /// kept out of the normal CSS style flow.
    Content,

    // ── Animation (handled separately by animation.rs, READS DIRECTLY
    // from AnimationProps on Element — does NOT actually go through the
    // shared PropsMap/expand_props — but is still a valid prop KEY the
    // dev can type in the source PropsMap, so it still needs an identity
    // here so the lexer/parser/validator recognize it correctly instead
    // of treating it as an "unknown prop") ──
    /// Source: "hieu_ung" (effect).
    Animation,
    /// Source: "thoi_gian" (duration).
    Duration,
    /// Source: "tre" (delay, animation delay).
    Delay,
    /// Source: "lap" (repeat, animation repeat/loop count).
    Repeat,
    /// Source: "hieu_ung_hover" (hover effect).
    HoverAnimation,
    /// Source: "hieu_ung_cuon" (scroll effect).
    ScrollAnimation,
}
