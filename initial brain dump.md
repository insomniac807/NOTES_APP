Skip to content
Chat history



You said:
I want to you take on the role of lead software engineer for my project and respond to all prompts as such
ChatGPT said:
Understood. I’ll take on the role of Lead Software Engineer for your project.

From here on, I’ll respond with that mindset:

System-level thinking (architecture, boundaries, long-term maintainability)

Engineering trade-offs (performance, complexity, extensibility, risk)

Clear technical direction (what to build first, what to defer, why)

Practical implementation guidance (not just theory)

Consistency with your existing goals, especially around complex C++ / VST3-style systems and scalable plugin architectures

I’ll treat your prompts as if:

This is a real project with future users

We care about clean abstractions, testability, and iteration speed

Bad architectural decisions are more expensive than writing extra code now

If at any point you want me to switch modes (e.g. architect only, code reviewer, devil’s advocate, hands-on implementer), just say so.

Whenever you’re ready, tell me what we’re working on next.

You said:
I want an all in one solution for academics and writers. It should take the best features from obsidian and notion note taking application. It should also include the entire feature suite functionality of the readwise software application. the highlighting and annotation features from the readwise part of this software, should include an option in the settings for a default referencing style including templates for every academic referencing style used across the world. the app should contain a built in browser which can be opened in a document tab style window. this browser should be able to load any research paper or article. It should have full compatibility with all major ereader devices for sharing highlights with the software's user account. It should be able to play videos and audiobooks, and auto transcribe them, for highlighting and annotating purposes. The documents for writing should be markdown files. The software should be modular and scaleable, and have a public API catering to plugin developers, with the ability to side load installed plugins. This software should be serverless if possible. And be able to run properly, over a private network between all of the end users devices. With all devices in perfect sync with each other. It should be designed with some potential future updates in mind. One update I would like to integrate into the software, is the ability to listen to audiobooks. and using the most advanced voice recognition technology available, allow the end user to capture highlights from the audio book, by assigning specific commands such as "get paragraph" to highlight the paragraph transcription of the current play head position in the audiobook, and add it annotated as a regular highlight. This project is to remain free and open source
ChatGPT said:
Alright — speaking as Lead Software Engineer, I’m going to do three things:

Translate your vision into concrete, non-hand-wavy requirements

Propose a realistic, scalable architecture that satisfies them

Lay out a phased execution plan that keeps this feasible as a free, open-source project

This is an ambitious but coherent idea. The danger here is not technical impossibility — it’s scope collapse. So we’ll design this like a platform, not a monolith.

1. Problem Definition (What We’re Actually Building)
You are not building “another notes app”.

You are building a personal knowledge operating system for academics and writers, combining:

Obsidian → local-first, Markdown, graph-based thinking

Notion → structured data, databases, rich documents

Readwise → ingestion, highlighting, annotation, spaced resurfacing

Zotero / reference managers → citation rigor

Media consumption tools → PDFs, web, ebooks, audiobooks, video

Future-forward voice & AI interaction

Key non-negotiables:

Free & open source

Modular, plugin-based

Local-first / serverless

Private-network sync

Perfect cross-device consistency

Markdown as the canonical writing format

This means Electron-style CRUD apps are insufficient. We need a local-first distributed system with a clean core and powerful extensions.

2. Core Design Philosophy
2.1 Local-First, Not Cloud-First
No mandatory central server. Sync must work:

Offline

Over LAN

Over encrypted peer-to-peer links

Optionally via third-party storage (WebDAV, S3, etc.)

This strongly suggests:

CRDT-based or op-log-based data model

Deterministic conflict resolution

Immutable history for auditability

Think Git + Obsidian + Readwise, not Google Docs.

2.2 Everything Is a Document (Even Media)
To keep the system sane:

Thing	Representation
Notes	Markdown
Highlights	Markdown + metadata blocks
Annotations	Markdown comments / blocks
References	CSL JSON
Media	Files + time-indexed transcripts
Commands	Events
Markdown stays human-readable, but enriched via structured frontmatter.

2.3 Hard Separation of Core vs Extensions
If we don’t do this, the project dies.

Core responsibilities (minimal):

File system abstraction

Document graph

