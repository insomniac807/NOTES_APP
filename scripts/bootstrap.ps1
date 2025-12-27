param(
    [switch]$SkipFrontend,
    [string]$RepoUrl = "https://github.com/insomniac807/NOTES_APP.git",
    [string]$Branch = "master",
    [string]$CloneDir = ""
)

$ErrorActionPreference = "Stop"
$originalLocation = Get-Location

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

# Detect repo root or clone if missing
$repoRoot = $null
$scriptRoot = Split-Path -Parent $MyInvocation.MyCommand.Definition
$candidate = Join-Path $scriptRoot ".."
if (Test-Path (Join-Path $candidate "Cargo.toml")) {
    $repoRoot = Resolve-Path $candidate
} elseif (Test-Path (Join-Path (Get-Location) "Cargo.toml")) {
    $repoRoot = Get-Location
} else {
    Require-Command git
    if ([string]::IsNullOrWhiteSpace($CloneDir)) {
        $CloneDir = Join-Path (Get-Location) "NOTES_APP"
    }
    if (-not (Test-Path $CloneDir)) {
        Write-Host "Cloning $RepoUrl (branch $Branch) to $CloneDir ..." -ForegroundColor Cyan
        git clone --branch $Branch $RepoUrl $CloneDir
    } else {
        Write-Host "Using existing directory at $CloneDir" -ForegroundColor Yellow
    }
    $repoRoot = Resolve-Path $CloneDir
}

Set-Location $repoRoot

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
Push-Location (Join-Path $repoRoot "apps/desktop/src-tauri")
cargo tauri build
Pop-Location

$releaseDir = Join-Path $repoRoot "apps/desktop/src-tauri/target/release"
Write-Host "Done. Binaries are under: $releaseDir" -ForegroundColor Green

Set-Location $originalLocation
