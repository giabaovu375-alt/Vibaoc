## File language directive

English is the default diagnostic language, so an English source file does not need a `lang` declaration. To request Vietnamese diagnostics, put this at the top of the file:

```vbao
lang = "vi";
```

English syntax remains accepted alongside the active locale.

# ViBao — English Syntax (0.1.5)

English is a **universal surface locale**. It is accepted together with the
Vietnamese vocabulary, and both resolve to the same semantic AST identities.

## Application skeleton

```vbao
app("My app") {
    page("/") {
        box {
            @spacing { padding: 16 }
            text("Hello")
        }
    }
}
```

## Common tags

`text`, `h1`, `h2`, `h3`, `p`, `label`, `image`, `video`, `icon`, `button`,
`input`, `link`, `flex`, `grid`, `stack`, `box`, `scroll`, `container`, `layer`,
`sticky`, `fixed`, `spacer`, `divider`, `form`, `input_group`, `radio`,
`checkbox`, `select`, `modal`, `tabs`, `accordion`, `carousel`, `pagination`,
`spinner`, `progress`, `table`, `chart`, `map`, `nav`, `editor`.

## Props and `@block` groups

Named props are split into groups (table below). Some tags are **required**
to use blocks: `box`, `flex`, `grid`, `input`, `textarea`, `radio`,
`checkbox`, `select` — their named props can only be written inside
`@block { ... }` in the `{ }` body. Every other tag (`text`, `button`,
`image`, ...) accepts both forms, with identical results.

```vbao
box {
    @color { background_color: xam_nhat }
    @spacing { padding: 16, radius: 8 }
    @size { width: 320 }
    text("Content", font_size: 18, bold: true, color: xanh)
}

text("Content") {
    @typography { font_size: 18, bold: true }
}

image(source: "/images/logo.png", alt: "Logo")

input {
    @input { type: "text", placeholder: "Name", value: $name }
}
```

| Block | Props |
|---|---|
| `@color` | `background_color`, `color`, `border_color` |
| `@size` | `width`, `height`, `max_width`, `min_width`, `max_height`, `min_height` |
| `@spacing` | `radius`, `padding`, `margin`, `border`, `border_style`, `shadow`, `overflow`, `z_index` |
| `@typography` | `font_size`, `bold`, `italic`, `underline`, `align`, `line_height`, `letter_spacing`, `text_transform`, `font_family` |
| `@flex_layout` | `direction`, `align_items`, `wrap`, `gap`, `row_gap`, `column_gap` |
| `@grid_layout` | `columns`, `rows`, `gap`, `row_gap`, `column_gap` |
| `@position` | `translate_x`, `translate_y`, `position`, `offset` |
| `@media` | `fit`, `source`, `alt`, `lazy_load` |
| `@input` | `type`, `placeholder`, `value`, `option_value`, `required`, `disabled` |
| `@link` | `to`, `target`, `rel` |
| `@identity` | `id`, `class`, `html_tag` |

Rules:

- Blocks may appear in any order; each block can be declared **once** per
  tag (`@color` and `@mau` count as the same block).
- A prop placed in the wrong block is a compile error that names the right
  one (`Property 'font_size' belongs to block @typography, not @color`).
  `gap`/`row_gap`/`column_gap` are accepted in both `@flex_layout` and
  `@grid_layout`.
- The content stays positional: `text("Hello")`.
- `hover_animation`/`scroll_animation` are always written in `(...)`, even
  on tags that require blocks.
- `html_tag: "section"` in `@identity` turns the `<div>` of a container
  tag (`box`, `flex`, `grid`, ...) into a semantic element: `section`,
  `header`, `footer`, `main`, `article`, `aside`, `nav`, `figure`, `ul`,
  `li`. It works on elements only, not on component calls, and is ignored
  with a warning on tags that are not rendered as `<div>` (such as
  `button`).
- `id`/`class` go only in `@identity`, on component calls too. Component
  parameters keep their free names inside `(...)`.

`value: $name` on `input`/`textarea` activates two-way model binding for a
plain state variable.

## Dynamic classes

```vbao
button(class: { active: $selected, muted: $quiet }) {
    on_click { $selected = !$selected }
}
```

## Animation

```vbao
button(hover_animation: "phong_to", scroll_animation: "truot_len")
```

Hover supports `phong_to` and `lam_sang`. Scroll/load-in supports
`fade_in`, `truot_len`, `truot_xuong`, `phong_to`, `rung`.

## Events and actions

```vbao
button("Increment") {
    on_click {
        $count = $count + 1
        if $count >= 10 {
            notify("Reached 10!", kieu: thanh_cong)
        }
    }
}
```