Sync engine

Plugin host

Security model

Everything else is a plugin.

3. High-Level Architecture
3.1 Application Shell
Cross-platform desktop-first

Windows / macOS / Linux

Mobile later (same data model)

Recommended stack:

Tauri (Rust backend, Web frontend)

Smaller footprint than Electron

Strong security

Rust is ideal for sync + media + parsing

Frontend:

React or Svelte (lean, fast)

Markdown editor with extensibility (ProseMirror / CodeMirror)

3.2 Core Engine (Rust)
This is the heart.

Modules:
Document Store

Markdown files

Indexed metadata (SQLite)

Content-addressed storage (hashes)

Graph Engine

Backlinks

References

Media-to-text links

Citation edges

Sync Engine

CRDT or op-log based (e.g. Automerge-like model)

Transport-agnostic:

LAN multicast

Peer-to-peer

Optional relay

End-to-end encrypted

Plugin Runtime

WASM-based plugins (strong sandboxing)

JS/TS bindings

Capability-based permissions

Media Pipeline

Audio/video decoding

Time-aligned transcription

Segment indexing

4. Feature Breakdown (Mapped to Architecture)
4.1 Obsidian + Notion Hybrid
Markdown as source of truth

Optional “structured views”:

Tables

Kanban

Queries

Views are renderings, not formats

This avoids Notion’s biggest sin: data lock-in.

4.2 Readwise-Level Highlighting
Implement as Highlight Objects:

---
type: highlight
source: pdf
source_id: doi:10.1234/foo
location:
  page: 12
  start: 134
  end: 221
citation: APA
---
Highlighted text here
Works for:

PDFs

EPUBs

Web pages

Audio (via transcript offsets)

Video (timestamped)

Highlights are first-class citizens, not comments.

4.3 Global Citation & Referencing Engine
CSL (Citation Style Language) as backbone

Built-in CSL style registry

Templates per style

Auto-generated references

Inline citation commands

Example:

[@smith2022, p. 45]
Rendered per user’s default style.

4.4 Built-in Research Browser
Not a toy browser.

Requirements:

Tabs as documents

Persistent annotations

Highlight DOM selections

Save snapshots (like Zotero)

This is effectively:

A hardened WebView

With annotation hooks

With offline caching

4.5 eReader Compatibility
Strategy:

Import/export layer

NOT device-specific hacks in core

Support:

Kindle

Kobo

Apple Books

Mechanisms:

Email parsing

File import

Sync via Readwise-style ingestion plugins

4.6 Audio & Video Transcription
Pipeline:

Media ingestion

Local or optional external transcription

Time-aligned text blocks

Highlight ↔ timestamp mapping

This unlocks:

Audiobooks

Lectures

Podcasts

YouTube research

5. Voice Command Highlighting (Future Feature)
This is not fluff — but must be isolated.

Design now, implement later.

Architecture:
Audio playback emits timecode events

Transcript index maps time → paragraph

Voice recognition runs in parallel

Commands emit structured actions

Example:

User says: “Get paragraph”

System does:

Capture current playhead timestamp

Resolve transcript block

Create highlight

Attach annotation metadata

This should live in a Voice Interaction Plugin, not core.

6. Plugin & Public API
This decides whether the project lives or dies.

Plugin Types:
UI plugins

Importers

Exporters

Renderers

Automation

AI tools

Rules:
Plugins run sandboxed

Explicit permissions

Stable API versioning

Side-loading allowed

Think:

VS Code + Obsidian plugin ecosystems, but stricter.

7. Serverless & Private Network Sync
This is the hardest part — but solvable.

Approach:

Local-first datastore

Peer discovery on LAN

Manual pairing for remote devices

Optional third-party relay (user-hosted)

Sync guarantees:

Eventual consistency

Deterministic merges

No silent data loss

8. Phased Development Plan (Critical)
Phase 1 — Core Foundations
Markdown editor

Document store

Graph links

Plugin runtime

Local-only

Phase 2 — Readwise-Class Highlighting
PDFs

Web

Highlight objects

Annotation UI

Phase 3 — Sync & Multi-Device
LAN sync

Conflict resolution

Encryption

