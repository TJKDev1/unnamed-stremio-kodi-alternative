# 03 — Architecture

## Overview

```
┌──────────────────────────────────────────────────────────────────────┐
│  UI layer (web: TypeScript + SolidJS + CSS)                          │
│   default skin · user skins · themes (tokens) · sandboxed UI mods    │
│   focus engine · input mapper                                        │
└──────────────▲───────────────────────────────────┬───────────────────┘
               │ view-model patches (binary IPC)   │ actions / intents
┌──────────────┴───────────────────────────────────▼───────────────────┐
│  CORE (Rust kernel + built-in modules) — native or WASM              │
│  ┌────────────┐ ┌─────────────┐ ┌──────────────┐ ┌────────────────┐  │
│  │ State/store│ │ Addon host  │ │ Library/DB   │ │ Stream ranking │  │
│  │ (Elm-style)│ │ stremio/v2/ │ │ SQLite, FTS, │ │ engine         │  │
│  │            │ │ wasm/kodi   │ │ local scan   │ │                │  │
│  └────────────┘ └─────────────┘ └──────────────┘ └────────────────┘  │
│  ┌────────────┐ ┌─────────────┐ ┌──────────────┐ ┌────────────────┐  │
│  │ Image cache│ │ Sync engine │ │ Player ctl   │ │ Local API      │  │
│  │ + resizer  │ │ (optional)  │ │ (abstract)   │ │ (HTTP/WS)      │  │
│  └────────────┘ └─────────────┘ └──────┬───────┘ └────────────────┘  │
└────────────────────────────────────────┼─────────────────────────────┘
                                         │
        ┌────────────────────────────────┼──────────────────────────┐
        ▼                                ▼                          ▼
  libmpv (desktop)           AVPlayer / MPVKit (iOS)     webOS native pipeline
                                                         (via <video>/Luna)
  Sidecar processes (desktop/home server only):
   • kodi-host  (embedded CPython + xbmc* shim)
   • torrent-engine plugin (librqbit)
```

## Kernel + built-in addons = the app (D25)

The app is a **framework**. A small **kernel** provides only the plumbing, and **everything users see and use is a module (built-in addon)** written against the same public APIs that third-party addons use. The shipped app is the kernel plus a curated **bundle of built-in modules**, the way VS Code is an editor core plus built-in extensions, or a Linux distribution is a kernel plus packages.

```
┌───────────────────────────── The app ("default distribution") ─────────────────────────────┐
│ Built-in modules (each can be disabled, replaced or extended):                              │
│  default skin & screens · catalog organizer · stream ranker · release parser · ID mapper    │
│  library & local scanner · player integration · downloads · casting · trackers (Simkl,      │
│  Trakt) · sync · search · notifications · Stremio adapter · Kodi shim · theming engine      │
├─────────────────────────────────────────────────────────────────────────────────────────────┤
│ KERNEL: module lifecycle & dependency resolution · provider/slot registry · event bus ·      │
│ permission system · storage (SQLite/KV/secrets) · networking · scheduler · settings registry │
│ · UI host (view-model + template runtime) · player backend host · Local API                  │
└─────────────────────────────────────────────────────────────────────────────────────────────┘
```

- **Kernel responsibilities stay minimal and stable**; features live in modules. Each module declares a manifest (id, version, provides/consumes slots, dependencies, permissions, settings schema) exactly like a third-party addon.
- **Built-in modules are compiled natively** into the binary (Rust for logic, the default skin for UI). Same interface, no WASM overhead. Third-party modules run sandboxed (WASM / worker), so built-ins stay as fast as a monolith.
- **Trust tiers:** built-in (native, signed, ships with the app) → first-party optional (e.g. torrent engine) → third-party (sandboxed, permissioned). A small set of hot-path backends (player engines, torrent engine) can only be provided natively; replacing those requires a signed native plugin on desktop, behind an explicit warning.
- **Distributions/editions become possible**: a community could ship a different bundle (e.g. an "anime edition" with AniList tracker + anime-first skin preinstalled, or a minimal "TV-only" build) without forking the kernel.
- **Dogfooding is enforced**: CI checks that built-in modules only import the public SDK, never kernel internals. If a built-in needs something, the public API gets it.

