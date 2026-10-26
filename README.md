# ViBao

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Release](https://img.shields.io/github/v/release/giabaovu375-alt/Vibaoc)](https://github.com/giabaovu375-alt/Vibaoc/releases)

**ViBao** is a Vietnamese-first DSL and compiler for building web UIs.
It compiles directly to HTML/CSS/JS plus a small WebAssembly runtime —
no Node.js, no bundler, no JavaScript framework required.

> 📖 Tài liệu tiếng Việt: [`docs/README_VI.md`](docs/README_VI.md)

```vbao
app("My app") {
    page("/") {
        box {
            @spacing { padding: 16 }
            text("Count: ", bold: true)
            button("Increment") {
                on_click {
                    $count = $count + 1
                    if $count >= 10 {
                        notify("Reached 10!", kieu: thanh_cong)
                    }
                }
            }
        }
    }
}
```

The same program, written in Vietnamese:

```vbao
ứng_dụng("Ứng dụng của tôi") {
    trang("/") {
        khối {
            @khoảng_cách { đệm: 16 }
            text("Số đếm: ", đậm: true)
            button("Tăng") {
                khi_nhấn {
                    $đếm = $đếm + 1
                    nếu $đếm >= 10 {
                        thông_báo("Đã đạt 10!", kiểu: thành_công)
                    }
                }
            }
        }
    }
}
```

Vietnamese (with or without diacritics) and English keywords all resolve to the
exact same AST — write in whichever you prefer, mix them freely, and switch anytime.

## Goals

ViBao exists to explore an idea most languages don't: **UI code written
in keywords from your own language, not just English.** Vietnamese and
English are what's shipped today, but neither is the ceiling — the
locale layer is built so any language can plug in without touching the
compiler's core (see [Multi-language keywords](#multi-language-keywords)).

Beyond locale, ViBao is also a place to try syntax and features that
established frameworks haven't — the kind of ideas that don't fit
neatly into "another React clone." If you've got a weird idea for how
UI code could work, an unusual syntax, a small built-in library that
doesn't exist elsewhere — this project is meant to be a sandbox for
that, and contributions along those lines are very welcome, not just
bug fixes.

This is also a personal project built to learn compiler and language
design in the open, so expect the pace and scope of a side project
rather than a company-backed toolchain.

## Multi-language keywords

ViBao is not meant to stay a Vietnamese/English-only DSL. The locale
layer (`vibaoc/src/locale/`) is deliberately isolated from the rest of
the compiler: the lexer, parser, codegen, and validator only ever deal
in language-agnostic semantic identities (`Tag`, `PropKey`,
`ActionName`, `FunctionName`, defined in `vibao-ast`). Each spoken
language is just a table mapping surface words to those identities —
`vi.rs` and `en.rs` today, with the same shape available for a future
`ja.rs`, `es.rs`, `fr.rs`, and so on.

Concretely, adding a new keyword locale means:
1. Writing a new `vibaoc/src/locale/<lang>.rs` (plus `<lang>_action.rs`
   / `<lang>_function.rs` / `<lang>_prop.rs`) that maps that language's
   words to the existing `Tag`/`PropKey`/`ActionName`/`FunctionName`
   values — no new semantics, just new names for the same concepts.
2. Registering it in `vibaoc/src/locale/mod.rs` and
   `vibaoc/src/lexer/tables.rs`, following the exact pattern already
   used for Vietnamese and English.
3. Adding resolution tests confirming the new locale's keywords map to
   the same AST as the existing ones (see the `*_vi.rs` / `*_en.rs`
   test modules for the pattern to copy).

At runtime, the lexer always checks the active locale's table
*together with* English (English is the universal fallback locale), so
adding a new language never breaks existing `.vbao` files. Vietnamese
and English are the only two shipped today — contributions adding new
locales are very welcome; see [CONTRIBUTING.md](CONTRIBUTING.md).

## Why ViBao?

- **Compiles to a static web app.** The output is plain HTML/CSS/JS
  you can open directly in a browser or deploy anywhere.
- **Small runtime.** State, reactivity, routing, and events are
  handled by a lightweight WebAssembly runtime instead of a large JS
  framework.
- **Fails at compile time, not in production.** Unknown actions,
  unknown functions, props in the wrong block, misused theme variables
  and several other common mistakes are caught by the compiler with a
  clear message and a source location.
- **Light/dark themes built in.** Declare themes once, follow the OS
  automatically, switch with one action, and the choice is remembered
  without a flash on reload.
- **Talks to real APIs.** `goi_api` / `api_call` supports headers, reading
  nested responses (`$res.choices[0].message.content`) and background
  loading so the page stays interactive.
- **Static-host friendly.** `--site-url` writes `sitemap.xml`,
  `robots.txt`, canonical and Open Graph tags; multi-route apps get a
  `404.html` fallback.

## Quick start

Download a prebuilt binary for your platform from
[Releases](https://github.com/giabaovu375-alt/Vibaoc/releases) —
Linux, macOS (Intel/Apple Silicon), Windows, and Android (via Termux) —
no Rust or WASM tooling required — or install with one command:

```bash
curl -fsSL https://raw.githubusercontent.com/giabaovu375-alt/Vibaoc/main/scripts/install.sh | sh
```

See [`docs/INSTALLATION.md`](docs/INSTALLATION.md) for platform-specific
notes, including Android/Termux setup.

Then compile a `.vbao` file:

```bash
vibaoc check app.vbao                  # report errors only
vibaoc build app.vbao --out dist       # write dist/
vibaoc serve app.vbao --port 8000      # build, serve locally, rebuild on change
```

Open `dist/index.html` (or `http://127.0.0.1:8000`) in a browser.

Sample apps are in [`examples/`](examples/) (kept in the repository only,
not shipped in release archives): a multi-page app with a reusable component
(`multi_page.vbao`), a task manager with array CRUD
(`task_manager.vbao`), a personal portfolio with light/dark
themes and repositories loaded from GitHub (`portfolio.vbao`), and a chat
client for the Groq API (`chat_groq.vbao`, put your own API key in it first
and never commit the file with the key).

## Repository layout

```text
.
├── vibao-ast/        # Shared AST and semantic identity types
├── vibaoc/           # Compiler CLI crate (lexer, parser, resolver,
│                      # validator, codegen) + end-to-end tests
├── vibao-runtime/    # Browser runtime crate, compiled to WASM
├── docs/             # Language and project documentation
└── scripts/          # Build, release, install, and verification scripts
```

## Building from source

Requirements:
- Rust and Cargo
- The `wasm32-unknown-unknown` Rust target (only needed to build the
  browser runtime)
- `wasm-bindgen-cli` — `scripts/build-runtime.sh` installs the
  Cargo-locked matching version automatically if it's missing

```bash
cargo test --workspace          # run the full test suite
bash scripts/build-runtime.sh   # build the runtime package (WASM); the e2e tests need it
bash scripts/verify.sh          # fmt check, tests and build with warnings denied
bash scripts/build-release.sh 0.1.5   # build a release archive for the current platform
```

See [`scripts/RELEASE.md`](scripts/RELEASE.md) for the full release
process.

## Docs

| Doc | Description |
|---|---|
| [`docs/INSTALLATION.md`](docs/INSTALLATION.md) | Install instructions for every platform, plus building from source |
| [`docs/SYNTAX_EN.md`](docs/SYNTAX_EN.md) | English syntax cheat-sheet |
| [`docs/SYNTAX_VI.md`](docs/SYNTAX_VI.md) | Vietnamese syntax cheat-sheet (themes, API calls, components) |
| [`docs/TRIEN_KHAI.md`](docs/TRIEN_KHAI.md) | Deploying to static hosts: `--site-url`, `404.html`, sitemap (in Vietnamese) |
| [`CHANGELOG.md`](CHANGELOG.md) | Release history (in Vietnamese) |
| [`docs/VIBAO_SPEC.md`](docs/VIBAO_SPEC.md) | Full language specification |
| [`docs/LIMITATIONS.md`](docs/LIMITATIONS.md) | What's incomplete, disabled, or silently no-op in 0.1.5 |
| [`docs/PROFILE_GUIDE.md`](docs/PROFILE_GUIDE.md) | Building a personal web profile: assets, fonts, anchors, form (in Vietnamese) |
| [`docs/README_VI.md`](docs/README_VI.md) | This README, in Vietnamese |

## A slightly longer example

State, a conditional, a loop and a theme switch:

```vbao
lang = "vi";

ứng_dụng("Ví dụ ViBao") {
    chủ_đề sáng { $nền = trang, $chữ = xám_đậm }
    chủ_đề tối { $nền = "#1a202c", $chữ = "#e2e8f0" }

    giao_diện { màu_nền: $nền, màu: $chữ }

    trang("/") {
        state $đếm = 0
        state $việc = [
            {id: 1, tên: "Viết tài liệu", xong: false},
            {id: 2, tên: "Sửa lỗi giao diện", xong: true}
        ]

        flex {
            @khoảng_cách { đệm: 24 }
            @flex_bố_cục { hướng: cột, gap: 12 }

            text("Bộ đếm: $đếm", cỡ: 20, đậm: true)
            button("Tăng") {
                khi_nhấn { $đếm = $đếm + 1 }
            }

            nếu $đếm >= 10 {
                text("Đã đạt 10!", màu: xanh_lá)
            } không_thì {
                text("Chưa đến 10", màu: xám)
            }

            vòng_lặp $v trong $việc {
                text($v.tên)
            }

            button("Đổi giao diện") {
                khi_nhấn { đặt_chủ_đề() }
            }
        }
    }
}
```

## Project status

ViBao 0.1.5 is still early — usable for small apps and personal sites, but
with real rough edges. A few worth knowing about:

- Several components (`modal`, `tabs`, `table`, `chart`, and others) are
  still placeholders that render an empty `<div>`. Use `@the` to build
  the equivalent yourself in the meantime.
- `direction`/`gap`/`letter_spacing` have no effect on `box` — only on
  `flex`/`grid`/`scroll`.
- Every route shares one `index.html`, so titles and Open Graph tags are
  the same site-wide; hosting under a sub-path is not verified yet.
- API keys written in a `.vbao` file are visible in the browser.

The full list, including what's disabled, what needs more real-world
testing, and what's intentionally out of scope for 0.1.5, is in
[`docs/LIMITATIONS.md`](docs/LIMITATIONS.md). Bug reports and feedback —
on the rough edges above, the locale/keyword design, or anything else —
are very welcome; see [Contributing](#contributing).

## Testing

The workspace's test suite spans three layers:

- **Unit tests** inside each crate (`vibao-ast`, `vibaoc`,
  `vibao-runtime`) covering the lexer, parser, resolver, validator,
  codegen, and runtime logic in isolation.
- **Contract tests** (`vibao-ast/tests/serde_contract.rs`) that pin
  down the AST's `serde` (JSON) shape, since the runtime consumes
  compiler output across that boundary.
- **End-to-end tests** (`vibaoc/tests/e2e_*.rs`) that invoke the real
  `vibaoc` binary against real `.vbao` source files and inspect the
  compiled `index.html` / `app.js` / `style.css` — the same thing a
  `vibaoc build app.vbao` from an end user would produce.

```bash
bash scripts/build-runtime.sh   # once, so the e2e tests can find the runtime
cargo test --workspace
```

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) before opening a pull request.

## License

[MIT License](LICENSE).
