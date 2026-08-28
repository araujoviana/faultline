import { mount } from "svelte";
import App from "./App.svelte";
import "./app.css";
import { createMemoryCore } from "./lib/core";
import { createStudio } from "./lib/studio.svelte";

// TODO: swap to `await loadWasmCore()` (see lib/wasm-core.ts) once `bun run
// build:wasm` runs in predev/prebuild.
const studio = createStudio(createMemoryCore());

const target = document.getElementById("app");
if (!target) throw new Error("missing #app mount point");

const app = mount(App, {
  target,
  props: { studio },
});

export default app;
