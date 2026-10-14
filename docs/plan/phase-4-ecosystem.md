# Phase 4 — The better ecosystem

**Goal:** our own addon protocol, SDKs and sandboxes are good enough that first-party native addons replace Fen Light for daily use (D9, D18, D43).
**Depends on:** Phase 1 (can overlap with Phases 2–3), spike [S5](phase-0-spikes.md#s5-wasm-addon-host-needed-before-phase-4).

Milestones only.

**Phase exit criteria**
- Protocol v2 spec, JSON Schemas and conformance tests are in the repo.
- Simkl, Trakt, TorBox and TMDb run as sandboxed WASM addons through the public API, with no special access.
- A third party could write, test and ship an addon, skin or mod using only the docs and SDKs.

## M4.1 Protocol v2 spec
**Goal:** a v1 superset: config schema with secrets in headers, structured stream fields, streaming responses, events/segments/nextUp resources, capability negotiation, signed manifests.
**Exit criteria:** spec + JSON Schemas + conformance suite; a v2 addon still works (degraded) in Stremio.

## M4.2 WASM addon host
**Goal:** Extism/wasmtime host (an interpreter on iOS) with permissions shown at install, fuel/memory limits, background jobs, an OAuth device-flow helper, and generic host capabilities (rate-limited HTTP, parsers, crypto, KV, cache).
**Exit criteria:** a misbehaving addon is killed and flagged; permissions are enforced, not just declared.

## M4.3 Addon SDKs & dev tooling
**Goal:** TS and Rust SDKs with `--target http|wasm`, `addon dev` hot reload, a conformance test command, and a Stremio-SDK compat wrapper.
**Exit criteria:** the example addon from [04](../design/04-addons.md#is-it-hard-to-program-no-about-the-same-as-a-stremio-addon-and-the-sdk-hides-the-difference) builds to both targets.

## M4.4 Built-ins become replaceable
**Goal:** ranker, parser, mapper, scanner, search, downloader etc. can be replaced by WASM providers, with automatic fallback to the built-in (D25).
**Exit criteria:** "replace with…" and "reset to built-in" work for every single-slot provider; interfaces with two implementations are marked stable (rule of two).

## M4.5 First-party native addons
**Goal:** the Simkl and Trakt trackers move into the sandbox; a **TorBox cloud** addon; TMDb metadata as a replaceable addon (D19).
**Exit criteria:** the owner's daily setup runs with zero in-process tracker code.

## M4.6 Torrent engine plugin
**Goal:** an optional first-party librqbit sidecar (D8), not pre-installed.
**Exit criteria:** `infoHash` streams play and seek on desktop.

## M4.7 Tier 2 skins
**Goal:** public skin API, compiler (from S7), dev tools (focus debugger, slot inspector, perf overlay), layout presets.
**Spec:** [skin-format](../specs/skin-format.md)
**Exit criteria:** a partial skin overrides a few screens and survives an app update.

## M4.8 Tier 3 mods
**Goal:** worker/WASM sandbox, slots, commands, permissions, CPU/memory budgets, unsafe dev mode (D47).
**Exit criteria:** the example mods from [05](../design/05-theming-and-modding.md#example-mod-ideas-to-validate-the-api) can be built.

## M4.9 Community catalogs
**Goal:** add any `addon_catalog`-style index for addons, themes, skins and mods; the install screen shows permissions, signature and source (D46).
