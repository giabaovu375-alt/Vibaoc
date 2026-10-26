# Known limitations (0.1.5)

ViBao 0.1.5 is meant to be **usable for basic web apps and honest about
what isn't finished yet**. This page lists everything currently known to
be incomplete, broken, or intentionally out of scope, so you don't spend
time filing an issue for something already tracked here — and so you don't
get surprised by a silent no-op. If you hit something not listed here,
please [open an issue](https://github.com/giabaovu375-alt/Vibaoc/issues).

## Silent no-ops (compile fine, do nothing) — read this first

These don't error or warn, so they're the easiest things to get bitten by:

- **Unknown action option keys are ignored.** Only `kieu`/`type` and
  `thoi_gian`/`duration` on `thong_bao`/`notify` are read. Any other key
  (a typo such as `thoi_gain:`, or an option for an action that takes
  none) compiles with no error or warning and has no effect.
- **`huong` / `gap` are valid syntax but have no effect on
  `box` (`khoi`).** These layout props only produce CSS on `flex`,
  `grid`, and `scroll` — `khoi` only supports background/border/radius/
  padding/margin/size, not flex-direction/gap (see the "Layout-tag-only
  props" table above). Using one of these on `khoi` now gives a build
  warning naming exactly which tags do support it, rather than a
  generic "possible typo" message. Wrap the flex-direction content in a
  `flex` tag instead (nest a `khoi` inside a `flex`, or vice versa, if
  you need both a border AND a direction/gap on the same visual box —
  see `examples/portfolio.vbao` for this pattern).
- **`huong`/`gap` on a *Simple* tag (e.g. `nav`/`thanh_dieu_huong`,
  `text`) automatically sets `display: flex` alongside `flex-direction`/
  `gap`** — without this, the two props would be valid CSS properties
  set on an element that's still `display: block` by default, so they
  would have no visible effect. This only applies when the value is a
  static literal; a dynamic (`$variable`) value for `huong`/`gap` does
  not currently add `display: flex` automatically.
- **`mo_modal` / `dong_modal` have no valid target in 0.1.5.** They look
  for an HTML element with `id="vb-modal-<id>"`. `modal` itself is
  still an empty placeholder (see below), so even with a custom `id:`
  set on it, there's no real modal markup to show/hide yet. Both
  actions compile and run without error; they just have no visible
  effect yet.
- **A dynamic `den` (`den: $url`) is never an in-app route.** It only
  binds `href`, so the browser follows it: right for external URLs,
  `mailto:`, `tel:`, `#anchors` and files, but an in-app route would be a
  full page load. For a state-dependent in-app route use a `button` with
  `dieu_huong($variable)` in `on_click`. A static `den` is handled by the
  SPA router only when it starts with `/` (a route) or `#` (an anchor);
  `https://...`, `mailto:...` and relative files such as `cv.pdf` are left
  to the browser.

## Known gaps in 0.1.5

- **`[n]` index** only accepts an integer literal (`$list[0]`), not an
  expression.
- **`goi_api(..., chay_nen: true)`** only accepts the literal `true`, and a
  background call is not cancelled when the user leaves the page. Without
  it, a `goi_api` inside `on_tai` delays event binding for the whole page
  until the network answers.
- **Themes (`chu_de`):** after the user picks a theme by hand there is no
  way back to "follow the operating system". Theme variables cannot be used
  inside responsive blocks (`@di_dong`, ...), in expressions, or as content.
  `dat_chu_de("toi")` takes the theme name exactly as declared once accents
  are stripped.
- **API keys written in a `.vbao` file** end up in `index.html`/`app.js` and are
  visible to anyone who opens the page. Use them for local testing only.
- **Static hosting (`--site-url`):** every route shares one `index.html`, so
  the title and Open Graph tags are the same for the whole site. Hosting under
  a sub-path such as `user.github.io/repo/` has not been verified; the router
  works on real paths (`history.pushState`).
- **`@khoang_cach` also holds border, shadow, radius and overflow props**, not
  only padding and margin. A split is planned.

## Placeholder tags

These tags are recognized by the compiler but currently render as an empty
`<div>` — no real markup or runtime behavior is generated for them yet:

`modal`, `tabs`, `accordion`, `carousel`, `pagination`, `table`, `chart`,
`map`, `editor` (each has dedicated AST support and is treated as
a built-in complex component, but the compiler only emits a placeholder
container; the actual UI is left for you to build).

`spinner`, `progress`, and `input_group` are parsed correctly but have no
dedicated HTML mapping yet, so they also fall back to a plain `<div>`.

`radio`/`chon_mot`, `checkbox`/`hop_kiem`, `select`/`lua_chon`,
`nav`/`thanh_dieu_huong`, and `form` DO generate real semantic HTML
(`<input type="radio">`, `<input type="checkbox">`, `<select>`, `<nav>`,
`<form>`). `form`'s children (inputs, textarea, button...) render
normally, and `on_submit`/`khi_gui` fires like any other event - the
runtime automatically calls `preventDefault()` on the submit event so
the browser's default full-page reload doesn't happen.
For `select`, each direct child element becomes an `<option>` — its own
content (e.g. `text("Táo")`) becomes the visible label, and its
`gia_tri`/`value` prop becomes the option's value (defaults to the label
if omitted). A **dynamic** (state-bound) option list or option value is
not supported yet — only a static, literal list of options.

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
  `input[type=checkbox]`/`input[type=radio]` are bound through the
  `checked` property (a boolean), separately from the text-value path
  used for every other input type — double-check this branch if you add
  a new input type to `bind_model`.
- **`textarea`'s `noi_dung`/content prop should only be a static string**,
  not a `$variable`. A dynamic `noi_dung` on `textarea` inserts a
  `<span data-vb-text>` as a child, which `<textarea>` doesn't support
  (its value is its own text content, not child elements) — the bound
  text won't display correctly. Use `gia_tri`/`value` with a `$variable`
  instead (two-way binding), which works correctly.
- **`goi_api` / `api_call`** is a real `fetch`-based implementation and has
  been used against live endpoints (a chat completion API and the GitHub
  REST API), but only for JSON `GET`/`POST` with the options listed under
  "Known gaps in 0.1.5" below.
- **`vong_lap` (loop) re-renders** can accumulate stale subscribers over
  time in long-lived apps with frequent list updates. This is a known
  performance limitation, tracked for a later release.

## Color names

Only 14 fixed color names resolve to a hex value: `trang`, `den`, `do`,
`xanh`, `xanh_la`, `vang`, `hong`, `tim`, `cam`, `xam`, `xam_nhat`,
`xam_dam`, `luc`, `nau`. Any other identifier used where a color is
expected (e.g. a typo like `xanh_nhat`) is caught by the validator at
build time as a hard error — it is not silently accepted or printed
as-is into CSS. Double-check your color name is in this list.

## Where this list comes from

Everything above reflects the actual behavior of the compiler and runtime
as read from source at release time — not aspirational design. For the
full technical reference (which tag maps to which HTML element, the full
prop table, etc.), see [`VIBAO_SPEC.md`](VIBAO_SPEC.md).
