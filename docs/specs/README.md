# Specs

Exact contracts that code is written against. Specs exist so that AI-assisted work stays correct (D16): a task implements a spec, and the spec's conformance tests check it.

## Status levels

| Status | Meaning |
|---|---|
| `stub` | Purpose, scope and known constraints only. **Don't implement against a stub**; fill in the contract first. |
| `draft` | Contract written; may change after a spike or first implementation. |
| `unstable` | Implemented; built-ins may use it, third parties are warned it may change (D25 rule of two). |
| `stable` | Two real implementations or a concrete third-party need; semver applies. |

## Template

Every spec has these sections, in this order:
1. **Header**: status, plan tasks that implement it, decisions it rests on.
2. **Purpose**: one paragraph.
3. **Scope**: in / out.
4. **Contract**: types, functions, messages, file formats. Schemas live in `crates/sdk-schema` once code exists, and the spec links to them.
5. **Behaviour**: rules, edge cases, error handling, performance budget.
6. **Conformance tests**: what a test suite must check.
7. **Open items**: unresolved questions (link to [open questions](../design/08-open-questions.md) where relevant).

## Index

| Spec | Status | Needed by |
|---|---|---|
| [kernel-and-modules](kernel-and-modules.md) | stub | M1.1 |
| [slots](slots.md) | stub | M1.1 |
| [storage](storage.md) | stub | M1.1 |
| [view-model-ipc](view-model-ipc.md) | stub | S4, M1.1 |
| [stremio-client](stremio-client.md) | stub | S3, M1.2 |
| [player-backend](player-backend.md) | stub | S1, M1.3 |
| [stream-ranking](stream-ranking.md) | stub | M1.3, M1.6 |
| [focus-and-input](focus-and-input.md) | stub | S2, M1.4 |
| [tracker-api](tracker-api.md) | stub | M1.5 |
| [theme-tokens](theme-tokens.md) | stub | M1.9 |
| [id-mapping](id-mapping.md) | stub | M1.5, M1.11 |
| [skin-format](skin-format.md) | stub | S7, M4.7 |

### Planned specs
Not yet stubbed; written before their phase starts: `wasm-host` (S5, Phase 4), `protocol-v2` (M4.1), `mod-api` (M4.8), `kodi-shim` (S6, Phase 5), `sync-model` (M3.5), `local-api` (M2.5, may fold into view-model-ipc).
