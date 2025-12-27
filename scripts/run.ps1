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

function Start-Frontend($mode) {
    $frontendScript = Join-Path $root "scripts/frontend.ps1"
    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = "powershell"
    $psi.Arguments = "-NoProfile -ExecutionPolicy Bypass -File `"$frontendScript`" $mode"
    $psi.WorkingDirectory = $root
    $psi.UseShellExecute = $false
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $proc = New-Object System.Diagnostics.Process
    $proc.StartInfo = $psi
    $proc.Start() | Out-Null
    Write-Host "Started frontend (detached) pid=$($proc.Id)" -ForegroundColor Cyan
}

Push-Location $desktopDir
try {
    switch ($normalizedMode) {
        "dev" {
            Write-Host "Launching Notes Desktop in dev mode..." -ForegroundColor Cyan
            Start-Frontend "dev"
            cargo tauri dev
        }
        "build" {
            Write-Host "Building production bundle..." -ForegroundColor Cyan
            Start-Frontend "build"
            cargo tauri build
        }
    }
}
finally {
    Pop-Location
}