Phase 4 — Media & Transcription
Audio/video playback

Transcript indexing

Highlighting from media

Phase 5 — eReader & Browser
Import/export

Built-in browser

Citation engine

Phase 6 — Voice Interaction
Command system

Audio-triggered highlights

9. Reality Check (Lead Engineer Honesty)
This is not a small project.
It is:

Comparable in scope to Obsidian + Zotero + Readwise combined

Only viable if:

Core stays minimal

Community contributes plugins

You resist feature creep in core

If you want, next steps I can:

Define the exact MVP scope

Design the sync model in detail

Propose a repo structure

Act as architect + reviewer going forward

Tell me what you want to lock down next:

MVP definition

Tech stack finalization

Sync design

Plugin API spec

You said:
mvp definition
ChatGPT said:
Good — this is the most important decision we’ll make. As lead engineer, I’m going to be firm and conservative here, because the MVP determines whether this project ships or becomes an eternal design document.

MVP DEFINITION
Goal:
Deliver a usable, opinionated, local-first academic knowledge tool that clearly demonstrates the core value proposition:

“I can read, highlight, annotate, write in Markdown, and keep everything perfectly in sync across my devices — without the cloud.”

Everything else is deferred.

1. What the MVP MUST Prove
The MVP must prove three things unequivocally:

Local-first Markdown knowledge base works

Readwise-class highlighting & annotation works

Multi-device sync over a private network works

If any one of these is missing, the product is not credible.

2. MVP User Persona (We Design for ONE User)
Target user for MVP:

Academic / serious non-fiction writer

Reads PDFs & web articles

Writes long-form Markdown notes

Uses 2 devices (laptop + desktop)

Explicitly NOT in MVP:

Casual note-takers

Mobile-first users

Audiobook-only users

This keeps scope tight.

3. MVP Feature Set (Locked)
3.1 Core Editor (NON-NEGOTIABLE)
Included

Markdown editor

Local folder = vault

Frontmatter support

Wiki-style links [[ ]]

Backlinks panel

Graph view (basic)

Excluded

Tables/databases

Kanban

Rich blocks

AI writing tools

3.2 Highlighting & Annotation (Readwise Core)
Included

Highlight PDFs

Highlight web articles (via built-in browser)

Create annotations on highlights

Highlights stored as Markdown documents

Highlights linked to source documents

Highlight object schema is frozen in MVP

Excluded

Spaced repetition

Daily review resurfacing

Mobile capture

Email ingestion

3.3 Built-in Research Browser (MINIMAL)
Included

Tabbed browser inside app

Load any URL

Text selection → highlight

Persistent highlights per URL

Excluded

Offline web snapshots

Advanced reader mode

Website annotation syncing

3.4 Reference & Citation (MINIMUM VIABLE)
Included

CSL JSON support

One default citation style (APA)

Inline citation syntax

Automatic bibliography generation

Excluded

Full CSL style library

Citation switching

Advanced reference management UI

3.5 Sync (THIS IS THE HARD PART)
Included

Peer-to-peer sync over LAN

Manual device pairing

End-to-end encryption

Deterministic conflict resolution

Guarantee

Two devices editing the same vault converge safely

Excluded

Cloud accounts

Mobile sync

Public sharing

Team collaboration

3.6 Plugin System (BARE MINIMUM)
Included

Plugin loading

Versioned public API

Ability to:

Add UI panels

Register commands

Read/write documents

Excluded

Plugin marketplace

Plugin permissions UI

Hot reloading

4. Explicitly OUT OF MVP (Locked Out)
These are future milestones, not “maybe later”:

Audiobooks

Video playback

Transcription

Voice commands

eReader device sync

Advanced Notion-like databases

AI features

Mobile apps

Cloud sync

Collaboration

Full Readwise review system

If any of these creep in early, the project slows to a crawl.

5. MVP Technical Stack (Locked for MVP)
To avoid bikeshedding:

App shell: Tauri

Core engine: Rust

Editor: CodeMirror / ProseMirror

Data model: Markdown + frontmatter

Index: SQLite

Sync: Op-log or CRDT (minimal feature set)

Plugins: WASM + JS bridge

We can revisit later, but not during MVP.

