// S1 OSD: shows mpv state, drives tests, and measures its own frame times (rAF deltas) while
// mpv plays underneath. Plain script, no build step (throwaway spike code, D65).
"use strict";

const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

const $ = (id) => document.getElementById(id);
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

const PROPS = [
  "filename", "current-vo", "current-gpu-context", "hwdec-current", "video-sync",
  "display-names", "display-fps", "estimated-display-fps", "estimated-vf-fps",
  "video-params", "video-target-params", "target-colorspace-hint", "target-colorspace-hint-mode",
  "frame-drop-count", "decoder-frame-drop-count", "vo-delayed-frame-count", "mistimed-frame-count",
  "vsync-jitter", "time-pos", "duration", "pause",
];
const COUNTERS = ["frame-drop-count", "decoder-frame-drop-count", "vo-delayed-frame-count", "mistimed-frame-count"];
// The interesting parts of the big JSON properties.
const PARAM_KEYS = ["w", "h", "pixelformat", "hw-pixelformat", "primaries", "gamma", "colormatrix", "max-luma", "max-cll", "max-fall", "sig-peak", "light"];

let config = {};
let lastProps = {};

// ---- frame meter -------------------------------------------------------------------------
// Records every rAF delta. Gaps over 500 ms (minimised/hidden window) are counted, not timed.
const meter = { deltas: [], liveDeltas: [], hiddenGaps: 0, recording: false, last: 0 };
let probeX = 0;

function frame(t) {
  if (meter.last) {
    const d = t - meter.last;
    if (d <= 500) meter.liveDeltas.push(d);
    if (meter.recording) {
      if (d > 500) meter.hiddenGaps++;
      else meter.deltas.push(d);
    }
  }
  meter.last = t;
  probeX = (probeX + 6) % Math.max(1, innerWidth - 24);
  $("probe").style.transform = `translateX(${probeX}px)`;
  requestAnimationFrame(frame);
}
requestAnimationFrame(frame);

function summarize(deltas) {
  if (!deltas.length) return { frames: 0 };
  const s = [...deltas].sort((a, b) => a - b);
  const q = (p) => s[Math.min(s.length - 1, Math.floor(p * s.length))];
  const total = deltas.reduce((a, b) => a + b, 0);
  const median = q(0.5);
  return {
    frames: deltas.length,
    fps: +(1000 * deltas.length / total).toFixed(1),
    p50_ms: +median.toFixed(2),
    p95_ms: +q(0.95).toFixed(2),
    p99_ms: +q(0.99).toFixed(2),
    max_ms: +s[s.length - 1].toFixed(2),
    // A "missed" frame took at least 1.5 refresh intervals (refresh estimated from the median).
    missed: deltas.filter((d) => d > median * 1.5).length,
    over_33ms: deltas.filter((d) => d > 33.4).length,
  };
}

// Live readout over the last ~2 s.
setInterval(() => {
  const recent = meter.liveDeltas.splice(0);
  const r = summarize(recent);
  if (r.frames) $("meter").textContent = `OSD ${r.fps} fps · p50 ${r.p50_ms} · p99 ${r.p99_ms} · max ${r.max_ms} ms · missed ${r.missed}`;
}, 2000);

// ---- mpv state ---------------------------------------------------------------------------
function brief(name, value) {
  if (value == null) return "—";
  if (name === "video-params" || name === "video-target-params") {
    try {
      const o = JSON.parse(value);
      return PARAM_KEYS.filter((k) => k in o).map((k) => `${k}=${o[k]}`).join(" ");
    } catch {
      return value;
    }
  }
  return value;
}

async function refresh() {
  try {
    lastProps = await invoke("mpv_get", { names: PROPS });
    $("props").textContent = PROPS.map((n) => `${n.padEnd(28)} ${brief(n, lastProps[n])}`).join("\n");
    const pos = parseFloat(lastProps["time-pos"]), dur = parseFloat(lastProps["duration"]);
    if (dur > 0) $("progress-fill").style.width = `${(100 * pos) / dur}%`;
  } catch (e) {
    $("props").textContent = String(e);
  }
}
setInterval(refresh, 500);

