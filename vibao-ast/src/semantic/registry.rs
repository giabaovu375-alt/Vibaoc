// ============================================================
// VIBAO — vibao-ast/src/semantic/registry.rs
// Metadata for semantic identities (Tag, and later PropKey/ActionName/
// FunctionName) — does NOT redefine the enums here (that's tag.rs/
// prop.rs/action.rs/function.rs's job), ONLY looks up information ABOUT
// an identity that already exists.
//
// PRINCIPLE: an identity (enum) exists independently of any display
// language — "Tag::Khoi" IS ITSELF, not "the translation of khoi". This
// registry answers questions ABOUT a Tag that the compiler needs to
// generate correct code (which HTML tag, which processing group) — it
// does NOT answer "what is its Vietnamese name" (that's the LEXER's
// job; the reverse locale->Tag mapping lives in per-locale tables, NOT
// here — this registry is one-directional: Tag -> metadata, not
// name -> Tag).
// ============================================================

use super::tag::Tag;

/// The codegen processing group for a Tag — determines which function
/// in vibaoc::codegen handles it (gen_simple_element/resolve_layout_css/
/// gen_complex_component). Matches 1:1 with 3 SEPARATE tables that
/// existed before (LAYOUT_TAGS in layout.rs, BUILTIN_COMPLEX in
/// element.rs, and "every other tag" implicitly treated as Simple) —
/// now it's a PROPERTY of Tag itself, looked up through the registry,
/// instead of 2 separate string tables that could drift apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TagKind {
    /// A simple element — a single HTML tag, props map directly to
    /// style/attributes. Handled by gen_simple_element()/expand_props().
    Simple,
    /// A layout container (flex/grid/khoi/stack/...) — props map to a
    /// separate CSS-layout prop set, different from Simple's. Handled by
    /// resolve_layout_css() (layout.rs).
    Layout,
    /// Complex UI (modal/tabs/bang_chuyen/...) — the HTML is only a
    /// placeholder; the real behavior is built by the WASM runtime at
    /// mount time. Handled by gen_complex_component().
    Complex,
}

/// Full metadata for a Tag — everything the compiler needs to know to
/// generate correct code, NOT including any locale display name (see
/// the doc-comment at the top of the file).
#[derive(Debug, Clone, Copy)]
pub struct TagSpec {
    pub tag: Tag,
    pub kind: TagKind,
    /// The corresponding HTML tag when kind == Simple or Layout
    /// (a Complex tag always uses "div" as its placeholder, see
    /// gen_complex_component()) — irrelevant for Complex since the root
    /// tag doesn't matter; the runtime fully overrides the content.
    pub html_tag: &'static str,
}

