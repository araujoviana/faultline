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
  <span class="brand">Strata</span>
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
    font-weight: 700;
    letter-spacing: 0.02em;
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
