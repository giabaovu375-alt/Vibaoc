#!/usr/bin/env bash
set -euo pipefail

REPO="giabaovu375-alt/Vibaoc"
REQUESTED_VERSION="${1:-latest}"

info()  { printf '==> %s\n' "$1"; }
error() { printf 'error: %s\n' "$1" >&2; exit 1; }

OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
ARCH="$(uname -m)"
case "$OS" in
  linux) OS_NAME="linux" ;;
  darwin) OS_NAME="macos" ;;
  *) error "unsupported operating system '$OS'" ;;
esac
case "$ARCH" in
  x86_64|amd64) ARCH_NAME="x86_64" ;;
  arm64|aarch64) ARCH_NAME="arm64" ;;
  *) error "unsupported CPU architecture '$ARCH'" ;;
esac
TARGET_TRIPLE="${OS_NAME}-${ARCH_NAME}"

command -v curl >/dev/null 2>&1 || error "curl is required by the installer"
command -v tar >/dev/null 2>&1 || error "tar is required by the installer"

if [ "$REQUESTED_VERSION" = "latest" ]; then
  API_URL="https://api.github.com/repos/${REPO}/releases/latest"
else
  API_URL="https://api.github.com/repos/${REPO}/releases/tags/${REQUESTED_VERSION}"
fi

info "Looking for ViBao ${REQUESTED_VERSION} (${TARGET_TRIPLE})"
ASSET_URL="$(curl -fsSL "$API_URL" \
  | grep 'browser_download_url' \
  | grep "${TARGET_TRIPLE}\.tar\.gz" \
  | grep -o 'https://[^\"]*' \
  | head -n1)"
[ -n "$ASSET_URL" ] || error "no release asset was found for ${TARGET_TRIPLE}"

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

info "Downloading ViBao"
curl -fsSL "$ASSET_URL" -o "$TMP_DIR/vibao.tar.gz"
tar -xzf "$TMP_DIR/vibao.tar.gz" -C "$TMP_DIR"
EXTRACTED_DIR="$TMP_DIR"
[ -f "$EXTRACTED_DIR/vibaoc" ] || error "release archive has an unexpected layout (vibaoc binary not found)"

INSTALL_DIR=""
if [ -n "${VIBAO_INSTALL_DIR:-}" ]; then
  INSTALL_DIR="$VIBAO_INSTALL_DIR"
elif [ -n "${PREFIX:-}" ] && [ -d "$PREFIX/bin" ] && [ -w "$PREFIX/bin" ]; then
  INSTALL_DIR="$PREFIX/bin"
elif [ -d /usr/local/bin ] && [ -w /usr/local/bin ]; then
  INSTALL_DIR="/usr/local/bin"
elif [ -d "$HOME/.local/bin" ] && printf '%s' ":$PATH:" | grep -q ":$HOME/.local/bin:"; then
  INSTALL_DIR="$HOME/.local/bin"
elif [ -d "$HOME/bin" ] && printf '%s' ":$PATH:" | grep -q ":$HOME/bin:"; then
  INSTALL_DIR="$HOME/bin"
elif command -v sudo >/dev/null 2>&1 && [ -d /usr/local/bin ]; then
  INSTALL_DIR="/usr/local/bin"
else
  INSTALL_DIR="$HOME/.local/bin"
fi

if [ "$INSTALL_DIR" = "/usr/local/bin" ] && [ ! -w "$INSTALL_DIR" ]; then
  command -v sudo >/dev/null 2>&1 || error "cannot write to /usr/local/bin and sudo is unavailable; set VIBAO_INSTALL_DIR to a writable PATH directory"
  SUDO="sudo"
else
  SUDO=""
fi

info "Installing compiler to $INSTALL_DIR"
$SUDO mkdir -p "$INSTALL_DIR"
$SUDO rm -f "$INSTALL_DIR/vibaoc"
$SUDO cp "$EXTRACTED_DIR/vibaoc" "$INSTALL_DIR/vibaoc"
$SUDO rm -rf "$INSTALL_DIR/pkg"
$SUDO mkdir -p "$INSTALL_DIR/pkg"
$SUDO cp "$EXTRACTED_DIR/pkg/vibao_runtime.js" "$INSTALL_DIR/pkg/"
$SUDO cp "$EXTRACTED_DIR/pkg/vibao_runtime_bg.wasm" "$INSTALL_DIR/pkg/"
$SUDO chmod +x "$INSTALL_DIR/vibaoc"

case ":$PATH:" in
  *":$INSTALL_DIR:"*) ;;
  *)
    if [ "$SUDO" = "" ] && [ "$INSTALL_DIR" = "$HOME/.local/bin" ]; then
      SHELL_RC="$HOME/.profile"
      case "${SHELL:-}" in */zsh) SHELL_RC="$HOME/.zshrc" ;; */bash) SHELL_RC="$HOME/.bashrc" ;; esac
      PATH_LINE='export PATH="$HOME/.local/bin:$PATH"'
      if ! grep -qF "$PATH_LINE" "$SHELL_RC" 2>/dev/null; then
        printf '\n# ViBao\n%s\n' "$PATH_LINE" >> "$SHELL_RC"
      fi
      info "Added $INSTALL_DIR to $SHELL_RC; open a new shell to use vibaoc."
    fi
    ;;
esac

printf '\nViBao installed successfully.\n\n'
printf '  vibaoc --version\n'
printf '  vibaoc build app.vbao\n'
