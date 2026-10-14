// Generates the S1 test clips into .cache/s1-media/ with the local ffmpeg (needs libx265).
// Synthetic clips beat downloaded ones here: we know the exact HDR10 metadata to look for, and a
// fast-moving bar makes tearing obvious. Real-content clips can be dropped into the same folder.
//
//   hdr10-bars.mkv  HEVC Main10, BT.2020 + PQ, mastering display 1000 nit, MaxCLL 1000 / MaxFALL 400
//   sdr-bars.mkv    H.264 BT.709, same pattern (SDR reference)
//   tearing.mkv     H.264 BT.709 60 fps, a full-height white bar sweeping 32 px/frame
//
// usage: node spikes/s1-shell/tools/make-media.mjs

import { execFileSync } from "node:child_process";
import { existsSync, mkdirSync, renameSync } from "node:fs";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(fileURLToPath(import.meta.url), "../../../..");
const dir = join(root, ".cache/s1-media");
mkdirSync(dir, { recursive: true });

const SECONDS = "20";
// Colour bars + a moving test pattern side by side; the HDR file is tone-mapped from these
// 8-bit sources, so the point is the signalling, not the picture.
const PATTERN = `testsrc2=size=3840x2160:rate=60:duration=${SECONDS}`;

const clips = [
  {
    file: "hdr10-bars.mkv",
    args: [
      "-f", "lavfi", "-i", PATTERN,
      "-vf", "format=yuv444p,zscale=tin=bt709:pin=bt709:min=bt709:rin=tv:t=smpte2084:p=bt2020:m=bt2020nc:npl=1000,format=yuv420p10le",
      "-c:v", "libx265", "-preset", "fast", "-crf", "20",
      "-x265-params",
      "hdr10=1:repeat-headers=1:colorprim=bt2020:transfer=smpte2084:colormatrix=bt2020nc:" +
        "master-display=G(13250,34500)B(7500,3000)R(34000,16000)WP(15635,16450)L(10000000,1):" +
        "max-cll=1000,400",
      "-color_primaries", "bt2020", "-color_trc", "smpte2084", "-colorspace", "bt2020nc",
    ],
  },
  {
    file: "sdr-bars.mkv",
    args: ["-f", "lavfi", "-i", PATTERN, "-c:v", "libx264", "-preset", "fast", "-crf", "20", "-pix_fmt", "yuv420p"],
  },
  {
    file: "tearing.mkv",
    args: [
      "-f", "lavfi", "-i", `color=c=black:size=1920x1080:rate=60:duration=${SECONDS}`,
      "-f", "lavfi", "-i", "color=c=white:size=48x1080:rate=60",
      // overlay re-evaluates x every frame (drawbox only evaluates it once).
      "-filter_complex", "[0][1]overlay=x='mod(t*1920,1920)':y=0:shortest=1",
      "-c:v", "libx264", "-preset", "fast", "-crf", "18", "-pix_fmt", "yuv420p",
    ],
  },
];

for (const { file, args } of clips) {
  const out = join(dir, file);
  if (existsSync(out)) {
    console.log(`${file}: exists`);
    continue;
  }
  console.log(`${file}: encoding`);
  // Encode to a temp name so a failed run never leaves a file that looks finished.
  const tmp = out.replace(/\.mkv$/, ".partial.mkv");
  execFileSync("ffmpeg", ["-hide_banner", "-loglevel", "error", "-y", ...args, tmp], { stdio: "inherit" });
  renameSync(tmp, out);
}
console.log(`clips in ${dir}`);
