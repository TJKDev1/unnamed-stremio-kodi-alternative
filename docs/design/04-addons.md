# 04 — Addon systems

The app supports three addon ecosystems and puts them all behind one internal interface. The rest of the app (catalogs, detail pages, stream picker, player) doesn't care where data came from.

```
             ┌─────────────────────── Addon host (core) ───────────────────────┐
  intents →  │  Resource router: catalog · meta · stream · subtitles · events  │
             │         │                 │                    │               │
             │  StremioAdapter     NativeAdapter          KodiAdapter          │
             │  (HTTP v1)          (HTTP v2 + WASM)       (sidecar IPC)        │
             └─────────────────────────────────────────────────────────────────┘
                                   ▼
                     normalized models: Meta, Video, Stream, Subtitle
                     + ranking engine + cache + per-addon stats
```

Per-addon features, whatever the ecosystem:
- Enable/disable.
- Separate priority for **catalogs**, **metadata** and **streams**.
- Timeout and concurrency limits.
- Optional proxy.
- Per-profile enablement.
- An **inspector** showing request log, latency, error rate and cache hits.

---

## Stremio addon protocol (must be 100% compatible)

Routes: `GET {base}/manifest.json`, `GET {base}/{resource}/{type}/{id}.json`, `GET {base}/{resource}/{type}/{id}/{extra}.json`.
`extra` is a query-string-encoded object (`search=…&skip=100&genre=…`). Transport URLs: `https://…/manifest.json`, legacy `/stremio/v1`, and `ipfs://`/`ipns://` (low priority).

| Area | Must support |
|---|---|
| Manifest | `id, name, version, description, resources` (string or `{name, types, idPrefixes}`), `types`, `catalogs[] {type, id, name, extra[{name,isRequired,options,optionsLimit}]}`, legacy `extraSupported/extraRequired`, `addonCatalogs`, `config[]`, `behaviorHints {adult, p2p, configurable, configurationRequired, epgProvider}`, `logo, background` |
| Resources | `catalog`, `meta`, `stream`, `subtitles`, `addon_catalog` |
| IDs | `tt…`, `tt…:S:E`, `kitsu:…`, custom prefixes; movie without `videos` ⇒ video id = meta id |
| Meta | Full meta + preview objects, `videos[]` (incl. embedded `streams`, which suppresses other stream addons), `links`, `trailers`, `behaviorHints {defaultVideoId, isLive, hasScheduledVideos}` |
| Streams | `url, ytId, infoHash(+fileIdx, fileMustInclude, sources), externalUrl, nzbUrl+servers, rarUrls/zipUrls/7zipUrls/tgzUrls/tarUrls`, `behaviorHints {bingeGroup, notWebReady, proxyHeaders, countryWhitelist, videoHash, videoSize, filename}` |
| Subtitles | `{id, url, lang}`, with `videoHash`/`videoSize` passed in extra |
| Caching | `cacheMaxAge`, `staleRevalidate`, `staleError` |
| EPG | `GET /catalog/tv/{id}/date=YYYY-MM-DD.json` → `metasDetailed` |
| Configure | Opening `{base}/configure`. We embed it in a sandboxed in-app webview, capture the resulting manifest URL, and **hide the secret-bearing URL** from the UI and from shareable exports. |
| Import | Import the addon collection + library from a Stremio account (`addonCollectionGet`, `datastoreGet`), or from a pasted list of URLs |

Streams with `infoHash` need the torrent plugin. `nzbUrl` and archive streams need a future Usenet/archive plugin. Without one, the stream shows as "requires plugin X" instead of disappearing.

The Stremio protocol test suite: build a conformance test harness against ~20 popular addons (Cinemeta, OpenSubtitles, AIOStreams, Torrentio-style, MediaFusion, Comet, TMDB, Anime Kitsu, IPTV/EPG addons).

