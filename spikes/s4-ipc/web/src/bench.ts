// Runs scenario × diff × encoding over each transport and times every frame:
// core diff + encode (from the frame header), transport, decode, apply (incl. Solid's DOM
// updates). Frames are pulled one per animation frame, as the app would show them; each
// frame's clock starts after the browser has painted the previous one, so layout and paint
// (S2's subject) don't land in "transport".

import { decode } from "./decode";
import { applyGeneric, applyTyped, resetHome, snapshot } from "./store";
import { type Transport, text } from "./transport";
import type { DiffKind, Encoding, GenericOp, Intent, Scenario, TypedOp } from "./types";

export const SCENARIOS: Scenario[] = ["stream", "refresh"];
export const DIFFS: DiffKind[] = ["generic", "typed"];
export const ENCODINGS: Encoding[] = ["json", "msgpack", "postcard"];

/** Intents the "user" sends after frame N. The same sequence runs over every transport. */
const CLIENT_INTENTS: [number, Intent][] = [
  [10, { MarkWatched: { card: "addon-0-3" } }],
  [25, { LoadMore: { row: "addon-2" } }],
  [40, { MarkWatched: { card: "addon-2-p1-4" } }],
];

const HEADER = 20;

export interface FrameTimes {
  /** All in ms. `core` = diff + encode; `transport` = round trip minus all of the request's work in the core. */
  core: number;
  transport: number;
  decode: number;
  apply: number;
  total: number;
  bytes: number;
}

export interface RunResult {
  transport: string;
  scenario: Scenario;
  diff: DiffKind;
  enc: Encoding;
  frames: FrameTimes[];
  ok: boolean;
  error?: string;
}

/**
 * Resolves after the next paint: a task queued from rAF runs after that frame's rendering.
 * Falls back to a timer if no frame comes (a hidden or headless page may not render).
 */
function afterNextPaint(): Promise<void> {
  return new Promise((r) => {
    const fallback = setTimeout(r, 100);
    requestAnimationFrame(() => {
      clearTimeout(fallback);
      setTimeout(r, 0);
    });
  });
}

function canonical(v: unknown): string {
  return JSON.stringify(v, (_k, x: unknown) =>
    x && typeof x === "object" && !Array.isArray(x)
      ? Object.fromEntries(Object.entries(x as Record<string, unknown>).sort(([a], [b]) => (a < b ? -1 : 1)))
      : x,
  );
}

function apply(diff: DiffKind, ops: TypedOp[] | GenericOp[]): void {
  if (diff === "typed") applyTyped(ops as TypedOp[]);
  else applyGeneric(ops as GenericOp[]);
}

function header(buf: ArrayBuffer): { core: number; request: number } {
  const v = new DataView(buf);
  return { core: (v.getUint32(4, true) + v.getUint32(8, true)) / 1e6, request: v.getUint32(16, true) / 1e6 };
}

async function intentsAfter(t: Transport, frame: number): Promise<void> {
  for (const [at, intent] of CLIENT_INTENTS) {
    if (at === frame) await t.send({ cmd: "dispatch", intent });
  }
}

async function finalMatches(t: Transport): Promise<boolean> {
  const core = JSON.parse(text(await t.send({ cmd: "final" }))) as unknown;
  return canonical(core) === canonical(snapshot());
}

/** Live: each frame is requested, received, decoded and applied, one per animation frame. */
export async function runLive(t: Transport, scenario: Scenario, diff: DiffKind, enc: Encoding): Promise<RunResult> {
  const result: RunResult = { transport: t.name, scenario, diff, enc, frames: [], ok: false };
  resetHome();
  await t.send({ cmd: "start", scenario, diff, enc });
  try {
    for (let n = 1; ; n++) {
      await afterNextPaint();
      const t0 = performance.now();
      const buf = await t.send({ cmd: "next" });
      const t1 = performance.now();
      if (buf.byteLength === 0) break;
      const ops = decode(diff, enc, new Uint8Array(buf, HEADER));
      const t2 = performance.now();
      apply(diff, ops);
      const t3 = performance.now();
      const { core, request } = header(buf);
      result.frames.push({
        core,
        transport: Math.max(0, t1 - t0 - request),
        decode: t2 - t1,
        apply: t3 - t2,
        total: t3 - t0,
        bytes: buf.byteLength - HEADER,
      });
      await intentsAfter(t, n);
    }
    result.ok = await finalMatches(t);
  } catch (e) {
    result.error = String(e);
  }
  return result;
}