/// The Tag -> TagSpec lookup table — the SINGLE SOURCE OF TRUTH,
/// unifying 3 places that previously defined this SEPARATELY in vibaoc
/// (LAYOUT_TAGS, BUILTIN_COMPLEX, tag_to_html()'s match arms) — those 3
/// tables previously had to be kept in sync by hand, with nothing
/// enforcing it (e.g. adding a new layout tag but forgetting to add it
/// to LAYOUT_TAGS would silently treat it as Simple).
pub fn tag_spec(tag: Tag) -> TagSpec {
    use TagKind::*;
    let (kind, html_tag) = match tag {
        // ── Simple: Text ─────────────────────────────────────
        Tag::Text => (Simple, "p"),
        Tag::H1 => (Simple, "h1"),
        Tag::H2 => (Simple, "h2"),
        Tag::H3 => (Simple, "h3"),
        Tag::P => (Simple, "p"),
        Tag::Nhan => (Simple, "span"),
        // ── Simple: Media ────────────────────────────────────
        Tag::Image => (Simple, "img"),
        Tag::Video => (Simple, "video"),
        Tag::Icon => (Simple, "span"),
        // ── Simple: Interactive ──────────────────────────────
        Tag::Button => (Simple, "button"),
        Tag::Input => (Simple, "input"),
        Tag::Link => (Simple, "a"),
        // ── Layout ───────────────────────────────────────────
        Tag::Flex => (Layout, "div"),
        Tag::Grid => (Layout, "div"),
        Tag::Stack => (Layout, "div"),
        Tag::Khoi => (Layout, "div"),
        Tag::Cuon => (Layout, "div"),
        Tag::CanGiua => (Layout, "div"),
        Tag::Lop => (Layout, "div"),
        Tag::DinhDau => (Layout, "div"),
        Tag::DinhManHinh => (Layout, "div"),
        // ── Simple: Spacing (NOT Layout — see the note in
        // tag.rs) ────────────────────────────────────────────
        Tag::KhoangCach => (Simple, "div"),
        Tag::DuongKe => (Simple, "hr"),
        // ── Simple: Form (NOT Complex, unlike "Form" itself)
        // ────────────────────────────────────────────────
        Tag::NhomInput => (Simple, "div"),
        Tag::ChonMot => (Simple, "div"),
        Tag::HopKiem => (Simple, "div"),
        // KEEP "div" (matches the old tag_to_html() table in
        // element.rs — "lua_chon" had NO explicit branch and fell into
        // the "_ => div" fallback). A bug SELF-CAUGHT by the
        // cross-check test
        // (test_tag_to_html_semantic_matches_string_based_for_every_known_tag):
        // this was initially written as "select" because it "seemed
        // more sensible" semantically (lua_chon = "make a selection",
        // like an HTML <select>) — but that would have been exactly the
        // kind of UNILATERAL BEHAVIOR CHANGE during an identity
        // migration that the guiding principle warns against. If
        // "lua_chon" should later actually generate a real <select>,
        // that's a LANGUAGE DECISION requiring separate confirmation
        // (and could also affect how props are handled for this tag) —
        // not something a pure identity migration should do on its
        // own.
        Tag::LuaChon => (Simple, "div"),
        // ── Complex ──────────────────────────────────────────
        Tag::Form => (Complex, "div"),
        Tag::Modal => (Complex, "div"),
        Tag::Tabs => (Complex, "div"),
        Tag::GapMo => (Complex, "div"),
        Tag::BangChuyen => (Complex, "div"),
        Tag::XuongTrang => (Complex, "div"),
        Tag::Bang => (Complex, "div"),
        Tag::BieuDo => (Complex, "div"),
        Tag::BanDo => (Complex, "div"),
        Tag::TrinhSoanThao => (Complex, "div"),
        // ── Simple: was grouped under the old "Complex UI" comment
        // in component_set() BUT is ACTUALLY handled as Simple (not
        // present in the old 10-element BUILTIN_COMPLEX) — KEEPS the old
        // behavior, does NOT unilaterally "fix it to be more semantically
        // correct". If ThanhDieuHuong/VongQuay/ThanhTienTrinh should
        // later move to Complex, that's a LANGUAGE DECISION requiring
        // separate confirmation, not something this identity migration
        // step should do.
        Tag::ThanhDieuHuong => (Simple, "div"),
        // ── Simple: labeled "Complex UI" in the original comment BUT
        // actually handled as Simple (not part of the old
        // BUILTIN_COMPLEX) ────
        Tag::VongQuay => (Simple, "div"),
        Tag::ThanhTienTrinh => (Simple, "div"),
    };
    TagSpec { tag, kind, html_tag }
}