### Primary target: AIOStreams (+ Comet and other scrapers behind it)
This is the owner's daily setup, so it is **conformance target #1**.
- AIOStreams is a configurable addon that aggregates scrapers (Comet etc.) and returns **already filtered, sorted and formatted** debrid HTTP streams. It has its own expression language (SEL) and formatter templates ([docs](https://guides.viren070.me/stremio/addons/aiostreams/documentation-v1)).
- This setup **needs no torrent engine**: streams are plain HTTPS debrid URLs. The desktop MVP is fully usable without the torrent plugin.
- **Don't fight it.** The ranking engine gets a per-addon **"trust this addon's order"** mode (default for addons that declare themselves pre-sorted), so we don't re-sort AIOStreams output. Our rules can still apply on top if the user wants (e.g. device-aware de-ranking of DV profile 5 on a non-DV display).
- Its config URL contains debrid keys, so this is the main test case for the hidden-secret config flow.
- AIOStreams is open source and moves fast, which makes it the **ideal first adopter of protocol v2's structured stream fields**. Reach out to its maintainer once v2 is drafted (see [08](08-open-questions.md) Q19).

---

## Kodi Python shim

**Goal:** run Kodi **video plugin** addons (`xbmc.python.pluginsource` with `<provides>video</provides>`) plus their dependencies (`xbmc.python.module`), and a subset of `xbmc.service`. Desktop and home server only.

### Design
- **Sidecar process `kodi-host`** with an embedded CPython (3.12+; follow Kodi 22's 3.14), started on demand. Crashes or hangs never take down the app.
- **A warm interpreter pool per addon**, so we don't pay Kodi's per-navigation startup cost. We emulate the `reuselanguageinvoker` semantics and still give each addon the fresh state it expects.
- The shim implements the `xbmc`, `xbmcgui`, `xbmcplugin`, `xbmcaddon` and `xbmcvfs` modules in Python. They talk to the core over a JSON-RPC pipe.
- **Mapping:**
  - `addDirectoryItem`/`endOfDirectory` → our catalog/folder view-model.
  - `ListItem` (labels, `setArt`, `InfoTagVideo`, properties) → `Meta`/`Video`.
  - `setResolvedUrl` → `Stream`. `inputstream.adaptive` properties map to native HLS/DASH where possible; **DRM/Widevine is out of scope**.
- **Blocking dialogs** (`Dialog().select/yesno/input/ok`, `DialogProgress`, `Keyboard().doModal()`): the Python thread blocks while the core shows our own themed dialog and returns the answer. They look native in every skin.
- `settings.xml` (legacy and v2 formats) plus `strings.po` → rendered by our generic settings UI.
- `special://` path translation, a window property store, a `getInfoLabel`/`getCondVisibility` subset, and an `executebuiltin` subset (`Container.Refresh`, `RunPlugin`, `PlayMedia`, `Notification`, `ActivateWindow(videos, plugin://…)`).
- Player callbacks (`onAVStarted`, `onPlayBackStopped`, etc.) and `Monitor` (`waitForAbort`, `onSettingsChanged`) are fed from our player events, so addon scrobblers work.
- A **Kodi repository client**: reads `addons.xml`, resolves `<requires>` versions, installs zips. Unofficial repos get a clear warning screen; the official Kodi repo can be pre-listed.

### Primary target: Fen Light (and its forks)
The owner's Kodi addon. Inspected 2026-10-13 (FenLightAM 2.1.44 mirror, `plugin.video.fenlight`):
- Extension points: `xbmc.python.pluginsource` (video), **`xbmc.service`** (`service.py`), `xbmc.python.module`. Uses **`reuselanguageinvoker=true`**, so it expects a persistent interpreter.
- Dependencies: `script.module.requests` and **`script.module.pil` (Pillow, a native C extension)**, which we must bundle per platform.
- Talks directly to Real-Debrid, Premiumize, AllDebrid, TorBox, Offcloud, EasyDebrid, Easynews, Trakt, TMDb, OMDb and IMDb. It keeps its own SQLite caches in its profile folder and has an external-scraper hook, a downloader and a menu editor.
- **Heavy custom GUI: about 20 `WindowXMLDialog` skins** in `resources/skins/Default/1080i/`: `sources_results`, `sources_playback`, `select`, `confirm`, `ok`, `progress`, `extras`, `people`, `discover`, `settings_manager` (its settings UI is a custom window, not settings.xml), `textviewer`, `imageviewer`, `color`, `playback_notifications`, `downloads_manager`.
- Status: the original author has stopped supporting it; the forks (Fen Light+, FenLightAM) are where development continues.

**Consequence:** "WindowXML is out of scope" would mean Fen Light doesn't work. Revised plan:
1. **WindowXML-lite renderer**: render the Kodi skin XML *dialog* subset with our UI components, themed by the active theme. That covers `group`, `label`, `fadelabel`, `image`, `button`, `radiobutton`, `textbox`, `progress`, `list`/`panel`/`fixedlist` with `itemlayout`/`focusedlayout`, `$INFO[ListItem.*]`/`$INFO[Window.Property(*)]`/`$ADDON[]`, common `visible` conditions, `onup/ondown/onleft/onright`, `defaultcontrol`, and the Python side: `getControl`, `setFocusId`, `onClick`, `onAction`, `setProperty`, `addItems`. Full windows (`WindowXML`, non-dialog) and Estuary-include-dependent windows remain out of scope.
2. **Semantic overrides** (optional, per-addon "compat profiles"): recognise well-known dialogs and swap in our native equivalent. Fen Light's `sources_results` becomes **our stream picker** (with our ranking engine and themes), `progress` becomes our progress UI. Users get the native, consistent experience; addons don't notice.
   - **Default policy (decided, D17):** the faithful renderer (1) is the **compatibility baseline** and always works for any addon, because it's generic. Overrides (2) are **layered on top only where a tested compat profile exists**, and each one can be turned off (per addon, per dialog). If an override fails to recognise a dialog (e.g. a fork changed its XML), it **falls back to the faithful renderer automatically**. Built-in Kodi dialogs called through `xbmcgui.Dialog()` (select, yesno, ok, input, progress, notification) are always rendered natively, because their API is fixed and there's nothing to be faithful to.
3. **Fen Light is the Kodi conformance target #1, covering all three variants** (original Fen Light, Fen Light+, FenLightAM; they diverge, so each gets its own test run). The shim is "done" for phase 4 when Fen Light can browse, search, show sources, play with resume/next-episode via its `xbmc.Player` subclass, run its service, and open its settings manager.

**Long-term goal (D18): replace Fen Light with native addons.** Shim support is the bridge; the destination is native. That's also the test of whether our native system is "good enough": a set of native addons must be able to reproduce everything Fen Light does:

| Fen Light feature | Native replacement |
|---|---|
| TMDb/IMDb/OMDb metadata, discover, people | Metadata addon (TMDb) + `people` and `discover` resources |
| Trakt lists, watchlist, history, progress | **Trakt tracker addon** (see [Tracker & scrobbler addons](#tracker--scrobbler-addons)) |
| TMDb lists, personal lists, favourites, random widgets | Catalog addons + custom rows ([06](06-ux-input-player.md#catalog-organizer-must-have-free)) |
| Debrid cloud browsing (RD, PM, AD, TorBox, Offcloud, EasyDebrid) | **Debrid cloud addon(s)**, **TorBox first** (owner's service): catalogs of your cloud torrents, Usenet and web downloads matched to IDs, streams for matched titles, add/remove from cloud. Built on TorBox's official API (Main/Search APIs; official JS SDK exists) |
| Easynews search | Easynews stream addon (low priority: owner has it but doesn't use it) |
| External scrapers + source results window | Stream addons (e.g. AIOStreams) + our ranking engine + stream picker |
| Resume, next-episode, playback notifications | Core player features (resume, binge, next-up countdown) |
| Settings manager, menu editor | Core settings UI generated from addon config schemas + catalog organizer |
| Downloader | Core downloads feature (open question 6) |

### Out of scope
Full (non-dialog) WindowXML windows that depend on the active Kodi skin's includes, Kodi skins themselves, binary addons (PVR, inputstream, game cores), DRM, music/picture plugins (maybe later).

### Prior art to study
Kodistubs (API signatures), SAKÉ (partial emulator), kodi-plugin-routing and CodeQuick (routing frameworks addons depend on).

### Risk
The long tail of `getInfoLabel`/`executebuiltin` calls. Mitigation: an "addon compatibility report" that logs unimplemented calls, and a public compatibility list.

---

## Native addon system ("protocol v2")

### Is it actually better? Yes, in five concrete ways

| Problem today | v2 solution |
|---|---|
| Secrets and config live in the URL (they leak; editing means reinstalling) | Addons declare a **config schema**. The app stores values in the keychain and sends them in an `Authorization`/`X-Addon-Config` header. Config is editable in place. |
| Stream info is free text (`"4K HDR 🧲 12GB"`), so ranking means regex hacks | **Structured stream fields**: resolution, HDR type, codec, audio, languages, size, source type, cached status, release group, seeders, bitrate. The ranking engine reads real data. |
| Slow addons block the stream list | **Streaming responses** (NDJSON/SSE): results appear as each addon finds them. |
| Addons are blind to playback (no scrobbling, no skip-intro data) | **Events resource**: opt-in playback events (start, progress, stop, finished), plus `segments` resource (intro/recap/credits timestamps), `nextUp`, `ratings`, `similar`. |
| Everything needs a hosted server (hosting costs, rate limits, dead servers) | **Local WASM addons**: the same addon can run on your device, sandboxed. Aggregators, rankers and scrapers no longer depend on someone's server. |

Further improvements: semver'd protocol with **capability negotiation** (`protocol: "2.x"`), JSON Schema published for every type, ETag/conditional requests, batch endpoints (one request for meta for 50 IDs), signed manifests (author signing keys, shown at install), and localization (`Accept-Language`; Cinemeta is English-only today).

**Governance (D43):** v2 is developed **in this project first**, iterated until proven in our app, then published as a separate open spec for other clients and addon authors (AIOStreams being the first candidate).

**Backward compatibility:** a v2 manifest is a valid v1 manifest plus extra fields, so a v2 HTTP addon also works in Stremio, degraded. Authors can adopt it without losing Stremio users, and that's how adoption grows.

### Two ways to run the same addon

1. **Remote (HTTP)**: deploy anywhere, as with Stremio addons today.
2. **Local (WASM)**: a `.addon` bundle (manifest + `.wasm` + assets) that runs in the core's sandbox:
   - Host: Extism on wasmtime (desktop/server/Android); an interpreter (wasmi or wasmtime's Pulley) on iOS, which has no JIT.
   - **Permissions in the manifest, shown at install time**: allowed network domains, storage quota, which resources it can see, whether it receives playback events. No filesystem, no arbitrary sockets.
   - Fuel/time limits and a memory cap per call. A misbehaving addon gets killed and flagged.

### Is it hard to program? No. About the same as a Stremio addon, and the SDK hides the difference

```ts
// one TypeScript file → `addon build --target http` OR `--target wasm`
import { defineAddon } from "@app/addon-sdk";

export default defineAddon({
  id: "com.example.publicdomain",
  name: "Public Domain Movies",
  config: { region: { type: "select", options: ["US", "EU"], default: "US" } },
  permissions: { net: ["archive.org"] },
  catalogs: [{ type: "movie", id: "pd-top", name: "Public Domain" }],

  async catalog({ id, extra, config, http }) {
    const res = await http.get(`https://archive.org/advancedsearch.php?...`);
    return { metas: res.json().docs.map(toMeta) };
  },

  async streams({ id, http }) {
    return { streams: [{ url: "...", quality: { resolution: 1080, codec: "h264" } }] };
  },
});
```

- **SDKs:** TypeScript (compiled to WASM via Extism's JS PDK/QuickJS) and Rust first, then Go and Python (Extism PDKs exist).
- **Dev tooling:** `addon dev` gives hot reload inside the running app, the addon inspector shows every call, and a conformance test command checks the addon.
- **Trade-offs:**
  - JS-in-WASM is slower than native JS, but addon work is I/O-bound, so it rarely matters.
  - Addons can't use npm packages that need Node or browser APIs unless the SDK provides them (`http`, `cache`, `kv`, `crypto`, `html` parsing).
  - We maintain the SDKs and host functions.
- A Stremio addon written with the official `stremio-addon-sdk` can be wrapped by a compat helper so it builds to WASM too (a migration path).

### Plugin types beyond content addons (all WASM, permissioned)
- **Engine plugins:** torrent (first-party, native Rust sidecar, not WASM, because it needs sockets/disk), Usenet/NZB, archive streaming.
- **Trackers:** Trakt, Simkl, MAL/AniList, Letterboxd, etc. (see below).
- **Integrations:** Jellyfin/Plex/Emby sources, debrid clouds, OpenSubtitles.

### Tracker & scrobbler addons

**First tracker: Simkl** (owner is moving from Trakt to Simkl, which Stremio makes hard). Trakt comes second, mainly to migrate away from it.
- Simkl's API has `POST /scrobble/start` (sets "watching now"), `/scrobble/pause` (keeps a resumable session) and `/scrobble/stop` (marks watched at ≥ 80% progress), per the [mark-as-watched guide](https://api.simkl.org/guides/mark-as-watched) and the [SIMKL/API repo](https://github.com/SIMKL/API/blob/master/apiary.apib).
- **Use Simkl AUTH V2 only**: device flow via `POST /oauth2/device` with scope `media:read media:write` (a wrong scope silently gives a read-only token). AUTH V2 went live 2026-09-18 and **V1 (`/oauth/pin`) is retired around April 2027**, and V1 client IDs/tokens can't be upgraded ([jellyfin-plugin-simkl#72](https://github.com/jellyfin/jellyfin-plugin-simkl/issues/72)). Verify against [api.simkl.org](https://api.simkl.org) when we build it.
- Simkl distinguishes anime, so the core's ID mapping (Kitsu/MAL ↔ IMDb/TMDb) matters here.

Trakt, Simkl and friends are **ordinary native addons**, not hard-coded features (D18). The core ships no tracker. Stremio only allows first-party Trakt and Kodi has every addon roll its own, so this needs a proper API. A tracker addon declares the `tracker` capability and gets:

| Capability | What it gets | Permission shown to user |
|---|---|---|
| **Scrobble events** | `start`, `pause`, `resume`, `seek`, `progress` (throttled), `stop`, `finished` (configurable threshold, e.g. 90%), each with canonical IDs (IMDb, TMDb, TVDB, Kitsu, MAL…), season/episode, position, duration, and the source addon/stream (optional, separate permission) | "See what you watch" |
| **History & watched state (two-way)** | Push history/watched/unwatched to the core and receive local changes; conflict policy chosen by the user (tracker wins / local wins / newest wins) | "Read and change your watch history" |
| **Progress sync** | Provide resume points from other devices/apps (e.g. started on Trakt elsewhere) | "Read and change resume points" |
| **Lists as catalogs** | Watchlist, custom lists, recommendations, "up next" exposed as catalog rows; add/remove actions on detail pages | none extra (normal catalog) |
| **Ratings** | Show and submit ratings | "Rate titles on your behalf" |
| **Background jobs** | Scheduled sync (minimum interval enforced, e.g. ≥ 5 min), runs in the WASM host; best on desktop/home server, opportunistic on TV/iOS | "Run in the background" |
| **Auth helpers** | Host-provided **OAuth device-code flow** (shows code + QR on TV, opens browser on desktop) and token refresh; tokens stored in the OS keychain, never visible to other addons | "Sign in to trakt.tv" |

- **ID mapping** is a core service: the core resolves between IMDb/TMDb/TVDB/Kitsu/MAL/AniList IDs (via metadata addons + a cached mapping table), so trackers always receive the IDs they need, even for streams from addons that only know one ID type.
- **Primary tracker** (D52): one tracker is master by default (conflicts resolve in its favour); others mirror. Users can switch to local-master.
- **Multiple trackers at once** (e.g. Trakt for movies/shows + AniList for anime), each with per-profile enablement and type filters.
- **Offline queue**: events are stored and replayed when connectivity returns; duplicates are de-duplicated by event ID.
- **External players**: scrobbling still works when playback happens in an external player that reports progress back (see [06](06-ux-input-player.md#player)).
- **Tracker migration (core feature):** because every tracker addon exposes read *and* write for history, watchlist, ratings and progress, the core can **transfer between any two trackers**: Trakt → Simkl, Simkl → AniList, anything → local. Unlike Simkl's own Trakt importer, which is one-time only, won't update already-imported shows and skips Trakt collections ([Simkl support](https://support.simkl.com/knowledgebase/articles/735624-how-can-i-import-my-watch-history-to-simkl-from-tr)), ours is:
  - **Previewed**: a dry-run diff ("1,204 episodes, 87 movies, 312 ratings will be added; 14 conflicts").
  - **Repeatable and incremental**: run it again any time and only differences are sent.
  - **Transition-friendly**: scrobble to **both** trackers during a switch-over, then disable the old one.
  - **Local-first**: the app's own history is the hub (Trakt → local → Simkl), so you're never locked into any tracker.
- **Works with Kodi/Stremio addons too**: scrobble events are emitted from the core player, so a native Trakt addon tracks what you watch *through Fen Light or a Stremio addon* as well. Fen Light's own Trakt integration can be turned off to avoid double scrobbling (a compat-profile setting).
- **Ranking presets** and **stream formatters** (shareable files, not code).
- **UI mods:** see [05](05-theming-and-modding.md#tier-3-mods-code).

### Built-in but replaceable: everything is a provider (D25)

The app is a **kernel + built-in addons** ([03](03-architecture.md#kernel--built-in-addons--the-app-d25)). Every system **ships inside the app** so it works well out of the box, but each is just the **default implementation of a public interface**. Users can toggle it, and addon developers can replace or extend it. Same idea as VS Code's built-in extensions.

**Rules**
1. **No private APIs.** Built-ins use exactly the same provider interfaces as third-party addons. If a built-in needs something, the public API gets it.
2. **Every built-in is listed on the Addons screen** (marked "Built-in"), with enable/disable, settings, and "replace with…".
3. **Safe replacement**: if a replacement crashes, times out or is uninstalled, the app **falls back to the built-in** automatically and shows a notice. "Reset to built-in" is always one click.
4. **Performance**: built-ins can be native Rust behind the interface. Replacements run in the WASM sandbox with a time budget per call; the inspector shows if a replacement is slower than the built-in.
5. **Bare shell still holds** (D7): built-ins are *system* functionality, never content sources. No catalogs, streams or scrapers are built in.

**Slot types**
| Type | Meaning | Examples |
|---|---|---|
| **Single** (one active provider) | Exactly one provider is active; swapping replaces it | Stream ranker, release-name parser, ID/episode mapper, search engine, player engine selector, downloader, sync backend, subtitle renderer settings, home-screen composer |
| **Multi** (many active, ordered) | All enabled providers contribute, in user order | Metadata, catalogs, streams, subtitles, trackers, segments (skip intro/credits), notifications, ratings |
| **Pipeline / middleware** | Providers run in sequence and can transform data | Stream post-processing (filter, dedupe, format), metadata enrichment (e.g. add ratings badges), playback event hooks |

**Built-in providers (proposal)**
| System | Built-in default | Example replacement by a developer |
|---|---|---|
| Stream ranking | Rule engine ([06](06-ux-input-player.md#stream-ranking-engine-must-have)) | An ML ranker, or a port of AIOStreams' SEL engine |
| Release-name parser | Rust parser (video + anime, Anitomy-style) | A parser tuned for a specific language/region |
| ID & episode mapping | Offline dumps + patch overlay ([09](09-anime.md)) | A self-hosted mapping service |
| Local files scanner/matcher | Built-in scanner | A Kodi-NFO-only scanner, a Plex-style naming matcher |
| Player engine selection | Per-file auto choice (mpv/AVPlayer/Media3/webOS) | A rule set that always prefers an external player |
| Subtitles (local files, embedded tracks) | Built-in | OpenSubtitles etc. are normal multi-slot addons |
| Search | Local FTS + query fan-out to catalog addons | A meilisearch-backed or "smart" search |
| Downloader | Built-in download manager | A provider that hands off to a TorBox cloud download or aria2 |
| Trackers | **Simkl** and **Trakt** first-party, off until signed in | AniList, MAL, Letterboxd, Last.fm addons |
| Sync | Home-server sync | A WebDAV/Syncthing-file sync provider |
| Home screen composer | Rows from catalog organizer | A "smart home" that orders rows by time of day |
| Casting targets | Chromecast, AirPlay, DLNA, own devices | Another cast protocol |
| Torrent engine | First-party *optional* plugin (not pre-installed, D8) | Another engine |

**Metadata is built in (D40):** a **TMDb metadata module** ships enabled, so browsing, detail pages, search and artwork work immediately. It's metadata only (no catalogs of streams, no sources), so the bare-shell rule (D7) holds. It uses the app's TMDb API key by default, and users can enter their own key, change language/region, or replace the module (e.g. with Cinemeta via a Stremio addon, or a self-hosted provider). It provides IMDb/TVDB cross-IDs through TMDb's external IDs, which feeds the ID mapper.

### Scrapers: none built in, fully possible as addons (D24)

The app ships **no scraper and no source-finding logic** of its own (bare shell, D7). Third parties can write scrapers as ordinary addons:

| Way | How | Notes |
|---|---|---|
| Remote HTTP addon | Hosted service (like Comet, Torrentio, MediaFusion today) | Works with no changes, through the Stremio protocol or v2 |
| Local WASM addon | Runs sandboxed on the user's device or home server | No hosting costs or shared rate limits; the user grants network domains at install |
| Kodi addon | Through the Kodi shim (e.g. Fen Light's external-scraper hook) | Desktop/home server only |

**Generic host capabilities** that WASM addons get (deliberately not scraping-specific):
- HTTP client: custom headers, cookies (per-addon jar), redirects, gzip/brotli, user-agent, timeouts, **per-domain rate limiting** enforced by the host.
- Parsers: HTML (CSS selectors), XML, JSON, regex; text utilities and release-name parsing (shared with the ranking engine, so every addon parses `2160p.DV.HDR10.Atmos` the same way).
- Crypto (hashes, HMAC, AES) and base64/hex.
- KV storage and a response cache with TTLs, plus concurrency limits per addon.
- Optional **"wildcard network" permission** for scrapers that can't list domains up front. It shows a strong warning at install.

**What WASM addons can't do** (by design): raw sockets, DHT/tracker queries (that's the torrent engine plugin's job), running a headless browser or solving anti-bot challenges, or touching the filesystem. Scrapers that need these must be remote addons.

**Policy:** we host **no catalog at all** (D46), so we list nothing. Users install by URL, file or from community catalogs, and we don't police that. The legal line is the same as Stremio's and Kodi's: we ship a neutral platform and generic capabilities.

### Distribution
- Install from URL, file, or **addon catalogs** (the `addon_catalog` resource, compatible with Stremio's community catalogs).
- **No official catalog (D46).** We host and curate nothing. The app supports **community catalogs** (any URL serving an `addon_catalog`-style index, for addons, themes, skins and mods alike); the user adds the catalogs they trust. First-party modules ship with the app or as release downloads from the project repo.
- Because no one vets community catalogs, the **install screen carries the safety burden**: permissions, author signature/key fingerprint, source catalog, and update behaviour are all shown before install.
- Signed bundles, version pinning, rollback, auto-update toggle per addon.