6. MVP Success Criteria (Binary)
The MVP is DONE when:

A user can:

Open a PDF

Highlight text

Annotate it

See it as Markdown

That same vault:

Syncs to another machine on the LAN

With no data loss

Plugins:

Can add a panel

Can create a document

A 10k-note vault remains fast

Anything else is polish.

7. Estimated MVP Timeline (Honest)
With 1–2 serious contributors:

Core editor & store: 4–6 weeks

Highlighting (PDF + web): 4–6 weeks

Sync (LAN only): 6–8 weeks

Plugin API (minimal): 2–3 weeks

Total: ~4 months of focused work

This is realistic for an open-source v1.

8. Next Engineering Decision (We Must Choose)
Before writing a single line of code, we must lock one of these:

Sync model (CRDT vs op-log)

Document schema (exact Markdown + frontmatter format)

Plugin API surface (v0.1)

As lead engineer, I recommend starting with #2 (document schema) because it becomes irreversible fastest.

👉 Tell me which one you want to lock next, and I’ll design it to production quality.

You said:
2
ChatGPT said:
Good choice. Locking the document schema first is exactly what a disciplined project does.
Once this is set, everything else (sync, plugins, UI) has something solid to stand on.

Below is the v0.1 Canonical Document Schema. This is the contract of the system.

Canonical Document Schema (v0.1)
Core Principles (Non-Negotiable)
Plain Markdown remains readable outside the app

All structure lives in YAML frontmatter

Documents are immutable by identity

Everything links by stable IDs, not filenames

No hidden state

If a user opens these files in Vim in 10 years, they still make sense.

1. Universal Frontmatter (All Documents)
Every document must start with this block:

---
id: 01HV9X6QZ7KJ4R8W6N2A3B5C9D
type: note
created: 2025-03-01T14:22:10Z
updated: 2025-03-01T14:22:10Z
tags: []
links: []
---
Required Fields
Field	Purpose
id	ULID (sortable, globally unique)
type	Drives rendering & behavior
created	Immutable
updated	Updated on write
tags	Simple taxonomy
links	Explicit semantic links
Filenames are presentation only.
IDs are the source of truth.

2. Document Types (v0.1)
Only six document types exist in the MVP.

2.1 note (Default Writing Unit)
---
type: note
---
# Phenomenology and Intentionality

This connects to [[Being and Nothingness]].

See highlight: ![[01HVHIGHLIGHTID]]
Long-form writing

Research synthesis

Drafts

2.2 source (Books, Papers, URLs)
Represents anything you read or watch.

---
id: 01HVSOURCEID
type: source
source_type: pdf
title: Being and Nothingness
authors:
  - Sartre, Jean-Paul
year: 1943
identifiers:
  doi: null
  isbn: "9780671867805"
  url: null
file: files/being_and_nothingness.pdf
---
Allowed source_type:

pdf

web

epub

video

audio

Sources never contain highlights directly — they own them.

2.3 highlight (Core Readwise Equivalent)
This is the most important schema.

---
id: 01HVHIGHLIGHTID
type: highlight
source: 01HVSOURCEID
location:
  page: 123
  start_offset: 456
  end_offset: 789
citation:
  style: apa
  locator: "p. 123"
created: 2025-03-01T15:10:00Z
---
Consciousness is always consciousness *of* something.
Rules

Highlight text lives in the Markdown body

Location is precise and reproducible

Source linkage is mandatory

This same schema works for:

PDFs

Web

Audio (timestamps instead of pages)

Video

2.4 annotation (Commentary on Highlights)
Annotations are separate documents, not inline comments.

---
id: 01HVANNOTATIONID
type: annotation
target: 01HVHIGHLIGHTID
created: 2025-03-01T15:12:30Z
---
This is the core of Sartre’s rejection of Cartesian dualism.
Why separate?

Multiple annotations per highlight

Annotations can be linked, tagged, cited

2.5 reference (Citation Records)
CSL-compatible, Zotero-compatible.

---
id: smith2022
type: reference
csl:
  type: book
  title: Phenomenology Explained
  author:
    - family: Smith
      given: John
  issued:
    year: 2022
---
This enables:

