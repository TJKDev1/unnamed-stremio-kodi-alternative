# 01 — Vision & principles

## One-liner

An open-source (GPL/AGPL), local-first media center that gives the user **maximum control**. It runs Stremio addons on day one and Kodi video addons through a compatibility layer. It also has its own addon system, which should be better and should take over once enough addons support it. Every pixel of the UI can be themed, skinned and modded, and the app should feel instant on a PC with a mouse and on a TV with a remote or controller.

## Who it's for

1. **Stremio power users** who are tired of: addon ordering hacks, catalog rows they can't hide, config and secrets baked into addon URLs, paywalled profiles and catalog sorting (Supporter tier, June 2026), limited player controls, and needing AIOStreams just to filter streams.
2. **Kodi users** who love the skins, the player and remote navigation but hate: setup sprawl, sluggish UIs on cheap boxes, addons breaking on every major version, MySQL-based "sync", and the piracy-build reputation.
3. **Tinkerers and themers**: people who make Kodi skins, Spicetify themes or Vencord plugins and want a media app built to be modded.

## Core principles

1. **The user is in control.** Every default can be changed. Nothing is hidden behind "protected" flags (Cinemeta can't be removed in Stremio; here nothing is untouchable). Nothing is paywalled.
2. **A bare shell, with no content of our own.** Like Stremio, the app ships with **no content sources**. Every source, catalog and stream provider is an addon the user installs. We never curate or link piracy sources. This is a legal and reputational requirement (see Kodi's "Kodi box" history).
3. **Fast is a feature.** We set hard performance budgets (see [03](03-architecture.md#performance-budget)), and missing one is a bug. No interaction may wait on the network if cached data exists: show cached data immediately, refresh it in the background.
4. **Local-first.** No account is required. All state lives on the device or on your own home server. Sync is optional and pluggable.
5. **A framework, not a monolith.** The app is a small kernel plus built-in addons that make up the full experience. Every built-in can be disabled, replaced or extended by a third-party addon through the same public API (D25).
6. **Built to be modded.** Themes, skins, mods and addons are first-class products with versioned APIs, so they don't break on every release the way Kodi skins and addons do.
7. **One app, two input modes.** Mouse/keyboard/touch and D-pad/controller are both first-class in every screen. TV mode is not a separate, cut-down app.
8. **Compatibility is a bridge.** We support the Stremio and Kodi ecosystems so people can switch, but our own system has to be clearly better so that addon authors want to move to it.
9. **Open and honest.** Everything is GPL/AGPL: no closed streaming server (unlike Stremio's) and no closed TV apps. No telemetry by default.

## What success looks like

- A Stremio user can import their addon collection and be watching within 2 minutes, with better stream filtering than they had before.
- A Kodi user can install their favourite video plugin, and its dialogs and settings work.
- A themer can publish a full skin as a single file that keeps working across app versions.
- On an LG C3 (webOS 23), browsing catalog rows scrolls smoothly and focus moves in under one frame.

## Decision log

| # | Date | Decision | Notes |
|---|------|----------|-------|
| D1 | 2026-10-13 | **MVP platform is desktop** (Windows/macOS/Linux). **iOS and LG webOS** come next, since they're the owner's own devices. | Android/Android TV after that. tvOS not planned (see D3). |
| D2 | 2026-10-13 | **Open source, copyleft (GPL or AGPL)** | AGPL is worth considering for the server parts. Still to decide; see [08](08-open-questions.md). |
| D3 | 2026-10-13 | **Rust core + web UI** (HTML/CSS/TS) | Gives the best moddability and runs natively on webOS. Video is never played inside the webview on desktop or iOS. Rules against web performance problems are in [03](03-architecture.md). tvOS is ruled out because it has no WebKit. |
| D4 | 2026-10-13 | **Kodi support = embedded Python shim** for video plugin addons | Desktop and home server only. Skins and WindowXML addons are out of scope. |
| D5 | 2026-10-13 | **Local-first; sync is optional** | Self-hosted server, an optional hosted service, or third parties (Trakt/Simkl); Stremio account import. |
| D6 | 2026-10-13 | **UI moddability goes all the way: tokens → themes → skins → code mods** | Mods run sandboxed. |
| D7 | 2026-10-13 | **Bare shell**: no content sources ship with the app | |
| D8 | 2026-10-13 | **Torrent engine is an optional, separately installed first-party plugin** | The core plays HTTP/HLS/local files only. |
| D9 | 2026-10-13 | **Native addon system = remote HTTP (Stremio superset) + local sandboxed WASM** | See [04](04-addons.md) for why it's better and how hard it is to write one. |
| D10 | 2026-10-13 | **The core can run headless as a home server**, and the app ships with it built in (as Stremio bundles its streaming server) | Off by default for LAN access; one toggle turns it on. |
| D11 | 2026-10-13 | **The default skin uses a left rail + catalog rows**; layout is configurable | Other presets come later. |
| D12 | 2026-10-13 | **Early must-haves**: stream ranking rules, per-row catalog control, player power settings, remappable input | All free. |
| D13 | 2026-10-13 | Monetization **not decided** | Hard rule either way: features are never paywalled. |
| D14 | 2026-10-13 | **Reference TV = LG C3 (webOS 23, Chromium 94), rooted with Homebrew Channel** | Chromium 94 is the default-skin CSS/JS baseline. Root enables a "native core on TV" mode (Rust core as a homebrew native service). |
| D15 | 2026-10-13 | **Conformance targets #1: AIOStreams (with Comet etc.) for Stremio, Fen Light (and forks) for Kodi** | AIOStreams ⇒ MVP needs no torrent engine; the ranking engine gets a "trust addon order" mode. Fen Light ⇒ the Kodi shim needs a WindowXML-lite dialog renderer (see [04](04-addons.md#primary-target-fen-light-and-its-forks)). |
| D16 | 2026-10-13 | **Team: solo developer + AI assistant, no deadline** | Quality over speed, but build in **thin vertical slices**: each phase ends in something usable daily. Favour small crates, written specs, schemas and conformance tests so AI-assisted work stays correct. |
| D17 | 2026-10-13 | **Kodi custom dialogs: faithful renderer is the baseline; native overrides layered on top** | Overrides only where a tested compat profile exists, each switchable, with automatic fallback to faithful rendering. Built-in `xbmcgui.Dialog()` calls are always native. All Fen Light variants are test targets. |
| D18 | 2026-10-13 | **Native first: Fen Light's features are rebuilt as native addons; trackers (Trakt, Simkl, AniList…) are native addons with a first-class scrobble/sync API** | The core ships no tracker. The Kodi shim is a bridge, not the destination. |
| D19 | 2026-10-13 | **First-party addon order: Simkl tracker (AUTH V2) → TorBox cloud → Trakt tracker. Tracker-to-tracker migration is a core feature** | Owner uses Trakt today, wants to move to Simkl, and uses TorBox. Migration is repeatable, previewed, and supports dual-scrobbling during the switch. |
| D20 | 2026-10-13 | **Usenet/NZB streams, archive streams and the Easynews addon are low priority** | Owner doesn't use Usenet through TorBox; has Easynews but doesn't use it. |
| D21 | 2026-10-13 | **Minimal Simkl tracker, minimal Trakt reader and the Trakt→Simkl migration tool move into Phase 1** | First-party, in-process, on the same tracker interface sandboxed addons use later. No hard-coding. |
| D22 | 2026-10-13 | **iOS ships via sideloading** (AltStore/SideStore/TestFlight) | No App Store dependency. |
| D23 | 2026-10-13 | **Default skin direction: Apple TV/Infuse calm + Netflix/Disney+ hero and rows, with a left rail** | Details in [06](06-ux-input-player.md#visual-direction-of-the-default-skin-d23). |
| D24 | 2026-10-13 | **No built-in scraper, ever; third parties can build scrapers as remote, WASM or Kodi addons** | The host provides generic capabilities only (HTTP, parsing, cache, rate limits). See [04](04-addons.md#scrapers-none-built-in-fully-possible-as-addons-d24). |
| D25 | 2026-10-13 | **Framework architecture: kernel + built-in addons = the app.** Every system is a module on the public API; toggleable, replaceable, extendable | Built-ins are native (no perf cost); third-party is sandboxed. Built-ins are never content sources (D7 still holds). See [03](03-architecture.md#kernel--built-in-addons--the-app-d25). |
| D26 | 2026-10-13 | Live TV / IPTV / EPG: **later, but planned** | |
| D27 | 2026-10-13 | **Good anime support** (reference: Seanime) | Basic anime mapping in Phase 1, depth later. See [09](09-anime.md). |
| D28 | 2026-10-13 | Media scope: **video + music** | Music in a later phase. See [10](10-music.md). |
| D29 | 2026-10-13 | **No visible client/server split by default**: core runs embedded on every device; a remote core is optional configuration | Supersedes the "LAN toggle" framing of D10. |
| D30 | 2026-10-13 | **Android / Android TV / Fire TV right after desktop** | Then iOS + webOS. |
| D31 | 2026-10-13 | Casting: **phone-as-remote/handoff, Chromecast, AirPlay, DLNA/UPnP** | |
| D32 | 2026-10-13 | Profiles: **not urgent (single user per device) but important**; data model profile-aware from day one | |
| D33 | 2026-10-13 | **Downloads/offline viewing are a core feature** | Phase 1 on desktop. |
| D34 | 2026-10-13 | **Play button: Smart by default**, switchable between Smart / Always auto-play / Always list | |
| D35 | 2026-10-13 | **Wide audio support**, with headphone features (virtual surround, dialogue boost) and Windows HDR monitor support prioritised | Owner: HDR monitor + headset. |
| D36 | 2026-10-13 | **UX hard rules**: no autoplay noise, no ads/promos, no laggy navigation, ≤ 2 actions to play | See [06](06-ux-input-player.md#ux-rules-things-we-never-do-d36). |
| D37 | 2026-10-13 | **License: AGPL-3.0 for server/sync crates, GPL-3.0 for the app, MIT for SDKs** | Addon/theme authors can license their work freely. |
| D38 | 2026-10-13 | **No telemetry; opt-in crash reports; auto-update with stable/beta/nightly channels (can be disabled)** | |
| D39 | 2026-10-13 | Name vibe: **cinematic** | Shortlist still to brainstorm. |
| D40 | 2026-10-13 | **TMDb metadata module is built in** (enabled, replaceable, own API key optional) | Metadata only, so bare shell (D7) still holds. |
| D41 | 2026-10-13 | **Built-in removal, middle way:** disabled = zero footprint at runtime; modules are Cargo features so custom builds can exclude them; required slots must keep a provider | See [03](03-architecture.md#removing-built-in-modules-d41-the-middle-way). |
| D42 | 2026-10-13 | **Forks, editions and builds on top are welcome but not specifically promoted** | The point is that people build with it and on it, however they like. GPL/AGPL keeps it open. |
| D43 | 2026-10-14 | **Protocol v2: ours first, open later** | Publish as an open spec once proven. |
| D44 | 2026-10-14 | **Own Rust core; stremio-core is a reference only** | Use it for architecture ideas and protocol test cases. |
| D45 | 2026-10-14 | **Skins use HTML-like single-file templates** (no scripts) | Compiled to SolidJS. |
| D46 | 2026-10-14 | **No official catalog.** Community catalogs only, added by the user | Strongest legal posture; the install screen shows permissions, signatures and source. |
| D47 | 2026-10-14 | **Unsafe developer mode for mods: allowed, opt-in, red banner, never enabled by an install** | |
| D48 | 2026-10-14 | **Stremio account import: one-time** (addons, library, progress); credentials discarded afterwards | |
| D49 | 2026-10-14 | **Ambient: UI sounds (on by default), screensaver, dynamic colors, OS media controls; Discord Rich Presence opt-in** | See [06](06-ux-input-player.md#ambient--presence-d49). |
| D50 | 2026-10-14 | **Monetization decided much later** | The "never paywall features" rule still stands. |
| D51 | 2026-10-14 | **One unified Library** with filters (watchlist, watching, completed, downloaded, local) | |
| D52 | 2026-10-14 | **Primary tracker is master by default; toggle to local-master** | Local copy always kept; switching never deletes data. |
| D53 | 2026-10-14 | **Unified search via a search composer over providers, strongly biased to movies & shows** | Settings/commands in a separate strip. |
| D54 | 2026-10-14 | **Notifications: calendar, in-app inbox, OS notifications, push to phone** | Push via a replaceable provider (ntfy etc.). |
| D55 | 2026-10-14 | **Docs split into `design/` (why), `plan/` (phases → milestones → tasks) and `specs/` (contracts); tasks tracked as Markdown checklists in the repo** | Phase 0–1 broken into tasks; later phases at milestone level until the previous phase is about half done. See [plan](../plan/README.md). |
| D56 | 2026-10-14 | **Only spikes S1–S4 and S7 block Phase 1**; S5 (WASM), S6 (Kodi) and S8 (webOS core) run before their own phases | Phase 1 has no sandboxed third-party code, Kodi or webOS client. |
| D57 | 2026-10-14 | **The GitHub repo is public; CI runs on Windows, macOS and Linux on every push** | Public repos get free Actions minutes, including macOS runners, which bill at 10× on private repos. Fixtures must never contain secrets (T0.1.7 scrubber). |
| D58 | 2026-10-14 | **One branch + PR per milestone/spike, one commit per task ID; each PR is reviewed by GPT-6.1-Sol (Codex, via T3) and merged once review findings are resolved and CI is green** | A second model catches what the authoring model misses; milestone-sized PRs keep review count manageable solo. |
| D59 | 2026-10-14 | **License variants are "-or-later": GPL-3.0-or-later (app), AGPL-3.0-or-later (server/sync), MIT (SDKs)** | Refines D37. Lets the project adopt a future GPL version without relicensing consent from every contributor. |
| D60 | 2026-10-14 | **Protocol clients are sans-IO; HTTP sits behind a small `Transport` trait.** The default transport is reqwest + rustls on tokio; fan-out uses only `futures`, so protocol crates never name a runtime | Owner asked for the most elegant long-term option. Each platform (webOS service, iOS, WASM) can plug in its own HTTP, and conformance tests run on recorded fixtures without the network. See [stremio-client spec](../specs/stremio-client.md). |
| D61 | 2026-10-14 | **Fixtures for addons whose URL holds a secret are recorded by the owner, locally, with `stremio-cli record`**, which scrubs and anonymizes before writing to disk | Keys never pass through chat or an AI tool call; the same command works for any future contributor. |
| D62 | 2026-10-14 | **Stream and subtitle content in committed fixtures is anonymized**: work titles, release groups, info hashes and stream URLs become synthetic; technical tokens (resolution, codec, HDR, size, languages) are kept | The repo is public (D57). Tests need the shape and the ranking-relevant tokens, not real release names. |
| D63 | 2026-10-14 | **S3 conformance set:** AIOStreams, Comet, MediaFusion (owner's configs), Cinemeta, OpenSubtitles v3, Anime Kitsu, TMDB, M3U/EPG TV (iptv-org), Torrentio, WatchHub | The plan's six plus four that cover infoHash, externalUrl and scraper responses. Hyper (owner's find, unconfigured) added as an 11th: `name: null`, paywall `externalUrl` streams mixed with free direct ones, `tmdb:` ids, 404s. |
| D64 | 2026-10-14 | **S1 is tested end to end on Windows only; macOS and Linux prototypes are built in CI, and their documented plan is the fallback (a separate borderless player window under the web UI) until real hardware is available** | The owner has no Mac or Linux machine. Real-hardware tests on those OSes are required before any macOS/Linux release. |
| D65 | 2026-10-14 | **Spike code lives in `spikes/<id>/` as its own Cargo workspace**, outside the main workspace and its lint/CI matrix; a small CI job only checks it builds | Spikes are throwaway quality (Phase 0 rules) and need things the main workspace forbids (`unsafe` FFI, native libs). Code worth keeping moves to `crates/` in Phase 1. |
| D66 | 2026-10-14 | **libmpv: spikes and dev builds use a pinned, SHA-256-checked prebuilt (on Windows, the mpv project's own MinGW GPL libmpv build from its GitHub releases); release builds compile libmpv from source in CI with a recorded recipe** | Both are GPL, which matches the app (D59). A from-source build lets every release name its exact corresponding source (a GPL duty) and control codec flags, but costs days of setup that don't help the spike. Source build is a Phase 1 packaging task. |
| D67 | 2026-10-14 | **Desktop shell = Tauri v2.** Windows compositing: mpv embedded with `wid` = the window's HWND, its `"mpv"` child pushed to `HWND_BOTTOM` after every video reconfig, under a WebView2 with background alpha 0; `vo=gpu-next`, d3d11, `target-colorspace-hint=auto`, `hint-mode=source`. macOS/Linux: separate mpv window under the web window (D64) until tested; the embedded route there is the OpenGL render API (no HDR) | S1: every Windows criterion passed on the owner's HDR monitor with ~40 lines of Win32 beyond Tauri. A custom shell (stremio-shell-ng style) was not needed. Revisit when Tauri 3 is stable. See [S1](../spikes/S1.md). |
| D68 | 2026-10-14 | **In-app OSD dimming over HDR video**: a player setting "OSD brightness over HDR video" (default slightly dimmed), applied only while HDR video plays | In Windows HDR mode the webview is drawn at Windows' SDR content brightness, which the owner found too bright next to HDR video. Doesn't require changing a system-wide setting. |
| D69 | 2026-10-14 | **S4 scope: the < 2 ms budget is measured in WebView2 on the owner's PC and in headless Chromium 94 in CI; the LG C3 number comes from S2's IPK. Two diff methods (a generic value-tree diff and a typed per-struct diff) are each tested with three encodings (JSON as the baseline, MessagePack, postcard)** | A browser page or IPK on the TV isn't worth building twice; S2 packages one anyway. JSON is Tauri's default and `JSON.parse` is fast in V8, so binary has to beat it to be worth the extra decoder. |
| D70 | 2026-10-14 | **View-model patches are typed ops generated from the view-model schema**: keyed list edits (remove / move / insert, by `id`) plus one set per changed field, index-addressed and applied in order; one patch per frame, diffed from the last sent snapshot. The UI applies a frame's structural edits to a working copy of each array and assigns it once | S4: 3× cheaper in the core than a generic value-tree diff (0.13 vs 0.40 ms p95) and smaller. The schema already generates Rust and TS types (T1.1.2), so it generates the diff and apply too. Batching array edits took a full reshuffle from ~5 to ~0.5 ms in the Solid store. See [S4](../spikes/S4.md). |
| D71 | 2026-10-14 | **Patch encoding is JSON for now; MessagePack is dropped; JSON vs postcard is re-measured on the LG C3 in S2**, switching to postcard only if JSON is the bottleneck there | S4: `JSON.parse` decoded as fast as a hand-written postcard decoder and 3–7× faster than the MessagePack JS library on big frames. Postcard is 2–4× smaller but positional, so core and UI would need the exact same schema version, which hurts a remote core of another version and skins/mods reading view-models. |
| D72 | 2026-10-14 | **Desktop UI ↔ embedded core over Tauri IPC**, frames pushed through a Tauri `Channel` as raw bytes; WebSocket (per-device token) only for a remote core | S4: both fit the budget (Tauri round trip ~1.0–1.4 ms p95, localhost WebSocket ~0.3 ms; end to end 1.4–1.8 vs 0.7 ms p95). Tauri IPC opens no port, so nothing else on the machine can connect. |
