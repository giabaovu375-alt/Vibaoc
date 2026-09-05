#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

python3 scripts/check-consistency.py

if ! command -v cargo >/dev/null 2>&1; then
  echo "verify: cargo not found; static consistency check passed, Rust checks skipped."
  exit 0
fi

export RUSTFLAGS="${RUSTFLAGS:-} -D warnings"
cargo fmt --all -- --check
cargo test --workspace
cargo build --release

echo "verify: PASS"