[@smith2022, p. 45]
2.6 system (Internal Use Only)
Used for:

Sync metadata

Plugin state

Migration markers

Users can see them, but shouldn’t edit.

3. Linking Rules (Strict)
3.1 Wiki Links
[[Note Title]]
[[01HVNOTEID]]
Resolution order:

ID

Title

Alias

3.2 Embedded Documents
![[01HVHIGHLIGHTID]]
This allows:

Highlights to appear inside notes

Annotations to be expanded inline

4. Media Location Mapping (Critical for Future)
This is future-proofing without implementing audio yet.

Audio Highlight Example (Future-Compatible)
location:
  start_time_ms: 812345
  end_time_ms: 829120
PDFs use pages now — audio/video uses time later
Same schema. No migration.

5. Why This Schema Will Survive 10 Years
Git-friendly

Human-readable

Diffable

Mergeable

Extensible via frontmatter

Does not depend on UI assumptions

Most importantly:

The schema does not change when features change.

6. What Is Now Locked (Irreversible)
✅ Markdown is canonical
✅ Highlights are documents
✅ Annotations are documents
✅ Sources own highlights
✅ IDs are primary keys

Changing any of these later would require mass migration — so we don’t.

7. Next Decision (Now That Schema Is Locked)
As lead engineer, the next critical choice is:

Sync Model
CRDT (complex, powerful)

Op-log + deterministic merge (simpler, auditable)

This choice directly affects:

File layout

Conflict resolution

Plugin guarantees

👉 Next step:
“Design the sync model” or “Design the plugin API (v0.1)”

You said:
design the sync model
ChatGPT said:
Excellent. This is the hardest technical part of the entire system, so I’m going to be very explicit and conservative. What follows is a production-grade sync model that is realistic for an open-source, local-first system and aligns with the document schema we just locked.

I’m making an executive call here as lead engineer:

We will NOT use full CRDTs for MVP.
We will use an append-only operation log with deterministic merge rules.

I’ll explain why, then define the model precisely.

Sync Model (v0.1)
Executive Summary
Model:
➡️ Local-first, append-only operation log (op-log)
➡️ Deterministic, document-level conflict resolution
➡️ Peer-to-peer LAN sync
➡️ End-to-end encrypted transport

This gives us:

Predictable behavior

Debuggability

Git-like auditability

Plugin safety

A migration path to CRDTs if ever needed

1. Why NOT CRDTs (for MVP)
CRDTs are powerful, but dangerous at this stage:

Problems CRDTs introduce:
Hard-to-debug state

Non-human-readable merges

Tight coupling to data structures

Plugin complexity explosion

Rust + CRDT maturity issues

Your app is:

File-based

Markdown-centric

User-auditable

CRDTs fight those properties.

Instead, we embrace a Git-like mental model, but without Git’s UX pain.

2. Fundamental Sync Guarantees
The sync system must guarantee:

No silent data loss

Eventual consistency

Deterministic merges

Offline-first

Human-inspectable history

We do not guarantee:

Real-time collaborative editing

Paragraph-level merges (yet)

Those are future upgrades.

3. Data Model Overview
3.1 Canonical State
The filesystem + SQLite index is the derived state.

The operation log is the source of truth.

┌─────────────┐
│ Operation   │  ← canonical
│ Log         │
└──────┬──────┘
       ↓
┌─────────────┐
│ Markdown    │
│ Files       │
└─────────────┘
You never sync files directly.
You sync operations.

4. Operation Log (Op-Log)
4.1 Operation Structure
Each operation is immutable.

{
  "op_id": "01HVOPID",
  "device_id": "DEVICE_A",
  "timestamp": "2025-03-01T15:30:12.123Z",
  "type": "update_document",
  "document_id": "01HVNOTEID",
  "payload": {
    "before_hash": "abc123",
    "after_hash": "def456"
  }
}
Required Properties
Field	Purpose
op_id	ULID
device_id	Stable per device
timestamp	Wall-clock (used only for ordering hints)
type	Operation category
document_id	Stable ID
payload	Typed operation data
4.2 Supported Operation Types (MVP)
Only five operation types exist in v0.1:

create_document

update_document

delete_document

attach_file

detach_file

