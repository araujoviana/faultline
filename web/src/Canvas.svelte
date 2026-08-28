<script lang="ts">
import ActivityLog from "./lib/ActivityLog.svelte";
import { RESOURCE_KINDS } from "./lib/core";
import type { StudioStore } from "./lib/studio.svelte";

let { studio }: { studio: StudioStore } = $props();

const NODE_W = 120;
const NODE_H = 44;

function nodeById(id: string) {
  return studio.state.resources.find((r) => r.id === id);
}
</script>

<div class="layout">
  <aside class="palette">
    <h2>Add</h2>
    {#each RESOURCE_KINDS as kind (kind)}
      <button onclick={() => studio.addResource(kind, kind)}>+ {kind}</button>
    {/each}
    <hr />
    <button disabled={!studio.canUndo} onclick={() => studio.undo()}>Undo</button>
    <button class="ghost" onclick={() => studio.reset()}>Reset</button>
  </aside>

  <svg class="canvas" viewBox="0 0 700 620" preserveAspectRatio="xMidYMid meet" role="img" aria-label="Architecture canvas">
    {#each studio.state.edges as edge (edge.from + "->" + edge.to)}
      {@const a = nodeById(edge.from)}
      {@const b = nodeById(edge.to)}
      {#if a && b}
        <line
          x1={a.x + NODE_W / 2}
          y1={a.y + NODE_H / 2}
          x2={b.x + NODE_W / 2}
          y2={b.y + NODE_H / 2}
          class="edge"
        />
      {/if}
    {/each}

    {#each studio.state.resources as node (node.id)}
      <g transform="translate({node.x} {node.y})">
        <rect width={NODE_W} height={NODE_H} rx="9" class="node" data-kind={node.kind} />
        <text x={NODE_W / 2} y="19" class="label">{node.label}</text>
        <text x={NODE_W / 2} y="34" class="kind">{node.kind}</text>
      </g>
    {/each}

    {#if studio.state.resources.length === 0}
      <text x="350" y="310" class="hint">
        Add a resource — or ask your agent to.
      </text>
    {/if}
  </svg>

  <ActivityLog />
</div>

<style>
  .layout {
    display: grid;
    grid-template-columns: 9rem 1fr 18rem;
    gap: 1rem;
    align-items: start;
  }
  .palette {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }
  .palette h2 {
    font-size: 0.7rem;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--muted);
    margin: 0 0 0.2rem;
  }
  .palette button {
    text-align: left;
  }
  .palette hr {
    border: none;
    border-top: 1px solid var(--line);
    margin: 0.4rem 0;
  }
  .canvas {
    width: 100%;
    height: 70vh;
    border: 1px solid var(--line);
    border-radius: 12px;
    background:
      radial-gradient(circle at 1px 1px, var(--line) 1px, transparent 0) 0 0 / 24px 24px;
  }
  .node {
    fill: var(--node-fill);
    stroke: var(--node-stroke);
    stroke-width: 1.5;
  }
  .node[data-kind="database"] { stroke: #7c5cff; }
  .node[data-kind="load-balancer"] { stroke: #1aa3a3; }
  .node[data-kind="queue"] { stroke: #d98324; }
  .node[data-kind="object-store"] { stroke: #4a8ddc; }
  .node[data-kind="cache"] { stroke: #d94a7a; }
  .label {
    text-anchor: middle;
    font-size: 11px;
    font-weight: 600;
    fill: var(--fg);
  }
  .kind {
    text-anchor: middle;
    font-size: 8px;
    fill: var(--muted);
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }
  .edge {
    stroke: var(--muted);
    stroke-width: 1.5;
  }
  .hint {
    text-anchor: middle;
    fill: var(--muted);
    font-size: 13px;
  }
</style>
