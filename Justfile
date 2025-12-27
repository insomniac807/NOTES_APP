set shell := ["bash", "-c"]
set windows-shell := ["powershell", "-NoLogo", "-NoProfile", "-Command"]

# One-stop bootstrap (Unix)
bootstrap:
    ./scripts/bootstrap.sh

# One-stop bootstrap (Windows)
bootstrap-windows:
    powershell -NoLogo -NoProfile -ExecutionPolicy Bypass -File ./scripts/bootstrap.ps1

lint:
    cargo fmt --all -- --check
    cargo clippy --all-targets --workspace -- -D warnings

test: lint
    cargo test --workspace

web-build:
    npm --prefix packages/frontend ci
    npm --prefix packages/frontend run build

desktop-build: web-build
    cargo tauri build --manifest-path apps/desktop/src-tauri/Cargo.toml

check: lint
    cargo test --workspace
