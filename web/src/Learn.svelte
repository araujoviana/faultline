<script lang="ts">
import type { WebMcpTool } from "./lib/webmcp-bridge";

let { tools }: { tools: WebMcpTool[] } = $props();

function params(tool: WebMcpTool): string[] {
  const schema = tool.inputSchema as {
    properties?: Record<string, unknown>;
    required?: string[];
  };
  const all = Object.keys(schema.properties ?? {});
  const required = new Set(schema.required ?? []);
  return all.map((p) => (required.has(p) ? p : `${p}?`));
}

const reads = $derived(tools.filter((t) => t.annotations?.readOnlyHint));
const writes = $derived(tools.filter((t) => !t.annotations?.readOnlyHint));
</script>

<section class="learn">
  <header class="intro">
    <h1>What your agent can do here</h1>
    <p>
      Faultline registers these {tools.length} tools on <code>document.modelContext</code>. Your agent
      discovers and calls them directly — it reads the schema, not the screen. This is the exact
      surface it sees.
    </p>
  </header>

  {#snippet card(tool: WebMcpTool)}
    <article>
      <header>
        <code class="name">{tool.name}</code>
        {#if tool.annotations?.readOnlyHint}<span class="tag">read-only</span>{/if}
        {#if tool.annotations?.untrustedContentHint}<span class="tag warn">untrusted output</span>{/if}
      </header>
      <p>{tool.description}</p>
      {#if params(tool).length}
        <p class="params">
          {#each params(tool) as p (p)}<code>{p}</code>{/each}
        </p>
      {:else}
        <p class="params none">no input</p>
      {/if}
      <details>
        <summary>input schema</summary>
        <pre>{JSON.stringify(tool.inputSchema, null, 2)}</pre>
      </details>
    </article>
  {/snippet}

  <h2>Changes the design</h2>
  <div class="grid">
    {#each writes as tool (tool.name)}{@render card(tool)}{/each}
  </div>

  <h2>Read-only — analysis &amp; output</h2>
  <div class="grid">
    {#each reads as tool (tool.name)}{@render card(tool)}{/each}
  </div>
</section>

<style>
  .learn {
    max-width: 68rem;
  }
  .intro {
    max-width: 42rem;
  }
  .intro p {
    color: var(--muted);
    font-size: var(--text-base);
  }
  h1 {
    font-size: var(--text-lg);
    margin: 0 0 var(--space-3);
  }
  .learn h2 {
    font-size: var(--text-2xs);
    text-transform: uppercase;
    letter-spacing: 0.09em;
    color: var(--muted);
    margin: var(--space-6) 0 var(--space-3);
    padding-bottom: var(--space-2);
    border-bottom: 1px solid var(--line);
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(min(19rem, 100%), 1fr));
    gap: var(--space-3);
  }
  article {
    border: 1px solid var(--line);
    border-radius: var(--radius-lg);
    padding: var(--space-4);
    min-width: 0;
    background: var(--bg);
  }
  article > header {
    display: flex;
    align-items: baseline;
    gap: var(--space-2);
    flex-wrap: wrap;
    margin-bottom: var(--space-2);
  }
  article > p {
    color: var(--muted);
    font-size: var(--text-sm);
    margin: 0 0 var(--space-3);
  }
  .name {
    font-size: var(--text-base);
    font-weight: 650;
    color: var(--fg);
  }
  .params {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-1);
  }
  .params code {
    font-size: var(--text-2xs);
    background: var(--code-bg);
    border: 1px solid var(--line);
    border-radius: var(--radius-sm);
    padding: 0.05rem 0.4rem;
    color: var(--fg);
  }
  .params.none {
    font-size: var(--text-2xs);
    color: var(--muted);
    font-style: italic;
  }
  .tag {
    font-size: var(--text-2xs);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--muted);
    border: 1px solid var(--line-strong);
    border-radius: 999px;
    padding: 0.05rem 0.5rem;
  }
  .tag.warn {
    color: var(--status-degraded);
    border-color: var(--status-degraded);
  }
  summary {
    font-size: var(--text-xs);
    color: var(--muted);
    cursor: pointer;
    user-select: none;
    transition: color var(--dur) var(--ease);
  }
  summary:hover {
    color: var(--fg);
  }
  pre {
    background: var(--code-bg);
    border: 1px solid var(--line);
    border-radius: var(--radius-md);
    padding: var(--space-3);
    margin: var(--space-2) 0 0;
    max-height: 18rem;
    overflow: auto;
    font-size: var(--text-xs);
    line-height: 1.55;
  }

  @media (prefers-reduced-motion: reduce) {
    summary {
      transition: none;
    }
  }
</style>
