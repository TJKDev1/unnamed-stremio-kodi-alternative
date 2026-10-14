# 06 — UX, input & player

## Navigation model (default skin)

- **Persistent, collapsible left rail**: Home, Discover, Library, Search, Calendar, Addons, Settings, plus user-pinned items. It collapses to icons and expands on focus or hover. (Lesson from Plex: never remove it without an opt-out.)
- **Home = catalog rows**, fully user-controlled (see the catalog organizer below).
- **Detail page**: hero art, metadata, seasons/episodes, a big **Play** button and a **Streams** button for the full ranked list.
- **Play button modes (D34)**, switchable globally, per profile and per content type:
  1. **Smart (default)**: auto-plays the top stream when it clearly wins (e.g. matches your preset and is cached/instant, with a clear score gap to #2); otherwise opens the stream list with the best one focused.
  2. **Always auto-play**: plays the top-ranked stream immediately.
  3. **Always show list**: Stremio-style, you pick each time.
  - Whatever the mode, a long-press / secondary button does the opposite (list ↔ play), and "explain ranking" is one press away.
- **Every layout choice is configurable** and other layouts ship as presets/skins.

### Visual direction of the default skin (D23)
A blend of **Apple TV / Infuse** (calm, artwork-first, glassy surfaces, restrained motion, excellent typography) and **Netflix / Disney+** (hero banner, dense rows, previews on focus), combined with the left rail.
- **Home**: a hero/billboard at the top (featured or continue-watching item with logo art, backdrop, one-line synopsis, Play / More), then rows. The hero can be disabled or replaced with a compact header.
- **Focus = preview**: focusing a card crossfades the page backdrop to that title's art and shows a short info strip (year, runtime, rating, quality badges such as 4K/DV/Atmos). Netflix-style trailer autoplay on focus is a setting: **off by default on TV**, delay and mute configurable.
- **Surfaces**: dark by default, translucent/blurred panels Apple-style where the engine can afford it (blur is a token that auto-disables on low-end devices and in TV "lite" mode).
- **Cards**: poster and landscape (Disney+-style) both first-class; logo-over-backdrop landscape cards for continue-watching.
- **Motion**: short, eased, focus scale-up + soft shadow; respects `motion.scale` and reduced-motion.
- **Detail page**: full-bleed backdrop, title logo instead of plain text when available, episodes as horizontal landscape strips (Apple TV-style).

## UX rules: things we never do (D36)
From the owner's dislikes. These are hard rules for the default skin and core behaviour; skins/mods may opt into anything the *user* chooses.
- **No autoplay noise**: trailers/previews never autoplay with sound; autoplay previews are off by default on every device, and when enabled they're muted and only start after a configurable focus delay. No autoplaying hero video.
- **No ads, promos or "featured" rows we pick.** Every row on screen is there because the user or one of their addons put it there, and it can be removed.
- **No laggy navigation**: no blocking spinners; cached-first rendering; rows render immediately with placeholders; focus never waits on the network (see the [performance budget](03-architecture.md#performance-budget)).
- **Minimal clicks**: **≤ 2 actions from Home to playback** (focus title → Play). Settings reachable via search in one action. Anything used often can be pinned to the rail or bound to a key/button.

## Two input modes, both first-class

| | Pointer mode (mouse/touch) | Focus mode (keyboard/D-pad/controller/remote) |
|---|---|---|
| Switch | Automatic on last input used | Automatic; "Force TV mode" setting on desktop |
| Hover/focus | Hover previews, scroll wheel on rows | Spatial focus with remembered position per row, focus restored on Back |
| Density | Compact option | Larger targets, overscan-safe margins |

### Focus engine
- A single focus engine in the UI layer (built on ideas from Norigin Spatial Navigation / BBC LRUD): spatial by default, with **focus groups**, **memory per group**, **traps** for dialogs, and explicit overrides for skins.
- Focus changes are **synchronous and local**: never waiting on data. Placeholders and skeletons are focusable.
- Long-press and hold-to-repeat with acceleration for fast scrolling through long rows.

### Input remapping (must-have)
- **Every action is a named command** (`nav.up`, `player.seek.forward.small`, `player.subtitles.cycle`, `app.search`, `mod.random-pick`…).
- Bind any command to keyboard keys, controller buttons/axes, remote keys (CEC, LG Magic Remote, media keys) and **chords/long-press**.
- **Per-device profiles** (Xbox pad vs DualSense vs LG remote) with correct **button glyphs** in the UI.
- Context-aware bindings (different in player vs browse).
- Import/export keymaps; mods can register new commands.
- Desktop: **command palette** (Ctrl+K) reaching any command or setting.

## Catalog organizer (must-have, free)

- **Every row is an object you control**: reorder, hide, rename, change card style (poster/landscape/compact), set item count, pin to rail.
- **Merge rows** (e.g. "Trending" from three addons → one deduped row).
- **Custom rows**: a saved filter over any catalog(s) or your library ("Unwatched 4K sci-fi in my library", "Continue watching, anime only").
- Per profile. Changes sync if sync is on.
- New addon installs add their rows **at the bottom, collapsed** (configurable) so your home screen isn't reshuffled.

## Stream ranking engine (must-have)

The built-in version of what AIOStreams does externally:

1. **Collect** streams from all enabled stream addons in parallel; show them as they arrive.
2. **Parse**: use structured v2 fields when present; otherwise a robust parser for names/descriptions (resolution, HDR/DV, codec, audio, languages, size, release group, cached/debrid markers, seeders).
3. **Filter**: rules like `resolution >= 1080`, `exclude: CAM/TS`, `size <= 20GB`, `languages contains nl or en`, `regex`, `exclude DV profile 5 if device can't do DV`, `max per addon`.
4. **Dedupe** by hash/filename/size across addons.
5. **Sort** by a user-defined priority list (e.g. cached > resolution > HDR > audio > size closeness to target > addon priority).
6. **Format**: user-defined templates for how each stream row looks.
7. **Auto-play**: "Play" picks the top result; with binge mode, next episodes reuse the same `bingeGroup`/release where possible.

- **Works with aggregators, not against them.** Each addon can be set to "trust this addon's order" (e.g. AIOStreams, which already filters and sorts), and its results are then kept as-is. You can still apply only the device-aware rules on top, or merge it with other addons' streams using its order as a strong signal.
- Rule sets are **presets**, per profile and per device (the TV preset prefers DV, the laptop preset prefers small files).
- **Device-aware**: the core knows what the current player/display can decode (HDR, DV profile, audio passthrough) and can de-rank incompatible streams automatically.
- Shareable as files. Simple UI for basic users, plus a rule editor (and raw TOML) for power users.
- An "Explain ranking" view shows why a stream is in position #1.

## Local files are not a separate ghetto

From the requirements: local media mostly supports streaming and must neither interrupt it nor be so separated that it's useless.

- Local folders (and SMB/NFS/WebDAV via the home server) are scanned **incrementally in the background** with low priority. Scanning never blocks the UI or playback.
- Local files are **matched to the same IDs** (IMDb/TMDB/Kitsu) as addon content, so:
  - On a title's detail page, a local copy appears as the **top stream source** ("Local · 2160p DV · Atmos") alongside addon streams, ranked by the same engine.
  - The library and continue-watching treat local and streamed playback as one history.
  - Local items can feed custom rows ("In my library").
- Unmatched files go into a "Needs attention" list with inline fix-match, not a separate mode.
- Optional NFO/artwork read support (Kodi-compatible) for people migrating libraries.

## Profiles (free)
- Multiple profiles with optional PIN, kids mode (age-rating filter + allowed addons), and per-profile addons, rows, ranking, theme/skin, subtitle/audio preferences and history.

## Player

Must-haves (power settings):
- **mpv.conf / input.conf editor** (desktop), shader presets (e.g. upscalers), `vo=gpu-next` with HDR tone-mapping controls.
- **Audio (wide support, D35)**: track selection rules by language, **audio delay**, **night mode / dynamic range compression**, **stereo downmix**, normalization (EBU R128), passthrough toggles per codec (AC3/E-AC3/TrueHD/DTS/DTS-HD, Atmos/DTS:X) for AVR/soundbar users, output device selection, exclusive mode (WASAPI) on desktop.
- **Headphones** (owner uses a headset): **virtual surround / HRTF binaural downmix** of 5.1/7.1 (mpv/ffmpeg `sofalizer` with selectable HRTF profiles), dialogue boost (centre-channel emphasis), and per-output-device audio presets (headphones vs speakers vs AVR switch automatically when the output changes).
- **Subtitles**: language priority rules, forced/SDH handling, styling (font, size, color, outline, position, background), **subtitle delay**, ASS override toggle, per-show memory, addon subtitle search, auto-sync (optional, via audio alignment later).
- **Video**: zoom/crop/aspect presets (ultrawide), deinterlace, refresh-rate matching (where the platform allows), HDR passthrough vs tone-map. **Windows HDR monitors** (owner's setup): proper HDR10 output via mpv `gpu-next` on d3d11, auto-switching Windows HDR mode on/off for HDR content (optional), SDR-in-HDR brightness setting, Dolby Vision → HDR10/tone-mapped via libplacebo.
- **Navigation**: scrubber thumbnails (via a second mpv instance or keyframe extraction in the core), chapter markers, **skip intro/recap/credits** (via v2 `segments` addons; chapters as fallback), next-episode countdown, hold-to-2x, frame step.
- **Stats overlay** (codec, bitrate, dropped frames, buffer, HDR metadata).
- **External players** (VLC, mpv, MPC-HC, Infuse on iOS, etc.) with resume and progress tracking via IPC/callbacks where the player supports it.
- **Engine auto-selection** per file with manual override (see [03](03-architecture.md#player-abstraction)).
- PiP / mini-player on desktop and iOS.

## Downloads (D33, core feature)
- Download any HTTP/debrid stream (and local-network files) for offline playback; works on desktop, Android and iOS.
- Queue with pause/resume, parallel limit, bandwidth limit, Wi-Fi-only option on mobile.
- Quality choice at download time (or "use my ranking preset"), plus external subtitles saved alongside.
- Storage limit per device; **auto-delete after watched** (configurable delay); "download next N episodes" for series.
- Downloaded items appear as the top "Local · Downloaded" stream for that title, and progress/scrobbling works offline (queued tracker events).
- The downloader is a built-in, replaceable provider (e.g. hand off to TorBox cloud or aria2).
- Torrent streams can only be downloaded via the optional torrent plugin.

## Casting & other devices (D31)
- **Phone as remote / handoff**: browse on the phone, press Play, choose "Play on: Living room TV". Our own app-to-app protocol over the LAN (Local API), so it carries full features (subtitle choice, resume, ranking). The phone becomes the remote with a rich OSD.
- **Chromecast**, **AirPlay** (the LG C3 is an AirPlay 2 receiver), **DLNA/UPnP** renderers as cast targets. When a target can't play a stream's format, the core can remux/transcode via ffmpeg if a capable device is available, otherwise it warns and offers compatible streams (ranking is target-aware).
- Casting targets are built-in, replaceable providers.

## Live TV / IPTV (D26, later)
- Planned for a later phase: Stremio live/EPG addons, M3U + XMLTV, a guide grid, channel favourites and catch-up where the source supports it.

## Library (D51): one unified library
- A single **Library** screen with filters/tabs: **Watchlist** (saved), **Watching**, **Completed**, **Downloaded**, **Local files**, plus custom smart filters ("unwatched anime", "4K in my files").
- Tracker lists (Simkl/Trakt watchlist, history) sync **into** it; local files and downloads are just more filters. Nothing lives "somewhere else".
- Every item shows where its state comes from (e.g. "from Simkl", "local file") on hover/focus.

## Watch progress: source of truth (D52)
- **Default: the primary tracker is master.** When a tracker is connected, you pick one as **primary** (e.g. Simkl). Its watched state, progress and watchlist win conflicts; the app keeps a local copy as cache and offline queue, and other trackers mirror.
- **Toggle: local master.** The app's own history wins and trackers mirror it (useful offline-heavy setups or during a tracker migration).
- With no tracker connected, local history is the master automatically.
- Switching master never deletes data; a dry-run diff is shown first (same machinery as tracker migration).

## Search (D53)
**One search box, everything, with movies and shows first**, done elegantly through the provider system:
- Search is a **multi-slot**: catalog addons, library, local files, people, settings/commands (and music later) each return typed results in parallel; a built-in **search composer** merges them.
- **Movies & shows always lead**: results from different addons are deduplicated by ID into one result per title, ranked by relevance plus type weights (movies/shows > library > people > anime-specific entries > settings/commands). Weights are configurable.
- Results stream in as providers answer; slow addons never block the first results.
- Settings and commands appear as a compact "Also found" strip at the bottom, never mixed into title results.
- Typo-tolerant suggestions while typing, recent searches, and voice input wherever the platform keyboard/remote provides it (LG Magic Remote, phones). No custom speech engine.

## Notifications (D54)
- **Calendar** of upcoming episodes (from trackers and metadata).
- **In-app inbox** (new episodes of shows you watch, finished downloads, addon updates, sync conflicts).
- **OS notifications** on desktop and mobile.
- **Push to phone** via a notification provider module (e.g. ntfy, Pushover, or our app on the phone through the remote core). Replaceable like everything else.
- Per-show "notify me" toggles; quiet hours.

## Ambient & presence (D49)
All are built-in, replaceable modules and themeable:
- **UI sounds**: subtle navigation/select/back sounds, **on by default** (owner leaning), with a volume slider and one-tap off; sound packs are part of themes.
- **Screensaver**: artwork slideshow, clock, now-playing; starts after an idle delay; screensaver styles are skinnable.
- **Dynamic colors**: accent/background tinted from the focused or playing artwork (computed in the core); on by default in the default skin, a token switch for other themes.
- **Now-playing presence**: OS media controls (Windows SMTC, macOS Now Playing, MPRIS on Linux, iOS lock screen, Android media session) **on by default**; **Discord Rich Presence off by default** (opt-in, with privacy options such as hiding the title).

## Settings UX
- One searchable settings screen; levels **Basic / Advanced / Expert** (like Kodi, but discoverable via search at any level).
- Every setting: description, preview where visual, "reset to default", and an indicator if a profile/theme overrides it.
- All settings exportable/importable as a file. Everything the UI can set, a config file can set too.

## Onboarding (bare shell)
1. Welcome → choose input mode/preset (desktop or TV).
2. **"Bring your stuff"**: import from a Stremio account or a list of addon URLs; point at a Kodi install to import plugin addons; add local folders.
3. Or start empty: a friendly empty state explaining what addons are and how to find addon catalogs. **We don't recommend specific content sources.**

## Accessibility & i18n
- Screen-reader labels from view-models, high-contrast theme, reduced motion, scalable type, full keyboard operation.
- UI localization from day one (string tables, RTL support); metadata language preference passed to addons.
