# Phase 0 — Foundations & spikes

**Goal:** a working repo with CI, and proof that the riskiest pieces of the architecture actually work, before Phase 1 builds on them.

Spikes are **throwaway-quality code with keep-quality findings**. Each spike ends with a short write-up (`docs/spikes/S<n>.md`: what we tried, numbers, verdict) and, where relevant, a decision logged as a D#. Code worth keeping gets cleaned up in Phase 1, not during the spike.

**Which spikes block Phase 1:** S1, S2, S3, S4, S7. The others de-risk later phases and can run any time before those phases start:

| Spike | Needed before | Why it can wait |
|---|---|---|
| S5 WASM addon host | Phase 4 | Phase 1 built-ins are native; nothing third-party is sandboxed yet |
| S6 Kodi shim | Phase 5 | Kodi support is Phase 5 |
| S8 Core on webOS | Phase 3 | webOS client is Phase 3 (S2 already covers UI perf on the TV) |

Suggested order: M0.1 → S3 → S1 → S4 → S2 → S7 → M0.2. S3 goes first because it's the cheapest and it produces real data (your AIOStreams catalogs) for the UI spikes to render.

---

## M0.1 Repo bootstrap

**Goal:** an empty-but-real workspace where every later task has a place to land.
**Depends on:** nothing.

