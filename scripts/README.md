# Tooling

- `Justfile` – common tasks: `just lint`, `just test`, `just web-build`, `just desktop-build`, `just bootstrap` (Unix) or `just bootstrap-windows`.
- `scripts/check.ps1` – runs `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`; with `-Frontend` also runs `npm ci` + `npm run build` in `packages/frontend`.
- `scripts/bootstrap.sh` / `scripts/bootstrap.ps1` – one-shot setup + build: installs deps (npm), fetches cargo crates, builds the frontend, and runs `cargo tauri build`.

One-command bootstrap from GitHub (default branch = master; requires public access or a tokenized URL):
- Windows (PowerShell 5+): `powershell -NoLogo -NoProfile -ExecutionPolicy Bypass -Command "iwr https://raw.githubusercontent.com/insomniac807/NOTES_APP/master/scripts/bootstrap.ps1 -UseBasicParsing | iex"`
- macOS/Linux (bash): `curl -sSf https://raw.githubusercontent.com/insomniac807/NOTES_APP/master/scripts/bootstrap.sh | bash`

If the repository is private or the files are not pushed yet, clone the repo first and run locally:
- Windows: `powershell -NoLogo -NoProfile -ExecutionPolicy Bypass -File scripts/bootstrap.ps1`
- macOS/Linux: `bash scripts/bootstrap.sh`

CI: `.github/workflows/ci.yml` runs lint/test on Linux/macOS/Windows and a Ubuntu Tauri build that uploads bundles.
