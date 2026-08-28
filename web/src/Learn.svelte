<script lang="ts">
import type { WebMcpTool } from "./lib/webmcp-bridge";

let { tools }: { tools: WebMcpTool[] } = $props();
</script>

<section class="learn">
  <h1>What the agent can do here</h1>
  <p>
    These WebMCP tools are registered on <code>document.modelContext</code> right now. Your agent
    discovers and calls them directly — no screenshots, no clicking.
  </p>

  {#each tools as tool (tool.name)}
    <article>
      <header>
        <code class="name">{tool.name}</code>
        {#if tool.annotations?.readOnlyHint}<span class="tag">read-only</span>{/if}
        {#if tool.annotations?.untrustedContentHint}<span class="tag">untrusted output</span>{/if}
      </header>
      <p>{tool.description}</p>
      <pre>{JSON.stringify(tool.inputSchema, null, 2)}</pre>
    </article>
  {/each}
</section>

<style>
  .learn {
    max-width: 44rem;
  }
  h1 {
    font-size: 1.4rem;
  }
  article {
    border: 1px solid var(--line);
    border-radius: 12px;
    padding: 1rem;
    margin-top: 1rem;
  }
  article header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }
  .name {
    font-size: 1rem;
    font-weight: 700;
  }
  .tag {
    font-size: 0.68rem;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--muted);
    border: 1px solid var(--line);
    border-radius: 999px;
    padding: 0.05rem 0.5rem;
  }
  pre {
    background: var(--code-bg);
    border-radius: 8px;
    padding: 0.75rem;
    overflow-x: auto;
    font-size: 0.8rem;
  }
</style>
