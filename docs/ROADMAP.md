# Roadmap

This document tracks near-term and mid-term goals for the notes app across reliability, security, UX, and extensibility.

## Phase 1: Harden the foundation (immediate)
- **Sync robustness**: retries/backoff, per-peer status, richer error taxonomy (trust/signature/PSK/hash/connect), conflict resolution UI, auto-refreshing sync log.
- **Security**: add Noise/TLS transport option alongside PSK; tighten validation; plan for E2E later.
- **Hot-reload**: fully reload discovery/listener/auto-sync when ports/PSK change, with user feedback.
- **Plugin runtime MVP**: sandboxed WASM with capability-scoped APIs (docs read/write, panels, commands); plugin listing/management; sample plugin.
- **Editor base**: richer preview (links/code/italics/bold), command palette, and shortcut polish.

## Phase 2: Feature parity (near-term)
- **Backlinks/graph/block model**: add backlinks, link graph, block refs, and richer editing to approach Obsidian/Notion experience; tag/type filters and fuzzy search.
- **Conflict UX**: merge/accept UI for conflicts; show conflict copies.
- **Retrying sync**: batch/missing-op sync (by hash/window) and backoff strategy.
- **Plugin ecosystem**: capability APIs, signed manifests, local cache/update checks.

## Phase 3: Readwise-style ingestion & citations
- **Built-in reader**: webview/PDF tabs with highlighting/annotations; export highlights to Markdown notes.
- **Citation templates**: settings for default referencing style; templates for major academic styles.
- **Readwise-like import/export**: e-reader/device highlight import/export hooks.

## Phase 4: Media & voice
- **Media ingestion**: video/audio playback with transcription pipeline; highlight/annotate transcripts.
- **Voice commands**: map voice triggers (e.g., “get paragraph”) to capture current audiobook transcript segments as highlights.

## Phase 5: UX polish & theming
- **Theming/layout**: light/dark, compact/dense modes; smoother list/preview; keyboard-driven flows (jump-to-note, quick create).
- **Search/graph**: advanced filters, link graph preview, link autocomplete.

## Testing & ops (ongoing)
- E2E tests (playwright/tauri-driver) for CRUD/sync/trust.
- Fuzzing for parsers/sync envelopes.
- Optional local logging/telemetry (opt-in) for sync/discovery/trust.