// ════════════════════════════════════════════════════════════
// PropSpec — LIGHTWEIGHT metadata for PropKey (see prop.rs for the enum
// identity)
// ════════════════════════════════════════════════════════════
//
// ARCHITECTURE (settled with the user, see ARCHITECTURE_PROPOSAL.md):
//
//   PropKey (prop.rs)        = pure identity
//         ↓
//   PropSpec (here)          = MINIMAL metadata (only "which TagKind
//                               does this prop apply to" — carries NO
//                               CSS property name, NO value-mapping
//                               logic, NO is_dynamic logic)
//         ↓
//   props.rs / layout.rs     = the real behavior + mapping + CSS logic
//                               (NOT changed in this round — the ~57
//                               existing match arms stay as they are,
//                               only the match KEY changes from String
//                               to PropKey)
//
// Why value-mapping logic is NOT folded in here (even though it
// theoretically could be): the same PropKey (e.g. `Align`) maps to a
// DIFFERENT CSS property depending on tag context (textAlign for
// text-bearing tags, justifyContent for flex, justifyItems for stack)
// — cramming all of that logic into one static struct would turn
// PropSpec into a nested match table even more complex than the
// current props.rs/layout.rs, simplifying nothing and just moving the
// problem elsewhere. This keeps the same boundary already established
// for Tag/TagSpec: the registry only answers a CLASSIFICATION question
// ("where is this valid"), not a BEHAVIOR question ("what does it map
// to").
//
// The specific problem this (lightweight) PropSpec solves: BUG-25
// (AUDIT.md) — a typo'd prop name on a layout element was never
// flagged, because KNOWN_PROP_KEYS (props.rs) only listed props valid
// for Simple and had no idea what props layout used. With PropSpec,
// `element.rs`/`layout.rs` (once migrated to PropKey in the next code
// round) can directly ask "does PropKey::X apply_to Layout" instead of
// needing a 3rd separate String table for layout.
//
// FIX (after a bug review from the user): the first version only had 2
// fields (`applies_to_simple`/`applies_to_layout`) and TEMPORARILY
// FILED `Hidden` ("an" — only valid inside a responsive block) under
// `applies_to_layout = true` as "the closest fit of the two available
// options". This is EXACTLY the kind of false-negative PropSpec exists
// to PREVENT (its core purpose is catching props used in the wrong
// context), so quietly forcing a case that didn't cleanly fit into the
// 2 existing fields directly contradicts this struct's own purpose. A
// 3rd field, `responsive_only`, was added to CORRECTLY represent
// `Hidden`'s context instead of a rough approximation.

use super::prop::PropKey;

/// Which Tag group(s) this prop applies to — uses
/// `applies_to_simple`/`applies_to_layout` (2 separate bool fields
/// instead of 1 `Vec<TagKind>`/bitflags) because, among the current
/// props, NONE of them apply to `TagKind::Complex` (Complex UI manages
/// its own props through the runtime, not through
/// expand_props()/resolve_layout_css()) — 2 simple bool fields are more
/// readable than a set type for a case where only 2 possibilities are
/// actually ever used for "which Tag group is this valid in". The
/// separate question of "which block of that element is it valid in"
/// (responsive or not) is a DIFFERENT DIMENSION of information, split
/// out into the `responsive_only` field below — not folded together,
/// to avoid repeating the exact mistake that was just fixed.
#[derive(Debug, Clone, Copy)]
pub struct PropSpec {
    pub key: PropKey,
    /// True if this prop is recognized by `expand_props()` (props.rs,
    /// Simple Element) — matches the old `KNOWN_PROP_KEYS`.
    pub applies_to_simple: bool,
    /// True if this prop is recognized by AT LEAST ONE of the 9
    /// `resolve_*` functions (layout.rs, Layout Element) — matches the
    /// full cross-reference table built in ARCHITECTURE_PROPOSAL.md,
    /// section 3.3 (there was NO equivalent String constant table
    /// before — this IS the FIRST source of truth for this
    /// information).
    ///
    /// IMPORTANT NOTE: this field ALONE is not enough to know "this
    /// prop is valid on every layout element" — SOME props (currently:
    /// only `Hidden`) are only valid INSIDE a responsive block
    /// (`@di_dong`/`@may_tinh_bang`/`@may_tinh`), and are NOT valid on
    /// a normal layout element outside that block — see the
    /// `responsive_only` field below. Any code that uses
    /// `applies_to_layout` to flag an "unknown prop" warning MUST also
    /// check `responsive_only` to get the context right, or it will
    /// produce a false negative (a prop that should have been flagged
    /// outside a responsive block gets treated as valid).
    pub applies_to_layout: bool,
    /// True if this prop is ONLY valid when it appears INSIDE a
    /// responsive block (`@di_dong`/`@may_tinh_bang`/`@may_tinh`,
    /// `resolve_responsive_css()` in layout.rs), and is NOT valid
    /// standalone on a normal layout or simple element.
    ///
    /// Currently (verified: `grep "an"` across the entirety of
    /// `layout.rs`/`props.rs` shows EXACTLY 1 match, inside
    /// `resolve_responsive_css`) ONLY `PropKey::Hidden` (source "an")
    /// belongs to this group — EVERY other prop, even those with
    /// special-cased handling inside a responsive block (e.g. `co`/
    /// `rong`/`cao`/`dem`/`huong` — different CSS property name/value
    /// format), is STILL valid on a normal layout/simple element
    /// OUTSIDE a responsive block, so they do NOT belong to
    /// `responsive_only` — they're already correctly classified via
    /// `applies_to_simple`/`applies_to_layout`.
    ///
    /// This field does NOT exclude `applies_to_simple`/
    /// `applies_to_layout` — a prop with `responsive_only = true` can
    /// still have `applies_to_layout = true` (meaning "valid on a
    /// layout element, BUT only when inside that element's responsive
    /// block") — callers need to read BOTH fields together depending on
    /// what context is being validated (a responsive block vs. a normal
    /// layout element), not read just one field and conclude from that
    /// alone.
    pub responsive_only: bool,
}

