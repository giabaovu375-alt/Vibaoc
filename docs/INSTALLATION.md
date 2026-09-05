# Installation

ViBao ships as a prebuilt binary. You do not need Rust, Cargo, or any WASM
tooling to use it — those are only needed if you want to build ViBao itself
from source (see [Building from source](#building-from-source)).

## One-command install (Linux / macOS / Android via Termux)

```bash
curl -fsSL https://raw.githubusercontent.com/giabaovu375-alt/Vibaoc/main/scripts/install.sh | sh
```

This detects your OS and CPU architecture, downloads the matching release
archive from [Releases](https://github.com/giabaovu375-alt/Vibaoc/releases),
and installs the `vibaoc` binary plus its runtime package to a directory on
your `PATH` — preferring `/usr/local/bin`, `~/.local/bin`, or Termux's
`$PREFIX/bin` on Android, in that order. If the chosen directory isn't
already on `PATH`, the installer adds it to your shell profile for you.

Verify it worked:

```bash
vibaoc --version
```

## Manual install

Prebuilt archives are published on the
[Releases page](https://github.com/giabaovu375-alt/Vibaoc/releases). Each
`v0.1.0` release currently includes:

| Archive | Platform |
|---|---|
| `vibao-0.1.0-linux-x86_64.tar.gz` | Linux, x86_64 |
| `vibao-0.1.0-linux-arm64.tar.gz` | Linux / Android (Termux), arm64 |
| `vibao-0.1.0-macos-x86_64.tar.gz` | macOS, Intel |
| `vibao-0.1.0-macos-arm64.tar.gz` | macOS, Apple Silicon |
| `vibao-0.1.0-windows-x86_64.tar.gz` | Windows, x86_64 |

Each archive contains:

```text
vibaoc                  # the compiler binary
pkg/
├── vibao_runtime.js     # the browser runtime (WASM glue)
└── vibao_runtime_bg.wasm
README.txt
```

Steps:

1. Download the archive matching your platform.
2. Extract it: `tar -xzf vibao-0.1.0-<platform>.tar.gz`
3. Place `vibaoc` on your `PATH`, and keep the `pkg/` folder next to it —
   `vibaoc build` looks for `pkg/` beside its own binary at runtime.

### Android (Termux)

Use the Linux arm64 archive inside [Termux](https://termux.dev/). The
one-command installer above already detects Termux's `$PREFIX/bin` and
installs there automatically, so no manual `PATH` editing is needed.

### Windows

Extract the archive and either add the folder to your `PATH` manually, or
run `vibaoc.exe` with a full path. Windows packaging is newer than the Unix
targets, so please report anything unexpected via
[Issues](https://github.com/giabaovu375-alt/Vibaoc/issues).

## Building from source

If you want to build ViBao itself — to contribute, or to use an unreleased
change — you'll need:

- Rust and Cargo
- The `wasm32-unknown-unknown` target (only needed for the browser runtime)
- `wasm-bindgen-cli` — `scripts/build-runtime.sh` installs the version
  pinned in `Cargo.lock` automatically if it's missing

```bash
git clone https://github.com/giabaovu375-alt/Vibaoc.git
cd Vibaoc

cargo build --workspace          # build the compiler
cargo test --workspace           # run the test suite
bash scripts/build-runtime.sh    # build the WASM runtime into vibao-runtime/pkg/
```

Once the runtime is built, `cargo run -p vibaoc -- build app.vbao` works
directly from the source tree — the compiler discovers `vibao-runtime/pkg/`
automatically. `VIBAO_PKG_DIR` can override this path if needed.

To build a distributable release archive for your current platform:

```bash
bash scripts/build-release.sh 0.1.0
```

See [`scripts/RELEASE.md`](../scripts/RELEASE.md) for the full release
process and [`CONTRIBUTING.md`](../CONTRIBUTING.md) for the project layout.

## Next steps

Once `vibaoc` is on your `PATH`:

```bash
vibaoc app.vbao
# or, explicitly:
vibaoc build app.vbao --out dist
```

Open `dist/index.html` in a browser. See [`docs/SYNTAX_EN.md`](SYNTAX_EN.md)
or [`docs/SYNTAX_VI.md`](SYNTAX_VI.md) to start writing `.vbao` files.
