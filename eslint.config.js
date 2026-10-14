// @ts-check
import js from "@eslint/js";
import compat from "eslint-plugin-compat";
import globals from "globals";
import tseslint from "typescript-eslint";

export default tseslint.config(
  { ignores: ["**/dist/**", "**/node_modules/**", "**/target/**", ".cache/**"] },
  js.configs.recommended,
  ...tseslint.configs.strict,
  // Flags web APIs missing from the browserslist target (Chromium 94, D14).
  compat.configs["flat/recommended"],
  {
    files: ["ui/**/*.ts"],
    languageOptions: { globals: globals.browser },
  },
  {
    files: ["spikes/*/web/src/**/*.{ts,tsx}"],
    languageOptions: { globals: globals.browser },
  },
  {
    files: ["spikes/*/web/**/*.js"],
    languageOptions: { sourceType: "script", globals: globals.browser },
  },
  {
    files: ["*.js", "scripts/**/*.mjs", "spikes/*/tools/**/*.mjs", "ui/*/vite.config.ts", "spikes/*/web/vite.config.ts"],
    languageOptions: { globals: globals.node },
  },
);
