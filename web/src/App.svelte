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

<style>
  header {
    display: flex;
    align-items: baseline;
    gap: 1.5rem;
    padding: 0.75rem 1.25rem;
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
    padding: 1.25rem;
  }
</style>
