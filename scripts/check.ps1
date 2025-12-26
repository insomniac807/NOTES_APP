param(
    [switch]$Frontend
)

Write-Host "Running Rust format + clippy + tests" -ForegroundColor Cyan
cargo fmt
cargo clippy --all-targets --workspace -- -D warnings
cargo test --workspace

if ($Frontend) {
    Write-Host "Running frontend lint/build" -ForegroundColor Cyan
    npm --prefix packages/frontend run build
}
