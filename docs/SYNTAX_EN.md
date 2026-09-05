## File language directive

English is the default diagnostic language, so an English source file does not need a `lang` declaration. To request Vietnamese diagnostics, put this at the top of the file:

```vbao
lang = "vi";
```

English syntax remains accepted alongside the active locale.

# ViBao — English Syntax (0.1.0)

English is a **universal surface locale**. It is accepted together with the
Vietnamese vocabulary, and both resolve to the same semantic AST identities.

## Application skeleton

```vbao
app("My app") {
    page("/") {
        box(padding: 16) {
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

## Props

```vbao
box(background_color: xam_nhat, padding: 16, radius: 8, width: 320) {
    text("Content", font_size: 18, bold: true, color: xanh)
}

image(source: "/images/logo.png", alt: "Logo")
input(type: "text", placeholder: "Name", value: $name)
```

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

**Heads up:** action option keys (`kieu:`/`thoi_gian:` above) are not
locale-resolved — the parser stores the literal key string as-is, and the
runtime only ever reads the exact strings `"kieu"`/`"thoi_gian"`. Writing
the English-looking `type:`/`duration:` instead compiles with no error or
warning, but is silently ignored at runtime (the toast always falls back
to its default type/duration). Always write `kieu:`/`thoi_gian:` here,
even in an otherwise all-English file. The same applies to the
`goi_api`/`api_call` response-callback block: only the literal names
`thanh_cong`/`that_bai` are recognized, not `success`/`failure`. See
[`LIMITATIONS.md`](LIMITATIONS.md) for the full list of gaps like this.

English actions: `notify`, `alert`, `navigate`, `open_new_tab`, `open_modal`,
`close_modal`, `scroll_to`, `scroll_to_top`, `save_data`, `load_data`,
`copy_to_clipboard`, `api_call`, `array_push`, `array_remove_by_id`,
`array_update_by_id`.

Note: `open_modal`/`close_modal` currently have no valid target element
in 0.1.0 — they look for an HTML id of the form `vb-modal-<id>`, but no
prop exists anywhere in the language to set a custom `id` on an
element, and the only tag that naturally gets that id (`modal`) is
still an unimplemented placeholder tag (renders an empty `<div>`). Both
actions still compile without error; they simply have no working
effect yet.

English expression functions: `format_price`, `format_date`, `truncate`,
`uppercase`, `format_percent`, `round`.

## Components

```vbao
@the Card(title: chuoi) {
    box(padding: 16) {
        text($title, bold: true)
    }
}
Card(title: "Hello")
```

Runtime code does not depend on the English locale.


## Template strings and ranges

- In a template string, `$name` interpolates a variable. A `$` followed by a
  number or another non-variable-start character remains a literal character
  (for example, `"Price: $50"`).
- `vong_lap $i tu N1 den N2` is an inclusive ascending range. If `N1 > N2`,
  the compiler reports an error; descending ranges are not supported in 0.1.0.
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
everything else that's incomplete in 0.1.0.
