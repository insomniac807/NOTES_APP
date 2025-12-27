#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SKIP_FRONTEND=0

for arg in "$@"; do
  case "$arg" in
    --skip-frontend) SKIP_FRONTEND=1 ;;
  esac
done

require_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "Required command '$1' not found in PATH." >&2
    exit 1
  fi
}

echo "Notes app bootstrap starting..."
require_cmd cargo
require_cmd npm

if ! command -v cargo-tauri >/dev/null 2>&1; then
  echo "cargo-tauri not found; installing tauri-cli (^2)..."
  cargo install tauri-cli --version "^2"
fi

echo "Fetching Rust dependencies..."
cargo fetch

if [ "$SKIP_FRONTEND" -eq 0 ]; then
  echo "Installing frontend dependencies..."
  npm --prefix "$ROOT/packages/frontend" ci
  echo "Building frontend..."
  npm --prefix "$ROOT/packages/frontend" run build
fi

echo "Building desktop app (Tauri)..."
cargo tauri build --manifest-path "$ROOT/apps/desktop/src-tauri/Cargo.toml"

RELEASE_DIR="$ROOT/apps/desktop/src-tauri/target/release"
echo "Done. Binaries are under: $RELEASE_DIR"
