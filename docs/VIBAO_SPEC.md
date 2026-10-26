# ViBao — Language Specification (0.1.5)

This is the full technical reference for ViBao's syntax and compiled
output. For a quick cheat-sheet instead, see
[`SYNTAX_EN.md`](SYNTAX_EN.md) / [`SYNTAX_VI.md`](SYNTAX_VI.md). For
what's incomplete or behaves unexpectedly, see
[`LIMITATIONS.md`](LIMITATIONS.md) — this document describes what *is*
implemented.

## 1. Overview

ViBao is a small language that compiles to a static web app — similar in
spirit to a Svelte/Vue compiler, but with a Vietnamese-first (unaccented,
`snake_case`) surface syntax and an English surface syntax that resolves
to the exact same AST.

```
app.vbao → vibaoc build → HTML + CSS + JS + WASM → browser
```

- Source files use the `.vbao` extension.
- A build produces one real single-page app: every page is merged into a
  single `index.html`, navigation happens through a JS/WASM router (the
  History API), with no full page reloads. Every HTML `id` in the app is
  globally unique across all pages, not just within one page.
- The runtime — state, reactive expressions, DOM binding, actions,
  animation, `switch`, page lifecycle — is WebAssembly compiled from
  Rust. There is no `eval()` or `new Function()` anywhere in the
  generated JavaScript.

## 2. File language directive

English is the default diagnostic language. To get Vietnamese
error/warning messages, add this as the first line of a file:

```vbao
lang = "vi";
```

This only affects which language *diagnostics* are shown in — it does not
change which surface keywords are accepted. English surface syntax is
always accepted alongside the active locale, and vice versa; you can mix
both in the same file. Unsupported language codes are rejected at parse
time.

## 3. Program structure

```vbao
app("My app") {
    page("/") {
        // home page content
    }

    page("/about", "About", background_color: light_gray, description: "Learn more about me") {
        // another page, with a display name, background color, and SEO description
    }
}
```

- `app("Name")` — the required root block that wraps everything else.
- `page("/path", "Name", background_color: <color>, description: "...")` —
  declares one route. The second positional argument (display name),
  `background_color`, and `description`/`mo_ta` are all optional and can
  appear in any order after the route.
  - The display name becomes that page's `<title>` and
    `description`/`mo_ta` becomes its `<meta name="description">` -
    both update automatically on every SPA navigation to match the
    active page (this is a single-page app under the hood, so there's
    only one real `<title>`/`<meta>` tag, kept in sync with the current
    route at runtime). A page that omits one of these leaves the
    previous page's value in place rather than clearing it.
- Routes support dynamic segments with `:name` (e.g. `"/product/:id"`).
  The router matches these at navigation time and injects the matched
  value into state under that name (e.g. `$id`).

## 4. Tags

**Only the tags below actually generate meaningful HTML.** Any other tag
name is accepted by the lexer/parser but falls back to an empty `<div>`
at codegen time — see [`LIMITATIONS.md`](LIMITATIONS.md) for the current
placeholder list.

