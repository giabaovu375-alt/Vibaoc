#!/usr/bin/env bash
set -euo pipefail

VERSION="${1:-0.1.0}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
RELEASE_DIR="$ROOT_DIR/release"
STAGE_DIR="$RELEASE_DIR/stage"

info() { printf '==> %s\n' "$1"; }
error() { printf 'error: %s\n' "$1" >&2; exit 1; }

command -v cargo >/dev/null 2>&1 || error "cargo is required to build a release from source"
command -v rustc >/dev/null 2>&1 || error "rustc is required to build a release from source"

OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
ARCH="$(uname -m)"
case "$OS" in
  linux) OS_NAME="linux" ;;
  darwin) OS_NAME="macos" ;;
  *) error "unsupported operating system '$OS'" ;;
esac
case "$ARCH" in
  x86_64|amd64) ARCH_NAME="x64" ;;
  arm64|aarch64) ARCH_NAME="arm64" ;;
  *) error "unsupported CPU architecture '$ARCH'" ;;
esac

TARGET_TRIPLE="${OS_NAME}-${ARCH_NAME}"
PKG_NAME="vibao-${VERSION}-${TARGET_TRIPLE}"

info "Building ViBao $VERSION for $TARGET_TRIPLE"
cd "$ROOT_DIR"

cargo build --release -p vibaoc
"$ROOT_DIR/scripts/build-runtime.sh"

BIN_NAME="vibaoc"
[ "$OS_NAME" = "windows" ] && BIN_NAME="vibaoc.exe"

rm -rf "$STAGE_DIR"
mkdir -p "$STAGE_DIR/$PKG_NAME/pkg"
cp "target/release/$BIN_NAME" "$STAGE_DIR/$PKG_NAME/"
cp vibao-runtime/pkg/vibao_runtime.js "$STAGE_DIR/$PKG_NAME/pkg/"
cp vibao-runtime/pkg/vibao_runtime_bg.wasm "$STAGE_DIR/$PKG_NAME/pkg/"

cat > "$STAGE_DIR/$PKG_NAME/README.txt" <<EOF2
ViBao Compiler $VERSION ($TARGET_TRIPLE)

Install this package with scripts/install.sh, or put this directory on PATH.
The compiler and browser runtime are bundled together.

Usage:
  vibaoc build app.vbao
  vibaoc check app.vbao
EOF2

mkdir -p "$RELEASE_DIR"
cd "$STAGE_DIR"
tar -czf "$RELEASE_DIR/${PKG_NAME}.tar.gz" -C "$PKG_NAME" .

info "Release archive: $RELEASE_DIR/${PKG_NAME}.tar.gz"
