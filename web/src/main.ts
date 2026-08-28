import { mount } from "svelte";
import App from "./App.svelte";
import "./app.css";
import { createStudio } from "./lib/studio.svelte";
import { loadWasmCore } from "./lib/wasm-core";

// The real Rust core, compiled by `build:wasm` (wired into predev/prebuild).
const studio = createStudio(await loadWasmCore());

const target = document.getElementById("app");
if (!target) throw new Error("missing #app mount point");

const app = mount(App, {
  target,
  props: { studio },
});

export default app;
