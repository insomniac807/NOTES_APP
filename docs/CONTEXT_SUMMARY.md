# Project Context Summary (for future sessions)

## Stack & Structure
- **Desktop shell**: Tauri (Rust) in `apps/desktop/src-tauri`.
- **Frontend**: Vite/React in `packages/frontend`.
- **Core crates**: `crates/core` (docs/frontmatter), `crates/oplog` (ops schema + hashes), `crates/store` (vault + SQLite index + op apply/conflict/hash checks), `crates/sync` (discovery/sync, trust, device identity, optional PSK crypto), `crates/plugin-host` (manifest validation, plugin registry).
- **Docs**: `docs/ARCHITECTURE.md` (expanded), `docs/ROADMAP.mp` (roadmap), `docs/CONTEXT_SUMMARY.md` (this file).
- **Repo**: Git initialized with remote `origin https://github.com/insomniac807/NOTES_APP`. `.gitignore` added (targets, node_modules, dist, .idea/.vscode, etc.). License: AGPL-3.0 (short file pointing to FSF link).

## Data Model & Files
- Vault stores Markdown + YAML frontmatter (`<id>.md`), hashed for conflicts.
- SQLite index (`index.db`) in vault for list/search (title/tags/id LIKE).
- Op-log (`oplog.json`) persisted in app data dir; dedup by `(device_id, op_id)`.
- Config (`config.json`) fields: `vault_root`, `discovery_port`, `sync_port`, `auto_sync_enabled`, optional `transport_secret` (32-byte hex PSK).
- Device identity (`device.json`): ULID + ed25519 public/secret; auto-heals missing keys.
- Trust store (`trust.json`): device_id, public_key, added, `allow_auto_sync` flag.
- Default paths under `%APPDATA%/notes-desktop` (via `dirs`).

## Sync/Trust/Security
- Discovery: UDP broadcast; optional identity packet. Ports configurable; background advertise loop.
- Sync envelopes: ops + device_id + public_key + ed25519 signature over serialized ops.
- Optional transport encryption: PSK (ChaCha20-Poly1305). When `transport_secret` set, envelopes are encrypted/decrypted with versioned nonce packet.
- Sync listener: decrypt (if PSK), trust-check, signature verify, dedup, apply ops to store, merge into op-log.
- Auto-sync loop: discover peers, sync only if trusted AND `allow_auto_sync` true, gated by global `auto_sync_enabled`.
- Store enforces `before_hash` (conflict -> writes conflict copy) and `after_hash` (HashMismatch) for create/update.

## Frontend (key features)
- Doc list/search/edit, markdown preview with basic styling; empty states; keyboard shortcuts Ctrl/Cmd+N (new), Ctrl/Cmd+S (save).
- Sync/trust UI: discover peers, manual trust, per-device auto-sync toggle, global auto-sync toggle, shows device ID/public key with copy buttons.
- Network settings form: edit discovery/sync ports and PSK (requires restart to fully apply).
- Plugin loader: load manifest path (host validates/registers).
- Status messages for health, sync, peer discovery.

## Backend (Tauri commands & state)
- CRUD: `create_document`, `update_document`, `delete_document`, `get_document`, `list_documents`, `search_documents`.
- Config: `get_vault_root`, `set_vault_root`, `get_device_identity`, `get_network_config`, `set_network_config`, `get_auto_sync_enabled`, `set_auto_sync_enabled`.
- Trust: `list_trusted_devices`, `add_trusted_device`, `remove_trusted_device`, `set_trusted_auto_sync`.
- Sync: `discover_peers` (uses configured ports/PSK), `sync_now` (sends signed/encrypted envelope).
- Background: op-log store (dedup/persist), sync listener (trust + signature + optional PSK), auto-sync loop (trusted+allowed peers), advertise loop.

## Roadmap Highlights (from docs/ROADMAP.mp)
- Near-term: sync error surfacing/log pane, hot-reload network settings, richer markdown rendering, improved trust UX, basic plugin listing/sample.
- Security: optional Noise/TLS, vault encryption, per-op signatures/audit.
- Reliability: conflict resolution UI, batch sync by hash, index hygiene.
- UX: command palette, search filters, theming.
- Plugins: sandboxed WASM with capabilities, distribution/signing, event subscriptions.
- Testing/Ops: E2E, fuzzing parsers, logging/telemetry (opt-in).

## Outstanding considerations
- Transport secret changes currently require restart; hot-reload not implemented.
- Plugin execution is still a stub (manifest/registration only; no WASM runtime).
- No E2E encryption; PSK is optional transport-level confidentiality.
