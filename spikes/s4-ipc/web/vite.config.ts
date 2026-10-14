import { defineConfig } from "vite";
import solid from "vite-plugin-solid";

export default defineConfig({
  plugins: [solid()],
  // Relative asset URLs: the page is served from Tauri, from `serve` and from file paths.
  base: "./",
  build: {
    // Chromium 94 baseline (D14).
    target: "chrome94",
    cssTarget: "chrome94",
  },
});
