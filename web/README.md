# strata-web

The Svelte 5 + Vite shell for **Strata** — the design → simulate → harden cloud-architecture studio.

## Commands

```sh
bun install
bun run dev        # build:wasm, then the dev server (+ @mcp-b/global WebMCP polyfill)
bun run test       # vitest — WebMCP tool contract tests
bun run check      # svelte-check / typecheck
bun run lint       # biome (config lives at the repo root)
bun run build      # build:wasm, then the production bundle -> dist/
bun run build:wasm # compile ../wasm to src/lib/wasm/ (needs wasm-pack; runs in predev/prebuild)
```

Deploy (Cloudflare Pages via GitHub Actions): see [`../DEPLOY.md`](../DEPLOY.md).
`public/_headers` and `public/_redirects` ship the MIME/security headers and the
SPA fallback.

## Layout

| Path | What |
|---|---|
| `src/lib/core.ts` | `StudioCore` interface + `createMemoryCore` (used by the vitest contract tests) |
| `src/lib/wasm-core.ts` | `loadWasmCore()` — adapter over the real `strata-wasm` build; on the default path in `main.ts` |
| `src/lib/studio.svelte.ts` | reactive store: `$state` snapshot + single undo stack (human + agent edits) |
| `src/lib/webmcp-bridge.ts` | `registerTool` shim, feature-detect + dev polyfill, activity-log instrumentation |
| `src/lib/activity.svelte.ts` | the agent-activity log |
| `src/tools/` | one file per WebMCP tool: `{ descriptor, schema, execute }` + its contract test |
| `src/Canvas.svelte` / `src/Learn.svelte` | the `/` canvas and the `/learn` tool-surface page |

## WebMCP

Tools are registered on `document.modelContext` on mount and unregistered on teardown. In dev, the
`@mcp-b/global` polyfill is installed so tools work without Chrome 149+. The `/learn` route renders
the live tool list + schemas straight from `src/tools/registry.ts`.
