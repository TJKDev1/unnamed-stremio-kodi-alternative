// CSS must work on the Chromium 94 baseline (D14). The plugin checks every feature against
// browserslist (package.json: chrome >= 94), e.g. @layer (Chrome 99) or :has() (Chrome 105).
/** @type {import("stylelint").Config} */
export default {
  extends: ["stylelint-config-standard"],
  plugins: ["stylelint-no-unsupported-browser-features"],
  ignoreFiles: ["**/dist/**", "**/node_modules/**", "target/**", ".cache/**"],
  rules: {
    "plugin/no-unsupported-browser-features": [
      true,
      {
        severity: "error",
        // Partial support in Chrome 94 that is fine to use as-is.
        ignorePartialSupport: true,
      },
    ],
  },
};
