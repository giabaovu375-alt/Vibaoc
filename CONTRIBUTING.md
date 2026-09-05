# Contributing to ViBao

Thanks for considering a contribution! ViBao is young, and the project
benefits a lot from real usage, bug reports, and PRs.

## Development setup

```bash
git clone https://github.com/giabaovu375-alt/Vibaoc.git
cd ViBao
cargo build --workspace
cargo test --workspace
```

To also build the browser runtime (WASM), you'll additionally need the
`wasm32-unknown-unknown` Rust target and `wasm-bindgen-cli`:

```bash
rustup target add wasm32-unknown-unknown
bash scripts/build-runtime.sh
```

## Project layout

| Crate            | Responsibility                                          |
|-------------------|----------------------------------------------------------|
| `vibao-ast`       | Shared AST types and semantic identity (locale-agnostic) |
| `vibaoc`          | Compiler CLI: lexer, parser, resolver, validator, codegen |
| `vibao-runtime`   | Browser runtime compiled to WebAssembly                  |

The core design rule: **Vietnamese and English surface syntax always
resolve to the same AST.** If you add a new tag, prop, action, or
function, it needs an entry in both `vibaoc/src/locale/*_vi.rs` and
`vibaoc/src/locale/*_en.rs` (or the shared tables in
`vibaoc/src/lexer/tables.rs`), plus tests confirming both surface names
resolve to the same semantic identity.

## Before opening a PR

- `cargo test --workspace` should pass.
- If you touched compiler-user-facing behavior (new syntax, a changed
  error message, a new action/function/prop), add or update an
  end-to-end test under `vibaoc/tests/` — see the existing
  `e2e_*.rs` files for the black-box style used there (they drive the
  real `vibaoc` binary against real `.vbao` source, not internal
  functions).
- If you fixed a bug, prefer a regression test that would have failed
  before your fix.
- Keep comments and identifiers in English where practical, since
  the code is meant to be equally approachable regardless of which
  surface language a contributor writes ViBao programs in.

## Reporting bugs

Please include:
- The `.vbao` source that triggers the issue (a minimal reproduction
  helps a lot).
- The exact command you ran (`vibaoc build ...` / `vibaoc check ...`).
- What you expected vs. what happened, including any error output.

## Scope for 0.1.0

See [`docs/LIMITATIONS.md`](docs/LIMITATIONS.md) for what's intentionally
out of scope, disabled, or still rough in this release, so issues don't
get filed against known gaps.

## License

By contributing, you agree that your contributions will be licensed
under the [MIT License](LICENSE).
