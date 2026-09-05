# Known limitations (0.1.0)

ViBao 0.1.0 is meant to be **usable for basic web apps and honest about
what isn't finished yet**. This page lists everything currently known to
be incomplete, broken, or intentionally out of scope, so you don't spend
time filing an issue for something already tracked here — and so you don't
get surprised by a silent no-op. If you hit something not listed here,
please [open an issue](https://github.com/giabaovu375-alt/Vibaoc/issues).

## Silent no-ops (compile fine, do nothing) — read this first

These don't error or warn, so they're the easiest things to get bitten by:

- **Action option keys are Vietnamese-only, even in English files.**
  `notify(text, kieu: canh_bao, thoi_gian: 5000)` works. The English-looking
  `notify(text, type: canh_bao, duration: 5000)` compiles with no error, but
  `type`/`duration` are silently ignored — the toast falls back to its
  default type and duration. Always write `kieu:` / `thoi_gian:` here.
  The same applies to `goi_api`/`api_call` response callbacks: only the
  literal block names `thanh_cong` / `that_bai` are recognized, not
  `success` / `failure`.
- **`huong` / `gap` / `khoang_chu` have no effect on `box` (`khoi`).**
  These layout props only produce CSS on `flex`, `grid`, and `scroll`. Use
  `flex` if you need to control layout direction or spacing.
- **`mo_modal` / `dong_modal` have no valid target in 0.1.0.** They look
  for an HTML element with `id="vb-modal-<id>"`, but there is currently no
  prop to set a custom `id` on any element, and the one tag that would
  naturally have that id (`modal`) is still an empty placeholder (see
  below). Both actions compile and run without error; they just have no
  visible effect yet.
- **`den` (a link's destination) only accepts a static string literal.**
  If the destination depends on state, use a `button` with
  `dieu_huong($variable)` in `on_click` instead.

## Placeholder tags

These tags are recognized by the compiler but currently render as an empty
`<div>` — no real markup or runtime behavior is generated for them yet:

`modal`, `tabs`, `accordion`, `carousel`, `pagination`, `table`, `chart`,
`map`, `form`, `editor` (each has dedicated AST support and is treated as
a built-in complex component, but the compiler only emits a placeholder
container; the actual UI is left for you to build).

`spinner`, `progress`, `input_group`, `radio`, `checkbox`, `select`, and
`nav` are parsed correctly but have no dedicated HTML mapping yet, so
they also fall back to a plain `<div>` rather than the semantic element
you'd expect (`<select>`, `<input type="radio">`, `<nav>`, etc).

If you need one of these today, build the equivalent with a custom
`@the` component (see [SYNTAX_EN.md](SYNTAX_EN.md#components)).

## Vietnamese-only "soft keywords"

Most of ViBao's syntax has both a Vietnamese and English surface form, but
three connector words are still hard-coded and have no English equivalent
yet:

- `trong` (the "in" of a collection loop: `loop $item trong $list { ... }`)
- `tu` / `den` (the "from"/"to" of a numeric range: `loop $i tu 1 den 5`)

Responsive breakpoint names (`@di_dong`, `@may_tinh_bang`, `@may_tinh`) are
similarly Vietnamese-only for now — there's no `@mobile` / `@tablet` /
`@desktop` form yet.

## Disabled or unfinished features

- **`sao_chep` / `copy_to_clipboard` is disabled.** It needs a special
  build flag (`--cfg=web_sys_unstable_apis`) that isn't wired up yet, to
  avoid risking a broken build for everyone else.
- **Auth / route guards** have no syntax in the language yet — there's no
  session or token model in the runtime.
- **Dynamic routes** (`/product/:id`-style params) are matched by the
  router, but nested/complex route trees aren't a finished contract yet.

## Works, but needs more real-world testing

- **`@the` components** work for the common case (static prop literals,
  one level of nesting) but haven't been stress-tested with deeply nested
  components. Component props are **not fully reactive**: if a prop's
  value is a variable that changes after the component mounts, the prop
  won't automatically re-render (the value read at mount time is
  correct, it just won't update live). Static literal props — the only
  style used in the current examples — are unaffected by this.
- **A `@the` component called directly inside a `vong_lap`, with no
  `neu`/element wrapping it in between** (e.g.
  `vong_lap $bv trong $ds { TheBaiViet(tieu_de: $bv.tieu_de, ...) }`) is
  the most fragile shape a component call can take. The compiler warns
  about this pattern at build time and suggests wrapping the call in a
  container instead. A related issue in this exact shape — component
  instances inside a loop that is itself inside a `truong_hop`/switch
  branch receiving the wrong props — is still open and tracked for
  0.1.1 rather than blocking this release. Wrapping the component call
  in a `khoi`/element, as the build-time warning suggests, avoids the
  whole class of issue.
- **`model` / two-way binding** on `input`/`textarea`/`select` has runtime
  code and unit tests, but hasn't had a real-browser integration pass yet.
- **`goi_api` / `api_call`** has a real `fetch`-based runtime implementation,
  but hasn't been exercised against a live API endpoint yet — only the URL
  resolution logic has direct tests.
- **`vong_lap` (loop) re-renders** can accumulate stale subscribers over
  time in long-lived apps with frequent list updates. This is a known
  performance limitation, tracked for after 0.1.0.

## Color names

Only 14 fixed color names resolve to a hex value: `trang`, `den`, `do`,
`xanh`, `xanh_la`, `vang`, `hong`, `tim`, `cam`, `xam`, `xam_nhat`,
`xam_dam`, `luc`, `nau`. Any other identifier used where a color is
expected (e.g. a typo like `xanh_nhat`) is treated as a plain string and
printed as-is into CSS — which CSS won't understand. Double-check your
color name is in this list.

## Where this list comes from

Everything above reflects the actual behavior of the compiler and runtime
as read from source at release time — not aspirational design. For the
full technical reference (which tag maps to which HTML element, the full
prop table, etc.), see [`VIBAO_SPEC.md`](VIBAO_SPEC.md).
