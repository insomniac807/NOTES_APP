param(
    [switch]$Frontend
)

Write-Host "Running Rust format + clippy + tests" -ForegroundColor Cyan
cargo fmt --all -- --check
cargo clippy --all-targets --workspace -- -D warnings
cargo test --workspace

if ($Frontend) {
    Write-Host "Installing frontend deps + building" -ForegroundColor Cyan
    npm --prefix packages/frontend ci
    npm --prefix packages/frontend run build
}