| ViBao tag | HTML output | Notes |
|---|---|---|
| `text` | `<p>` | |
| `h1`, `h2`, `h3` | `<h1>` / `<h2>` / `<h3>` | |
| `p` | `<p>` | |
| `label` | `<span>` | |
| `button` | `<button>` | |
| `link` | `<a>` | `den:` / `to:` generates `href` and SPA navigation on click (see §5). `muc_tieu:`/`target:` and `quan_he:`/`rel:` set the plain HTML `target`/`rel` attributes (e.g. `target: "_blank", rel: "noreferrer"` for an external link) |
| `image` | `<img>` | |
| `video` | `<video>` | |
| `icon` | `<span>` | |
| `input` | `<input>` | |
| `textarea` | `<textarea>` | multi-line text input; use `gia_tri`/`value` with a `$variable` for two-way binding, same as `input` |
| `radio` | `<input type="radio">` | default `type="radio"` unless overridden via `loai`/`type`. For a **radio group** (several radios sharing one choice), give every radio the same `gia_tri`/`value` (a `$variable` — this becomes the shared two-way binding key) plus its own `chon`/`option_value` (a fixed string — this radio's own value within the group). See the example below the table. |
| `checkbox` | `<input type="checkbox">` | default `type="checkbox"` unless overridden via `loai`/`type` |
| `select` | `<select>` | each child element becomes an `<option>` — its content is the label, `gia_tri`/`value` is the option value (defaults to the label); static options only, see [`LIMITATIONS.md`](LIMITATIONS.md) |
| `nav` | `<nav>` | |
| `form` | `<form>` | children render normally; `on_submit`/`khi_gui` fires on submit, and the browser's default full-page reload is automatically suppressed |
| `spacer` | `<div>` | |
| `divider` | `<hr>` | |
| `scroll` | `<div>` | also a layout tag, see below |
| `container` | `<div>` | also a layout tag, see below |

**Radio group example** — chọn chủ đề sáng/tối:

```vbao
state $chu_de = "sáng"

chon_mot {
    @nhap_lieu { gia_tri: $chu_de, chon: "sáng" }
}
text("Sáng")
chon_mot {
    @nhap_lieu { gia_tri: $chu_de, chon: "tối" }
}
text("Tối")
```

Both radios share `gia_tri: $chu_de` (the two-way binding key — only one
of them can be selected at a time), but each has its own `chon` value
identifying which choice it represents. Lưu ý: `chon`/`gia_tri` ở đây so
sánh đúng **nội dung chuỗi ký tự** (`"sáng"` khác `"sang"`), vì đây là
string literal, không phải identifier — cơ chế "chấp nhận cả có dấu lẫn
không dấu" chỉ áp dụng cho **tên** (tên biến, tên tag, tên prop...), không
áp dụng cho nội dung bên trong dấu ngoặc kép. Viết `chon: "sáng"` ở mọi
radio trong cùng nhóm và giữ nguyên chính tả đó xuyên suốt là đủ.

### Layout tags (full flexbox/grid support)

| Tag | CSS `display` |
|---|---|
| `flex` | `display: flex` |
| `grid` | `display: grid` |
| `box` | block element with padding/color/border support |
| `stack` | grid forced to `1fr` (stacks children on top of each other) |
| `scroll` | `box` with `overflow` |
| `container` | centered box with a `max-width` cap |
| `layer` | `position: relative` (an anchor for absolutely-positioned children) |
| `sticky` | `position: sticky; top: 0` |
| `fixed` | `position: fixed` |

## 5. Element syntax

A tag takes named props in one of two forms, depending on the tag.

**Block form** — props are grouped by meaning into `@block { ... }`
inside the tag's `{ ... }` body:

```vbao
box {
    @color { background_color: xam_nhat, border_color: xam }
    @spacing { padding: 16, border: 1, radius: 8 }
    @size { width: 320 }

    text("Hello")
}
```

**Simple form** — `tag("content", key: value, ...) { children }`, with
the props inside `(...)`:

```vbao
text("Hello", font_size: 16, bold: true)
```

- `box`, `flex`, `grid`, `input`, `textarea`, `radio`, `checkbox` and
  `select` (Vietnamese: `khoi`, `flex`, `grid`, `input`, `vung_van_ban`,
  `chon_mot`, `hop_kiem`, `lua_chon`) **require** the block form. Inside
  `(...)` they accept only a positional first value and the animation
  props (`hover_animation`, `scroll_animation`); any other named prop is
  a compile error that names the block to use.
- Every other tag accepts both forms. Both produce the same AST: a prop
  written in `(...)` is placed into its block automatically, and the two
  styles can be mixed on one tag.
- Block order is free. A block can be declared once per tag; the
  Vietnamese and English names of a block are the same block, so
  `@mau` followed by `@color` is a duplicate-block error.
- A prop must be in the block it belongs to. `gap`, `row_gap` and
  `column_gap` are accepted in both `@flex_layout` and `@grid_layout`.
- A prop name the compiler does not recognize is not a parse error; it
  is reported later as an "unrecognized prop" build warning.
- For text-bearing tags (`text`, `button`, `link`, ...), the first
  unnamed string argument is the content, not a prop:

  ```vbao
  text("Hello", font_size: 16)   // content = "Hello", font_size = 16
  ```

- Besides children and event blocks (`on_click { ... }`), the body of a
  tag can hold `@block` groups and `@di_dong`/`@may_tinh_bang`/
  `@may_tinh` responsive blocks. Inside a responsive block, props are
  written flat (`@di_dong { width: 320, hidden: true }`), without
  sub-blocks.

### Props by block

| Block (vi / en) | Props (Vietnamese / English) |
|---|---|
| `@mau` / `@color` | `mau_nen`/`background_color`, `mau`/`color` (also `mau_chu`), `mau_vien`/`border_color` |
| `@kich_thuoc` / `@size` | `rong`/`width`, `cao`/`height`, `max_rong`/`max_width`, `min_rong`/`min_width`, `max_cao`/`max_height`, `min_cao`/`min_height` |
| `@khoang_cach` / `@spacing` | `radius`, `dem`/`padding`, `le`/`margin`, `vien`/`border`, `kieu_vien`/`border_style`, `bong`/`shadow`, `cuon_tran`/`overflow`, `tang_z`/`z_index` |
| `@chu` / `@typography` | `co`/`font_size`, `dam`/`bold`, `nghieng`/`italic`, `gach_chan`/`underline`, `giu_dong`/`preserve_lines` (keeps line breaks and spaces, `white-space: pre-wrap`; static value only), `can`/`align`, `hang`/`line_height`, `khoang_chu`/`letter_spacing`, `bien_doi`/`text_transform`, `phong_chu`/`font_family` |
| `@flex_bo_cuc` / `@flex_layout` | `huong`/`direction`, `doc`/`align_items`, `boc`/`wrap`, `gap`, `gap_doc`/`row_gap`, `gap_ngang`/`column_gap` |
| `@grid_bo_cuc` / `@grid_layout` | `cot`/`columns`, `hang_luoi`/`rows`, `gap`, `gap_doc`/`row_gap`, `gap_ngang`/`column_gap` |
| `@vi_tri` / `@position` | `tran_x`/`translate_x`, `tran_y`/`translate_y`, `vi_tri`/`position`, `offset` |
| `@media` | `khop`/`fit`, `nguon`/`source`, `mo_ta_anh`/`alt`, `tai_cham`/`lazy_load` |
| `@nhap_lieu` / `@input` | `loai`/`type`, `chu_tro`/`placeholder`, `gia_tri`/`value`, `chon`/`option_value`, `bat_buoc`/`required`, `vo_hieu`/`disabled` |
| `@lien_ket` / `@link` | `den`/`to`, `muc_tieu`/`target`, `quan_he`/`rel` |
| `@dinh_danh` / `@identity` | `id`, `lop`/`class`, `the_html`/`html_tag` |

Notes:

- `can`/`align` (text alignment) lives in `@chu`; `doc`/`align_items`
  (cross-axis alignment) lives in `@flex_bo_cuc`.
- `den`/`to` on `link`: a static value starting with `/` is an in-app
  route and `#id` an in-page anchor (both handled by the SPA router);
  `https://...`, `mailto:`, `tel:` and relative files (`cv.pdf`) are
  followed by the browser. A dynamic `$variable` only binds `href` (never
  an in-app route); use `button` + `navigate()` for that. `muc_tieu`/
  `target` and `quan_he`/`rel` are plain HTML attributes (e.g.
  `"_blank"`, `"noreferrer"`).
- A `$variable` as `gia_tri`/`value` on `input`/`textarea` (or on
  `checkbox`/`radio`) activates two-way binding.
- `id` is the element's HTML `id`, useful as an anchor-link target.
- `the_html`/`html_tag` replaces the `<div>` of a container tag by one of
  `section`, `header`, `footer`, `main`, `article`, `aside`, `nav`,
  `figure`, `ul`, `li` (a static string). Elements only; on a tag that is
  not rendered as `<div>` it is ignored with a warning.

### Layout values

- `huong`/`direction` on `flex` (also inside `@di_dong`): `cot`, `doc` or
  `column` is a column; `hang`, `ngang` or `row` is a row. Any other
  value is a row.
- Inside `@di_dong`/`@may_tinh_bang`/`@may_tinh` the props handled are
  `cot`, `huong`, `co`, `rong`, `cao`, `dem`, `an`, `gap`, `max_rong`
  and `le`. Any other name is a build warning.

### Global style: `giao_dien` / `global_style`

An optional block at the app level, next to pages and components:

```vbao
giao_dien {
    phong_chu: "Inter, sans-serif"
    co: 16
    mau: xam_dam
    mau_nen: trang
    hang: 1.6
    tai_phong: "https://fonts.googleapis.com/css2?family=Inter"
    cuon_muot: true
    cuon_lui: 72
}
```

`phong_chu`/`font_family`, `co`/`font_size`, `mau`/`color`, `mau_nen`/
`background_color` and `hang`/`line_height` become `body` rules;
`cuon_muot`/`smooth_scroll` and `cuon_lui`/`scroll_offset` become `html`
rules (`scroll-behavior`, `scroll-padding-top`); `tai_phong`/`font_url`
adds a `<link rel="stylesheet">` to the HTML shell. Values must be static
(a state-dependent value is ignored with a warning), any other key is a
build error, and only one block may be declared, in the main file.

### Themes: `chu_de` / `theme`

```vbao
chu_de sang { $nen = trang, $chu = xam_dam }
chu_de toi  { $nen = "#1a202c", $chu = "#e2e8f0" }

giao_dien { mau_nen: $nen, mau: $chu }
```

- Every theme must declare the same set of `$variables` with constant values (color, number, or string).
  The first theme is the default; a theme named `toi` or `dark` follows `prefers-color-scheme: dark` until the user
  picks one.
- A theme variable may only be the whole value of a style prop (`@mau`, `@kich_thuoc`, `@khoang_cach`, `@chu`,
  `@flex_bo_cuc`, `@grid_bo_cuc`, `@vi_tri`) or of `giao_dien`. Using it as content, in a condition, in an
  expression, in a template string, or inside a responsive block is a build error.
- `dat_chu_de(name)` / `set_theme(name)` selects a declared theme; with no argument it switches to the next one. The
  choice is saved in `localStorage` (`vb-theme`) and restored before first paint when two or more themes exist.
- CSS: `:root` holds the default theme, `@media (prefers-color-scheme: dark)` the dark one under
  `:root:not([data-vb-theme])`, and `:root[data-vb-theme="name"]` every theme; each rule also sets
  `--vb-theme: <name>`.

### Static files and the HTML shell

`vibaoc build app.vbao` copies the `public/` directory next to the
source file into the output directory (images, a PDF, fonts, ...). A
top-level entry named `index.html`, `app.js`, `style.css` or `pkg` is
skipped with a warning. Reference these files with a relative path such
as `image(nguon: "avatar.svg")` or `link("CV", den: "cv.pdf")`. A
`favicon.svg`, `favicon.png` or `favicon.ico` in `public/` is linked
automatically.

The shell `index.html` carries the `<title>` and `mo_ta` of the `/`
page (or the first page) as `<title>`, `<meta name="description">` and
Open Graph `og:title`/`og:description`. After the first page the runtime
updates only the title and description while navigating.

### Props each layout tag uses

Layout tags only read the props below. Anything else on them (such as
`huong` on `box`) is accepted but produces a build warning naming the
tags that do use it — see [`LIMITATIONS.md`](LIMITATIONS.md).

| Tag | Props |
|---|---|
| `flex` | `direction`, `gap`, `row_gap`, `column_gap`, `align`, `align_items`, `wrap`, `width`, `height`, `padding`, `background_color`, `radius` |
| `grid` | `columns`, `rows`, `gap`, `row_gap`, `column_gap`, `width`, `padding`, `background_color` |
| `box` | `background_color`, `border_color`, `border_style`, `border`, `width`, `height`, `min_width`, `max_width`, `min_height`, `max_height`, `radius`, `padding`, `margin`, `shadow`, `overflow`, `translate_x`, `translate_y`, `z_index` |
| `stack` | `align`, `align_items`, `width`, `height` |
| `scroll` | `direction`, `width`, `height` |
| `container` | `max_width`, `padding` |
| `sticky` | `offset` |
| `fixed` | `position`, `width`, `height` |
| `layer` | none |

### Compile errors for block syntax

| Mistake | Message |
|---|---|
| Named prop in `(...)` of a tag that requires blocks | `Tag 'box' requires block syntax for property 'padding': write box { @spacing { padding: ... } } instead of using '(...)'` |
| Prop in the wrong block | `Property 'font_size' belongs to block @typography, not @color: move it to @typography { font_size: ... }` |
| Prop that no block accepts (`an`, `noi_dung`, ...) | `Property 'an' cannot be used inside block @color` |
| Same block twice | `Block '@color' is already declared on this tag and cannot be repeated: merge it into a single '@color' block` |
| Key other than `id`/`lop` in `@dinh_danh` | `'dem' is not valid in block @dinh_danh: only 'id' and 'lop' are accepted` |
| `@mau`-style block on a component call | `'@mau' is a semantic block of Element and does not apply to component calls: ...` |

With `lang = "vi";` the same messages are shown in Vietnamese.

### Colors

Only these 14 names resolve to a hex value, in their Vietnamese spelling
(there are no English color names). Any other identifier where a color
is expected is a hard build error.

```
trang=#FFFFFF   den=#000000     do=#E53E3E      xanh=#3182CE
xanh_la=#38A169 vang=#F59E0B    hong=#D53F8C    tim=#805AD5
cam=#DD6B20     xam=#718096     xam_nhat=#F7FAFC
xam_dam=#2D3748 luc=#25855A     nau=#7B341E
```

### Animation

```vbao
button("Click", hover_animation: "phong_to", scroll_animation: "fade_in")
```

Hover animation supports `phong_to` and `lam_sang`. Scroll/load-in
animation supports `fade_in`, `truot_len`, `truot_xuong`, `phong_to`,
`rung`. The effect name must be a static string. Vietnamese prop names:
`hieu_ung_hover`, `hieu_ung_cuon`. They are always written in `(...)`,
including on tags that require blocks (`khoi(hieu_ung_cuon: "fade_in")`).
These generate `data-vb-anim-hover` / `data-vb-anim-scroll` attributes;
the runtime binds them with real DOM events (`mouseenter`/`mouseleave`,
`IntersectionObserver`) via `web-sys` — no JavaScript animation library.

## 6. Expressions

```vbao
$name                  // read a variable — no $ when declaring, $ when using in an expression
$a + $b                // + is numeric addition OR string concatenation, depending on type
$a - $b, $a * $b, $a / $b, $a % $b
$a == $b, $a != $b     // strict equality, no type coercion
$a > $b, $a >= $b, $a < $b, $a <= $b
$a && $b, $a || $b, !$a
"Hello $name"          // template string interpolation; a $ before a digit or
                        // a non-identifier character is a literal character
round($n)               // built-in function, see §8
$list[0].field          // array element by integer literal index (not an expression)
$list.rong, $list.do_dai // empty / length of an array or string (Vietnamese-only names)
```

## 7. Control flow

```vbao
if $condition {
    // ...
} else {
    // ...
}
```

```vbao
loop $item in $list {
    text($item.name)
    button("Delete") {
        on_click { remove($item) }
    }
}

// Ascending inclusive range. N1 > N2 is a build-time error.
loop $i from 1 to 3 {
    text("Number: $i")
}
```

> Note: `in`/`from`/`to` above are shown in English for readability. The
> three connector words that join a loop's parts (`trong` for the
> collection form, `tu`/`den` for the range form) currently only have a
> Vietnamese surface form — see [`LIMITATIONS.md`](LIMITATIONS.md).

