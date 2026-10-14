# S4 IPC spike

Core ↔ UI IPC: view-model patches from a Rust core to a SolidJS store, over Tauri IPC and WebSocket. Throwaway code with keep-quality findings: the report is [docs/spikes/S4.md](../../docs/spikes/S4.md). Own Cargo workspace and pnpm root (D65).

| Path | What |
|---|---|
| `core/src/vm.rs` | Toy view-model (home: rows of cards) and intents (T0.S4.1) |
| `core/src/scenario.rs` | Deterministic load: `stream` (10 rows filling in, the "typical load") and `refresh` (full 10 × 50 home, then two full reshuffles) |
| `core/src/keyed.rs`, `generic.rs`, `typed.rs` | Keyed list diff; generic value-tree diff; typed per-field diff (T0.S4.2) |
| `core/src/session.rs` | One run: ticks → one patch per frame (batched), 16-byte timing header; the control-message handler shared by both transports |
| `core/src/wire.rs` | JSON, MessagePack, postcard encoders (T0.S4.3) |
| `core/src/server.rs` | WebSocket core + static file server (T0.S4.4) |
| `core/src/bin/bench.rs` | Rust-side diff/encode times and frame sizes |
| `core/src/bin/serve.rs` | Serves the page and the WebSocket core; with `--browser`, runs it headless and checks the results |
| `shell/` | Tauri v2 shell: the page in WebView2 over Tauri IPC (raw `ipc::Response`) and WebSocket |
| `web/src/decode/` | JSON / MessagePack / hand-written postcard decoders |
| `web/src/store.ts` | SolidJS store + typed and generic apply (T0.S4.5) |
| `web/src/bench.ts` | Runs and times every combination; checks the final view-model against the core's |

## Run
```sh
cd spikes/s4-ipc/web && pnpm install && pnpm build && cd ..
cargo run --release -p s4-core --bin bench                  # Rust side only
cargo run --release -p s4-shell -- --auto                    # WebView2: Tauri IPC + WebSocket + replay
cargo run --release -p s4-core --bin serve                   # any browser: open the printed URL, "Run all"
cargo run --release -p s4-core --bin serve -- --browser PATH # headless run (CI uses Chromium 94)
```
Results go to `.cache/s4-results/`. Page query options: `transports=tauri,ws,replay`, `repeat=N` (default 3), `auto=1`.

## What a frame's numbers mean
- **core**: diff (including the snapshot the next frame diffs against) + encode, measured in Rust and sent in the frame header.
- **transport**: the page's request → reply time minus core. Frames are *pulled* (one request per frame), so this is a full round trip; a pushed frame only pays the reply leg.
- **decode**, **apply**: in the page. Apply includes Solid's synchronous DOM updates, not layout or paint (S2's subject). Each frame's clock starts after the previous frame has painted.
- **replay dom / store**: all frames fetched first, then only decode + apply timed, with the home view mounted (`dom`) or not (`store`).

Browsers coarsen `performance.now()` to 0.1 ms here, so sub-0.1 ms columns read 0.10.
