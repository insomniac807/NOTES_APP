param(
    [switch]$SkipFrontend
)

$ErrorActionPreference = "Stop"
$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")

function Require-Command {
    param([string]$Name)
    if (-not (Get-Command $Name -ErrorAction SilentlyContinue)) {
        Write-Error "Required command '$Name' not found in PATH."
        exit 1
    }
}

Write-Host "Notes app bootstrap starting..." -ForegroundColor Cyan
Require-Command cargo
Require-Command npm

if (-not (Get-Command cargo-tauri -ErrorAction SilentlyContinue)) {
    Write-Host "cargo-tauri not found; installing tauri-cli (^2)..." -ForegroundColor Yellow
    cargo install tauri-cli --version "^2"
}

Write-Host "Fetching Rust dependencies..." -ForegroundColor Cyan
cargo fetch

if (-not $SkipFrontend) {
    Write-Host "Installing frontend dependencies..." -ForegroundColor Cyan
    npm --prefix (Join-Path $repoRoot "packages/frontend") ci
    Write-Host "Building frontend..." -ForegroundColor Cyan
    npm --prefix (Join-Path $repoRoot "packages/frontend") run build
}

Write-Host "Building desktop app (Tauri)..." -ForegroundColor Cyan
cargo tauri build --manifest-path (Join-Path $repoRoot "apps/desktop/src-tauri/Cargo.toml")

$releaseDir = Join-Path $repoRoot "apps/desktop/src-tauri/target/release"
Write-Host "Done. Binaries are under: $releaseDir" -ForegroundColor Green