const setStatus = (msg) => {
  $("status").textContent = msg;
  invoke("page_log", { line: `${performance.now().toFixed(0)} ${msg}` });
};

// ---- tests -------------------------------------------------------------------------------
async function counters() {
  const p = await invoke("mpv_get", { names: COUNTERS });
  return Object.fromEntries(COUNTERS.map((k) => [k, parseInt(p[k] ?? "0", 10)]));
}

async function phase(name, run) {
  setStatus(`running ${name}…`);
  const before = await counters();
  meter.deltas = [];
  meter.hiddenGaps = 0;
  meter.recording = true;
  await run();
  meter.recording = false;
  const after = await counters();
  const mpvDelta = Object.fromEntries(COUNTERS.map((k) => [k, after[k] - before[k]]));
  const result = { phase: name, osd: summarize(meter.deltas), hidden_gaps: meter.hiddenGaps, mpv: mpvDelta };
  setStatus(`${name}: ${JSON.stringify(result.osd)} mpv ${JSON.stringify(mpvDelta)}`);
  return result;
}

const seconds = () => config.seconds || 10;
const PHASES = {
  bench: () => phase("bench", () => sleep(seconds() * 1000)),
  resize: () => phase("resize", () => invoke("stress", { kind: "resize", seconds: seconds() })),
  "fs-loop": () => phase("fullscreen", () => invoke("stress", { kind: "fullscreen", seconds: seconds() })),
  minimize: () => phase("minimize", () => invoke("stress", { kind: "minimize", seconds: seconds() })),
};

async function runAll(names) {
  const phases = [];
  for (const n of names) phases.push(await PHASES[n]());
  await refresh();
  const report = {
    config,
    user_agent: navigator.userAgent,
    device_pixel_ratio: devicePixelRatio,
    child_windows: await invoke("child_windows"),
    mpv_props: lastProps,
    phases,
  };
  const path = await invoke("save_result", { name: "results", json: JSON.stringify(report, null, 2) });
  setStatus(`saved ${path}`);
  return path;
}

// ---- controls ----------------------------------------------------------------------------
const actions = {
  load: () => invoke("mpv_command", { args: ["loadfile", $("media").value] }),
  pause: () => invoke("mpv_command", { args: ["cycle", "pause"] }),
  back: () => invoke("mpv_command", { args: ["seek", "-10"] }),
  fwd: () => invoke("mpv_command", { args: ["seek", "10"] }),
  fullscreen: async () => {
    const w = window.__TAURI__.window.getCurrentWindow();
    await w.setFullscreen(!(await w.isFullscreen()));
  },
  bench: PHASES.bench,
  resize: PHASES.resize,
  "fs-loop": PHASES["fs-loop"],
  minimize: PHASES.minimize,
  all: () => runAll(["bench", "resize", "fs-loop", "minimize"]),
};

document.addEventListener("click", (e) => {
  const cmd = e.target.closest("[data-cmd]")?.dataset.cmd;
  if (cmd) actions[cmd]().catch((err) => setStatus(`error: ${err}`));
});

listen("mpv-event", (e) => {
  if (e.payload === "VideoReconfig") invoke("child_windows").then((c) => console.log("children", c));
});

async function main() {
  config = await invoke("config");
  $("mode").textContent = `window=${config.window} embed=${config.embed} hint=${config.hint} hint-mode=${config.hint_mode}`;
  const media = await invoke("list_media");
  $("media").innerHTML = media.map((m) => `<option>${m}</option>`).join("");
  if (config.file) $("media").value = config.file;

  if (config.auto) {
    // Let playback settle, then run, save and quit (used for unattended measurements).
    await sleep(3000);
    const names = config.auto === "stress" ? ["bench", "resize", "fs-loop", "minimize"] : ["bench"];
    try {
      await runAll(names);
    } finally {
      await invoke("quit");
    }
  }
}
main().catch((e) => setStatus(`error: ${e}`));
