// S4 bench page. Query: ?ws=ws://host:port/ &transports=tauri,ws,replay &repeat=N &auto=1
// With auto=1 it runs everything once loaded and sends the results to the core over
// WebSocket (`serve` / the shell write them to .cache/s4-results/).

import { For, Show, createSignal } from "solid-js";
import { render } from "solid-js/web";

import { DIFFS, ENCODINGS, type RunResult, SCENARIOS, runLive, runReplay, summarize } from "./bench";
import { home } from "./store";
import { type Transport, tauriAvailable, tauriTransport, wsTransport } from "./transport";
import "./style.css";

const params = new URLSearchParams(location.search);
const wsUrl = params.get("ws") ?? `ws://${location.hostname || "127.0.0.1"}:7311/`;
const transports = (params.get("transports") ?? (tauriAvailable() ? "tauri,ws,replay" : "ws,replay")).split(",");
const repeat = Number(params.get("repeat") ?? "3");

function Home() {
  return (
    <div class="home">
      <For each={home.rows}>
        {(row) => (
          <section class="row">
            <h2>
              {row.title}
              <Show when={row.loading}> …</Show>
              <span class="count">{row.cards.length}</span>
            </h2>
            <div class="cards">
              <For each={row.cards}>
                {(card) => (
                  <div class="card">
                    <div class="poster" />
                    <div class="title">{card.title}</div>
                    <div class="meta">
                      {card.year ?? ""} {card.badges.join(" ")}
                    </div>
                    <Show when={card.progress !== null}>
                      <div class="progress" style={{ width: `${card.progress ?? 0}%` }} />
                    </Show>
                  </div>
                )}
              </For>
            </div>
          </section>
        )}
      </For>
    </div>
  );
}

function App() {
  const [status, setStatus] = createSignal("idle");
  const [summary, setSummary] = createSignal("");
  const [mounted, setMounted] = createSignal(true);

  async function runAll(): Promise<void> {
    const results: RunResult[] = [];
    const ws = await wsTransport(wsUrl);
    const live: Transport[] = [];
    if (transports.includes("tauri") && tauriAvailable()) live.push(tauriTransport());
    if (transports.includes("ws")) live.push(ws);
    for (let i = 0; i < repeat; i++) {
      for (const scenario of SCENARIOS) {
        for (const diff of DIFFS) {
          for (const enc of ENCODINGS) {
            for (const t of live) {
              setStatus(`run ${i + 1}/${repeat}: ${t.name} ${scenario} ${diff} ${enc}`);
              results.push(await runLive(t, scenario, diff, enc));
            }
            if (transports.includes("replay")) {
              setStatus(`run ${i + 1}/${repeat}: replay ${scenario} ${diff} ${enc}`);
              results.push(await runReplay(ws, "dom", scenario, diff, enc));
              setMounted(false);
              results.push(await runReplay(ws, "store", scenario, diff, enc));
              setMounted(true);
            }
          }
        }
      }
    }
    const text = summarize(results);
    setSummary(text);
    const ok = results.every((r) => r.ok);
    setStatus(ok ? "done: every run's final view-model matches the core" : "done: MISMATCH, see table");
    await ws.send({
      cmd: "results",
      data: { ok, summary: text, userAgent: navigator.userAgent, repeat, results },
    });
  }

  function start(): void {
    runAll().catch(async (e: unknown) => {
      const error = e instanceof Error ? (e.stack ?? e.message) : String(e);
      setStatus(`error: ${error}`);
      // Report the failure too, so an unattended run doesn't wait for results that never come.
      const ws = await wsTransport(wsUrl);
      await ws.send({ cmd: "results", data: { ok: false, summary: `page error: ${error}`, userAgent: navigator.userAgent } });
    });
  }

  if (params.get("auto") === "1") setTimeout(start, 500);

  return (
    <>
      <header>
        <button type="button" onClick={start}>
          Run all
        </button>
        <span class="status">{status()}</span>
      </header>
      <Show when={summary()}>
        <pre class="summary">{summary()}</pre>
      </Show>
      <Show when={mounted()}>
        <Home />
      </Show>
    </>
  );
}

const root = document.getElementById("root");
if (root) render(() => <App />, root);