**Exit criteria**
- `cargo build`, `cargo test`, the UI build and lint all run in CI on Windows, macOS and Linux.
- The [repository layout](../design/03-architecture.md#repository-layout) exists with placeholder crates/packages.
- License files and per-crate license fields match D37.

**Tasks**
- [x] T0.1.1 Cargo workspace with placeholder crates from the layout (`kernel`, `sdk-schema`, `core`, `addons-stremio`, `library`, `ranking`, `player`, `server`).
- [x] T0.1.2 UI workspace (pnpm) with `ui/app`, `ui/skin-default`, `ui/sdk`; TypeScript strict; build targets that cover Chromium 94 (D14).
- [x] T0.1.3 CI (GitHub Actions): fmt, clippy, tests, UI typecheck/lint/build, on the three desktop OSes.
- [x] T0.1.4 License files: GPL-3.0-or-later at root, AGPL-3.0-or-later for `server`/sync crates, MIT for SDKs; CI check on `Cargo.toml` license fields (D37, D59).
- [x] T0.1.5 `CONTRIBUTING.md`: task IDs in commits, spec-first workflow, definition of done (from [plan/README](README.md#definition-of-done-every-task)).
- [x] T0.1.6 `docs/spikes/` folder with a write-up template.
- [x] T0.1.7 Test fixtures folder for recorded addon responses (no secrets: a scrubber for debrid keys in URLs).

---

## S3 Rust Stremio protocol client

**Goal:** the core can talk to real Stremio addons, starting with your AIOStreams config.
**Spec:** [stremio-client](../specs/stremio-client.md) · D15, D44

**Exit criteria**
- All resources and extras parse for 10 real addons, including configurable ones (AIOStreams first, Cinemeta, OpenSubtitles, Anime Kitsu, a TMDB addon, an IPTV/EPG addon).
- Streams arrive progressively: a CLI prints each addon's streams as soon as that addon answers.
- Recorded responses are saved as fixtures (secrets scrubbed) for the conformance suite.

**Tasks**
- [x] T0.S3.1 Normalized models (`Manifest`, `MetaPreview`, `Meta`, `Video`, `Stream`, `Subtitle`) with serde, tolerant of unknown fields.
- [x] T0.S3.2 Manifest fetch + validation, including the legacy `extraSupported/extraRequired` forms and resource-as-object.
- [x] T0.S3.3 Route building (`/{resource}/{type}/{id}/{extra}.json`) with correct extra encoding.
- [x] T0.S3.4 Resource router: pick addons by `types`/`idPrefixes`; fan out in parallel; per-addon timeout.
- [x] T0.S3.5 `cacheMaxAge`/`staleRevalidate`/`staleError` honoured (in-memory is fine for the spike).
- [x] T0.S3.6 Dev CLI: `stremio-cli catalog|meta|streams <addon-url> …` printing results as they arrive.
- [~] T0.S3.7 Record fixtures from the 10 addons; write the S3 report. 9 recorded (incl. Hyper as an extra; AIOStreams recorded by the owner 2026-10-14) + report done; Comet and MediaFusion still need the owner's config (D61): `stremio-cli record`, see [fixtures/README](../../fixtures/README.md#recording-an-addon).

## S1 libmpv under a transparent webview

**Goal:** prove the desktop shell can show a web UI over native mpv video. **Highest risk in the project.**
**Spec:** [player-backend](../specs/player-backend.md) · D3, D35

**Exit criteria**
- Windows: no tearing; correct HDR10 output on an HDR monitor (`gpu-next`, d3d11); resize without flicker; web OSD over video at 60 fps.
- macOS and Linux: same, or a documented fallback (separate borderless player window) that works.
- Verdict logged as a D#: **Tauri v2 vs custom shell** (stremio-shell-ng style).

Scope (D64): Windows is tested end to end on the owner's PC. macOS and Linux prototypes are built in CI; until real hardware is available, their plan is the documented fallback. Spike code lives in `spikes/s1-shell/` (D65) and uses a pinned prebuilt libmpv (D66).

**Tasks**
- [x] T0.S1.1 Read how Rillio and `stremio-shell-ng` composite mpv + webview; write down the approach per OS. See [S1 prior art](../spikes/S1.md#prior-art-t0s11).
- [x] T0.S1.2 Windows prototype with Tauri v2 + WebView2 + libmpv (render API or child window).
- [x] T0.S1.3 Windows HDR: HDR10 file on the owner's HDR monitor; check metadata passthrough and SDR UI brightness over HDR video.
- [x] T0.S1.4 Resize/fullscreen/minimise stress test; measure OSD frame times.
- [-] ~~T0.S1.5 Same prototype on a custom shell if Tauri hits a wall (only if needed).~~ Not needed: Tauri passed (D67).
- [x] T0.S1.6 macOS (WKWebView) and Linux (WebKitGTK) prototypes; note WebKitGTK performance. Built in CI only, defaulting to the separate-window fallback (D64); not run on hardware.
- [x] T0.S1.7 Report + D#: shell choice and per-OS compositing method. [S1](../spikes/S1.md), D67, D68.

## S4 Core ↔ UI IPC

**Goal:** the core can push view-model updates to the UI fast enough for 60 fps.
**Spec:** [view-model-ipc](../specs/view-model-ipc.md) · D3, D29

**Exit criteria**
- < 2 ms per frame for patch encode + transport + apply at typical load (home screen with 10 rows streaming in).
- Same patches work over Tauri IPC and over WebSocket (remote core).
- Verdict logged as a D#: patch format and encoding (MessagePack vs postcard vs other).

Scope (D69): measured in WebView2 on the owner's PC and in headless Chromium 94 in CI (the C3 number comes from S2). Generic and typed diffs, each with JSON, MessagePack and postcard. Spike code lives in `spikes/s4-ipc/` (D65).

**Tasks**
- [ ] T0.S4.1 Define a toy view-model (home: rows of cards) and intent set in Rust.
- [ ] T0.S4.2 Structural diff → patch list, batched per frame.
- [ ] T0.S4.3 Encodings: MessagePack and postcard (+ a JS decoder for postcard); benchmark both.
- [ ] T0.S4.4 Transport over Tauri IPC (from S1's shell) and over WebSocket.
- [ ] T0.S4.5 SolidJS store that applies patches with fine-grained updates.
- [ ] T0.S4.6 Report + D#.

## S2 Web UI performance on the LG C3

**Goal:** prove the web UI can hit the TV performance budget, and pick the UI framework.
**Spec:** [focus-and-input](../specs/focus-and-input.md) · D14, Q25

**Exit criteria**
- On the LG C3: 60 fps row scroll (30 fps floor), < 2-frame focus response, memory within the app limit, with 10 rows × 50 posters.
- Verdict logged as a D#: **SolidJS vs Svelte 5** (closes open question 25).

**Tasks**
- [ ] T0.S2.1 Virtualized row with DOM node recycling (about 12 live nodes per row) in SolidJS.
- [ ] T0.S2.2 Minimal spatial focus engine: groups, memory per group, hold-to-repeat.
- [ ] T0.S2.3 Poster images pre-sized to card size (served by a tiny local image server).
- [ ] T0.S2.4 Package as an IPK; sideload via Developer Mode / Homebrew Channel; measure with remote DevTools.
- [ ] T0.S2.5 Same test in Svelte 5 (only the row + focus, not everything).
- [ ] T0.S2.6 Run in desktop WebView2 too, as the "easy" baseline.
- [ ] T0.S2.7 Report + D#.

## S7 Skin compiler proof

**Goal:** prove the `.skin` template format (D45) compiles to the chosen UI framework and can override one component.
**Spec:** [skin-format](../specs/skin-format.md) · D45

**Exit criteria**
- One `.skin` component (`home.row`) compiles and replaces the default `home.row` without touching the default skin.
- Hot reload works while editing.
- Generated code stays within the Chromium 94 baseline.

**Tasks**
- [ ] T0.S7.1 Template grammar sketch: bindings, `if`, loops, slots, `focus-group`, scoped `<style>`.
- [ ] T0.S7.2 Parser + compiler to SolidJS (or Svelte, per S2).
- [ ] T0.S7.3 Slot registry in the UI host; default-skin fallback for unoverridden slots.
- [ ] T0.S7.4 Hot reload via file watcher.
- [ ] T0.S7.5 Report.

## S5 WASM addon host (needed before Phase 4)

**Spec:** [wasm-host](../specs/README.md#planned-specs) (not yet written) · D9

**Exit criteria:** an Extism host runs a TS addon compiled to WASM that does HTTP + parsing; cold call < 50 ms; a disallowed domain is blocked.

- [ ] T0.S5.1 Extism host in Rust with an HTTP host function and a domain allowlist.
- [ ] T0.S5.2 TS addon via the Extism JS PDK returning a catalog.
- [ ] T0.S5.3 Measure cold/warm call time; test fuel/memory limits.
- [ ] T0.S5.4 Check an interpreter path for iOS (wasmi or Pulley) runs the same addon.
- [ ] T0.S5.5 Report.

## S8 Rust core as a webOS homebrew native service (needed before Phase 3)

**Spec:** none yet · D14, D29

**Exit criteria:** the core starts as a native service on the rooted C3, serves Stremio catalogs to the web UI, and memory stays reasonable.

- [ ] T0.S8.1 Run `uname -a` on the TV over SSH; confirm armv7 vs aarch64 userland (closes the follow-up on Q10).
- [ ] T0.S8.2 Cross-compile a minimal core with the webosbrew toolchain.
- [ ] T0.S8.3 Register a Luna service; call it from the web app.
- [ ] T0.S8.4 Serve S3's catalogs through it; measure memory.
- [ ] T0.S8.5 Report.

## S6 Kodi shim proof (needed before Phase 5)

**Spec:** none yet · D4, D15, D17

**Exit criteria:** a simple video plugin lists folders and resolves a stream through embedded CPython; Fen Light's main menu loads; its `select.xml` dialog renders via a WindowXML-lite prototype and returns a choice.

- [ ] T0.S6.1 `kodi-host` sidecar with pyo3-embedded CPython and a JSON-RPC pipe.
- [ ] T0.S6.2 Minimal `xbmc`/`xbmcgui`/`xbmcplugin`/`xbmcaddon`/`xbmcvfs` stubs (start from Kodistubs signatures).
- [ ] T0.S6.3 Blocking `Dialog().select()` round-trip to the UI.
- [ ] T0.S6.4 Fen Light main menu (needs `requests` + Pillow bundled).
- [ ] T0.S6.5 WindowXML-lite: render `select.xml`, return the choice.
- [ ] T0.S6.6 Report.

---

## M0.2 Phase 0 wrap-up

**Goal:** turn spike results into decisions and a confirmed Phase 1 plan.
**Depends on:** S1, S2, S3, S4, S7.

**Exit criteria**
- Every blocking spike has a report and its D# logged.
- Specs touched by spikes are bumped from `stub` to `draft`.
- Phase 1 milestones re-checked against what the spikes found.

**Tasks**
- [ ] T0.2.1 Log D#s: shell (S1), UI framework (S2), IPC format (S4), skin compiler approach (S7).
- [ ] T0.2.2 Update [architecture](../design/03-architecture.md) and [open questions](../design/08-open-questions.md) to match.
- [ ] T0.2.3 Update specs: player-backend, view-model-ipc, stremio-client, focus-and-input, skin-format.
- [ ] T0.2.4 Re-order or re-scope Phase 1 milestones if needed; update the [status table](README.md#status).