/// The PropKey -> PropSpec lookup table — the SINGLE SOURCE OF TRUTH
/// for "which prop is valid where", unifying 2 places that previously
/// had NOTHING enforcing they stayed in sync (`KNOWN_PROP_KEYS` in
/// props.rs only knew about Simple; layout.rs had no table at all, only
/// implicitly determined by whether a match arm was PRESENT in one of
/// the 9 `resolve_*` functions).
pub fn prop_spec(key: PropKey) -> PropSpec {
    use PropKey::*;
    let (applies_to_simple, applies_to_layout) = match key {
        // ── Both Simple AND Layout (17 props — see the full
        // cross-reference in ARCHITECTURE_PROPOSAL.md, section 3.3) ─────────────────────
        BackgroundColor | Width | Height | MaxWidth | Radius | Padding
        | Margin | Border | Shadow | Overflow | ZIndex | FontSize
        | Direction | Gap | AlignItems | Wrap | Align => (true, true),

        // ── Simple only (28 props) ─────────────────────────────────
        Color | BorderColor | BorderStyle | Bold | Italic | Underline
        | LineHeight | LetterSpacing | TextTransform | FontFamily
        | Fit | Source | Alt | LazyLoad | Type | Placeholder | Value
        | Required | Disabled | ClassBinding | To | Content | Animation | Duration
        | Delay | Repeat | HoverAnimation | ScrollAnimation => (true, false),

        // ── Layout only (12 props — the exact group that caused
        // BUG-25; previously NOT part of KNOWN_PROP_KEYS, so typos went
        // unflagged) ─────────────────────────────────────────────────
        MinWidth | MaxHeight | MinHeight | RowGap | ColumnGap | Columns
        | Rows | TranslateX | TranslateY | Position | Offset
        // Hidden ("an"): only valid INSIDE a responsive block — still
        // set applies_to_layout=true (correctly meaning "valid on a
        // layout element WHEN inside that element's responsive block"),
        // BUT now has its own `responsive_only=true` to explicitly
        // distinguish it from the other 12 layout-only props (which are
        // valid on a layout element AT ALL TIMES, with no "must be
        // inside a responsive block" condition) — see the
        // `responsive_only` doc-comment above for the full reasoning
        // (fixed based on a review comment: the old version folding
        // everything into applies_to_layout produced a false negative
        // when used to flag unknown props in layout.rs — see
        // AUDIT.md).
        | Hidden => (false, true),
    };
    let responsive_only = matches!(key, Hidden);
    PropSpec { key, applies_to_simple, applies_to_layout, responsive_only }
}

#[cfg(test)]
mod prop_tests {
    use super::*;

    #[test]
    fn test_prop_spec_covers_every_variant() {
        // Rust match exhaustiveness guarantees at compile time that
        // every PropKey variant has a branch — this test just confirms
        // a few important cases.
        assert!(prop_spec(PropKey::BackgroundColor).applies_to_simple);
        assert!(prop_spec(PropKey::BackgroundColor).applies_to_layout);
        assert!(prop_spec(PropKey::Color).applies_to_simple);
        assert!(!prop_spec(PropKey::Color).applies_to_layout);
        assert!(!prop_spec(PropKey::MinWidth).applies_to_simple);
        assert!(prop_spec(PropKey::MinWidth).applies_to_layout);
    }

