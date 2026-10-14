// Boots the built UI (ui/app/dist) in a real Chromium 94 (D14 baseline: LG C3, webOS 23)
// and fails if the app doesn't reach its boot marker or logs an uncaught error.
//
// Lint rules and type libs can't know every API Chromium 94 lacks; running the bundle in
// the actual engine can. Linux only: Chromium snapshot 911494 (94.0.4606.0) has no
// Windows/macOS build. Install it with `pnpm chromium94:install`.
//
// usage: node scripts/smoke-chromium94.mjs [path/to/chrome]   (or set CHROMIUM94)

import { spawn } from "node:child_process";
import { existsSync, readFileSync, statSync } from "node:fs";
import { createServer } from "node:http";
import { extname, join, normalize, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(fileURLToPath(import.meta.url), "../..");
const dist = join(root, "ui/app/dist");
const chrome =
  process.argv[2] ??
  process.env.CHROMIUM94 ??
  join(root, ".cache/browsers/chromium/linux-911494/chrome-linux/chrome");

const BOOT_OK = 'data-boot="ok"';
const TYPES = {
  ".html": "text/html",
  ".js": "text/javascript",
  ".css": "text/css",
  ".svg": "image/svg+xml",
  ".png": "image/png",
  ".json": "application/json",
};

function fail(message) {
  console.error(`smoke-chromium94: ${message}`);
  process.exit(1);
}

if (!existsSync(join(dist, "index.html"))) fail(`no build at ${dist}; run pnpm build first`);
if (!existsSync(chrome)) fail(`no Chromium 94 at ${chrome}; run pnpm chromium94:install`);

// Module scripts don't load from file://, so serve dist over HTTP.
const server = createServer((req, res) => {
  const path = normalize(decodeURIComponent(new URL(req.url ?? "/", "http://x").pathname));
  let file = join(dist, path);
  if (!file.startsWith(dist)) {
    res.writeHead(403).end();
    return;
  }
  if (existsSync(file) && statSync(file).isDirectory()) file = join(file, "index.html");
  if (!existsSync(file)) {
    res.writeHead(404).end();
    return;
  }
  res.writeHead(200, { "content-type": TYPES[extname(file)] ?? "application/octet-stream" });
  res.end(readFileSync(file));
});

server.listen(0, "127.0.0.1", () => {
  const address = server.address();
  const url = `http://127.0.0.1:${typeof address === "object" && address ? address.port : 0}/`;
  const proc = spawn(chrome, [
    "--headless",
    "--no-sandbox",
    "--disable-gpu",
    "--no-first-run",
    "--enable-logging=stderr",
    "--v=0",
    "--virtual-time-budget=5000",
    "--dump-dom",
    url,
  ]);
  let dom = "";
  let log = "";
  proc.stdout.on("data", (chunk) => (dom += chunk));
  proc.stderr.on("data", (chunk) => (log += chunk));
  const timer = setTimeout(() => proc.kill("SIGKILL"), 60_000);
  proc.on("close", (code) => {
    clearTimeout(timer);
    server.close();
    const errors = log.split("\n").filter((line) => /CONSOLE.*Uncaught|Uncaught \w*Error/.test(line));
    if (errors.length > 0) fail(`uncaught errors in Chromium 94:\n${errors.join("\n")}`);
    if (!dom.includes(BOOT_OK)) {
      fail(`app did not boot in Chromium 94 (exit ${code}); DOM was:\n${dom.slice(0, 2000)}\n${log.slice(-2000)}`);
    }
    console.log("smoke-chromium94: app booted in Chromium 94");
  });
});
