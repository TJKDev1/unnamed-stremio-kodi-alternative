# Phase 5 — Kodi, anime depth & library

**Goal:** Fen Light works end to end through the Kodi shim, anime support reaches Seanime level, and the local library grows up.
**Depends on:** Phase 4 (the shim relies on its dialog/settings machinery), spike [S6](phase-0-spikes.md#s6-kodi-shim-proof-needed-before-phase-5).

Milestones only.

**Phase exit criteria**
- All three Fen Light variants can browse, search, show sources, play with resume/next-episode, run the service and open the settings manager.
- AniList/MAL/Kitsu trackers, OP/ED skip and filler badges work.

## M5.1 Kodi shim core
**Goal:** `kodi-host` sidecar, warm interpreter pool (`reuselanguageinvoker`), `xbmc*` modules, settings.xml/strings.po, `special://`, an info-label and builtins subset, player callbacks and `Monitor` (D4).
**Exit criteria:** a compatibility report logs every unimplemented call.

## M5.2 Kodi repository client
**Goal:** read `addons.xml`, resolve `<requires>`, install zips; a warning screen for unofficial repos; Pillow bundled per platform.

## M5.3 WindowXML-lite renderer & compat profiles
**Goal:** the faithful dialog renderer as baseline; native overrides only where a tested compat profile exists, each switchable, with automatic fallback (D17).
**Exit criteria:** Fen Light's `sources_results` can be either the native stream picker or the faithful one.

## M5.4 Fen Light conformance
**Goal:** each of the three variants passes a scripted test run.

## M5.5 Anime depth
**Goal:** AniList/MAL/Kitsu tracker addons, OP/ED skip and filler providers, AniList-style vs TMDB-style season view (D27, [09](../design/09-anime.md)).

## M5.6 Media servers & library
**Goal:** Jellyfin/Plex/Emby as sources; NFO/artwork reading, fix-match, file watching.

## M5.7 Live TV / IPTV / EPG
**Goal:** Stremio live/EPG addons, M3U + XMLTV, a guide grid, favourites, catch-up (D26).
