<script lang="ts">
import { onMount } from "svelte";
import Canvas from "./Canvas.svelte";
import Learn from "./Learn.svelte";
import type { StudioStore } from "./lib/studio.svelte";
import { instrumentTool, registerTools } from "./lib/webmcp-bridge";
import { buildToolRegistry } from "./tools/registry";

let { studio }: { studio: StudioStore } = $props();

// `studio` is a stable singleton created once in main.ts.
// svelte-ignore state_referenced_locally
const tools = buildToolRegistry(studio);
let path = $state(window.location.pathname);

function go(to: string, event: MouseEvent) {
  event.preventDefault();
  window.history.pushState({}, "", to);
  path = to;
}

onMount(() => {
  const onPop = () => {
    path = window.location.pathname;
  };
  window.addEventListener("popstate", onPop);

  let controller: AbortController | undefined;
  registerTools(tools.map(instrumentTool)).then((c) => {
    controller = c;
  });

  return () => {
    window.removeEventListener("popstate", onPop);
    controller?.abort();
  };
});
</script>

<header>
  <span class="brand">
    <svg class="mark" viewBox="0 0 32 32" aria-hidden="true">
      <path d="M0 0 H19 V9 H14 V19 H20 V30 H15 V32 H0 Z" fill="#8b5cf6" />
      <path d="M21 0 H32 V32 H17 V30 H22 V19 H16 V9 H21 Z" fill="#6d28d9" />
    </svg>
    Faultline
  </span>
  <nav>
    <a href="/" aria-current={path === "/learn" ? undefined : "page"} onclick={(e) => go("/", e)}>
      Canvas
    </a>
    <a href="/learn" aria-current={path === "/learn" ? "page" : undefined} onclick={(e) => go("/learn", e)}>
      Learn
    </a>
  </nav>
</header>

<main>
  {#if path === "/learn"}
    <Learn {tools} />
  {:else}
    <Canvas {studio} />
  {/if}
</main>

<footer>
  <span>Faultline · Design → Simulate → Harden</span>
  <a
    href="https://github.com/araujoviana"
    target="_blank"
    rel="noopener noreferrer"
    aria-label="Matheus Araujo on GitHub"
  >
    <svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true" fill="currentColor">
      <path
        d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.01 8.01 0 0 0 16 8c0-4.42-3.58-8-8-8Z"
      />
    </svg>
    <span>araujoviana</span>
  </a>
</footer>

<style>
  header {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 0.5rem 1.5rem;
    padding: 0.75rem clamp(0.9rem, 3vw, 1.25rem);
    border-bottom: 1px solid var(--line);
  }
  .brand {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    font-weight: 700;
    letter-spacing: 0.02em;
  }
  .mark {
    width: 1.1em;
    height: 1.1em;
    border-radius: 4px;
  }
  nav {
    display: flex;
    gap: 1rem;
    margin-left: auto;
  }
  nav a {
    color: var(--muted);
    text-decoration: none;
  }
  nav a[aria-current="page"] {
    color: var(--fg);
    font-weight: 600;
  }
  main {
    width: 100%;
    max-width: 1600px;
    margin-inline: auto;
    padding: clamp(0.9rem, 3vw, 1.25rem);
  }
  footer {
    max-width: 1600px;
    margin-inline: auto;
    padding: 1rem clamp(0.9rem, 3vw, 1.25rem) 2rem;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem 1rem;
    font-size: 0.8rem;
    color: var(--muted);
  }
  footer a {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    color: var(--muted);
    text-decoration: none;
  }
  footer a:hover {
    color: var(--fg);
  }
</style>
