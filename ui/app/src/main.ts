import { describe } from "./describe.js";

// Placeholder shell (T0.1.2). The UI framework is picked by spike S2 (SolidJS vs Svelte 5, Q25).
const root = document.getElementById("root");
if (root) {
  root.textContent = describe();
}
