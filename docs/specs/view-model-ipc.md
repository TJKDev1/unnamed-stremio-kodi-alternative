# Spec: View-models & IPC

**Status:** draft (after S4) · **Implemented by:** S4, M1.1 (T1.1.2, T1.1.8–T1.1.10), M2.5 · **Decisions:** D3, D29, D70, D71, D72

## Purpose
How the UI and the core talk: the UI sends **intents**, the core sends **view-model patches**. The same messages work in-process (Tauri IPC) and over the network (WebSocket, remote core / Local API), so the UI doesn't know which it's using.

## Scope
- **In:** intent format, view-model shapes per screen, patch format, encoding, batching, transports, versioning.
- **Out:** Local API endpoints for third-party automation (may become a separate `local-api` spec).

## Contract
Prototype: [`spikes/s4-ipc`](../../spikes/s4-ipc/README.md). Field-level schemas move to `crates/sdk-schema` in T1.1.2.

### View-models
- One root per screen (home, detail, streams, player, library, search, settings…), defined once in the schema; Rust structs and TS types are generated.
- Every list item that can move has a stable string `id`, unique within its list. Lists without ids are replaced whole when they change.

### Intents
- `{ <IntentName>: { …fields } }`, externally tagged (serde's default), typed per screen in the schema. Example: `{ "MarkWatched": { "card": "tt0111161" } }`.
- Sent as JSON. Fire-and-forget: the effect shows up as a later patch.

### Patches (D70)
A **patch** is the ordered list of ops that turns the view-model the UI has into the core's current one. Ops are **generated per view-model type** from the schema:
- **List edits**, by key: `<List>Insert { …parent indices, index, item }`, `<List>Remove { …, index }`, `<List>Move { …, from, to }`.
- **Field sets**: `<Type><Field> { …indices, value }`, one per changed field (e.g. `CardProgress { row, card, progress }`).
- **Fallbacks**: `<List>` replace-all when ids repeat; a whole-root replace when root-level ids repeat.

Rules:
- Ops are index-addressed and apply **in order**, each to the state the previous op left. The keyed diff emits, per list: removes (highest index first), then for each new position in order either a move into place or an insert, followed by that item's field sets. After an item at index *i* is diffed, later ops only touch indices above *i*.
- **One patch per frame:** the core diffs its current view-model against the last snapshot it sent, at most once per display frame; changes within a frame coalesce.
- A frame with no changes sends nothing.

### Encoding (D71)
- **JSON** (UTF-8), ops externally tagged. MessagePack is not used. Postcard stays a candidate until the C3 measurement in S2 (T0.S2.8).
- Readers ignore unknown fields (additive changes are non-breaking).

### Frames and transports (D72)
- **Frame:** 16-byte header (little-endian u32: sequence number, core diff ns, core encode ns, op count) + JSON payload. The timings feed a debug overlay; the sequence number lets the UI detect gaps.
- **Desktop (embedded core):** Tauri IPC. Intents via `invoke`; frames pushed as raw bytes through a Tauri `Channel` (no JSON wrapping of the bytes).
- **Remote core:** WebSocket, per-device token. Intents and control messages as text, frames as binary messages.

### UI apply rules
- A frame's ops apply inside one store transaction (Solid: one `produce`), so the DOM updates once per frame.
- **Structural list edits go to a plain working copy** of each touched array (holding the same store proxies) and the copy is assigned once at the end of the frame. Field sets go through the store proxies, so only touched items re-render. (Splicing a store array in place notifies every later index: ~10× slower for a full reorder.)

## Behaviour
- Budget: < 2 ms per frame for core diff + encode + transport + decode + apply at typical load. S4 measured 1.4–1.8 ms p95 over Tauri IPC (pull) and 0.7 ms over WebSocket on the owner's PC.
- No JSON round-trip per item: one message per frame.
- Startup: render last-known home snapshot immediately, then hydrate.
- Skins and mods bind to these view-models, so view-models are public API (semver'd).

## Conformance tests
- Patch round-trip, property-tested per view-model: `apply(diff(a, b), a) == b`, with ids drawn from small pools so moves, inserts, removes and repeated-id fallbacks all occur (S4: `keyed.rs`, `typed.rs`).
- The same intent sequence over Tauri IPC and WebSocket ends in the core's view-model, compared field by field in the UI (S4: `bench.ts`).
- Generated TS apply and Rust apply agree on recorded patch streams.
- Perf benchmark in CI: headless Chromium 94, decode + apply of recorded scenarios (reported, not gated: runners are noisy).

## Open items
- Postcard vs JSON on the C3 (T0.S2.8, D71).
- Back-pressure: the core should wait for the UI's ack of frame *n* before diffing frame *n+1*, so a slow UI gets fewer, bigger patches instead of a queue. To confirm in Phase 1.
- Versioning handshake between a UI and a remote core of another version (schema hash + semver of the view-model package).
