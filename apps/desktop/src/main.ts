// =============================================================================
// Svelte App Entry Point
// =============================================================================
// This mounts the root Svelte component into the DOM.
// Everything else is managed by Svelte's component tree.
// =============================================================================

import App from "./App.svelte";

const app = new App({
  target: document.getElementById("app")!,
});

export default app;
