# Spec: Storage

**Status:** stub · **Implemented by:** M1.1 (T1.1.6), M1.12 · **Decisions:** D5, D32, D52

## Purpose
How the core persists user data, caches and secrets, so that every module stores things the same way and the data model is profile-aware and sync-ready from day one.

## Scope
- **In:** SQLite layout and migrations, per-module KV, secrets (OS keychain), caches on disk (addon responses, images), profile scoping.
- **Out:** sync protocol (`sync-model`, M3.5).

## Contract (to fill in)
- Core tables: profiles, addons (install URL stored as a secret ref), library items, watch progress, history events, settings, rows/catalog layout.
- Every user-data row carries `profile_id`.
- Record identity: stable IDs + `updated_at` (+ device ID) so either LWW or CRDT sync can be added later without migration pain.
- Per-module KV namespace with quotas.
- Secrets API: `put/get/delete(module, key)`; never readable by other modules.

## Behaviour (known constraints)
- SQLite with FTS5 for library and search.
- Response cache honours `cacheMaxAge` / `staleRevalidate` / `staleError`.
- Secrets: Windows Credential Manager, macOS/iOS Keychain, libsecret; encrypted file fallback on the home server.
- Switching tracker master never deletes data (D52).

## Conformance tests
- Migrations run forward from every released schema version.
- Profile isolation: queries for profile A never return profile B's rows.
- Secrets never appear in logs, exports or the action log.

## Open items
- LWW vs CRDT (open question 27): decide in the M3.5 spike, but pick IDs/timestamps now that support both.