### Removing built-in modules (D41): the middle way
Fully uninstalling code that's compiled into the binary is awkward, so removal works on two levels instead:
- **Runtime: "disabled" means zero footprint.** A disabled module is never initialised: no memory, no background work, no network, no UI traces, no settings pages. For users this is the same as uninstalled, and it costs nothing extra because the kernel already resolves dependencies.
- **Build time: modules are Cargo features.** Anyone can compile a build without any module (e.g. a Kodi-only build without the Stremio adapter): `--no-default-features --features kodi,mpv,...`. No code changes are needed.
- **Required slots.** A few slots must always have a provider for the app to work (UI shell/skin, player backend, storage). Their built-ins can be *replaced* but not removed without a replacement; the UI says so.
- **Dependents degrade cleanly.** If a module's dependency is disabled, the module is disabled too, with a clear message ("Downloads needs the Player module").

### Guardrails that keep this elegant (and not Eclipse)
Plugin architectures go wrong in known ways: Eclipse/OSGi became complex "plugin hell", and Firefox's XUL extensions exposed internals, so Firefox had to break all of them to move to WebExtensions. Good references: **Bevy** (Rust; even its renderer is a plugin in `DefaultPlugins`), VS Code, Neovim. Rules:
1. **Tiny kernel.** If something can be a module, it is one. The kernel API should stay small enough to read in an afternoon.
2. **Few, coarse slots at first.** Start with about 10 slots (metadata, catalogs, streams, subtitles, ranker, mapper, tracker, player-backend, downloader, screens), not 50. Split a slot only when a real need appears.
3. **Rule of two.** An interface is marked **stable/public** only once it has two real implementations (e.g. Simkl + Trakt trackers) or a concrete third-party need. Until then it's `unstable`: built-ins can use it, but third parties are warned it may change.
4. **Narrow, data-oriented contracts.** Prefer pure-ish functions over objects with hidden state, e.g. the ranker is `rank(streams, context) → ordered streams + explanations`, and the mapper is `map(id, episode) → ids`. These are easy to test, replace and sandbox. Never expose internal structs.
5. **Contracts are schemas.** Every interface is defined once (schema/IDL), generating Rust traits, TS types, WASM bindings and docs. One source of truth; nothing drifts.
6. **Events for "tell", slots for "ask".** Modules notify each other through the event bus (playback started, item watched) and request data through slots. No module calls another module directly, so there's no hidden coupling.
7. **Lazy loading.** Modules start on first use; startup only loads what the home screen needs (performance budget).
8. **Versioned with compatibility windows.** Semver per interface, adapters for N−1 majors, deprecation warnings in the addon inspector.

## The Rust core

- **One crate graph, many targets.** It builds as a native library (desktop, iOS, home server) and as WASM (a fallback for the webOS/browser client when no home server is reachable).
- **State model:** unidirectional, Elm-style (model → update → effects), as in stremio-core. We should study stremio-core closely, and possibly reuse parts of it (**check its license is compatible with GPL**). Advantages:
  - Deterministic, testable, replayable (bug reports can include an action log).
  - The UI is a pure projection of view-models, so swapping skins is cheap and safe.
- **Async runtime:** tokio on native, wasm-bindgen-futures in WASM.
- **Storage:** SQLite with FTS5 for library, search, history and cache metadata. On disk: image cache, addon response cache (honours `cacheMaxAge`/`staleRevalidate`).
- **Networking:** reqwest/hyper, HTTP/2, connection pooling, per-addon concurrency limits and timeouts, optional SOCKS5/HTTP proxy per addon (a Stremio feature request).
- **Secrets:** OS keychain (Windows Credential Manager, macOS/iOS Keychain, libsecret). Encrypted file fallback on the home server.

