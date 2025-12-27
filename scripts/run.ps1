param(
    [ValidateSet("dev", "prod", "build")]
    [string]$Mode = "dev"
)

$ErrorActionPreference = "Stop"
$root = Resolve-Path (Join-Path $PSScriptRoot "..")
$desktopDir = Join-Path $root "apps/desktop/src-tauri"
$env:NOTES_APP_ROOT = $root

function Require-Command {
    param(
        [string]$Name,
        [string]$Hint
    )
    if (-not (Get-Command $Name -ErrorAction SilentlyContinue)) {
        throw "Missing required command '$Name'. $Hint"
    }
}

function Ensure-TauriCli {
    $null = & cargo tauri --version 2>$null
    if ($LASTEXITCODE -ne 0) {
        Write-Host "Installing tauri-cli (v2)..." -ForegroundColor Yellow
        cargo install tauri-cli --version "^2" --locked
    }
}

Set-Location $root
Require-Command "cargo" "Install Rust from https://rustup.rs/."
Require-Command "npm" "Install Node.js 18+ from https://nodejs.org/."
Ensure-TauriCli

$normalizedMode = $Mode.ToLower()
if ($normalizedMode -eq "prod") {
    $normalizedMode = "build"
}

Push-Location $desktopDir
try {
    switch ($normalizedMode) {
        "dev" {
            Write-Host "Launching Notes Desktop in dev mode..." -ForegroundColor Cyan
            cargo tauri dev
        }
        "build" {
            Write-Host "Building production bundle..." -ForegroundColor Cyan
            cargo tauri build
        }
    }
}
finally {
    Pop-Location
}
