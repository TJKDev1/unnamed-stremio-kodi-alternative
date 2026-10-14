# Spec: Slots

**Status:** stub · **Implemented by:** M1.1 (T1.1.4), then each feature milestone adds its slot · **Decisions:** D25, D40, D53

## Purpose
The list of extension points (slots) and the contract of each. Built-ins and third-party providers implement the same contracts. Design: [everything is a provider](../design/04-addons.md#built-in-but-replaceable-everything-is-a-provider-d25).

## Scope
- **In:** slot kinds, the initial slot list, each slot's function signatures and fallback behaviour.
- **Out:** the kernel mechanics of registering slots ([kernel-and-modules](kernel-and-modules.md)).

## Contract (to fill in)

Slot kinds: **single** (one active), **multi** (many, ordered), **pipeline** (sequence, transform).

Initial slots (about 10; split only when a real need appears):

| Slot | Kind | Sketch | Built-in | Milestone |
|---|---|---|---|---|
| metadata | multi | `meta(id) → Meta` | TMDb (D40) | M1.2 |
| catalogs | multi | `catalog(type, id, extra) → MetaPreview[]` | — (addons only, D7) | M1.2 |
| streams | multi | `streams(type, id) → Stream[]` (progressive) | — (addons only, D7) | M1.3 |
| subtitles | multi | `subtitles(videoId, hints) → Subtitle[]` | local/embedded | M1.3 |
| ranker | single | `rank(streams, context) → ordered + explanations` | rule engine | M1.6 |
| mapper | single | `map(id, episode) → ids` | offline dumps | M1.5 / M1.11 |
| tracker | multi | see [tracker-api](tracker-api.md) | Simkl, Trakt | M1.5 |
| player-backend | single (native only) | see [player-backend](player-backend.md) | mpv | M1.3 |
| downloader | single | `enqueue(stream, opts) → job` | built-in | M1.10 |
| search | multi + composer | `search(query) → typed results` (progressive) | FTS + fan-out | M1.8 |
| screens | multi | UI screens/routes | default skin | M1.1 |

## Behaviour (known constraints)
- Contracts are narrow and data-oriented; never expose internal structs.
- Single-slot replacement falls back to the built-in on crash/timeout/uninstall, with a notice (Phase 4).
- Multi-slot providers run in parallel; slow providers never block the first results.

## Conformance tests
- Per slot: a reference provider and a test suite any implementation must pass.

## Open items
- Is `screens` a slot or part of the skin system?
- Release-name parser: separate slot or part of the ranker?
