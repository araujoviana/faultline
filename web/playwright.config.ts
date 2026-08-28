import { defineConfig, devices } from "@playwright/test";

/**
 * One integrated smoke spec: real WASM core + Svelte canvas + the `@mcp-b/global`
 * WebMCP polyfill, driven through the cold-open demo.
 *
 * Runs against the Vite dev server, not `dist/` — the polyfill is intentionally
 * dev-only (see `webmcp-bridge.ts`), so a production build has no
 * `document.modelContext` to drive. Real browsers get the flag; local automation
 * gets the polyfill. See `web/TESTING.md`.
 */
export default defineConfig({
  testDir: "./e2e",
  outputDir: "node_modules/.cache/playwright-artifacts",
  fullyParallel: false,
  workers: 1,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 1 : 0,
  reporter: "list",
  timeout: 30_000,
  use: {
    baseURL: "http://localhost:5173",
    trace: "on-first-retry",
  },
  projects: [{ name: "chromium", use: { ...devices["Desktop Chrome"] } }],
  webServer: {
    command: "bun run dev",
    url: "http://localhost:5173",
    reuseExistingServer: !process.env.CI,
    timeout: 180_000,
  },
});
