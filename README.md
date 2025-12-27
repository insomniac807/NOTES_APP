# Notes Desktop

Rust + Tauri desktop app with a React/Vite UI for managing Markdown notes, syncing changes between trusted devices, and experimenting with plugins.

## Prerequisites
- Rust toolchain with `cargo` (install via https://rustup.rs/).
- Node.js 18+ with `npm`.
- Tauri bundler deps: Windows needs Visual Studio Build Tools + WebView2; macOS needs Xcode command line tools; Linux needs GTK/WebKit packages (e.g. `sudo apt install libgtk-3-dev libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev`).

## Clone and run
```powershell
git clone https://github.com/insomniac807/NOTES_APP.git
cd NOTES_APP
pwsh ./scripts/run.ps1 dev
```

## One-command workflows
- Dev/debug: `pwsh ./scripts/run.ps1 dev`  
  Installs `tauri-cli` if missing, then Tauri runs `npm install` + `npm run dev -- --host` for the frontend and launches the desktop shell against `http://localhost:5173`.
- Production bundle: `pwsh ./scripts/run.ps1 prod`  
  Runs `cargo tauri build`; Tauri installs/builds the frontend first and emits installers/binaries under `apps/desktop/src-tauri/target/release/bundle`.

The commands above assume PowerShell (`pwsh` on macOS/Linux, `powershell` on Windows). Run from the repo root.

## What the script/config automate
- `scripts/run.ps1` verifies `cargo`/`npm`, installs `tauri-cli` (`cargo install tauri-cli --version "^2"`), and calls `cargo tauri dev|build` with the desktop manifest.
- `apps/desktop/src-tauri/tauri.conf.json` runs `scripts/frontend.ps1` for dev/build (via `powershell -NoProfile`), which installs frontend deps and starts Vite (dev) or builds the UI (prod) with paths that handle spaces; the hooks `Set-Location $env:NOTES_APP_ROOT` first.
- `scripts/run.ps1` sets `NOTES_APP_ROOT` for the Tauri hooks; if you run `cargo tauri dev` manually, set that env var to the repo root first.

## Project layout
- `apps/desktop/src-tauri` – Tauri shell, commands, config, bundling.
- `packages/frontend` – React/Vite UI.
- `crates/*` – core logic: documents/oplog, storage/index, sync/trust, plugin host.
- `scripts` – automation (`run.ps1` for dev/build, `check.ps1` for fmt/clippy/tests with `-Frontend` to build the UI).

## Troubleshooting
- If `cargo` or `npm` is missing, install the prerequisites above.
- First runs take time while Rust/Node dependencies download.
- Use `pwsh ./scripts/check.ps1 -Frontend` to verify formatter, clippy, tests, and frontend build.