No partial edits.
No inline deltas.
Whole-document updates only.

This is intentional.

5. Deterministic Merge Strategy
5.1 Conflict Detection
A conflict exists only if:

Two update_document ops

Same document_id

Same before_hash

Different after_hash

This is simple and reliable.

5.2 Conflict Resolution Rule (Locked)
Documents never auto-merge at text level in MVP.

Instead:

Both versions are preserved

One becomes the canonical version

The other becomes a conflict document

Example
note.md
note (conflict DEVICE_B 2025-03-01).md
The conflict document:

Has full frontmatter

Contains metadata about the conflict

Is linked automatically

This is exactly how Git behaves — but surfaced cleanly in UI.

5.3 Why This Is the Right Call
Academics prefer safety over magic

Markdown merges are ambiguous

Plugin developers get deterministic guarantees

We can add smarter merging later

6. File Hashing & Integrity
Each document has:

content_hash: sha256(...)
Used to:

Detect corruption

Detect no-op updates

Validate sync correctness

7. Device Identity
Each device generates once:

{
  "device_id": "ULID",
  "public_key": "...",
  "created": "timestamp"
}
Stored locally

Used for encryption + attribution

Never changes

8. Peer-to-Peer Sync Protocol (LAN)
8.1 Discovery
mDNS / UDP broadcast

Devices announce:

Device ID

Vault ID

Protocol version

8.2 Pairing
Manual trust step:

QR code or shared secret

Exchange public keys

Store trust record

No silent pairing. Ever.

8.3 Sync Exchange
Devices exchange:

Latest op_id per device

Each sends missing ops

Receiver:

Verifies signatures

Applies ops in order

Rebuilds derived state

This is eventual consistency, not real-time.

9. Encryption Model
All ops encrypted in transit

Per-vault symmetric key

Keys exchanged via device trust

Optional at-rest encryption later

10. Failure Modes (Designed, Not Ignored)
Failure	Behavior
Clock skew	ULIDs still order
Partial sync	Resume later
Corrupt op	Rejected
Conflicting edits	Conflict document
Device lost	Vault remains intact
11. Why This Sync Model Scales
Works over LAN

Works over USB

Works over future relays

Works with 2 devices or 20

Works without internet

And most importantly:

Users can inspect and understand what happened.

12. What Is Locked Now
✅ Op-log is canonical
✅ Whole-document ops only
✅ Deterministic conflict preservation
✅ No magic merges
✅ LAN-first P2P

Changing these later is possible, but painful — so we won’t casually.

13. Next Critical Design Decision
Now that:

Document schema is locked

Sync semantics are locked

The next domino is:

Plugin API v0.1
Because plugins must know:

What guarantees sync gives

What operations they’re allowed to emit

What they must never do

👉 Say “plugin API” and I’ll design it as if third-party developers will depend on it for years.

You said:
plugin API
ChatGPT said:
Perfect. This is the third and final foundational pillar.
If we get this right, the project gains leverage through community effort.
If we get it wrong, the core team becomes a bottleneck forever.

I’m going to design Plugin API v0.1 with these constraints:

Safe by default

Deterministic with sync

Stable for years

Powerful enough to build Readwise/Zotero-class features

Boring in the core, flexible at the edges

Plugin API v0.1 — Canonical Design
1. Plugin Philosophy (Non-Negotiable)
Plugins extend behavior, not data guarantees

Plugins never bypass the op-log

Plugins are sandboxed

Plugins declare capabilities explicitly

Core never depends on plugins

If a plugin misbehaves, the vault must remain valid and syncable.

2. Plugin Runtime Model
2.1 Execution Environment
Primary target:

WASM (WebAssembly)

Supported languages (initial):

TypeScript → WASM

Rust → WASM

Why WASM

Sandboxing

Deterministic execution

Cross-platform

Future mobile compatibility

No native DLLs. No arbitrary filesystem access.

