# webmcp-hackathon

A **Rust-based submission to the [OpenAI WebMCP Challenge](https://openai.com/webmcp-challenge/)** — a
web app that gets meaningfully better when a human and their AI agent use it together, built on
[WebMCP](https://webmachinelearning.github.io/webmcp/).

> **Status:** early scaffolding.

## What is WebMCP?

An experimental W3C (Web Machine Learning CG) browser API. A page declares JavaScript functions as
typed **tools** (name + natural-language description + JSON Schema) on `document.modelContext`; a
browser-side AI agent discovers and calls them instead of guessing its way through the UI. The page
acts as an in-browser MCP server whose tools run client-side code and touch the DOM directly.

- Spec: <https://webmachinelearning.github.io/webmcp/>
- Explainer: <https://github.com/webmachinelearning/webmcp>
- Runs today in: Chrome 149 / Edge 150 (Origin Trial), ChatGPT Desktop, Brave (Leo).

## Development

```sh
rustup show                 # toolchain is pinned in rust-toolchain.toml
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo nextest run           # tests (TDD — write the test first)
cargo deny check            # dependency audit
```

See [`AGENTS.md`](AGENTS.md) for contributor and agent guidance. CI runs fmt · clippy · tests · build ·
dependency audit on every push (`.github/workflows/ci.yml`).

## License

[MIT](LICENSE) © 2026 Matheus Araujo ([@araujoviana](https://github.com/araujoviana))
