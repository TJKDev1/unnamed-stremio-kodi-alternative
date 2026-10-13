# 07 — Roadmap

No dates yet. Team size and time budget are open questions; see [08](08-open-questions.md). Phases are ordered by risk first, then value.

Team: solo + AI, no deadline (D16). Each phase must end in something you can use daily. Built-in systems are written against public provider interfaces from Phase 1 (D25), even before third parties can replace them.

## Phase 0: Technical spikes (de-risk before committing)

| # | Spike | Pass criteria |
|---|-------|---------------|
| S1 | **libmpv under a transparent webview** (Tauri v2 / custom shell) on Windows, then macOS and Linux | No tearing; correct HDR on Windows; resize without flicker; web OSD over video at 60 fps |
| S2 | **Web UI performance on your LG TV**: SolidJS virtualized rows, 10 rows × 50 posters, focus engine | 60 fps scroll, < 2-frame focus response, memory within app limits |
| S3 | **Rust Stremio protocol client** against 10 real addons, including configurable ones; **your AIOStreams config first** | All resources and extras parse; streams arrive progressively |
| S4 | **Core ↔ UI IPC**: view-model patches over Tauri IPC and over WebSocket (home server) | < 2 ms per frame of patches at typical load |
| S5 | **WASM addon host**: Extism + a TS addon compiled to WASM doing HTTP + parsing | Cold call < 50 ms; permission enforcement works |
| S6 | **Kodi shim proof**: run one simple video plugin through embedded CPython, then **Fen Light's main menu + one `WindowXMLDialog` (`select.xml`)** via a WindowXML-lite prototype | Folder listing + resolve + a blocking `Dialog().select()` round-trip; Fen Light's select dialog renders and returns a choice |
| S7 | **Skin compiler proof**: one `.skin` component compiled to SolidJS with hot reload | Override `home.row` without touching the default skin |
| S8 | **Rust core as a webOS homebrew native service** on the C3 (cross-compile, register a Luna service, talk to the web UI) | Core starts on boot/app launch, serves Stremio catalogs to the UI, memory stays reasonable |

## Phase 1: Desktop MVP

- Rust core: state machine, SQLite, cache, image service, addon host.
- **Stremio addons**: full protocol, configure flow with hidden secrets, Stremio account import.
- Default skin: left rail + rows, detail page, search, library/continue watching, calendar.
- **Catalog organizer** (reorder/hide/rename/merge rows).
- **Stream ranking engine** with presets and "explain ranking".
- **Player**: mpv backend, **HDR output on Windows HDR monitors**, power settings, subtitles/audio rules, **headphone features** (D35), external player support.
- **Focus engine + controller/remote support + full input remapping**.
- **Tier 0/1 theming**: settings + token themes + theme editor.
- Profiles (local; can be basic at first, but the data model is profile-aware from day one, D32).
- **Downloads** (D33): download queue for HTTP/debrid streams, quality choice, storage limit, auto-delete after watched.
- **Basic anime support**: `kitsu:` IDs, ID/episode mapping service with offline dumps, anime ranking fields and language rules ([09](09-anime.md)).
- **Smart Play button** (D34) with all three modes.
- **Tracker interface (core API) + first-party Simkl tracker** (AUTH V2 device flow, scrobbling, two-way history) and a **minimal Trakt tracker** (read history/watchlist/ratings, optional dual-scrobble), so the **Trakt→Simkl migration tool** works from day one. Both run in-process on the same tracker interface that sandboxed WASM addons get in Phase 4, and move into the sandbox then.
- Local folders as sources, matched into the same catalogs (basic scanner).
- Safe mode, log viewer, addon inspector.

## Phase 2: Android & Android TV (D30)

- **Android + Android TV / Google TV / Fire TV**: Android WebView UI + Rust core in-process, mpv (mpv-android builds) + **Media3** for DV/passthrough/refresh-rate. Largest TV-box market, so it matters for adoption.
- Core embedded on-device by default (no visible server, D29).
- Phone-as-remote and "play on another device" between desktop and Android.

## Phase 3: Your devices (iOS + LG webOS)

- **LG webOS client**: core as a homebrew native service on rooted TVs (S8), standalone WASM core otherwise; optional remote core.
- **iOS client** (WKWebView + AVPlayer/MPVKit; sideload distribution).
- Optional remote-core/home-server configuration + LAN pairing; sync between devices.
- **Casting**: Chromecast, AirPlay, DLNA/UPnP targets (D31).

## Phase 4: The better ecosystem

- **Protocol v2** spec + JSON Schemas + conformance tests.
- **Addon SDKs** (TS + Rust) with `--target http|wasm`, `addon dev` hot reload.
- **WASM addon host** with permissions, background jobs and OAuth device-flow helper.
- **Built-in providers made replaceable** via the WASM host (ranker, parser, mapper, scanner…, D25).
- **First-party native addons** (also the proof that v2 is good enough): Simkl and Trakt trackers moved into the WASM sandbox, **TorBox cloud**, TMDb metadata. (Easynews later, low priority.) The goal is that they replace Fen Light for daily use.
- **Torrent engine plugin** (librqbit).
- **Tier 2 skins** (public skin API, compiler, dev tools) and layout presets.
- **Tier 3 mods** (worker sandbox, slots, commands, permissions).
- Community catalog support for addons, themes, skins and mods (we host none, D46).

## Phase 5: Kodi, anime depth & library

- **Kodi Python shim** (pluginsource + modules + services subset, repository client, settings.xml UI), **WindowXML-lite dialog renderer** and semantic overrides; done when **Fen Light** works end to end.
- **Anime depth** ([09](09-anime.md)): AniList/MAL/Kitsu trackers, OP/ED skip and filler providers, AniList-style season view. (Basic anime ID mapping and `kitsu:` support already land in Phase 1.)
- Jellyfin/Plex/Emby as sources.
- Improved local library (NFO, artwork, fix-match, file watching).
- **Live TV / IPTV / EPG** (D26: later, but planned).

## Phase 6: Reach & music

- **Music** ([10](10-music.md)): local library, music addons, Navidrome/Subsonic/Jellyfin/Plex, Last.fm/ListenBrainz.
- Optional hosted sync/relay (if we decide to offer one).
- Samsung Tizen (web app, similar to webOS).
- Usenet/archive plugin (low priority, D20).

## Engineering practices from day one
- Performance budgets as CI tests (headless Chromium + a real-device lab run on the LG TV).
- Protocol conformance suites (Stremio v1, our v2, Kodi shim).
- Public, versioned API docs for themes, skins, mods and addons, generated from the same schemas the code uses.
- Crash reporting **opt-in only**.
