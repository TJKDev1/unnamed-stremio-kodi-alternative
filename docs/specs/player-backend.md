# Spec: Player backend

**Status:** stub · **Implemented by:** S1 ([report](../spikes/S1.md)), M1.3, M1.7, M2.2, M3.3, M3.4 · **Decisions:** D3, D35

## Purpose
One Rust `PlayerBackend` trait with backends for mpv, AVPlayer, MPVKit, webOS and Media3. The UI sees only the common player view-model, so one OSD skin works everywhere. Design: [player abstraction](../design/03-architecture.md#player-abstraction), [player features](../design/06-ux-input-player.md#player).

## Scope
- **In:** trait, capability reporting, engine selection, player events, player view-model.
- **Out:** OSD layout (skin), scrobbling logic ([tracker-api](tracker-api.md) consumes player events).

## Contract (to fill in)
- Commands: load(stream, options), play/pause, seek, set tracks (audio/sub), set rate, set properties (backend-specific escape hatch).
- Events: started, progress (throttled), paused, seeked, buffering, track list changed, ended, error.
- Capabilities: containers, codecs, HDR types, DV profiles, audio passthrough formats, subtitle types, display HDR state.
- Engine selector (single slot): `choose(stream, capabilities, rules) → backend`.

## Behaviour (known constraints)
- Video never plays inside the webview on desktop or iOS.
- Capabilities feed device-aware ranking.
- External players are a backend variant with partial events.
- **Desktop mpv (from S1, D67):** Windows embeds mpv with `wid` and keeps its `"mpv"` child window at the bottom of the z-order (re-asserted after every video reconfig); the webview has a transparent background and owns all input (`input-default-bindings=no`, `input-vo-keyboard=no`, `input-cursor=no`). HDR passthrough needs `vo=gpu-next` with a d3d11 context and `target-colorspace-hint`, so it is impossible through libmpv's render API (OpenGL/software only). macOS/Linux use a separate mpv window until tested (D64).
- The OSD is dimmed while HDR video plays (setting, D68).

## Conformance tests
- A fixture set of media files (HDR10, DV P5/P8, Atmos, ASS subs, MKV chapters) with expected track/capability results per backend.

## Open items
- How much mpv-specific config to expose through the trait vs mpv-only settings pages.
