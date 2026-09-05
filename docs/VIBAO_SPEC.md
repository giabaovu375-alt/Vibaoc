# ViBao — Language Specification (0.1.0)

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

    page("/about", "About", background_color: light_gray) {
        // another page, with an optional display name and background color
    }
}
```

- `app("Name")` — the required root block that wraps everything else.
- `page("/path", "Name", background_color: <color>)` — declares one
  route. The second positional argument (display name) and the
  `background_color` option are both optional and can appear in any
  order after the route.
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
| `link` | `<a>` | `den:` / `to:` generates `href` and SPA navigation on click (see §5) |
| `image` | `<img>` | |
| `video` | `<video>` | |
| `icon` | `<span>` | |
| `input` | `<input>` | |
| `spacer` | `<div>` | |
| `divider` | `<hr>` | |
| `scroll` | `<div>` | also a layout tag, see below |
| `container` | `<div>` | also a layout tag, see below |

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

```vbao
tag(prop1: value1, prop2: value2, ...) {
    // children, event blocks, animation/responsive blocks
}
```

- Props (`key: value`) **must be inside `(...)`** right after the tag
  name — you cannot put `key: value` inside `{...}`.
- `{...}` only contains child elements and event blocks (`on_click { ... }`).
- For text-bearing tags (`text`, `button`, `link`, ...), the first
  unnamed string argument is automatically bound to the `content` prop:

  ```vbao
  text("Hello", font_size: 16)   // content = "Hello", font_size = 16
  ```

### Prop table (simple elements — text/button/input/...)

| Prop | CSS/HTML | Notes |
|---|---|---|
| `color` / `text_color` | `color` | |
| `border_color` | `border-color` | |
| `width` | `width` | e.g. `200` (px) or `"50%"` |
| `height` | `height` | |
| `max_width` | `max-width` | |
| `radius` | `border-radius` | |
| `padding` | `padding` | |
| `margin` | `margin` | |
| `border` | `border-width` | |
| `border_style` | `border-style` | |
| `shadow` | `box-shadow` | |
| `overflow` | `overflow` | |
| `z_index` | `z-index` | |
| `font_size` | `font-size` | |
| `bold` | `font-weight: bold` (bool) | |
| `italic` | `font-style: italic` (bool) | |
| `underline` | `text-decoration: underline` (bool) | |
| `align` | `text-align` (`left`/`right`/`center`/`justify`) | |
| `line_height` | `line-height` | |
| `letter_spacing` | `letter-spacing` | |
| `transform` | `transform` | |
| `font` | `font-family` | |
| `direction` | `flex-direction` (on `flex` elements) | |
| `gap` | `gap` | |
| `wrap` | flex-wrap related | |
| `fit` | `object-fit` (image/video) | |
| `alt` | `alt` (image) | |
| `lazy` | `loading="lazy"` (bool) | |
| `type` | `type` (input) | |
| `placeholder` | `placeholder` (input) | |
| `required` | `required` (bool) | |
| `disabled` | `disabled` (bool) | |
| `value` | `value` (input) — a `$variable` here activates two-way binding | |
| `content` | text content | auto-filled from the first positional string |
| `den` / `to` | `href` + SPA navigation | **`link` only.** Must be a static string literal — dynamic routes aren't supported, use `button` + `navigate()` instead |

### Layout-tag-only props (`box`/`flex`/`grid`/...)

| Prop | Applies to | Notes |
|---|---|---|
| `color` | all layout tags | **background-color, not text color** |
| `padding` | all layout tags | |
| `margin` | `box` | |
| `radius` | `box` | |
| `border` | `box` | |
| `shadow` | `box` | |
| `overflow_x`, `overflow_y` | `box` | |
| `direction` | `flex` | row/column — **no effect on `box`**, see `LIMITATIONS.md` |
| `gap`, `gap_x`, `gap_y` | `flex`/`grid` | **no effect on `box`** |
| `justify` | `flex` | `justify-content` — `start`/`end`/`center` |
| `align` | `flex` | `align-items` — `start`/`end`/`center`/`stretch` |
| `columns`, `rows` | `grid` | `grid-template-columns`/`rows` |
| `min_width`, `max_width`, `min_height`, `max_height` | `container` | |
| `position` | `sticky`/`fixed` | `top`/`bottom`/`left`/`right` |

### Colors

Only these 14 names resolve to a hex value; anything else is printed
as-is into CSS (invalid):

```
white=#FFFFFF   black=#000000    red=#E53E3E     blue=#3182CE
green=#38A169   yellow=#F59E0B   pink=#D53F8C    purple=#805AD5
orange=#DD6B20  gray=#718096     light_gray=#F7FAFC
dark_gray=#2D3748  emerald=#25855A  brown=#7B341E
```

(Vietnamese source uses the equivalents: `trang`, `den`, `do`, `xanh`,
`xanh_la`, `vang`, `hong`, `tim`, `cam`, `xam`, `xam_nhat`, `xam_dam`,
`luc`, `nau`.)

### Animation

```vbao
box() {
    hover_animation: "grow"     // set alongside other props, inside (...)
}
```

Hover animation supports `grow` and `brighten`. Scroll/load-in animation
supports `fade_in`, `slide_up`, `slide_down`, `grow`, `shake`. These
generate `data-vb-anim-hover` / `data-vb-anim-scroll` attributes; the
runtime binds them with real DOM events (`mouseenter`/`mouseleave`,
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

`on_unload` for the outgoing page runs before it's hidden; `on_load` for
the incoming page runs after it's shown and bound.

### Built-in actions

All of these run as real Rust/WASM, not JavaScript:

| Function | Description |
|---|---|
| `notify(text, kieu: ..., thoi_gian: ...)` | Temporary toast — **option keys must be the Vietnamese `kieu`/`thoi_gian` even in English files**, see `LIMITATIONS.md` |
| `alert(text)` | `window.alert()` |
| `navigate(path)` | Real SPA navigation (History API, no reload) |
| `open_new_tab(path)` | Opens a new tab |
| `open_modal(id)` / `close_modal(id)` | No working target in 0.1.0, see `LIMITATIONS.md` |
| `scroll_to(target)` | Smooth-scrolls to an element |
| `scroll_to_top()` | Scrolls to the top of the page |
| `save_data(endpoint, data)` | Real `POST` via `fetch` |
| `load_data(endpoint)` | Real `GET` via `fetch` |
| `copy_to_clipboard(text)` | **Disabled**, see `LIMITATIONS.md` |
| `array_push`, `array_remove_by_id`, `array_update_by_id` | Array CRUD helpers for state arrays with an `id` field |

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
@the Member(name: string, age: number) {
    box(padding: 16, color: light_gray) {
        text($name, font_size: 18, bold: true)
        text("Age: $age", font_size: 14)
    }
}

Member(name: "An", age: 20)
Member(name: "Binh", age: 25)
```

Parameter types are declared as `name: type`. Only 7 types are accepted:
`string`, `number`, `color`, `bool`, `array`, `object`, `action`
(Vietnamese: `chuoi`, `so`, `mau`, `bool`, `mang`, `doi_tuong`,
`hanh_dong`). There is no `any` type.

Component props work well for the common case — static literal values,
one level of nesting — with some rough edges under heavier use; see
[`LIMITATIONS.md`](LIMITATIONS.md#works-but-needs-more-real-world-testing).

## 10. Build & run

```bash
vibaoc build app.vbao          # writes dist/: HTML/CSS/JS + pkg/ (WASM runtime)
vibaoc check app.vbao --ast    # debug: print the parsed AST, writes nothing
```

`dist/` is build output and should not be committed (it's in
`.gitignore`, the same as `target/`). Release archives already bundle the
runtime WASM, so end users need no extra setup. When building from
source, run `scripts/build-runtime.sh` once — `vibaoc build` then finds
the runtime automatically in `vibao-runtime/pkg/` (or `pkg/` next to the
compiler binary). `VIBAO_PKG_DIR` is available as a maintainer override.
If the runtime isn't found, the build stops with an error rather than
producing an app with no interactivity.
