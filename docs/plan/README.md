# Plan

The working plan: what gets built, in what order, and how we know it's done. The *why* behind each item is in [`../design/`](../README.md#design-docs); the exact contracts are in [`../specs/`](../specs/README.md).

Team: solo + AI, no deadline (D16). Phases are ordered by risk first, then value. **Every milestone is a thin vertical slice that ends in something usable**, not a layer ("all of the DB") that only pays off later.

## Phases

| Phase | File | Goal | Detail level |
|---|---|---|---|
| 0 | [Foundations & spikes](phase-0-spikes.md) | Repo bootstrap + de-risk the scariest tech before committing | Full tasks |
| 1 | [Desktop MVP](phase-1-desktop-mvp.md) | Replace Stremio (and the Trakt→Simkl move) on the owner's PC | Full tasks |
| 2 | [Android & Android TV](phase-2-android.md) | Same app on Android phones and TV boxes (D30) | Milestones |
| 3 | [Your devices: webOS + iOS](phase-3-webos-ios.md) | LG C3 + iPhone, remote core, sync, casting | Milestones |
| 4 | [The better ecosystem](phase-4-ecosystem.md) | Protocol v2, SDKs, WASM host, skins, mods, first-party addons | Milestones |
| 5 | [Kodi, anime depth & library](phase-5-kodi-anime-library.md) | Fen Light works end to end; deep anime; media servers; live TV | Milestones |
| 6 | [Reach & music](phase-6-reach-music.md) | Music, Tizen, hosted sync, Usenet | Milestones |

Later phases are deliberately rough. A phase gets broken into tasks **when the phase before it is about half done**, so the breakdown uses what we learned.

## Status

| | Current |
|---|---|
| Phase | 0 — in progress |
| Done | [M0.1 Repo bootstrap](phase-0-spikes.md#m01-repo-bootstrap) (2026-10-14, PR #1); [S3 Stremio client](phase-0-spikes.md#s3-rust-stremio-protocol-client) code + report ([S3](../spikes/S3.md)) |
| Next up | [S1 libmpv under a transparent webview](phase-0-spikes.md#s1-libmpv-under-a-transparent-webview) |
| Blocked on | nothing. Owner to-do: record Comet and MediaFusion fixtures to fully close S3 (T0.S3.7) |

Update this table whenever a milestone closes.

## How the plan is written

### IDs
- **Milestone:** `M<phase>.<n>`, e.g. `M1.3`. Spikes keep their existing IDs (`S1`–`S8`) inside Phase 0.
- **Task:** `T<phase>.<milestone>.<n>`, e.g. `T1.3.2`; spike tasks use the spike ID, e.g. `T0.S3.2`. IDs are never reused; a dropped task is struck through, not deleted.
- Commits and PRs reference the task ID (`T1.3.2: mpv under webview on Windows`).

### Task checkboxes
```
- [ ] T1.2.3 Short imperative title — spec: [stremio-client](../specs/stremio-client.md) · D15
- [~] in progress
- [x] done
- [-] ~~dropped~~ (one line saying why)
```

### Every milestone has
- **Goal**: one sentence, user-visible.
- **Depends on**: milestones that must be closed first.
- **Exit criteria**: checkable statements. The milestone is closed when all of them pass, not when all tasks are ticked.
- **Tasks**: the checklist. Adding tasks mid-milestone is normal.

### Definition of done (every task)
1. Code merged to `main` behind no flags unless the task says so.
2. Tests for the behaviour (unit for logic, conformance for protocols, perf tests where a [budget](../design/03-architecture.md#performance-budget) applies).
3. If it implements a spec: the spec is updated to match what was actually built and its status bumped.
4. If it made a decision: logged as a D# in the [decision log](../design/01-vision.md#decision-log) and removed from [open questions](../design/08-open-questions.md).

### Working with AI on a task
1. Read the task, its spec and the decisions it cites.
2. If the spec is a stub, fill in the contract **first** (separate commit) and get it agreed.
3. Implement against the spec; conformance tests come from the spec.
4. Tick the box, update the status table.

## Engineering practices (from day one, all phases)
- **Small crates** with a narrow public API each (D16). Built-in modules only import the public SDK; CI enforces it (D25).
- **Contracts are schemas**: one IDL source generates Rust traits, TS types, WASM bindings and docs ([kernel spec](../specs/kernel-and-modules.md)).
- **Performance budgets as CI tests**: headless Chromium in CI, plus a manual real-device run on the LG C3 per milestone that touches UI.
- **Conformance suites**: Stremio v1 from Phase 1, our v2 from Phase 4, Kodi shim from Phase 5.
- **Licensing**: GPL-3.0-or-later app, AGPL-3.0-or-later server/sync crates, MIT SDKs (D37, D59); a CI check that every crate declares the right one.
- **No telemetry**; crash reports opt-in only (D38).
- **Review**: one PR per milestone/spike, one commit per task; GPT-6.1-Sol reviews each PR before merge (D58).
