import { defineConfig } from "vite";

export default defineConfig({
  build: {
    // Chromium 94 baseline (D14): the LG C3's webOS 23 browser engine.
    target: "chrome94",
    cssTarget: "chrome94",
  },
});