/**
 * Replay: fetch every frame first (untimed), then time decode + apply alone. `label` says
 * whether the home view is mounted ("dom") or not ("store": patching the store only).
 */
export async function runReplay(
  t: Transport,
  label: string,
  scenario: Scenario,
  diff: DiffKind,
  enc: Encoding,
): Promise<RunResult> {
  const result: RunResult = { transport: `replay ${label}`, scenario, diff, enc, frames: [], ok: false };
  await t.send({ cmd: "start", scenario, diff, enc });
  const bufs: ArrayBuffer[] = [];
  for (let n = 1; ; n++) {
    const buf = await t.send({ cmd: "next" });
    if (buf.byteLength === 0) break;
    bufs.push(buf);
    await intentsAfter(t, n);
  }
  resetHome();
  try {
    for (const buf of bufs) {
      await afterNextPaint();
      const t1 = performance.now();
      const ops = decode(diff, enc, new Uint8Array(buf, HEADER));
      const t2 = performance.now();
      apply(diff, ops);
      const t3 = performance.now();
      const { core } = header(buf);
      result.frames.push({ core, transport: 0, decode: t2 - t1, apply: t3 - t2, total: core + t3 - t1, bytes: buf.byteLength - HEADER });
    }
    result.ok = await finalMatches(t);
  } catch (e) {
    result.error = String(e);
  }
  return result;
}

export interface Stats {
  p50: number;
  p95: number;
  max: number;
  mean: number;
}

export function stats(xs: number[]): Stats {
  if (xs.length === 0) return { p50: 0, p95: 0, max: 0, mean: 0 };
  const s = [...xs].sort((a, b) => a - b);
  const at = (q: number) => s[Math.round((s.length - 1) * q)] as number;
  return { p50: at(0.5), p95: at(0.95), max: s[s.length - 1] as number, mean: s.reduce((a, b) => a + b, 0) / s.length };
}

const f = (x: number) => x.toFixed(2);

/** Markdown table, one line per transport × scenario × diff × encoding (repeats merged). */
export function summarize(results: RunResult[]): string {
  const groups = new Map<string, RunResult[]>();
  for (const r of results) {
    const key = `${r.transport} | ${r.scenario} | ${r.diff} | ${r.enc}`;
    groups.set(key, [...(groups.get(key) ?? []), r]);
  }
  const lines = [
    "| transport | scenario | diff | encoding | frames | total ms p50 / p95 / max | core p95 | transport p95 | decode p95 | apply p95 | KB max | ok |",
    "|---|---|---|---|---|---|---|---|---|---|---|---|",
  ];
  for (const [key, rs] of groups) {
    const frames = rs.flatMap((r) => r.frames);
    const total = stats(frames.map((x) => x.total));
    const p95 = (k: keyof FrameTimes) => f(stats(frames.map((x) => x[k])).p95);
    const kb = stats(frames.map((x) => x.bytes)).max / 1024;
    const ok = rs.every((r) => r.ok) ? "yes" : `NO ${rs.map((r) => r.error ?? "").join(" ")}`;
    lines.push(
      `| ${key} | ${frames.length} | ${f(total.p50)} / ${f(total.p95)} / ${f(total.max)} | ${p95("core")} | ${p95("transport")} | ${p95("decode")} | ${p95("apply")} | ${kb.toFixed(1)} | ${ok} |`,
    );
  }
  return lines.join("\n");
}
