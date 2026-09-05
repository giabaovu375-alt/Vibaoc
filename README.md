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
        box(padding: 16) {
            text("Count: ", bold: true)
            button("Increment") {
                on_click {
                    $count = $count + 1
                    if $count >= 10 {
                        notify("Reached 10!", type: success)
                    }
                }
            }
        }
    }
}
```

The same program, written in Vietnamese:

```vbao
ung_dung("Ung dung cua toi") {
    trang("/") {
        khoi(dem: 16) {
            text("So dem: ", dam: true)
            button("Tang") {
                khi_nhan {
                    $dem = $dem + 1
                    neu $dem >= 10 {
                        thong_bao("Da dat 10!", kieu: thanh_cong)
                    }
                }
            }
        }
    }
}
```

Vietnamese and English keywords both resolve to the exact same AST —
write in whichever you prefer, mix them freely, and switch anytime.

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
  unknown functions, and several other common mistakes are caught by
  the compiler with a clear message and a source location.

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
vibaoc app.vbao
# or, explicitly:
vibaoc build app.vbao --out dist
```

Open `dist/index.html` in a browser to see the result.

More sample apps are in [`examples/`](examples/): a counter
(`counter.vbao`), a multi-page app with a reusable component
(`multi_page.vbao`), and a small task manager with array CRUD
(`task_manager.vbao`).

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
bash scripts/build-runtime.sh   # build the runtime package (WASM)
bash scripts/build-release.sh 0.1.0   # build a release archive for the current platform
```

See [`scripts/RELEASE.md`](scripts/RELEASE.md) for the full release
process.

## Docs

| Doc | Description |
|---|---|
| [`docs/INSTALLATION.md`](docs/INSTALLATION.md) | Install instructions for every platform, plus building from source |
| [`docs/SYNTAX_EN.md`](docs/SYNTAX_EN.md) | English syntax cheat-sheet |
| [`docs/SYNTAX_VI.md`](docs/SYNTAX_VI.md) | Vietnamese syntax cheat-sheet |
| [`docs/VIBAO_SPEC.md`](docs/VIBAO_SPEC.md) | Full language specification |
| [`docs/LIMITATIONS.md`](docs/LIMITATIONS.md) | What's incomplete, disabled, or silently no-op in 0.1.0 |
| [`docs/README_VI.md`](docs/README_VI.md) | This README, in Vietnamese |

## A slightly longer example

A small counter app with state, a conditional, and a loop:

```vbao
lang = "vi";

ung_dung("Vi du ViBao") {
    trang("/") {
        state $dem = 0
        state $tasks = [
            {id: 1, tieu_de: "Viet tai lieu", xong: false},
            {id: 2, tieu_de: "Sua loi giao dien", xong: true}
        ]

        khoi(dem: 24, huong: cot, khoang_chu: 12) {
            text("Bo dem: $dem", co: 20, dam: true)
            button("Tang") {
                khi_nhan { $dem = $dem + 1 }
            }

            neu $dem >= 10 {
                text("Da dat 10!", mau: xanh_la)
            } khong_thi {
                text("Chua den 10", mau: xam)
            }

            vong_lap $task trong $tasks {
                text($task.tieu_de)
            }
        }
    }
}
```

## Project status

ViBao 0.1.0 is an early, first public release — usable for basic apps,
but with real rough edges. A few worth knowing about:

- Action option keys (`kieu:`/`thoi_gian:` in `notify(...)`) currently
  only resolve in Vietnamese, even in an all-English file — the
  English-looking `type:`/`duration:` compiles with no error but is
  silently ignored.
- Several components (`modal`, `tabs`, `table`, `chart`, and others) are
  still placeholders that render an empty `<div>`. Use `@the` to build
  the equivalent yourself in the meantime.
- `direction`/`gap`/`letter_spacing` have no effect on `box` — only on
  `flex`/`grid`/`scroll`.

The full list, including what's disabled, what needs more real-world
testing, and what's intentionally out of scope for 0.1.0, is in
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
cargo test --workspace
```

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) before opening a pull request.

## License

[MIT License](LICENSE).