```vbao
switch $status {
    "loading" {
        text("Loading...")
    }
    "error" {
        text("Something went wrong")
    }
    default {
        text("Ready")
    }
}
```

`switch` compares with strict `==`, same as every other comparison in
ViBao. `default` is optional, and at most one `default` block is allowed.

## 8. Events and actions

```vbao
button("Click me") {
    on_click {
        notify("Hello!", kieu: thanh_cong)
    }
}
```

Supported events: `on_click`, `on_hover`, `on_blur`, `on_focus`,
`on_change`, `on_submit`, `on_scroll`.

### Page lifecycle events

```vbao
page("/") {
    on_load {
        // runs when the router navigates TO this page (including first boot)
    }
    on_unload {
        // runs when the router navigates AWAY from this page
    }
    // ... page content
}
```

`on_unload` for the outgoing page runs before it's hidden. `on_load` for
the incoming page is awaited before the page is bound, so a plain
`api_call` inside it delays event binding until the network answers;
use `api_call(..., background: true)` to detach it.

### Built-in actions

All of these run as real Rust/WASM, not JavaScript:

| Function | Description |
|---|---|
| `notify(text, kieu: ..., thoi_gian: ...)` | Temporary toast — **option keys must be the Vietnamese `kieu`/`thoi_gian` even in English files**, see `LIMITATIONS.md` |
| `alert(text)` | `window.alert()` |
| `navigate(path)` | Real SPA navigation (History API, no reload) |
| `open_new_tab(path)` | Opens a new tab |
| `open_modal(id)` / `close_modal(id)` | No working target in 0.1.5, see `LIMITATIONS.md` |
| `scroll_to(target)` | Smooth-scrolls to an element |
| `scroll_to_top()` | Scrolls to the top of the page |
| `save_data(endpoint, data)` | Real `POST` via `fetch` |
| `load_data(endpoint)` | Real `GET` via `fetch` |
| `copy_to_clipboard(text)` | **Disabled**, see `LIMITATIONS.md` |
| `array_push`, `array_remove_by_id`, `array_update_by_id` | Array CRUD helpers for state arrays with an `id` field |
| `set_theme(name?)` | Select a declared theme, or the next one with no argument; stores the choice in `localStorage` |
| `api_call(method, url, data?, headers: {...}, background: true)` | `fetch` with a JSON body. `headers` adds or overrides request headers; `background: true` (literal only) detaches the call. The response is assigned to the variable on the left of `=`, also on failure, and the `success` / `failure` blocks run when it arrives |

