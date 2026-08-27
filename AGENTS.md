# AGENTS.md

Guidance for anyone (human or AI assistant) contributing to this repo.

## What this is

A solo submission to the **OpenAI WebMCP Challenge** — a web app that is meaningfully better when a
human and their AI agent use it together, via [WebMCP](https://webmachinelearning.github.io/webmcp/)
tools. Submission deadline **2026-09-03**.

## Stack

| Layer | Choice |
|---|---|
| UI shell | **Svelte 5** (runes) + **Vite** |
| Compute core | **Rust → WASM** (`wasm-bindgen` + `wasm-pack`) — a pure `input → output` library, no async runtime |
| WebMCP glue | Thin **TypeScript** against `document.modelContext` |
| Hosting | Static deploy to an edge host + custom domain |

No full-stack Rust web framework. The Rust surface is a small, pure, heavily-tested compute library
compiled to WASM; the web app itself is Svelte/TS.

## Hard rules

- **TDD.** Write the failing test first. `cargo nextest` for the core; `wasm-bindgen-test` (headless
  Chrome) for the WASM boundary; a contract test for every WebMCP tool *before* the tool exists.
- **`#![forbid(unsafe_code)]`** in the Rust core. Commit `Cargo.lock`; build with `--locked`.
- Multi-step changes happen on a short-lived branch, merged to `main` only when CI is green. Never
  push without the maintainer's explicit go-ahead.
- **Target browsers:** Chrome 149+, Edge 150+, and the ChatGPT Desktop browser. Test all three
  before submitting. Code against `document.modelContext` (feature-detect; polyfill for wider local
  testing).
- Keep the WebMCP toolset **small and intent-level**. Register tools dynamically as app state
  changes. Mark user/network-derived tool output with `untrustedContentHint`. Require explicit human
  confirmation for high-impact actions.

## Commands

```sh
rustup show                                              # toolchain pinned in rust-toolchain.toml
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo nextest run
cargo deny check
# web (once scaffolded):
bun install && bun run dev
bunx --bun @biomejs/biome check --write .
```

## Conventions

- CI (`.github/workflows/ci.yml`): fmt · clippy · nextest · build · cargo-deny · Biome. Keep it green.
- Conventional-ish commit messages.
- Formatting: `rustfmt` + `clippy` for Rust, Biome for web. Run before committing.

## WebMCP integration notes

- Tools in `web/src/tools/`, one file per tool: `{ descriptor, schema, execute }`. `execute`
  delegates to the Svelte store and/or the WASM core — it never inlines logic the UI also needs.
- Return the MCP shape: `{ content: [{ type: "text", text }] }`. Wire `options.signal` into any
  async work.
- Agent edits should land as reviewable, uncommitted changes with full undo — not silent mutation.
- Ship a `/learn` route that renders the live tool list + schemas.
- Rust bridge: a small JS shim (`web/src/webmcp-bridge.ts`) wraps `registerTool`; `web-sys` has no
  `modelContext` bindings yet.
