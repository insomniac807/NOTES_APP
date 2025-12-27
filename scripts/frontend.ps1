param(
    [ValidateSet("dev", "build")]
    [string]$Mode = "dev"
)

$ErrorActionPreference = "Stop"
$root = Resolve-Path (Join-Path $PSScriptRoot "..")
$frontendDir = Join-Path $root "packages/frontend"

function Require-Command {
    param(
        [string]$Name,
        [string]$Hint
    )
    if (-not (Get-Command $Name -ErrorAction SilentlyContinue)) {
        throw "Missing required command '$Name'. $Hint"
    }
}

Require-Command "npm" "Install Node.js 18+ from https://nodejs.org/."

Push-Location $frontendDir
try {
    Write-Host "Installing frontend dependencies..." -ForegroundColor Cyan
    npm install
    switch ($Mode.ToLower()) {
        "dev" {
            Write-Host "Starting Vite dev server..." -ForegroundColor Cyan
            npm run dev -- --host
        }
        "build" {
            Write-Host "Building frontend..." -ForegroundColor Cyan
            npm run build
        }
    }
}
finally {
    Pop-Location
}