    #[test]
    fn test_hidden_is_responsive_only_and_only_hidden_is() {
        // Bug already fixed (from a user review comment): `Hidden`
        // ("an") is only valid INSIDE a responsive block — if this
        // field were ever missed or set wrong, any code using
        // `prop_spec()` to flag "unknown prop" in layout.rs would get a
        // false negative (no warning when someone types "an" directly
        // on a layout element outside a responsive block).
        assert!(
            prop_spec(PropKey::Hidden).responsive_only,
            "Hidden (\"an\") MUST have responsive_only = true"
        );
        // Confirms Hidden is the ONLY such case — if a future
        // responsive-only prop is added and this field is forgotten,
        // the count test below will catch the mismatch immediately.
        let responsive_only_count = ALL_PROP_KEYS_FOR_TEST
            .iter()
            .filter(|k| prop_spec(**k).responsive_only)
            .count();
        assert_eq!(responsive_only_count, 1, "currently exactly 1 prop should be responsive_only (Hidden)");
    }

    #[test]
    fn test_responsive_only_props_still_have_applies_to_layout_true() {
        // responsive_only does NOT exclude applies_to_layout — see the
        // `responsive_only` doc-comment on PropSpec: a prop can have
        // BOTH applies_to_layout=true (valid on a layout element) AND
        // responsive_only=true (but ONLY when inside that element's
        // responsive block) at the same time. This test confirms that
        // invariant holds for Hidden — if a future responsive_only prop
        // accidentally has applies_to_layout=false, that's a sign of
        // broken logic (a responsive-only prop should always be "valid
        // on some element type, just with an extra condition", never
        // both "not valid anywhere" and "only valid in responsive" at
        // once).
        assert!(prop_spec(PropKey::Hidden).applies_to_layout);
    }

    #[test]
    fn test_no_prop_applies_to_neither() {
        // Every prop in the list MUST apply to AT LEAST ONE of the 2
        // groups — if a PropKey ends up as (false, false), that's a
        // sign it doesn't belong in PropKey at all (it might belong to
        // ActionName/FunctionName or a separate Tag instead of a Prop).
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
        // Matches the count computed by hand while building the
        // cross-reference table (see ARCHITECTURE_PROPOSAL.md, section
        // 3.3): 17 BOTH props + 28 SIMPLE-only props = 45 props with
        // applies_to_simple = true. This is the count of semantic
        // identities applying to Simple; surface aliases don't increase
        // the PropKey count.
        let count = ALL_PROP_KEYS_FOR_TEST
            .iter()
            .filter(|k| prop_spec(**k).applies_to_simple)
            .count();
        assert_eq!(count, 45, "must match 17 (both) + 28 (simple-only) = 45");
    }

    #[test]
    fn test_layout_group_matches_known_layout_prop_count() {
        // 17 BOTH props + 12 LAYOUT-only props = 29 props with
        // applies_to_layout = true.
        let count = ALL_PROP_KEYS_FOR_TEST
            .iter()
            .filter(|k| prop_spec(**k).applies_to_layout)
            .count();
        assert_eq!(count, 29, "must match 17 (both) + 12 (layout-only) = 29");
    }

    #[test]
    fn test_total_prop_key_count_is_57() {
        // Matches the current total number of semantic identities: 57
        // PropKeys. The Vietnamese source name count is 58 because the
        // "mau"/"mau_chu" alias both point to PropKey::Color.
        assert_eq!(ALL_PROP_KEYS_FOR_TEST.len(), 57);
    }

