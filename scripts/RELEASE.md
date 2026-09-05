# ViBao Release Process

Goal: a release user installs one prebuilt package and does not need Rust or WASM tooling.

## Maintainer requirements

A source release builder needs Rust/Cargo, the `wasm32-unknown-unknown` target, and
`wasm-bindgen-cli`. End users need none of these.

## Build a release

```bash
./scripts/build-release.sh 0.1.0
```

The script builds the native compiler, builds the runtime WASM, runs
`wasm-bindgen`, and bundles everything together.

## Release archive

```text
vibao-0.1.0-linux-arm64/
├── vibaoc
├── pkg/
│   ├── vibao_runtime.js
│   └── vibao_runtime_bg.wasm
└── README.txt
```

## One-command install

After publishing the archive, end users install with one command:

```bash
curl -fsSL https://raw.githubusercontent.com/giabaovu375-alt/Vibaoc/main/scripts/install.sh | sh
```

The installer detects Linux/macOS and x64/arm64, chooses a suitable PATH
location (including Termux `$PREFIX/bin`), installs the compiler and browser
runtime together, and updates a user shell profile when necessary. Users do
not install Rust, Cargo, rustup, wasm-pack, wasm-bindgen, or manually copy WASM.

After installation:

```bash
vibaoc build app.vbao
```

## Smoke test a release package

```bash
cd release/stage/vibao-0.1.0-linux-arm64
./vibaoc --version
./vibaoc build ../../../app.vbao --out /tmp/vibao-test
ls /tmp/vibao-test/pkg
```

Expected runtime files are `vibao_runtime.js` and `vibao_runtime_bg.wasm`.

## Publish

Create a GitHub Release with tag `v0.1.0` and upload each target archive.

## Supported targets

`build-release.sh` packages the host OS/CPU. Build separately on each target
that should be published. Windows packaging remains future work.
