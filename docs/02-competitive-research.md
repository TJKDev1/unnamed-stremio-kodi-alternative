# 02 — Competitive research

Researched 2026-10-13. Sources are linked inline. A few items come from secondary sources (Reddit and blog mirrors). Treat those as signals, not facts.

---

## Stremio

### How it works
- **stremio-core** (Rust): a shared state machine with an Elm-style model/update design. It covers addon transport, library, sync, player state and settings, and compiles to WASM for the web UI. ([docs](https://www.mintlify.com/Stremio/stremio-core/introduction))
- **stremio-web** (React) runs on top of the core compiled to WASM, and every desktop shell reuses it. Stremio 5 shells:
  - Windows: Rust + WebView2 + mpv drawing straight into the window ([stremio-shell-ng](https://github.com/Stremio/stremio-shell-ng)).
  - Linux: GTK4/WebKitGTK + libmpv, on Flathub since about August 2026.
- **Streaming server**: a closed-source Node bundle on `localhost:11470`. It handles torrents, HLS transcoding, header proxying, Usenet and archive streaming, and casting.
- **Mobile and TV apps are closed source.** Android TV uses Media3 with a libVLC fallback.
- **Addons** are stateless HTTP/JSON services. Protocol details are in [04](04-addons.md#stremio-addon-protocol-must-be-100-compatible).

### Flaws, and how we respond

| Stremio flaw | Our answer |
|---|---|
| No native addon reordering for about 10 years. Users had to reinstall addons or hand their authKey to third-party managers. | Drag-to-reorder for addons, plus **separate** priority settings for catalogs, metadata and streams. |
| Cinemeta is "protected" and can't be removed ([#567](https://github.com/Stremio/stremio-features/issues/567)) | Nothing is protected. |
| Removing an addon removes all of its catalog rows; you can't hide just one row | Per-row hide, rename, reorder and merge, per profile ([06](06-ux-input-player.md#catalog-organizer)) |
| **Catalog organizing and profiles moved behind a $40/yr Supporter tier** (June 2026) | Free, always. |
| Addon config lives in the URL, so **debrid keys leak** whenever a URL is shared, and editing config means reinstalling | Config is stored client-side in the secret store and sent in a header (native protocol). For legacy addons we hide the URL and show the config through a UI. |
| Users need external aggregators (AIOStreams) just to filter, dedupe and rank streams | A **built-in stream ranking engine** ([06](06-ux-input-player.md#stream-ranking-engine)) |
| Player gaps: no audio delay, night mode or downmix, crop/zoom, scrubber thumbnails or PiP. External players lose resume. | mpv with every option exposed, plus external players with resume tracking via IPC |
| Subtitles randomly disappear or re-enable each episode; no forced-subtitle auto-select | Per-profile subtitle rules: language priority, forced/SDH handling, remembered per show |
| Addons can't receive playback events ([#824](https://github.com/Stremio/stremio-features/issues/824)), so only first-party scrobbling is possible | Playback event hooks in the native protocol |
| No real local library or offline downloads | Local folders are first-class sources, merged into the same catalogs ([06](06-ux-input-player.md#local-files-are-not-a-separate-ghetto)) |
| Sync requires a cloud account | Local-first; sync is optional |
| Closed streaming server causes bugs nobody else can fix ("Stream is not supported", no audio) | Everything is open. Torrents are an open Rust plugin (librqbit). |
| Store-policy risk: iOS Lite was pulled in Jan 2026; webOS was delisted, then relisted around Aug 2026 | Plan sideloading and web-app distribution from day one ([03](03-architecture.md#platform-matrix)) |
| Torrentio rate limits and Real-Debrid's May 2026 filtering made the ecosystem fragile | Not ours to fix, but client-side ranking plus local WASM addons cut the dependency on hosted aggregators |

### Strengths to keep
- The addon protocol: language-agnostic, cacheable, installed from a URL. **Being compatible means instant access to the whole ecosystem.**
- Metadata and streams are separated by ID (`tt…`, `kitsu:…`), so any addon can supply streams for any title.
- Zero-setup onboarding compared with Kodi.
- Binge auto-next (`bingeGroup`), continue watching, calendar, EPG support.
- One Rust core across platforms.

---

## Kodi

### How it works
- `addon.xml` declares extension points: `xbmc.python.pluginsource` (virtual directories), `script`, `service`, `module`, `repository`, `gui.skin`, `context.item`, scrapers, subtitles; plus binary C++ addons such as inputstream.adaptive and PVR clients.
- Python API modules: `xbmc`, `xbmcgui`, `xbmcplugin`, `xbmcaddon`, `xbmcvfs`. Plugins are invoked through `plugin://` URLs with `sys.argv = [base, handle, query]`. A fresh interpreter starts for every navigation step unless the addon sets `reuselanguageinvoker`.
- Skins are XML files mapped to hard-coded window and control IDs, with includes, variables, visibility expressions and animations.
- Current versions: 21.3 Omega is stable, 22 Piers is at RC1 (Python 3.14, FFmpeg 8).

### Flaws, and how we respond

| Kodi flaw | Our answer |
|---|---|
| Setup sprawl: sources, scrapers, advancedsettings.xml, settings tiers that are hard to find | Searchable settings, Basic/Advanced/Expert levels, every setting shows its effect, per-setting reset |
| Sluggish UI on cheap boxes; slow plugins freeze navigation behind a spinner | UI never blocks on addons. Results stream in progressively and each addon has its own timeout. |
| A fresh Python interpreter for every navigation step | The shim keeps a warm interpreter pool per addon ([04](04-addons.md#kodi-python-shim)) |
| Python 2→3 (v19), the InfoTag change (v20) and the `sys.argv` change (v21) broke addons; `xbmc.gui` bumps break skins every release | Semver-versioned plugin and skin APIs, compatibility shims for N−2 versions, deprecation warnings |
| Skins use magic window/control IDs, verbose XML, absolute 1080p layouts, and no component or state model | Skins are component-based, bind to typed view-models, use responsive layout, and hot-reload ([05](05-theming-and-modding.md)) |
| Library scanning is slow and match fixes are manual | Incremental filesystem watching, parallel scraping, inline "fix match" |
| Multi-device library needs a shared MySQL that breaks on upgrades | The home server *is* the shared library; or sync over our protocol |
| Piracy-build reputation (Amazon pulled Kodi) | Bare shell; we host no catalog at all, only community catalogs the user adds |
| Poor touch and mobile UX | Every component designed for pointer, touch and D-pad |
| Debugging means reading kodi.log | In-app log viewer and an addon inspector (requests, timings, errors) |

### Strengths to keep
- **Player quality**: FFmpeg-based, HDR10/HDR10+/Dolby Vision, refresh-rate matching, passthrough, ASS/PGS subtitles.
- **Remote and controller navigation** that works everywhere; keymaps; CEC.
- **The skinning culture**: total visual freedom and a huge library of community skins.
- A local library with NFO files and artwork that needs no server.
- PVR/Live TV with EPG.
- A JSON-RPC remote API (used by the Kore and Yatse remote apps). **We should offer the same: a documented local API for remotes and automation.**

---

## Other apps: lessons

| App | Lesson |
|---|---|
| **Stremio Native** (Rust + Slint + libmpv, Windows) | Reports about 360 MB of RAM vs 814 MB for the official app, and 1 process vs 10. **A Rust-native core is much leaner.** It also ships layout presets. ([GitHub](https://github.com/perpetus/stremio-native)) |
| **Rillio** (Tauri v2 + stremio-web + libmpv + librqbit) | **Closest to our planned desktop stack.** Locks the webview down with a CSP and an allowlist of mpv commands. Uses libplacebo for Dolby Vision. ([GitHub](https://github.com/pek100/rillio)) |
| **Plex** redesign backlash (2024–26) | Removing the sidebar and adding clicks forced a rollback. **Never force a navigation change; keep a left rail on TV; always offer an opt-out.** |
| **Jellyfin Desktop** | Web UI on top with mpv underneath works well, but toolkit churn (Qt5→6) cost years. Keep the shell thin. |
| **Findroid / Emby Theater** | Exposing `mpv.conf` editing is cheap and power users love it. |
| **Swiftfin** | Making users choose a player per file is a UX failure. **Pick the best engine for each file automatically, and let the user override it.** |
| **Infuse** | The quality benchmark on Apple: DV P5/P8, Atmos, direct play by default. Apple users pay for quality plus sync. |
| **Moonfin** | Flutter on nine platforms plus a runtime community theme catalog. Proof that a theme catalog drives engagement. |
| **Plezy** | Multiple server types in one app with cross-server search; HDR and audio badges on cards; a "force TV mode" on desktop. |
| **NuvioTV / Omni / Vidi / Fusion** | Stremio-addon clients exist and users pay for them, but Vidi supports only a subset of the protocol. Full compatibility is a selling point. |
| **Syncler / Wako** | Depending on Trakt's API and third-party "provider packages" is fragile. |
| **Obsidian / BetterDiscord / Vencord** | Unsandboxed plugins lead to real malware incidents. **Sandbox mods from day one.** |
| **VS Code** | A separate extension host keeps the UI responsive (but is not a security boundary). |
| **Playnite** | Separate desktop and fullscreen (controller) themes. We should allow per-mode skin variants. |

No working bridge between the Stremio and Kodi addon ecosystems exists. **Supporting both in one app is a gap nobody has filled.**