### Why Rust is the right call (your question)
- Stremio's own core is Rust, and the leanest community clients (Stremio Native, Rillio) are Rust. They use about half the RAM of the official app.
- It compiles to every target we need: Windows, macOS, Linux, iOS, Android, and WASM for webOS/browser.
- **The hurdles aren't in the core; they're in the UI.** No Rust UI toolkit covers iOS + webOS + desktop with good moddability today. That's why the UI is web-based (D3), while all heavy work (networking, parsing, ranking, DB, images, addon sandboxing) stays in Rust.
- What Rust makes harder:
  - Slower iteration than TS for UI-adjacent logic. We keep the UI layer in TS.
  - Embedding CPython for Kodi works via pyo3, but adds build complexity. It lives in a sidecar process.
  - iOS builds need Xcode glue (Swift wrapper around the Rust static library).
- Nothing on our feature list is impossible because of Rust.

## The UI layer (web), and how we avoid web performance problems

You're right that "web" brings bottlenecks. The plan is to keep the webview doing only what it's good at (layout, text, CSS theming) and push everything else elsewhere.

| Bottleneck | Mitigation |
|---|---|
| Video in a webview (no MKV, broken DV detection, no passthrough, no refresh-rate switching) | **Video never plays in the webview** on desktop or iOS. A native player renders beneath or beside a transparent webview. On webOS, `<video>` is backed by the TV's hardware media pipeline, which is what native LG apps use too. |
| Framework overhead (virtual DOM diffing) | **SolidJS**: fine-grained reactivity, no virtual DOM, tiny runtime. Svelte 5 is the alternative; we'll decide in a spike. |
| JS doing heavy logic | All logic lives in the Rust core. The UI only renders view-models and dispatches intents. |
| Big DOM trees | Virtualized rows and grids with **DOM node recycling**. A row of 100 posters keeps about 12 nodes alive. |
| Layout thrash and jank | Animate only `transform`/`opacity`. Use `contain: layout paint` on cards and rows. No layout reads in animation frames. |
| Image decode stalls | The core's image service **resizes posters to the exact card size**, caches them on disk, serves WebP/AVIF, and pre-warms the next row. The UI uses `decoding="async"` with fixed aspect boxes. |
| IPC overhead between core and UI | Patches are batched per frame (structural diffs of view-models, encoded with MessagePack or postcard). No JSON round-trip per item. |
| Startup time | Render the last-known home screen from a snapshot immediately, then hydrate. |
| Old Chromium on TVs | Baseline is **Chromium 94** (LG C3 / webOS 23; see the webOS section below). Build targets cover it, no bleeding-edge CSS in the default skin, and skins declare a minimum engine. |
| Mods slowing everything down | Mod code runs in a Worker or WASM, never on the main thread. Mods render via declarative templates, and each has a CPU/time budget ([05](05-theming-and-modding.md)). |

**Escape hatch:** skins and mods target our **view-model + template API**, not the DOM directly. If a native renderer that understands HTML/CSS becomes viable later (e.g. Servo or Blitz once they mature), we could switch renderers without breaking themes. We are not betting on that now.

### Desktop shell
- **Tauri v2** (Rust, uses the OS webview: WebView2/WKWebView/WebKitGTK), or a custom shell modelled on Stremio's `stremio-shell-ng` (Rust + WebView2 + mpv in the same window).
- **Spike #1** (highest risk): composite **libmpv beneath a transparent webview** with no tearing, correct HDR, and resizing that doesn't flicker, on Windows, macOS and Linux. Rillio and shell-ng show it's doable. If compositing fails on a platform, the fallback is a separate borderless player window with a native OSD and the web OSD drawn over it.
- WebKitGTK on Linux is the slowest webview. Test it early.

### iOS
- Swift host app, WKWebView for the UI, Rust core as a static library.
- Player: **AVPlayer** for DV/Atmos/HLS, **MPVKit** for MKV/ASS. The engine is chosen automatically per file, with a manual override.
- **Distribution risk**: Apple has repeatedly pulled Stremio-like apps. Plan for **sideloading** (AltStore, SideStore, EU alternative marketplaces) and TestFlight. A bare-shell app *might* pass review, but we can't rely on it.
- Kodi Python addons and the torrent engine are **not run on iOS**. iOS uses them through the home server.

