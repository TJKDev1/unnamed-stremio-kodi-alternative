# Spec: Tracker API

**Status:** stub · **Implemented by:** M1.5, M4.5, M5.5 · **Decisions:** D18, D19, D21, D52

## Purpose
The interface every tracker (Simkl, Trakt, AniList, MAL, Last.fm…) implements, plus the core's scrobble, sync and migration machinery. Design: [tracker & scrobbler addons](../design/04-addons.md#tracker--scrobbler-addons).

## Scope
- **In:** scrobble events, two-way history/watched, progress, lists as catalogs, ratings, auth helpers, background jobs, master/mirror policy, migration.
- **Out:** sandboxing (Phase 4 `wasm-host`); the core ships no tracker of its own (D18).

## Contract (to fill in)
- Events: `start`, `pause`, `resume`, `seek`, `progress` (throttled), `stop`, `finished` (threshold), each with event ID, canonical IDs, season/episode, position, duration; source addon/stream behind a separate permission.
- Provider functions: `pullHistory(since)`, `pushHistory(changes)`, `pullProgress`, `pushProgress`, `lists()`, `rate(id, value)`, `auth()`.
- Migration: `diff(source, target) → plan`, `apply(plan)`; incremental.

## Behaviour (known constraints)
- Simkl: AUTH V2 device flow only, scope `media:read media:write` (V1 retires ~April 2027). Verify against api.simkl.org before building.
- Primary tracker is master by default; toggle to local-master; local copy always kept (D52).
- Offline queue with de-duplication by event ID.
- Same interface for in-process built-ins (Phase 1) and sandboxed addons (Phase 4).

## Conformance tests
- Mock tracker server; scripted watch sessions → expected API calls.
- Migration dry-run diff on a fixture history; re-run sends nothing.
- Offline replay without duplicates.

## Open items
- Conflict resolution details when two devices scrobble the same episode offline.