Toast options accept both spellings: `type:`/`kieu:` and `duration:`/
`thoi_gian:`. The toast kind is one of `success`, `error`, `warning`,
`info` (Vietnamese: `thanh_cong`, `loi`, `canh_bao`, `info`). The
`api_call` response blocks are `success { ... }` and `failure { ... }`
(Vietnamese: `thanh_cong`, `that_bai`); any other block name is a build
error. An option key an action does not read is still ignored without a
warning. See [`LIMITATIONS.md`](LIMITATIONS.md) for the full list of gaps.

English actions: `notify`, `alert`, `navigate`, `open_new_tab`, `open_modal`,
`close_modal`, `scroll_to`, `scroll_to_top`, `save_data`, `load_data`,
`copy_to_clipboard`, `api_call`, `array_push`, `array_remove_by_id`,
`array_update_by_id`.

Note: `open_modal`/`close_modal` currently have no valid target element
in 0.1.5 — they look for an HTML id of the form `vb-modal-<id>`. A custom
`id` can be set through `@identity { id: ... }`, but the only tag that
naturally gets that id (`modal`) is still an unimplemented placeholder
tag (renders an empty `<div>`). Both actions still compile without
error; they simply have no working effect yet.

English expression functions: `format_price`, `format_date`, `truncate`,
`uppercase`, `format_percent`, `round`.

## Components

```vbao
@the Card(title: chuoi) {
    box {
        @spacing { padding: 16 }
        text($title, bold: true)
    }
}
Card(title: "Hello")
Card(title: "With id") {
    @identity { id: "card-with-id" }
}
```

Component parameter types are written in Vietnamese only: `chuoi`, `so`,
`mau`, `bool`, `mang`, `doi_tuong`, `hanh_dong`.

Runtime code does not depend on the English locale.


## Template strings and ranges

- In a template string, `$name` interpolates a variable. A `$` followed by a
  number or another non-variable-start character remains a literal character
  (for example, `"Price: $50"`).
- `vong_lap $i tu N1 den N2` is an inclusive ascending range. If `N1 > N2`,
  the compiler reports an error; descending ranges are not supported in 0.1.5.
- Page options use the shared property locale resolver, so both `mau_nen` and
  `background_color` are accepted for the page background option.

## A few Vietnamese-only "soft keywords" — no English form exists yet

The `loop` keyword itself has a full English form, but 3 small connector
words inside loop syntax are recognized as plain hard-coded strings, not
through the locale layer used everywhere else — so no English equivalent
exists for them yet, even in an all-English file:

- `trong` (the "in" connector of a collection loop: `loop $item trong
  $list { ... }`) — writing `in` instead is not recognized.
- `tu` / `den` (the "from"/"to" of a numeric range: `loop $i tu 1 den 5
  { ... }`) — no `from`/`to` form exists.

Responsive breakpoint names (`@di_dong`, `@may_tinh_bang`, `@may_tinh`)
are similarly Vietnamese-only for now — there is no `@mobile`/
`@tablet`/`@desktop` form. See [`LIMITATIONS.md`](LIMITATIONS.md) for
everything else that's incomplete in 0.1.5.

## Themes, background loading and deployment (0.1.5)

```vbao
app("Demo") {
    theme light { $bg = "#ffffff", $fg = "#2d3748" }
    theme dark  { $bg = "#1a202c", $fg = "#e2e8f0" }

    page("/") {
        state $repos = []
        state $status = "loading"

        on_load {
            $repos = api_call("GET", "https://api.github.com/users/name/repos",
                    headers: {Accept: "application/vnd.github+json"}, background: true) {
                success { $status = "done" }
                failure { $status = "error" }
            }
        }

        box {
            @color { background_color: $bg, color: $fg }
            button("Theme") { on_click { set_theme() } }
        }
    }
}
```

- `theme` blocks must declare the same set of `$variable`s, with constant values (colors written as hex strings,
  numbers, strings). The first theme is the default; a theme named `dark` (`toi` in Vietnamese) follows the operating
  system's dark mode. `set_theme()` switches to the next theme, `set_theme("dark")` picks one by name. The choice is
  stored in `localStorage` and restored before first paint.
- `api_call` accepts `headers: { ... }` and `background: true`. `$list[0].field` reads an array element by literal index.
  The Vietnamese-only fields `$list.rong` and `$list.do_dai` (empty / length) work on arrays and strings.
- `preserve_lines: true` keeps line breaks and spaces of a string (`white-space: pre-wrap`).
- `giao_dien { ... }` (document font, background, smooth scrolling) and the named color list are Vietnamese-only; see
  [`SYNTAX_VI.md`](SYNTAX_VI.md).
- `vibaoc build app.vbao --site-url https://example.com` also writes `sitemap.xml`, `robots.txt`, canonical and
  Open Graph tags; with two or more routes it writes `404.html`. See [`TRIEN_KHAI.md`](TRIEN_KHAI.md) (Vietnamese).
