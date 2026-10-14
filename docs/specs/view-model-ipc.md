# Spec: View-models & IPC

**Status:** stub · **Implemented by:** S4, M1.1 (T1.1.8–T1.1.10), M2.5 · **Decisions:** D3, D29

## Purpose
How the UI and the core talk: the UI sends **intents**, the core sends **view-model patches**. The same messages work in-process (Tauri IPC) and over the network (WebSocket, remote core / Local API), so the UI doesn't know which it's using.

## Scope
- **In:** intent format, view-model shapes per screen, patch format, encoding, batching, transports, versioning.
- **Out:** Local API endpoints for third-party automation (may become a separate `local-api` spec).

## Contract (to fill in after S4)
- **Intent:** `{ screen, action, payload }`, typed per screen.
- **View-model:** one root per screen (home, detail, streams, player, library, search, settings…), typed in the schema, with TS types generated.
- **Patch:** structural diff ops (set / insert / remove / move) addressed by path; batched per frame.
- **Encoding:** MessagePack or postcard (S4 decides).
- **Transport:** Tauri IPC; WebSocket with per-device token.

## Behaviour (known constraints)
- < 2 ms per frame for patches at typical load.
- No JSON round-trip per item.
- Startup: render last-known home snapshot immediately, then hydrate.
- Skins and mods bind to these view-models, so view-models are public API (semver'd).

## Conformance tests
- Patch round-trip: apply(diff(a, b), a) == b, property-tested.
- Same intent sequence over IPC and WebSocket produces identical view-models.
- Perf benchmark in CI.

## Open items
- Encoding choice (S4).
- Back-pressure if the UI falls behind (drop intermediate frames?).
