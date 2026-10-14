# S1 shell spike

libmpv under a transparent webview, in Tauri v2. Throwaway code with keep-quality findings: the report is [docs/spikes/S1.md](../../docs/spikes/S1.md). Own Cargo workspace (D65); libmpv is loaded at run time, so it builds without libmpv installed.

## Run (Windows)
```sh
node spikes/s1-shell/tools/fetch-libmpv.mjs   # pinned libmpv → .cache/libmpv (D66)
node spikes/s1-shell/tools/make-media.mjs     # test clips → .cache/s1-media (needs ffmpeg with libx265)
cd spikes/s1-shell
cargo run -- --file ../../.cache/s1-media/hdr10-bars.mkv
```
macOS/Linux: install libmpv from the package manager (or set `S1_LIBMPV`). There mpv opens its own window by default (the documented fallback, D64); `--embed` tries `wid` embedding anyway.

| Flag | Meaning |
|---|---|
| `--file PATH` | Load a file at start (the page also lists `.cache/s1-media/`) |
| `--window transparent\|opaque` | Tauri transparent window (Rillio) or normal window with a transparent webview background (stremio-shell-ng). Default `transparent` |
| `--embed` / `--no-embed` | mpv inside the window via `wid` (Windows default), or in its own window |
| `--hint-mode source\|target` | `target-colorspace-hint-mode`. Default `source` |
| `--no-hint` | Turn off `target-colorspace-hint` (SDR output, tone-mapped) |
| `--auto bench\|stress` | Unattended: run the tests, save results, quit |
| `--seconds N` | Length of each test phase (default 10) |

Results go to `.cache/s1-results/<unix time>-{results.json,mpv.log,page.log}`.

## What the page measures
- **OSD frame times:** every `requestAnimationFrame` delta while a phase runs (p50/p95/p99/max; "missed" = longer than 1.5× the median refresh interval).
- **mpv counters** over the phase: `frame-drop-count`, `decoder-frame-drop-count`, `vo-delayed-frame-count`, `mistimed-frame-count`. `frame-drop-count` resets when the VO re-initializes (fullscreen toggles), so it can go negative.
- **Phases:** `bench` (just play), `resize` (window size swept every 16 ms), `fullscreen` (toggled every 1.5 s), `minimize` (minimise/restore loop).
- **Window tree:** direct child windows in z-order; `WRY_WEBVIEW` must be above `mpv`.

## Manual checks (need eyes)
1. **Tearing:** play `tearing.mkv` fullscreen; the white bar must never look split or sheared.
2. **HDR:** with Windows HDR on, `hdr10-bars.mkv` must show the swapchain in PQ (`RGB_FULL_G2084_NONE_P2020` in the mpv log) and look right next to the same file in standalone mpv. The swatches (top right) show how bright the SDR UI is over HDR video; Windows' "SDR content brightness" slider sets it.
3. **Resize/fullscreen/minimise:** drag the window corner over video: no black or white flashes, video stays filled and under the OSD.