    /// The FULL list of 57 PropKeys — hand-copied once for this
    /// cross-check test, NOT imported directly from anywhere else
    /// (Rust has no automatic "list of every variant" outside the enum
    /// itself — if prop.rs adds or removes a variant and this TEST is
    /// forgotten, the count tests above will immediately catch the
    /// mismatch, fulfilling this list's role as a cross-check).
    const ALL_PROP_KEYS_FOR_TEST: [PropKey; 57] = [
        PropKey::BackgroundColor, PropKey::Color, PropKey::BorderColor,
        PropKey::Width, PropKey::Height, PropKey::MaxWidth, PropKey::MinWidth,
        PropKey::MaxHeight, PropKey::MinHeight,
        PropKey::Radius, PropKey::Padding, PropKey::Margin, PropKey::Border,
        PropKey::BorderStyle, PropKey::Shadow, PropKey::Overflow, PropKey::ZIndex,
        PropKey::FontSize, PropKey::Bold, PropKey::Italic, PropKey::Underline,
        PropKey::Align, PropKey::LineHeight, PropKey::LetterSpacing,
        PropKey::TextTransform, PropKey::FontFamily,
        PropKey::Direction, PropKey::Gap, PropKey::RowGap, PropKey::ColumnGap,
        PropKey::AlignItems, PropKey::Wrap, PropKey::Columns, PropKey::Rows,
        PropKey::TranslateX, PropKey::TranslateY, PropKey::Position, PropKey::Offset,
        PropKey::Hidden,
        PropKey::Fit, PropKey::Source, PropKey::Alt, PropKey::LazyLoad,
        PropKey::Type, PropKey::Placeholder, PropKey::Value, PropKey::Required,
        PropKey::Disabled,
        PropKey::ClassBinding,
        PropKey::To,
        PropKey::Content,
        PropKey::Animation, PropKey::Duration, PropKey::Delay, PropKey::Repeat,
        PropKey::HoverAnimation, PropKey::ScrollAnimation,
    ];
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tag_spec_covers_every_variant() {
        // Rust match exhaustiveness already guarantees at compile time
        // that every Tag variant has a branch in tag_spec() — this test
        // only confirms correct runtime behavior for a few important
        // cases; no need to re-verify that the match has every branch
        // (the compiler already handles that).
        assert_eq!(tag_spec(Tag::Khoi).kind, TagKind::Layout);
        assert_eq!(tag_spec(Tag::Text).kind, TagKind::Simple);
        assert_eq!(tag_spec(Tag::Modal).kind, TagKind::Complex);
    }

    #[test]
    fn test_layout_group_has_exactly_9_members() {
        // Matches the count of the old LAYOUT_TAGS (9) — a regression
        // test in case someone accidentally changes a layout tag's
        // kind.
        let count = [
            Tag::Flex, Tag::Grid, Tag::Stack, Tag::Khoi, Tag::Cuon,
            Tag::CanGiua, Tag::Lop, Tag::DinhDau, Tag::DinhManHinh,
        ]
        .iter()
        .filter(|t| tag_spec(**t).kind == TagKind::Layout)
        .count();
        assert_eq!(count, 9);
    }

    #[test]
    fn test_complex_group_matches_old_builtin_complex_exactly() {
        // Matches the old BUILTIN_COMPLEX EXACTLY (10 elements, see
        // vibaoc/src/codegen/element.rs) — this identity migration step
        // does NOT unilaterally add or remove any element (even where a
        // different grouping "seems more sensible", e.g.
        // thanh_dieu_huong — see the note in tag_spec()). Preserving
        // behavior is priority #1.
        let complex_tags = [
            Tag::Form, Tag::Modal, Tag::Tabs, Tag::GapMo, Tag::BangChuyen,
            Tag::XuongTrang, Tag::Bang, Tag::BieuDo, Tag::BanDo, Tag::TrinhSoanThao,
        ];
        assert_eq!(complex_tags.len(), 10, "must match the old BUILTIN_COMPLEX's 10 elements exactly");
        let count = complex_tags.iter().filter(|t| tag_spec(**t).kind == TagKind::Complex).count();
        assert_eq!(count, complex_tags.len());

        // ThanhDieuHuong MUST be Simple (preserving old behavior),
        // despite being grouped under the "Complex UI" comment in the
        // original component_set().
        assert_eq!(tag_spec(Tag::ThanhDieuHuong).kind, TagKind::Simple);
    }
}
