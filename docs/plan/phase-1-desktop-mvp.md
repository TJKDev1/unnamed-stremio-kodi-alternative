# Phase 1 — Desktop MVP

**Goal:** on Windows (then macOS/Linux), the app replaces Stremio for daily use and handles the Trakt → Simkl move.

**Phase exit criteria**
- The owner uses it instead of Stremio for a full week on the PC: AIOStreams streams, HDR monitor, headset.
- Simkl is the primary tracker, Trakt history is migrated, and scrobbling works.
- A Stremio user can import their addons and be watching within 2 minutes ([success criteria](../design/01-vision.md#what-success-looks-like)).
- Desktop [performance budget](../design/03-architecture.md#performance-budget) is green in CI.

**Milestone order and why**

```
M1.1 Skeleton ─► M1.2 Browse ─► M1.3 Play ══► daily driver from here
                                   │
        ┌──────────┬──────────┬────┴─────┬──────────┬──────────┐
        ▼          ▼          ▼          ▼          ▼          ▼
     M1.4       M1.5       M1.6       M1.7       M1.8       M1.9
     Input    Trackers    Ranking    Player     Library    Theming
                  │                    power    & search
                  ▼                                │
               M1.11 Anime basics ◄────────────────┘
     M1.10 Downloads (after M1.3) · M1.12 Local folders (after M1.8)
                         ▼
                  M1.13 Polish & release
```

After M1.3 the order of M1.4–M1.12 is flexible; pick whatever hurts most in daily use. The suggested order is: M1.5 (Simkl move is time-sensitive: Simkl AUTH V1 retires around April 2027, D19) → M1.4 → M1.7 → M1.8 → M1.6 → M1.9 → M1.10 → M1.11 → M1.12 → M1.13.

---

## M1.1 Kernel & shell skeleton

**Goal:** the app window opens, runs the kernel, and lists its built-in modules on an Addons screen.
**Depends on:** Phase 0 (S1, S4).
**Specs:** [kernel-and-modules](../specs/kernel-and-modules.md), [slots](../specs/slots.md), [storage](../specs/storage.md), [view-model-ipc](../specs/view-model-ipc.md) · D25, D41, D32

**Exit criteria**
- A built-in module is defined by a manifest, registered in a slot, can be disabled at runtime (zero footprint) and excluded with a Cargo feature.
- A CI check fails if a built-in module imports kernel internals.
- State survives a restart (SQLite); the data model is profile-aware.

**Tasks**
- [ ] T1.1.1 Write the [kernel-and-modules](../specs/kernel-and-modules.md) and [slots](../specs/slots.md) specs (stub → draft).
- [ ] T1.1.2 `sdk-schema`: IDL → Rust traits + TS types (pick the tool: e.g. JSON Schema + typify/ts generation). Also generates each view-model's typed diff (Rust), op types and apply (TS) (D70).
- [ ] T1.1.3 Kernel: module manifest, lifecycle, dependency resolution, lazy start, "disabled = never initialised".
- [ ] T1.1.4 Slot registry: single / multi / pipeline slots; required slots must keep a provider.
- [ ] T1.1.5 Event bus ("tell") and slot calls ("ask"); no module-to-module calls.
- [ ] T1.1.6 Storage: SQLite + migrations, KV per module, OS keychain secrets; `profile_id` on every user-data table.
- [ ] T1.1.7 Settings registry: modules declare settings schemas; one settings store.
- [ ] T1.1.8 Elm-style core loop (model → update → effects) with an action log for bug reports.
- [ ] T1.1.9 Desktop shell from S1 + IPC from S4, cleaned up into `platforms/desktop`.
- [ ] T1.1.10 UI app shell: left rail, router, view-model store, empty screens.
- [ ] T1.1.11 Addons screen listing built-in modules with enable/disable (D41 rules, dependent modules degrade with a message).
- [ ] T1.1.12 CI lint: built-ins import only the public SDK.

## M1.2 Browse with Stremio addons

**Goal:** install AIOStreams and Cinemeta-style addons and browse their catalogs and detail pages.
**Depends on:** M1.1, S3.
**Specs:** [stremio-client](../specs/stremio-client.md) · D7, D15, D40, D48

**Exit criteria**
- Installing AIOStreams by URL works, including its configure page, and the secret-bearing URL is never shown in the UI or exports.
- Home shows catalog rows from installed addons, cached-first (renders instantly from cache, refreshes in background).
- Detail pages show TMDb metadata, seasons and episodes.
- Stremio account import brings over addons, library and progress, then discards credentials.

**Tasks**
- [ ] T1.2.1 Promote the S3 client into `addons-stremio`; disk cache honouring `cacheMaxAge`/`staleRevalidate`/`staleError`.
- [ ] T1.2.2 Addon install by URL / pasted list; per-addon enable, priority (catalogs / meta / streams separately), timeout.
- [ ] T1.2.3 Configure flow: sandboxed in-app webview for `{base}/configure`, capture the manifest URL, store it as a secret.
- [ ] T1.2.4 TMDb metadata built-in module (D40): app key by default, own key optional, language/region; external IDs feed the mapper.
- [ ] T1.2.5 Image service in the core: resize to card size, disk cache, WebP/AVIF, pre-warm next row.
- [ ] T1.2.6 Home screen: hero + catalog rows (virtualized, from S2), placeholders, cached-first rendering.
- [ ] T1.2.7 Detail page: hero art, logo, metadata, seasons/episodes strip (D23).
- [ ] T1.2.8 Discover screen: browse any catalog with its extras (genre, year…).
- [ ] T1.2.9 Stremio account import (one-time, D48): `addonCollectionGet`, `datastoreGet`.
- [ ] T1.2.10 Conformance suite v0: the S3 fixtures run in CI.

## M1.3 Play — first daily driver

**Goal:** press Play on a title and watch it in mpv, with resume and continue-watching.
**Depends on:** M1.2, S1.
**Specs:** [player-backend](../specs/player-backend.md), [stream-ranking](../specs/stream-ranking.md) (trust-order only) · D3, D15, D34

**Exit criteria**
- Streams from AIOStreams appear as they arrive, in AIOStreams' order (trust-order mode).
- Playback in mpv under the web OSD: play/pause, seek, audio and subtitle track selection, fullscreen.
- Progress is saved; Continue Watching row works; resume asks or auto-resumes (setting).
- Next episode plays with the same `bingeGroup` where possible.
- **The owner uses it for real from here on.**

**Tasks**
- [ ] T1.3.1 `PlayerBackend` trait + mpv backend (from S1: compositing per D67; re-assert mpv z-order on every VO re-init).
- [ ] T1.3.2 Stream list screen: progressive results, per-addon groups, "requires plugin X" for torrent/NZB streams (D8, D20).
- [ ] T1.3.3 Ranking stub: per-addon "trust this addon's order" (default for AIOStreams); simple merge across addons.
- [ ] T1.3.4 Player view-model + OSD: scrubber, tracks, chapters, buffering, stats overlay.
- [ ] T1.3.5 Watch progress: periodic save, watched threshold, resume prompt.
- [ ] T1.3.6 Continue Watching row and "Up next" on detail page.
- [ ] T1.3.7 Binge: next-episode countdown, `bingeGroup` matching.
- [ ] T1.3.8 Play button in "Always show list" and "Always auto-play" modes (Smart comes in M1.6).
- [ ] T1.3.9 Subtitles from Stremio subtitle addons (`videoHash`/`videoSize` in extra).
- [ ] T1.3.10 Windows installer + portable build for the owner's daily use (unsigned is fine).

## M1.4 Focus, controller & remapping

**Goal:** the whole app works with a controller or keyboard alone, and every action can be rebound.
**Depends on:** M1.3.
**Specs:** [focus-and-input](../specs/focus-and-input.md) · D12, D36

**Exit criteria**
- Every screen is fully usable with an Xbox pad, a DualSense and a keyboard; ≤ 2 actions from Home to playback.
- Automatic switching between pointer and focus mode; "Force TV mode" on desktop.
- Any command can be bound to keys, buttons, chords and long-press; keymaps import/export.
- Focus move < 1 frame on desktop (CI perf test).

**Tasks**
- [ ] T1.4.1 Production focus engine (from S2): groups, memory per group, traps for dialogs, restore on Back, skin overrides.
- [ ] T1.4.2 Command registry: every action is a named command (`nav.*`, `player.*`, `app.*`).
- [ ] T1.4.3 Input sources: keyboard, gamepad (Gamepad API or native), media keys.
- [ ] T1.4.4 Binding engine: context-aware (browse vs player), chords, long-press, hold-to-repeat with acceleration.
- [ ] T1.4.5 Per-device profiles with correct button glyphs.
- [ ] T1.4.6 Remapping UI + import/export.
- [ ] T1.4.7 Command palette (Ctrl+K) over commands and settings.
- [ ] T1.4.8 TV mode density: larger targets, overscan-safe margins.

## M1.5 Trackers & Trakt → Simkl migration

**Goal:** Simkl is the primary tracker, Trakt history has been migrated, and everything watched in the app is scrobbled.
**Depends on:** M1.3.
**Specs:** [tracker-api](../specs/tracker-api.md) · D18, D19, D21, D52

**Exit criteria**
- Simkl sign-in via AUTH V2 device flow (`media:read media:write`); scrobble start/pause/stop works.
- Trakt read: history, watchlist, ratings.
- Migration Trakt → local → Simkl: dry-run diff, apply, re-run sends only differences.
- Dual-scrobble mode during the switch; primary tracker is master by default, toggle to local-master.
- Offline queue: events made offline are replayed and de-duplicated.

**Tasks**
- [ ] T1.5.1 Write the [tracker-api](../specs/tracker-api.md) spec: scrobble events, history/watched two-way, progress, lists, ratings, auth helpers.
- [ ] T1.5.2 Tracker slot (multi) + scrobble events emitted from the core player.
- [ ] T1.5.3 Host OAuth device-code helper (code + QR on TV, browser on desktop); tokens in keychain.
- [ ] T1.5.4 Simkl tracker module: AUTH V2, scrobble, two-way history, watchlist (verify against api.simkl.org first).
- [ ] T1.5.5 Trakt tracker module: read history/watchlist/ratings/progress; optional dual-scrobble.
- [ ] T1.5.6 Local history as the hub; master/mirror policy (D52) with conflict resolution.
- [ ] T1.5.7 Migration tool: diff engine, preview screen, apply, incremental re-run.
- [ ] T1.5.8 Offline event queue with event IDs.
- [ ] T1.5.9 Minimal ID mapping (IMDb ↔ TMDb ↔ TVDB via TMDb external IDs) so trackers get the IDs they need.
- [ ] T1.5.10 Run the owner's real migration; log anything that didn't map.

## M1.6 Stream ranking & Smart Play

**Goal:** the built-in ranker filters, sorts and explains streams, and Play does the right thing by default.
**Depends on:** M1.3.
**Specs:** [stream-ranking](../specs/stream-ranking.md) · D12, D15, D34

**Exit criteria**
- Release-name parser extracts resolution, HDR/DV, codec, audio, languages, size, group, cached markers from real AIOStreams/Comet names (fixture tests).
- Filter → dedupe → sort → format pipeline with presets; "Explain ranking" shows why #1 is #1.
- Device-aware de-ranking (e.g. DV profile 5 on a non-DV display), also on top of trust-order addons.
- Smart Play (default): auto-plays a clear winner, otherwise opens the list with the best focused; long-press does the opposite.

**Tasks**
- [ ] T1.6.1 Release-name parser crate with a large fixture corpus.
- [ ] T1.6.2 Rule model + TOML format (filters, sort keys, per-addon caps).
- [ ] T1.6.3 Pipeline: collect → parse → filter → dedupe → sort → format, progressive.
- [ ] T1.6.4 Display capability probe (HDR, DV, audio passthrough) feeding device-aware rules.
- [ ] T1.6.5 Presets (per profile, per device) + simple UI + raw TOML editor.
- [ ] T1.6.6 Stream formatter templates.
- [ ] T1.6.7 Explain-ranking view.
- [ ] T1.6.8 Smart Play logic (score gap, cached, preset match) + settings per profile and content type.

## M1.7 Player power features

**Goal:** the player is better than Stremio's for the owner's HDR monitor and headset.
**Depends on:** M1.3.
**Specs:** [player-backend](../specs/player-backend.md) · D35

**Exit criteria**
- HDR10 on the Windows HDR monitor with optional auto HDR-mode switching and SDR-in-HDR brightness; DV → HDR10/tone-mapped via libplacebo.
- Headphones: virtual surround (sofalizer, selectable HRTF), dialogue boost, per-output-device presets that switch automatically.
- Audio: language rules, delay, night mode, downmix, normalisation, passthrough toggles, WASAPI exclusive.
- Subtitles: language rules, forced/SDH, styling, delay, ASS override, per-show memory.
- External players (VLC, mpv, MPC-HC) with resume and progress where supported.

**Tasks**
- [ ] T1.7.1 Windows HDR: `gpu-next` on d3d11, HDR mode switching, SDR brightness setting; OSD dimming over HDR video (D68).
- [ ] T1.7.2 Audio rules + audio settings page.
- [ ] T1.7.3 Headphone features + output-device detection and presets.
- [ ] T1.7.4 Subtitle rules + styling + per-show memory.
- [ ] T1.7.5 Video: zoom/crop/aspect presets, deinterlace, shader presets.
- [ ] T1.7.6 mpv.conf / input.conf editor (expert level).
- [ ] T1.7.7 Scrubber thumbnails, chapter markers, hold-to-2x, frame step.
- [ ] T1.7.8 External player support with progress callbacks where possible.
- [ ] T1.7.9 OS media controls (Windows SMTC; macOS Now Playing; MPRIS) (D49).
- [ ] T1.7.10 PiP / mini-player.
- [ ] T1.7.11 Motion smoothness on high-refresh displays: evaluate `video-sync=display-resample` + interpolation for 24/60 fps content on 120–240 Hz (S1 saw judder, same in plain mpv).

## M1.8 Library, catalog organizer & search

**Goal:** the home screen and library are fully under the user's control, and one search box finds everything.
**Depends on:** M1.3 (M1.5 for tracker-backed filters).
**Specs:** [slots](../specs/slots.md) (catalogs, search) · D11, D12, D51, D53, D54

**Exit criteria**
- Rows can be reordered, hidden, renamed, merged and restyled; new addon rows land at the bottom, collapsed.
- Custom rows from saved filters over catalogs and the library.
- One unified Library with Watchlist / Watching / Completed / Downloaded / Local filters, showing where each item's state comes from.
- Unified search: movies & shows first, deduped by ID, results stream in; settings/commands in a separate strip.
- Calendar of upcoming episodes.

**Tasks**
- [ ] T1.8.1 Row model (per profile) + catalog organizer UI.
- [ ] T1.8.2 Merge rows (dedupe by ID) + custom rows (saved filters).
- [ ] T1.8.3 Home-screen composer as a built-in provider.
- [ ] T1.8.4 Library screen + filters + state-source labels.
- [ ] T1.8.5 Search slot + search composer (relevance + type weights), local FTS5 index.
- [ ] T1.8.6 Settings/commands strip in search; recent searches; typo-tolerant suggestions.
- [ ] T1.8.7 Calendar screen (trackers + metadata).

## M1.9 Theming tiers 0/1 & settings UX

**Goal:** users can restyle the app without code, and settings are easy to find.
**Depends on:** M1.1.
**Specs:** [theme-tokens](../specs/theme-tokens.md) · D6, D23, D49

**Exit criteria**
- Default skin uses only tokens; changing a token updates instantly with no rebuild.
- `theme.toml` + scoped (sanitised) CSS themes load; light/dark/OLED and desktop/TV variants.
- Theme editor with live preview and export to a file.
- One searchable settings screen with Basic / Advanced / Expert levels, reset-to-default, import/export.

**Tasks**
- [ ] T1.9.1 Write the [theme-tokens](../specs/theme-tokens.md) spec; token set covers color, type, space, radius, elevation, motion, focus, card.
- [ ] T1.9.2 Token → CSS custom properties pipeline; default skin migrated to tokens.
- [ ] T1.9.3 Theme loader + CSS sanitiser.
- [ ] T1.9.4 Tier 0 settings (accent, fonts, density, poster shape, motion, blur).
- [ ] T1.9.5 Theme editor + export.
- [ ] T1.9.6 Dynamic colors from artwork (computed in the core).
- [ ] T1.9.7 UI sounds (on by default, volume, one-tap off; sound packs in themes).
- [ ] T1.9.8 Settings screen: search, levels, per-setting descriptions, override indicators, import/export.

## M1.10 Downloads

**Goal:** download HTTP/debrid streams and watch them offline.
**Depends on:** M1.3 (M1.6 for "use my ranking preset").
**Specs:** [slots](../specs/slots.md) (downloader) · D33

**Exit criteria**
- Queue with pause/resume, parallel and bandwidth limits; quality choice or ranking preset; external subtitles saved alongside.
- Storage limit; auto-delete after watched; "download next N episodes".
- Downloaded items show as the top "Local · Downloaded" stream; progress and scrobbles work offline.

**Tasks**
- [ ] T1.10.1 Downloader provider (built-in) with resumable HTTP range downloads.
- [ ] T1.10.2 Queue UI + limits.
- [ ] T1.10.3 Storage policy: limit, auto-delete, next-N episodes.
- [ ] T1.10.4 Downloaded items as a stream source + Library "Downloaded" filter.

## M1.11 Basic anime support

**Goal:** anime from Stremio addons works properly: `kitsu:` IDs, absolute numbering, anime ranking and language rules.
**Depends on:** M1.5 (ID mapping), M1.6 (ranking).
**Specs:** [id-mapping](../specs/id-mapping.md) · D27

**Exit criteria**
- Streams found under `kitsu:ID:ep` and `tt:S:E` show up for the same episode.
- Absolute ↔ season/episode conversion correct for a test set of tricky shows (split cours, specials, episode 0).
- Anime preset (JP audio, EN subs, preferred groups) applies automatically to anime entries.

**Tasks**
- [ ] T1.11.1 Write the [id-mapping](../specs/id-mapping.md) spec.
- [ ] T1.11.2 Import offline dumps (Anime-Lists XML, Fribb/manami crosswalk) + patch overlay.
- [ ] T1.11.3 Mapper as a built-in single-slot provider; absolute ↔ relative conversion; per-show offset.
- [ ] T1.11.4 `kitsu:` and other anime ID prefixes through the resource router.
- [ ] T1.11.5 Anime ranking fields (group, dual audio, multi-subs, softsub/hardsub, BD/WEB, v2).
- [ ] T1.11.6 Per-content-type language rules and presets.

## M1.12 Local folders

**Goal:** local files show up as sources on the same titles, without being a separate mode.
**Depends on:** M1.8, M1.6.
**Specs:** [storage](../specs/storage.md) · design: [local files](../design/06-ux-input-player.md#local-files-are-not-a-separate-ghetto)

**Exit criteria**
- Folders are scanned incrementally in the background without affecting UI or playback.
- Matched files appear as the top "Local" stream on their title, ranked by the same engine.
- Unmatched files go to a "Needs attention" list with inline fix-match.

**Tasks**
- [ ] T1.12.1 Scanner provider: incremental, low priority, file watching optional.
- [ ] T1.12.2 Matcher using the shared release-name parser + TMDb search; scored and explainable.
- [ ] T1.12.3 Local files as a stream source + Library "Local files" filter.
- [ ] T1.12.4 Needs-attention list with fix-match.

## M1.13 Polish & release

**Goal:** a release other people can install and recover from.
**Depends on:** all other Phase 1 milestones.
**Specs:** — · D32, D38, D47, D49

**Exit criteria**
- Onboarding: input mode → bring your stuff (Stremio import / URLs / folders) → or start empty.
- Basic local profiles (data model was profile-aware since M1.1).
- Safe mode (hold key at start or crash loop), log viewer, addon inspector (requests, latency, errors, cache hits).
- Signed installers for Windows/macOS/Linux; auto-update with stable/beta/nightly channels (can be disabled); opt-in crash reports.
- Phase exit criteria at the top of this file all pass.

**Tasks**
- [ ] T1.13.1 Onboarding flow (D7: no recommended content sources).
- [ ] T1.13.2 Profiles: create/switch, optional PIN, per-profile addons/rows/ranking/theme.
- [ ] T1.13.3 Safe mode + crash-loop detection.
- [ ] T1.13.4 Log viewer + addon inspector.
- [ ] T1.13.5 Discord Rich Presence (opt-in, privacy options) (D49).
- [ ] T1.13.6 Packaging, code signing, auto-update channels (D38).
- [ ] T1.13.10 libmpv built from source in CI per OS with a recorded recipe; each release names its exact libmpv source (GPL) (D66).
- [ ] T1.13.7 Opt-in crash reporting.
- [ ] T1.13.8 Accessibility pass: screen-reader labels, reduced motion, scalable type; string tables for i18n.
- [ ] T1.13.9 Week-long daily-use test; fix list; phase sign-off.
