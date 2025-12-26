# Architecture

This is a cross-platform notes application built as a Tauri desktop shell (Rust backend + Vite/React frontend) with a file-based vault, a SQLite index, an operation log, and a peer-to-peer sync layer. Below is a high-level map of the system and how the pieces fit together.

## Overview
- **Desktop shell (`apps/desktop/src-tauri`)**: Exposes Tauri commands for CRUD, search, sync, trust management, plugin loading, and configuration. Hosts background discovery/sync workers.
- **Frontend (`packages/frontend`)**: Vite/React UI for listing/searching/editing notes, peer discovery/trust/sync controls, plugin loader, and markdown preview.
- **Store (`crates/store`)**: Vault manager that persists markdown files with YAML frontmatter, maintains a SQLite index for listing/search, and applies ops with conflict/hash checks.
- **Op-log (`crates/oplog`)**: Defines operation schema and hashing helpers used for deduplication, signing, and sync payloads.
- **Sync (`crates/sync`)**: Device identity, trust store, UDP discovery, TCP sync with signed envelopes, optional PSK transport encryption, and APIs for advertise/request/serve.
- **Plugin host (`crates/plugin-host`)**: Validates plugin manifests, registers plugins/commands, and stores plugin bytes (WASM placeholder).
- **Core (`crates/core`)**: Markdown document model + frontmatter parsing/serialization.

## Data model
- **Documents**: Markdown bodies with YAML frontmatter (id, type, title, timestamps, tags, links). Stored under the vault root as `<id>.md`. Hash of content used for conflict detection and sync validation.
- **Index**: SQLite `documents` table (id, doc_type, updated, title, tags JSON) for list/search (LIKE on title/tags/id).
- **Operations** (`crates/oplog`):
  - Types: create, update, delete (attach/detach reserved).
  - Fields: `op_id`, `device_id`, `timestamp`, `op_type`, `document_id`, `payload` (frontmatter+body), `before_hash`, `after_hash`.
  - Hash/digest helpers for dedup/signing.
- **Op-log store**: JSON file (`oplog.json`) persisted in app data dir, with in-memory `seen` set for dedup.

## Backend flows (desktop)
- **Create/Update/Delete** (Tauri commands):
  - Build an `Operation` (before/after hashes when available), apply via `Store`, append to op-log, persist.
- **List/Search/Get**: Use `Store` to read from disk/SQLite.
- **Config**: `config.json` in app data dir; fields for vault root, ports, auto-sync flag, optional `transport_secret` (PSK).
- **Device identity**: `device.json` in app data dir with ULID, ed25519 public/secret. Auto-heals missing keys.
- **Trust store**: `trust.json` in app data dir; each trusted device stores id, public key, added timestamp, and `allow_auto_sync` flag.
- **Sync**:
  - **Discovery**: UDP broadcast; optional identity packet. Configurable `discovery_port`.
  - **Advertise loop**: Periodic broadcast in background.
  - **Sync listener**: TCP on `sync_port`; decrypts (if PSK), verifies trust, verifies ed25519 signature over ops, applies new ops to store, merges into op-log.
  - **Sync send**: `sync_now` builds signed envelope from current op-log and sends to a target address.
  - **Auto-sync**: Periodic discover + push ops to peers that are both trusted and marked `allow_auto_sync`, gated by global `auto_sync_enabled`.
  - **Crypto**: Ed25519 signatures over serialized ops. Optional PSK (ChaCha20-Poly1305) for envelope confidentiality/integrity.

## Frontend flows (Vite/React)
- **Docs**: List/search, edit, save; markdown preview; empty states and keyboard shortcuts (Ctrl/Cmd+N, Ctrl/Cmd+S).
- **Sync/Trust**: Discover peers, manual trust entry, per-device auto-sync toggle, global auto-sync toggle, show device id/public key with copy buttons, configurable ports/PSK (saved config; restart recommended).
- **Plugins**: Load manifest path (host-side validation/registration).
- **Status**: Health check, sync status messages, peer discovery messages.

## Error handling & validation
- Store enforces `before_hash` on update/create when provided; writes conflict copies; validates `after_hash` to detect tampering.
- Sync listener rejects untrusted devices, bad signatures, or bad PSK packets. Auto-sync only to trusted+allowed peers.
- Op-log deduplicates by `(device_id, op_id)` and only merges applied ops.

## Paths & persistence
- Config: `%APPDATA%/notes-desktop/config.json` (platform-appropriate via `dirs`).
- Device: `%APPDATA%/notes-desktop/device.json`
- Trust: `%APPDATA%/notes-desktop/trust.json`
- Op-log: `%APPDATA%/notes-desktop/oplog.json`
- Vault default: `%APPDATA%/notes-desktop/vault` (configurable).
- Index: `<vault>/index.db`
- Docs: `<vault>/<id>.md`

## Security considerations
- Identity: ed25519 keys per device; signatures on sync envelopes.
- Trust: explicit trust store; optional per-device auto-sync approval; optional PSK transport encryption (shared secret must match across peers).
- Integrity: `after_hash` validation prevents applying tampered payloads.
- Confidentiality: No E2E by default; set `transport_secret` for encrypted envelopes.

## Extensibility roadmap
- Richer markdown render and plugin execution (WASM sandbox).
- Conflict resolution UI and sync event log.
- Hot-reload network settings (ports/PSK) without restart.
- Migration to Noise/TLS for authenticated transport and key exchange.

## Module map
- `apps/desktop/src-tauri`: Tauri main, commands, background workers, config/device/trust/oplog management.
- `packages/frontend`: React UI (documents, sync/trust controls, plugins, settings).
- `crates/store`: Vault + SQLite index + hash/conflict-aware apply/update.
- `crates/oplog`: Operation schema and hashing.
- `crates/sync`: Discovery/sync transport, trust store, device identity, optional PSK crypto.
- `crates/plugin-host`: Manifest validation and plugin/command registry.
- `crates/core`: Document/frontmatter types and markdown parsing/serialization.
