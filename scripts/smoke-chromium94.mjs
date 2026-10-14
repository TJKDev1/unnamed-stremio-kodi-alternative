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
const ERRORS_ATTR = "data-smoke-errors";
// Injected as the first script of index.html (only when served by this test): records every
// uncaught error and unhandled rejection onto <html>, where --dump-dom will show it.
const ERROR_RECORDER = `<script>(function () {
  var errors = [];
  function record(message) {
    errors.push(String(message));
    document.documentElement.setAttribute("${ERRORS_ATTR}", JSON.stringify(errors));
  }
  addEventListener("error", function (e) { record("error: " + (e.error && e.error.stack || e.message)); });
  addEventListener("unhandledrejection", function (e) {
    record("unhandled rejection: " + (e.reason && e.reason.stack || e.reason));
  });
})();</script>`;
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
  let body = readFileSync(file);
  if (extname(file) === ".html") {
    const html = body.toString("utf8");
    if (!/<head>/i.test(html)) {
      res.writeHead(500).end();
      return;
    }
    body = Buffer.from(html.replace(/<head>/i, (head) => head + ERROR_RECORDER));
  }
  res.writeHead(200, { "content-type": TYPES[extname(file)] ?? "application/octet-stream" });
  res.end(body);
});

function decodeEntities(text) {
  return text
    .replace(/&quot;/g, '"')
    .replace(/&#39;/g, "'")
    .replace(/&lt;/g, "<")
    .replace(/&gt;/g, ">")
    .replace(/&amp;/g, "&");
}

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
  let timedOut = false;
  const timer = setTimeout(() => {
    timedOut = true;
    proc.kill("SIGKILL");
  }, 60_000);
  proc.on("error", (err) => fail(`could not start Chromium 94: ${err.message}`));
  proc.on("close", (code, signal) => {
    clearTimeout(timer);
    server.close();
    const tail = `\nDOM:\n${dom.slice(0, 2000)}\nlog:\n${log.slice(-2000)}`;
    if (timedOut) fail(`Chromium 94 timed out after 60 s${tail}`);
    if (code !== 0 || signal) fail(`Chromium 94 exited with ${signal ?? `code ${code}`}${tail}`);
    // Without the recorder in the page, "no errors recorded" proves nothing.
    if (!dom.includes('addEventListener("unhandledrejection"')) {
      fail(`error recorder missing from the page${tail}`);
    }
    const recorded = dom.match(new RegExp(`${ERRORS_ATTR}="([^"]*)"`));
    if (recorded) {
      const errors = JSON.parse(decodeEntities(recorded[1] ?? "[]"));
      fail(`uncaught errors in Chromium 94:\n${errors.join("\n")}`);
    }
    if (!dom.includes(BOOT_OK)) fail(`app did not boot in Chromium 94${tail}`);
    console.log("smoke-chromium94: app booted in Chromium 94 with no uncaught errors");
  });
});
