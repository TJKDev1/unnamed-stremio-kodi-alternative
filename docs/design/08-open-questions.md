# 08 — Open questions

Decisions already made are in the [decision log](01-vision.md#decision-log). Everything below is still open. Answer inline or in chat, and we'll move answers into the decision log.

## Product & scope
1. **Name.** Vibe: cinematic (D39). Shortlist still to brainstorm.
2. **Monetization.** Deliberately postponed until there are users (D50). Options: donations only; free app + optional paid hosted sync/relay; nothing ever. Rule we've already set: features are never paywalled.
3. ~~License~~ **Answered:** AGPL server, GPL app, MIT SDKs (D37).
4. ~~Team & time~~ **Answered:** solo + AI, no deadline (D16).
5. ~~Live TV~~ **Answered:** later, but planned (D26).
6. ~~Downloads~~ **Answered:** core feature (D33).
7. ~~Casting~~ **Answered:** all of them + phone-as-remote (D31).
8. ~~Media scope~~ **Answered:** video + music (D28).
9. **DRM services** (Netflix-type content via inputstream/Widevine) — assume permanently out of scope?

## Your devices
10. ~~Which LG TV model/year?~~ **Answered:** LG C3 (2023, webOS 23, Chromium 94), rooted, Homebrew Channel (~0.7.3). See D14. Follow-up: run `uname -a` on the TV over SSH and report the kernel/userland architecture (armv7 vs aarch64).
11. ~~iOS distribution~~ **Answered:** sideloading (AltStore/SideStore/TestFlight) is the main path (D22).
12. ~~Android priority~~ **Answered:** right after desktop (D30).
13. ~~Home server host~~ **Answered:** invisible/embedded by default; remote optional (D29).

## Addons & ecosystems
14. ~~Which Stremio addons?~~ **Answered:** AIOStreams with Comet and other scrapers (D15).
15. ~~Which Kodi addons?~~ **Answered:** Fen Light (D15). It needs `WindowXMLDialog`, a service and Pillow.
15a. ~~Fen Light variant / dialog strategy~~ **Answered:** all variants; faithful baseline + switchable native overrides (D17).
15b. ~~Keep Fen Light or go native?~~ **Answered:** native, if the native system is good enough; trackers as native addons (D18).
15c. ~~Trackers~~ **Answered:** Trakt now, moving to Simkl (D19).
15d. ~~Debrid~~ **Answered:** TorBox (D19).
15e. ~~Usenet via TorBox?~~ **Answered:** no. Has Easynews but doesn't use it. Usenet/`nzbUrl` and the Easynews addon stay low priority (D20).
16. ~~Catalog policy~~ **Answered:** no official catalog; community catalogs only (D46).
17. ~~Stremio import~~ **Answered:** one-time import; credentials not stored (D48). *Original question:* Comfortable using Stremio's (undocumented-ish) API for one-time import?
18. ~~Usenet/NZB and archive streams~~ **Answered:** later (D20). Until then, such streams show as "requires plugin".
19. ~~Protocol v2 governance~~ **Answered:** ours first, open later (D43). *Original question:* Do we want to propose v2 openly with other client devs (Nuvio, Omni, Vidi, Stremio forks) so it becomes a shared standard rather than "our" format? (Recommended: yes, this drives adoption.)

## UX & modding
20. ~~Visual identity & dislikes~~ **Answered:** D23 + D36.
21. ~~Skin format~~ **Answered:** HTML-like templates (D45). *Original question:* Is an HTML-like template syntax fine for skinners, or do you want something more Kodi-like (XML), or both (an XML importer)?
22. ~~Unsafe dev mode~~ **Answered:** allowed, opt-in with warning (D47). *Original question:* Allow it at all, or keep everything sandboxed forever?
23. ~~Sounds~~ **Answered:** D49. *Original question:* **Sounds**: UI navigation sounds (TV style) as a themable thing?
24. ~~Ambient~~ **Answered:** D49. *Original question:* **Ambient features**: screensaver with artwork, ambilight/dynamic background color, trailers autoplaying on focus (configurable)?

## Tech
25. **SolidJS vs Svelte 5** for the UI layer. Recommendation: decide after spike S2 on your TV.
26. ~~stremio-core~~ **Answered:** own core, study theirs (D44). *Original question:* Reuse saves time but ties us to their model and licensing. Proposal: study it, write our own with the same Elm-style architecture, and stay protocol-compatible.
27. **Sync data model.** Last-writer-wins records vs CRDTs. Decide in a spike.
28. ~~Telemetry~~ **Answered:** none; opt-in crash reports (D38).

## Answered 2026-10-14
Library unified (D51); primary tracker is master by default, toggleable (D52); unified search with movies/shows bias (D53); calendar, inbox, OS and push notifications (D54).

## New (from the framework decision, D25)
29. ~~Built-in metadata~~ **Answered:** yes, TMDb module (D40).
30. ~~Uninstall vs disable~~ **Answered:** middle way: zero-footprint disable at runtime + Cargo-feature removal at build time (D41).
31. ~~Community editions~~ **Answered:** accepted, not specifically promoted (D42).
