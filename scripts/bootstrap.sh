#!/usr/bin/env bash
set -euo pipefail

REPO_URL="${REPO_URL:-https://github.com/insomniac807/NOTES_APP.git}"
BRANCH="${BRANCH:-master}"
SKIP_FRONTEND=0
CLONE_DIR="${CLONE_DIR:-}"

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

# Detect repo root or clone if missing
SCRIPT_ROOT="$(cd "$(dirname "$0")" && pwd)"
CANDIDATE_ROOT="$(cd "$SCRIPT_ROOT/.." && pwd)"
if [ -f "$CANDIDATE_ROOT/Cargo.toml" ]; then
  ROOT="$CANDIDATE_ROOT"
elif [ -f "$PWD/Cargo.toml" ]; then
  ROOT="$PWD"
else
  require_cmd git
  if [ -z "$CLONE_DIR" ]; then
    CLONE_DIR="$PWD/NOTES_APP"
  fi
  if [ ! -d "$CLONE_DIR" ]; then
    echo "Cloning $REPO_URL (branch $BRANCH) to $CLONE_DIR ..."
    git clone --branch "$BRANCH" "$REPO_URL" "$CLONE_DIR"
  else
    echo "Using existing directory at $CLONE_DIR"
  fi
  ROOT="$(cd "$CLONE_DIR" && pwd)"
fi

if ! command -v cargo-tauri >/dev/null 2>&1; then
  echo "cargo-tauri not found; installing tauri-cli (^2)..."
  cargo install tauri-cli --version "^2"
fi

echo "Fetching Rust dependencies..."
(cd "$ROOT" && cargo fetch)

if [ "$SKIP_FRONTEND" -eq 0 ]; then
  echo "Installing frontend dependencies..."
  npm --prefix "$ROOT/packages/frontend" ci
  echo "Building frontend..."
  npm --prefix "$ROOT/packages/frontend" run build
fi

echo "Building desktop app (Tauri)..."
(cd "$ROOT/apps/desktop/src-tauri" && cargo tauri build)

RELEASE_DIR="$ROOT/apps/desktop/src-tauri/target/release"
echo "Done. Binaries are under: $RELEASE_DIR"
