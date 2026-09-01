# Faultline

**You and your AI agent design a cloud architecture on a canvas — then fail an availability zone and
watch the blast radius spread.** Lint the design against cited resilience principles and generate the
Terraform, together.

🔗 **Live:** <https://faultline-studio.pages.dev> · 📖 **Tool surface:** [`/learn`](https://faultline-studio.pages.dev/learn)

<!-- TODO(before public flip): embed the core-loop GIF here (sketch → simulate → lint → IaC). -->
<!-- ![Faultline core loop](docs/media/core-loop.gif) -->

A **Rust-based submission to the [OpenAI WebMCP Challenge](https://openai.com/webmcp-challenge/)** — a
web app that gets meaningfully better when a human and their AI agent use it together, built on
[WebMCP](https://webmachinelearning.github.io/webmcp/). The human sets goals and picks trade-offs; the
agent proposes topology, wires dependencies, runs the failure simulation, lints for anti-patterns, and
generates infrastructure-as-code — through typed tools registered on `document.modelContext`.

**Design → Simulate → Harden.** The compute core (graph model, blast-radius analysis, SPOF scan,
resilience-lint rule engine, Terraform HCL emitter) is a pure Rust library compiled to WASM; the shell
is Svelte 5. **No API key, no account, no backend** — WebMCP ships no model, so the judge's own agent
supplies the reasoning and the tools are just JavaScript. **$0 of AI spend.**

## Runs in

- **Chrome 149+** — enable `chrome://flags/#enable-webmcp-testing`, relaunch
- **Edge 150+** — enable `edge://flags/#enable-webmcp-testing`, relaunch
- **ChatGPT Desktop** in-app browser — no flag; just open the URL and talk to the agent

Full manual test script: [`web/TESTING.md`](web/TESTING.md).

## The loop

1. **Design.** Drop resources on the infinite canvas (compute, database, load balancer, queue, cache,
   object store, CDN, DNS, functions, API gateway). The agent wires dependencies, sets AWS variants
   and region/AZ placement, and lays the diagram out.
2. **Simulate.** `simulate-failure` knocks out one availability zone; the blast radius propagates
   along the dependency edges and the canvas lights up — red for down, amber for degraded, with a
   summary banner. `find-spofs` rings every single point of failure and names what it orphans.
3. **Harden.** `resilience-lint` runs deterministic checks over the graph; each finding cites the
   chapter of *Designing Data-Intensive Applications* (2nd ed.) that explains why it matters. Fix the
   design, re-simulate, and `generate-iac` emits the Terraform for review.

Every agent action is one step on a shared undo stack, and every tool call lands in a live activity
log. The human stays in charge.

## WebMCP tools

Eight intent-level tools (source of truth: `web/src/tools/`, rendered live at `/learn`):

| Tool | Kind | What it does |
|---|---|---|
| `add-resource` | write | Add a node of a given kind + label |
| `connect` | write | Directed dependency edge `from → to` |
| `move-resource` | write | Reposition a node (mirrors a human drag) |
| `configure-resource` | write | Set provider variant and/or region/AZ placement |
| `simulate-failure` | read-only | Fail one AZ; report the blast radius |
| `find-spofs` | read-only | List single points of failure and what they orphan |
| `resilience-lint` | read-only | Rule-based resilience checks, each with a DDIA citation |
| `generate-iac` | read-only | Emit the architecture as Terraform HCL |

## Stack

| Layer | Choice |
|---|---|
| Compute core | **Rust → WASM** (`wasm-bindgen` + `wasm-pack`), `#![forbid(unsafe_code)]`, pure `input → output`, no async |
| UI shell | **Svelte 5** (runes) + **Vite**; hand-rolled SVG canvas, no diagram library |
| WebMCP glue | Thin **TypeScript** against `document.modelContext`, one file per tool |
| Hosting | Static deploy to **Cloudflare Pages**, tight CSP (`default-src 'self'`) |

## What is WebMCP?

An experimental browser API (W3C Web Machine Learning CG). A page declares JavaScript functions as
typed **tools** (name + natural-language description + JSON Schema) on `document.modelContext`; a
browser-side AI agent discovers and calls them instead of guessing its way through the UI. The page
acts as an in-browser MCP server whose tools run client-side code and touch the DOM directly.

- Spec: <https://webmachinelearning.github.io/webmcp/>
- Explainer: <https://github.com/webmachinelearning/webmcp>

## Development

```sh
rustup show                 # toolchain is pinned in rust-toolchain.toml
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo nextest run           # core tests (TDD — write the test first)
cargo deny check            # dependency audit

cd web
bun install
bun run dev                 # localhost + @mcp-b/global dev polyfill, no browser flag needed
bun run test                # vitest contract tests
bun run test:e2e            # Playwright — drives the full cold-open demo
bun run build               # runs wasm-pack, then vite build
```

CI (`.github/workflows/ci.yml`) runs fmt · clippy · tests · WASM browser tests · `cargo-deny` ·
Biome · build · e2e on every push. See [`AGENTS.md`](AGENTS.md) for contributor guidance.

## License

[MIT](LICENSE) © 2026 Matheus Araujo ([@araujoviana](https://github.com/araujoviana))