2.2 Plugin Package Structure
my-plugin/
├─ manifest.json
├─ plugin.wasm
├─ README.md
└─ assets/
3. Plugin Manifest (Locked Schema)
{
  "id": "com.example.pdf-importer",
  "name": "PDF Importer",
  "version": "0.1.0",
  "api_version": "0.1",
  "description": "Imports and annotates PDFs",
  "author": "Jane Doe",
  "permissions": [
    "documents:read",
    "documents:write",
    "ui:panel",
    "commands:register"
  ],
  "entrypoint": "plugin.wasm"
}
Required Fields
Field	Purpose
id	Reverse-DNS, immutable
api_version	Enforced compatibility
permissions	Capability-based security
Plugins fail to load if permissions are invalid or unsupported.

4. Permission Model (Critical)
Plugins get nothing by default.

4.1 Available Permissions (v0.1)
Documents
documents:read

documents:write

documents:create

documents:delete

UI
ui:panel

ui:toolbar

ui:context-menu

Commands
commands:register

Events
events:subscribe

Media (future-safe)
media:read

media:annotate

No wildcard permissions.

5. Core Plugin API Surface (v0.1)
All APIs are async, pure, and side-effect constrained.

5.1 Lifecycle Hooks
onLoad(): void
onUnload(): void
No background threads.
No long-running loops.

5.2 Document API
getDocument(id: string): Document
listDocuments(filter?): DocumentSummary[]
createDocument(doc: NewDocument): string
updateDocument(id: string, content: string): void
deleteDocument(id: string): void
Important:

updateDocument emits an update_document op

Plugins never touch files directly

Core validates schema before accepting writes

5.3 Highlight & Annotation Helpers
createHighlight(data: HighlightInput): string
createAnnotation(data: AnnotationInput): string
Plugins do not invent schemas — they use canonical ones.

6. Command System (Extremely Important)
Commands are the spine of extensibility.

registerCommand({
  id: "pdf.import",
  title: "Import PDF",
  handler: () => {}
})
Commands can be triggered by:

Keyboard shortcuts

UI

Other plugins

Automation later

This is how:

Readwise ingestion works

Zotero imports work

Voice commands work later

7. UI Extension Model (Minimal but Sufficient)
7.1 Panels
registerPanel({
  id: "my-plugin.panel",
  title: "My Panel",
  render: () => HTMLElement
})
Panels:

Dockable

Persisted

Isolated DOM

No arbitrary DOM access.

7.2 Context Menu Extensions
registerContextMenuItem({
  target: "highlight",
  title: "Send to Summary Note",
  command: "summary.create"
})
8. Event System
Read-only, fire-and-forget.

subscribe("document:updated", (event) => {})
Available events:

document:created

document:updated

document:deleted

highlight:created

sync:completed

No event can mutate state directly.

9. Sync & Plugin Interaction (Hard Rule)
Plugins NEVER emit sync ops directly.

All state changes go through:

createDocument

updateDocument

etc.

This ensures:

Deterministic sync

Auditable history

Plugin safety

If a plugin crashes mid-operation, no partial state exists.

10. Plugin Versioning & Stability
10.1 API Versioning
Core exposes api_version

Plugin declares required version

Breaking changes → new major version

We freeze v0.1 once published.

11. What Plugins Can Build (Even in v0.1)
This API already enables:

Readwise importer

Zotero importer

PDF highlighter

Web clippers

Custom citation renderers

Audio transcription (future)

Voice command systems (future)

All without touching core.

12. Explicitly Forbidden (v0.1)
Plugins CANNOT:

Access raw filesystem

Modify op-log

Inject network traffic

Spawn threads

Persist hidden state

This keeps the system trustworthy.

13. What Is Now Locked
✅ WASM plugins
✅ Capability-based permissions
✅ Command-first extensibility
✅ No direct file or sync access
✅ Stable document guarantees

This is a serious, long-term plugin platform, not a hack.

14. Final MVP Foundations Status
At this point, we have locked:

✅ Document schema

✅ Sync model

✅ Plugin API

That means we can now start implementation without fear of rewrites.

Next Lead-Engineer Question (Critical)
Now we move from design to execution. The next decision determines developer velocity:

Choose ONE:
Repository & module structure

Core Rust engine internals

Editor & UI architecture

PDF highlighting pipeline

Tell me which one to design next, and I’ll treat it as production code, not a sketch.



No file chosenNo file chosen
ChatGPT can make mistakes. Check important info. See .
