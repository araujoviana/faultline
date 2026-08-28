import { svelte } from "@sveltejs/vite-plugin-svelte";
import wasm from "vite-plugin-wasm";
import { defineConfig } from "vitest/config";

// Target browsers (Chrome 149+, Edge 150+, ChatGPT Desktop) all support
// top-level await natively, so no `vite-plugin-top-level-await` is needed.
// https://vite.dev/config/
export default defineConfig({
  plugins: [svelte(), wasm()],
  test: {
    environment: "jsdom",
    include: ["src/**/*.test.ts"],
  },
});
