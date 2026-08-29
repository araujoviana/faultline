<script lang="ts">
import type { WebMcpTool } from "./lib/webmcp-bridge";

let { tools }: { tools: WebMcpTool[] } = $props();
</script>

<section class="learn">
  <header class="intro">
    <h1>What the agent can do here</h1>
    <p>
      These {tools.length} WebMCP tools are registered on <code>document.modelContext</code> right
      now. Your agent discovers and calls them directly — no screenshots, no clicking.
    </p>
  </header>

  <div class="grid">
    {#each tools as tool (tool.name)}
      <article>
        <header>
          <code class="name">{tool.name}</code>
          {#if tool.annotations?.readOnlyHint}<span class="tag">read-only</span>{/if}
          {#if tool.annotations?.untrustedContentHint}<span class="tag">untrusted output</span>{/if}
        </header>
        <p>{tool.description}</p>
        <details>
          <summary>input schema</summary>
          <pre>{JSON.stringify(tool.inputSchema, null, 2)}</pre>
        </details>
      </article>
    {/each}
  </div>
</section>

<style>
  .intro {
    max-width: 44rem;
  }
  .intro p {
    color: var(--muted);
  }
  h1 {
    font-size: 1.4rem;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(min(20rem, 100%), 1fr));
    gap: 1rem;
    margin-top: 1.5rem;
  }
  article {
    border: 1px solid var(--line);
    border-radius: 12px;
    padding: 1rem;
    min-width: 0;
  }
  article header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex-wrap: wrap;
  }
  article > p {
    color: var(--muted);
    font-size: 0.9rem;
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
  summary {
    font-size: 0.78rem;
    color: var(--muted);
    cursor: pointer;
    user-select: none;
  }
  pre {
    background: var(--code-bg);
    border-radius: 8px;
    padding: 0.75rem;
    margin: 0.5rem 0 0;
    max-height: 18rem;
    overflow: auto;
    font-size: 0.8rem;
  }
</style>