### Assignment and conditional actions

```vbao
$count = $count + 1
if $count > 10 {
    notify("That's enough!")
}
```

### Expression functions

`format_price`, `format_date`, `truncate`, `uppercase`, `format_percent`,
`round`.

## 9. Components (`@the`)

```vbao
@the Member(name: chuoi, age: so) {
    box {
        @color { background_color: xam_nhat }
        @spacing { padding: 16 }
        text($name, font_size: 18, bold: true)
        text("Age: $age", font_size: 14)
    }
}

Member(name: "An", age: 20)
Member(name: "Binh", age: 25) {
    @identity { id: "member-binh" }
}
```

Parameter types are declared as `name: type`. Only 7 types are accepted,
and only in their Vietnamese spelling: `chuoi` (string), `so` (number),
`mau` (color), `bool`, `mang` (array), `doi_tuong` (object), `hanh_dong`
(action). A parameter with no type annotation is untyped. There is no
`any` keyword.

Parameter names are free-named and written in `(...)`. The one fixed
property of a component call is `id`/`class`, which goes in an
`@identity { ... }` block in the call's body — never in `(...)`, so it
can never collide with a parameter of the same name.

Component props work well for the common case — static literal values,
one level of nesting — with some rough edges under heavier use; see
[`LIMITATIONS.md`](LIMITATIONS.md#works-but-needs-more-real-world-testing).

## 10. Build & run

```bash
vibaoc build app.vbao                       # writes dist/: HTML/CSS/JS + pkg/ (WASM runtime)
vibaoc build app.vbao --site-url https://example.com   # also sitemap.xml, robots.txt, canonical/og tags
vibaoc serve app.vbao --port 8000           # build, serve locally, rebuild on change
vibaoc check app.vbao --ast                 # debug: print the parsed AST, writes nothing
```

`404.html` (a copy of `index.html`) is written whenever the app has two or more routes. See
[`TRIEN_KHAI.md`](TRIEN_KHAI.md) for what `--site-url` generates and its limits.

`dist/` is build output and should not be committed (it's in
`.gitignore`, the same as `target/`). Release archives already bundle the
runtime WASM, so end users need no extra setup. When building from
source, run `scripts/build-runtime.sh` once — `vibaoc build` then finds
the runtime automatically in `vibao-runtime/pkg/` (or `pkg/` next to the
compiler binary). `VIBAO_PKG_DIR` is available as a maintainer override.
If the runtime isn't found, the build stops with an error rather than
producing an app with no interactivity.
