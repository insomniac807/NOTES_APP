# Roadmap

This document tracks near-term and mid-term goals for the notes app across reliability, security, UX, and extensibility.

## Near-term (1–2 sprints)
- **Sync correctness**: surface sync/validation errors in the UI; add a sync event log pane; retry/backoff for failed targets.
- **Network settings hot-reload**: restart listeners/transports when ports or PSK change (avoid manual restart).
- **Markdown UX**: richer render (links/code/italics), inline formatting shortcuts, and better conflict indicators.
- **Trust UX**: clearer “trusted + auto-sync” states; show our device ID/PK prominently in settings.
- **Plugin basics**: list loaded plugins, allow unregister/remove, and a sample WASM plugin with a simple command.

## Security & transport
- **Noise/TLS option**: offer authenticated transport (Noise NN/IK or TLS with device certs) as an alternative to PSK.
- **Op verification**: enforce `after_hash` on receive (already validated) plus optional per-op signatures for audit trails.
- **Storage**: optional vault encryption-at-rest; config/trust/oplog sealed with OS keyring or a user passphrase.

## Reliability & scaling
- **Conflict resolution UI**: show conflicts, allow merge/accept; automatic conflict naming already present.
- **Batch sync**: request missing ops by hash/window instead of full-log push.
- **Index hygiene**: vacuum/reindex commands; detect/repair corrupt docs/index.

## UX & productivity
- **Keyboard + commands**: command palette, jump-to-note, quick create, and link auto-complete between notes.
- **Search**: tag/type filters, fuzzy search, and link graph preview.
- **Theming**: light/dark themes and compact/dense list modes.

## Plugins & extensibility
- **Sandboxed WASM runtime**: execute plugins with capability-scoped APIs (docs read/write, UI panels).
- **Plugin distribution**: signed plugin manifests, local cache, and update checks.
- **Events**: emit note lifecycle events to plugins; subscription API in host.

## Testing & ops
- **E2E tests**: playwright/tauri-driver flows for CRUD/sync/trust.
- **Fuzzing**: fuzz frontmatter/parser and sync envelope parsing/decryption.
- **Telemetry/logging**: optional local logs for sync/discovery/trust; user opt-in telemetry plan (if ever enabled).
