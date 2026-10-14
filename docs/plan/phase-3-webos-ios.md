# Phase 3 — Your devices: LG webOS + iOS

**Goal:** the owner's LG C3 and iPhone run the app and share state with the desktop.
**Depends on:** Phase 2, spike [S8](phase-0-spikes.md#s8-rust-core-as-a-webos-homebrew-native-service-needed-before-phase-3).

Milestones only.

**Phase exit criteria**
- LG C3: browsing at 60 fps with < 2-frame focus; playback through the TV's native pipeline (HDR10/DV, passthrough).
- iPhone: browse and play (AVPlayer/MPVKit), sideloaded (D22).
- Library, progress and settings sync between desktop, phone and TV.

## M3.1 webOS client (native core on the rooted TV)
**Goal:** IPK with the webOS platform adapter (remote keys, Magic Remote pointer, Luna) and the Rust core as a homebrew native service (D14).
**Exit criteria:** the S8 approach productionised; the core starts with the app; LG C3 perf budget green.

## M3.2 webOS standalone & remote-core modes
**Goal:** non-rooted TVs work too: core as WASM in the browser engine, or connected to a remote core.
**Exit criteria:** Stremio and native HTTP addons work in standalone WASM mode within the TV's memory limits.

## M3.3 webOS playback
**Goal:** a `<video>`/Luna-backed player backend with track selection, subtitles and the shared OSD skin.
**Exit criteria:** HDR10/DV and passthrough verified on the C3.

## M3.4 iOS client
**Goal:** Swift host + WKWebView UI + Rust static library; AVPlayer for DV/Atmos/HLS, MPVKit for MKV/ASS; sideload distribution.
**Exit criteria:** browse/play/trackers/downloads on the owner's iPhone.

## M3.5 Remote core, pairing & sync
**Goal:** optional remote core (PC/NAS/Docker/TV) with QR/PIN pairing, per-device tokens and TLS; sync of library, progress, settings and addon collection.
**Before starting:** a sync-model spike (last-writer-wins records vs CRDTs; closes open question 27).
**Exit criteria:** change progress on the phone and see it on the TV within seconds; works offline and reconciles.

## M3.6 Casting
**Goal:** Chromecast, AirPlay (the C3 is an AirPlay 2 receiver) and DLNA/UPnP targets, with target-aware ranking (D31).
**Exit criteria:** cast from the iPhone and the desktop to the C3 by each method; incompatible streams are warned about or remuxed.
