#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RUNTIME_DIR="$ROOT_DIR/vibao-runtime"
PKG_DIR="$RUNTIME_DIR/pkg"
DEBUG_PKG_DIR="$ROOT_DIR/target/debug/pkg"
TARGET="wasm32-unknown-unknown"

info() { printf '==> %s\n' "$1"; }
error() { printf 'error: %s\n' "$1" >&2; exit 1; }

command -v cargo >/dev/null 2>&1 || error "cargo is required to build the ViBao runtime from source. Install Rust only for development; release users do not need it."
command -v rustc >/dev/null 2>&1 || error "rustc is required to build the ViBao runtime from source."

if ! rustc --print target-list | grep -qx "$TARGET"; then
  error "this Rust installation does not know target '$TARGET'. Install a Rust toolchain that provides it, then run this script again."
fi

WASM_BINDGEN_VERSION="$(awk '/name = "wasm-bindgen"/{found=1; next} found && /^version = /{gsub(/\"/, "", $3); print $3; exit}' "$ROOT_DIR/Cargo.lock")"
[ -n "$WASM_BINDGEN_VERSION" ] || error "could not determine the wasm-bindgen version from Cargo.lock"

WASM_BINDGEN=""
if command -v wasm-bindgen >/dev/null 2>&1; then
  WASM_BINDGEN="$(command -v wasm-bindgen)"
elif [ -x "$HOME/.cargo/bin/wasm-bindgen" ]; then
  WASM_BINDGEN="$HOME/.cargo/bin/wasm-bindgen"
fi

if [ -z "$WASM_BINDGEN" ] || ! "$WASM_BINDGEN" --version | grep -q "wasm-bindgen $WASM_BINDGEN_VERSION"; then
  info "Installing wasm-bindgen-cli $WASM_BINDGEN_VERSION"
  cargo install wasm-bindgen-cli --version "$WASM_BINDGEN_VERSION" --locked
  WASM_BINDGEN="$HOME/.cargo/bin/wasm-bindgen"
fi

info "Building vibao-runtime for $TARGET"
cd "$ROOT_DIR"
cargo build --release -p vibao-runtime --target "$TARGET"

WASM_FILE="$ROOT_DIR/target/$TARGET/release/vibao_runtime.wasm"
[ -f "$WASM_FILE" ] || error "runtime build completed but '$WASM_FILE' was not produced"

rm -rf "$PKG_DIR"
mkdir -p "$PKG_DIR"

info "Generating browser bindings"
"$WASM_BINDGEN" "$WASM_FILE" --target web --out-dir "$PKG_DIR"

rm -f "$PKG_DIR/vibao_runtime.d.ts" "$PKG_DIR/vibao_runtime_bg.wasm.d.ts"

rm -rf "$DEBUG_PKG_DIR"
mkdir -p "$DEBUG_PKG_DIR"
cp "$PKG_DIR/vibao_runtime.js" "$PKG_DIR/vibao_runtime_bg.wasm" "$DEBUG_PKG_DIR/"

info "Runtime package ready: $PKG_DIR"
info "Debug runtime package ready: $DEBUG_PKG_DIR"
