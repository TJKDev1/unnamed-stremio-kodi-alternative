// Downloads the pinned libmpv build for the S1 spike into .cache/libmpv/ and checks its
// SHA-256 (D66). Windows only for now: macOS/Linux use the system libmpv (D64).
//
// The pin is the mpv project's own MinGW build (GPL variant) from its rolling "git-release"
// GitHub release. That release is replaced every few months; when the URL 404s, pick the new
// libmpv-*-x86_64-w64-mingw32-gpl.zip asset there and update URL + SHA256 below.
//
// usage: node spikes/s1-shell/tools/fetch-libmpv.mjs

import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const URL =
  "https://github.com/mpv-player/mpv/releases/download/git-release/" +
  "libmpv-v0.41.0-dev-g270367e8a-38501694775-x86_64-w64-mingw32-gpl.zip";
const SHA256 = "de326996891959340fbf105512dc9c1eb4f803a6626f440339395437d5cbd201";

const root = resolve(fileURLToPath(import.meta.url), "../../../..");
const dir = join(root, ".cache/libmpv");
const zip = join(dir, "libmpv.zip");
const stamp = join(dir, "PINNED_SHA256");

if (process.platform !== "win32") {
  console.error("fetch-libmpv: Windows only. On macOS/Linux install libmpv from the system package manager.");
  process.exit(1);
}
if (existsSync(stamp) && readFileSync(stamp, "utf8").trim() === SHA256) {
  console.log(`libmpv already present in ${dir}`);
  process.exit(0);
}

rmSync(dir, { recursive: true, force: true });
mkdirSync(dir, { recursive: true });
console.log(`downloading ${URL}`);
const res = await fetch(URL);
if (!res.ok) {
  console.error(`download failed: HTTP ${res.status}. The rolling release may have moved; see the header of this file.`);
  process.exit(1);
}
const body = Buffer.from(await res.arrayBuffer());
const got = createHash("sha256").update(body).digest("hex");
if (got !== SHA256) {
  console.error(`SHA-256 mismatch: expected ${SHA256}, got ${got}`);
  process.exit(1);
}
writeFileSync(zip, body);
// Windows 10+ ships bsdtar, which reads zip. Call it by path: Git Bash puts GNU tar first.
const tar = join(process.env.SystemRoot ?? "C:\\Windows", "System32", "tar.exe");
execFileSync(tar, ["-xf", zip, "-C", dir], { stdio: "inherit" });
rmSync(zip);
writeFileSync(stamp, SHA256 + "\n");
console.log(`libmpv extracted to ${dir}`);
