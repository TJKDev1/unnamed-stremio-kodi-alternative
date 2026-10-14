# Phase 2 — Android & Android TV

**Goal:** the same app on Android phones, Android TV / Google TV and Fire TV, with the core embedded on-device (D29, D30).
**Depends on:** Phase 1.

Milestones only. Tasks get written when Phase 1 is about half done ([why](README.md#phases)).

**Phase exit criteria**
- Browse, play, trackers, ranking and downloads work on a phone and on an Android TV box.
- TV performance budget met on a mid-range Android TV device.
- The phone can act as a remote for the desktop app, and vice versa.

## M2.1 Android shell & embedded core
**Goal:** the Rust core runs in-process on Android with the web UI in Android WebView.
**Exit criteria:** the app launches on phone and TV; M1.2 browse works; no visible client/server split.

## M2.2 Android playback
**Goal:** mpv (mpv-android builds) plus **Media3** for Dolby Vision, passthrough and refresh-rate matching, picked per file by the engine selector.
**Spec:** [player-backend](../specs/player-backend.md)
**Exit criteria:** HDR10/DV and passthrough verified on a TV box; engine override works.

## M2.3 TV input & leanback polish
**Goal:** D-pad remotes, game controllers and Android TV system integration (launcher, Now Playing, voice search via the system keyboard).
**Exit criteria:** every screen usable with a basic remote; ≤ 2 actions to play.

## M2.4 Mobile specifics
**Goal:** touch UX, Wi-Fi-only downloads, background downloads, mobile media session, PiP.
**Exit criteria:** downloads survive app backgrounding; offline playback and the scrobble queue work.

## M2.5 Phone-as-remote & handoff
**Goal:** "Play on: Living room TV" between desktop and Android over the Local API (D31).
**Spec:** [view-model-ipc](../specs/view-model-ipc.md) (Local API)
**Exit criteria:** start on the phone, play on desktop with the chosen stream/subtitles; the phone shows a rich OSD.

## M2.6 Release
**Goal:** sideloadable APK via GitHub Releases (F-Droid optional), auto-update channels as on desktop.