### LG webOS
- **Packaged web app (IPK)**: the same UI bundle as desktop with a webOS platform adapter (remote keys, Magic Remote pointer, media pipeline, Luna services).
- Two modes:
  1. **Connected to a home server** (recommended): full features (Kodi addons, torrents, local library, sync).
  2. **Standalone**: the core runs as WASM in the TV's browser engine. Stremio and native HTTP addons, plus WASM addons if memory allows. No Kodi, no torrents.
- Install via LG Content Store (risky, same delisting history as Stremio) or Developer Mode / Homebrew Channel sideloading.

#### Reference device: LG C3 (2023, webOS 23), rooted, Homebrew Channel
- **Web engine: Chromium 94** ([LG spec](https://webostv.developer.lge.com/develop/specifications/web-api-and-web-engine)). For comparison, webOS 22 uses Chromium 87 and webOS 24 uses Chromium 108. This is our **default-skin baseline**:
  - ✅ Available: CSS custom properties, grid, flex `gap`, `aspect-ratio`, `contain`, `:is()`/`:where()`, WebAssembly (incl. SIMD), Web Workers, OffscreenCanvas, ES2021.
  - ❌ Not available, so not allowed in the default skin: container queries, `:has()`, cascade layers (`@layer`), native CSS nesting, View Transitions, `color-mix()`, top-level `await` in workers without bundling. The skin compiler down-levels or rejects these when a skin targets `webos>=23`.
- **Root lets us run a third mode, "native on TV":**
  3. **Native core on the TV**: the Rust core runs as a **homebrew native service** (Homebrew Channel apps can ship native services; PicCap/hyperion-webos does this). It's the same as the home server, just running on the TV itself: no PC needed, native speed instead of WASM. It supports Stremio, native HTTP and WASM addons, and plausibly the torrent plugin.
  - Build target: probably **armv7 (32-bit userland)**. Check `uname -a` and the userland architecture on the C3; aarch64 needs the experimental webosbrew 64→32 bridge for EGL, but the core doesn't need GL. Toolchain: webosbrew `meta-lg-webos-ndk` ([native dev](https://webosbrew.org/pages/native-development)).
  - Kodi Python on the TV is technically possible with root but heavy. Kodi addons stay on the home server for now.
  - This mode only applies to rooted TVs. Store/dev-mode installs fall back to modes 1 and 2.
- Video still goes through the TV's native media pipeline (hardware decode, DV/HDR10, passthrough). Whether native mpv on rooted webOS is possible is an open research item, not a plan.

### Android / Android TV (right after desktop, D30)
- Android WebView UI + Rust core. Player: libmpv (mpv-android builds) + **Media3** for DV/passthrough/refresh-rate. Kodi shim is possible there (CPython builds exist) but it's a later phase.

## Player abstraction

One Rust trait (`PlayerBackend`) with backends for mpv, AVPlayer, MPVKit, webOS and Media3. The core decides which backend plays a stream from its codec, container, HDR type, audio format, subtitle type and the user's override rules. The UI only sees a common player view-model (position, tracks, chapters, buffering, stats), so **one OSD skin works on every platform**.

Power features: see [06](06-ux-input-player.md#player).

## Home server mode

**Default: invisible (D29).** Every device runs the core **embedded, in-process** (desktop, Android, iOS) or on-device (rooted webOS: native service; other webOS: WASM). The user never sees a client/server split. Using a *remote* core (PC, NAS, Docker, the TV) is an **optional setting** for people who want one shared library/server, and the UI is identical either way because the Local API carries the same intents and view-models.


- The same core binary runs headless (`app --server`), exposing:
  - The **Local API** (HTTP + WebSocket) that thin clients use: the same intents and view-models as the in-process UI.
  - A media proxy and stream endpoints (local files, torrent plugin, header proxying, optional transcoding via ffmpeg).
- **Ships inside the desktop app**, like Stremio's bundled streaming server. A "Share with my devices on this network" toggle exposes it on the LAN. It is also available as a Docker image / NAS package.
- Pairing via QR or PIN code, per-device tokens, TLS with a pinned self-signed cert or user-provided cert. Off-LAN access is an opt-in later (relay or WireGuard/Tailscale guidance).
- The home server can act as **sync hub** for all your devices: library, progress, settings, addon collection.

## Sync (optional)

- Sync targets (pluggable): **own home server**, an optional hosted service (monetization TBD), **Trakt / Simkl / AniList etc. via native tracker addons** (history, watchlist, scrobble; see [04](04-addons.md#tracker--scrobbler-addons)), **Stremio account import** (addon collection + library via its datastore API).
- Data model: per-record last-writer-wins with vector clocks, or CRDTs for lists such as catalog order. Decide in a spike.

## Local API (for remotes and automation)

As with Kodi's JSON-RPC (which the Kore/Yatse remotes use): documented HTTP/WS endpoints for playback control, search, queueing and notifications. Users can build phone remotes, Home Assistant integrations and scripts.

## Performance budget

These are targets; CI perf tests fail the build when one is missed.

TV column = LG C3 / webOS 23 (the reference device). Older TVs (webOS 5–6, about 2020–21) are a stretch target with a "lite" preset (fewer effects, smaller images).

| Metric | Desktop | LG C3 (webOS 23) |
|---|---|---|
| Cold start to interactive home (cached) | < 800 ms | < 2 s |
| Focus move (input → visual) | < 1 frame (16 ms) | < 2 frames |
| Row scroll | 60 fps sustained | 60 fps target, 30 fps floor |
| Detail page open (cached meta) | < 100 ms | < 250 ms |
| First streams visible after "Play"/open streams | first result appears as soon as the fastest addon answers; never waits for all addons | same |
| Idle RAM (app + core, no playback) | < 250 MB | within the TV's app limit |
| Seek (local/debrid HTTP) | < 300 ms to first frame after the network responds | best effort |

## Platform matrix

| Capability | Desktop | Home server | iOS | LG webOS | Android/TV (later) |
|---|---|---|---|---|---|
| Stremio addons | ✅ | ✅ | ✅ | ✅ | ✅ |
| Native HTTP addons | ✅ | ✅ | ✅ | ✅ | ✅ |
| Native WASM addons | ✅ | ✅ | ✅ (interpreted, iOS has no JIT) | ✅ rooted (native core) / ⚠️ standalone WASM limited / ✅ via server | ✅ |
| Kodi Python addons | ✅ | ✅ | via server | via server | later |
| Torrent plugin | ✅ | ✅ | via server | ✅ rooted (native core, to verify) / via server | ✅ |
| Local files | ✅ | ✅ (serves them) | Files app + via server | via server / USB | ✅ |
| Full skins + mods | ✅ | n/a | ✅ | ✅ (perf-gated) | ✅ |
| mpv-grade player | ✅ | n/a | ✅ (MPVKit) | native pipeline | ✅ |

## Repository layout

Bootstrapped in M0.1; crates not yet created are added when their phase starts. Cargo packages are named `app-<dir>` until the app has a name.

```
/crates
  fixtures/        fixture secret scrubber + checker (dev tooling)
  kernel/          lifecycle, slot registry, event bus, permissions, storage, UI host
  sdk-schema/      interface definitions → Rust traits, TS types, WASM bindings
  core/            state machine, models, intents
  addons-stremio/  Stremio protocol client
  addons-native/   v2 protocol + WASM host (wasmtime / wasmi on iOS)
  kodi-host/       sidecar: CPython embedding + xbmc* shim
  library/         SQLite, scanners, matchers
  ranking/         stream ranking engine
  player/          PlayerBackend trait + mpv backend
  server/          Local API, home server binary
  plugin-torrent/  librqbit-based optional plugin
/ui
  app/             SolidJS app shell, focus engine, input mapper
  skin-default/    default skin (built on the public skin API)
  sdk/             theme/skin/mod SDK + types
/platforms
  desktop/  ios/  webos/  android/
/sdk
  addon-sdk-rust/  addon-sdk-ts/   (write once → HTTP server or WASM)
/fixtures          recorded addon responses for conformance tests (secrets scrubbed)
/scripts           dev/CI scripts (e.g. Chromium 94 smoke test)
/xtask             repo maintenance: cargo xtask <task> (license policy)
/docs
```
