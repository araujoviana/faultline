# strata-web

The Svelte 5 + Vite shell for **Strata** — the design → simulate → harden cloud-architecture studio.

## Commands

```sh
bun install
bun run dev        # dev server (installs the @mcp-b/global WebMCP polyfill automatically)
bun run test       # vitest — WebMCP tool contract tests
bun run check      # svelte-check / typecheck
bun run lint       # biome (config lives at the repo root)
bun run build      # production bundle -> dist/
bun run build:wasm # compile ../wasm to src/lib/wasm/ (needs wasm-pack)
```

## Layout

| Path | What |
|---|---|
| `src/lib/core.ts` | `StudioCore` interface + `createMemoryCore` (temporary stand-in for the Rust WASM core) |
| `src/lib/wasm-core.ts` | adapter over the real `strata-wasm` build (not yet on the default path) |
| `src/lib/studio.svelte.ts` | reactive store: `$state` snapshot + single undo stack (human + agent edits) |
| `src/lib/webmcp-bridge.ts` | `registerTool` shim, feature-detect + dev polyfill, activity-log instrumentation |
| `src/lib/activity.svelte.ts` | the agent-activity log |
| `src/tools/` | one file per WebMCP tool: `{ descriptor, schema, execute }` + its contract test |
| `src/Canvas.svelte` / `src/Learn.svelte` | the `/` canvas and the `/learn` tool-surface page |

## WebMCP

Tools are registered on `document.modelContext` on mount and unregistered on teardown. In dev, the
`@mcp-b/global` polyfill is installed so tools work without Chrome 149+. The `/learn` route renders
the live tool list + schemas straight from `src/tools/registry.ts`.
