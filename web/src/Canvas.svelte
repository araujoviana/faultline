<script lang="ts">
import ActivityLog from "./lib/ActivityLog.svelte";
import { RESOURCE_KINDS, type ResourceNode } from "./lib/core";
import type { StudioStore } from "./lib/studio.svelte";

let { studio }: { studio: StudioStore } = $props();

const NODE_W = 120;
const NODE_H = 48;

function nodeById(id: string) {
  return studio.state.resources.find((r) => r.id === id);
}

function variantName(node: ResourceNode): string {
  const list = studio.profile.variants[node.kind] ?? [];
  return list.find((v) => v.id === node.variant)?.display_name ?? node.variant ?? "";
}

function placementBadge(node: ResourceNode): string {
  return node.placement?.az ?? node.placement?.region ?? "unplaced";
}

function status(id: string): "down" | "degraded" | "" {
  const r = studio.lastReport;
  if (!r) return "";
  if (r.down.includes(id)) return "down";
  if (r.degraded.includes(id)) return "degraded";
  return "";
}

const spofIds = $derived(new Set(studio.spofs.map((s) => s.id)));

const zones = $derived(
  studio.profile.regions.flatMap((r) => r.azs.map((az) => ({ region: r.id, az }))),
);
let selectedZone = $state("");
$effect(() => {
  if (!selectedZone && zones.length) selectedZone = `${zones[0].region}|${zones[0].az}`;
});

function runSimulation() {
  const [region, az] = selectedZone.split("|");
  if (region && az) studio.simulateFailure(region, az);
}
</script>

<div class="layout">
  <aside class="palette">
    <h2>Add</h2>
    <div class="kind-list">
      {#each RESOURCE_KINDS as kind (kind)}
        <button onclick={() => studio.addResource(kind, kind)}>+ {kind}</button>
      {/each}
    </div>
    <hr />
    <h2>Simulate</h2>
    {#if zones.length}
      <select bind:value={selectedZone} aria-label="Availability zone to fail">
        {#each zones as z (z.region + z.az)}
          <option value={`${z.region}|${z.az}`}>{z.az}</option>
        {/each}
      </select>
      <button onclick={runSimulation}>Fail this AZ</button>
    {/if}
    <button onclick={() => studio.findSpofs()}>Scan for SPOFs</button>
  </aside>

  <div class="stage">
    <div class="stage-tools">
      <button disabled={!studio.canUndo} onclick={() => studio.undo()}>Undo</button>
      <button class="ghost" onclick={() => studio.reset()}>Reset</button>
    </div>

    {#if studio.lastReport}
      {@const r = studio.lastReport}
      <div class="banner" role="status">
        <strong>{r.target}</strong> — {r.down.length} down, {r.degraded.length} degraded
        <button class="ghost" onclick={() => studio.clearAnalysis()}>Clear</button>
      </div>
    {/if}

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
        <g transform="translate({node.x} {node.y})" class="node-group" data-status={status(node.id)}>
          {#if spofIds.has(node.id)}
            <rect
              x="-4" y="-4" width={NODE_W + 8} height={NODE_H + 8} rx="12"
              class="spof-ring"
            />
          {/if}
          <rect width={NODE_W} height={NODE_H} rx="9" class="node" data-kind={node.kind} />
          <text x={NODE_W / 2} y="16" class="label">{node.label}</text>
          <text x={NODE_W / 2} y="29" class="variant">{variantName(node) || node.kind}</text>
          <text x={NODE_W / 2} y="41" class="badge">{placementBadge(node)}</text>
        </g>
      {/each}

      {#if studio.state.resources.length === 0}
        <text x="350" y="310" class="hint">
          Add a resource — or ask your agent to.
        </text>
      {/if}
    </svg>

    <div class="legend">
      <span class="swatch down"></span> down
      <span class="swatch degraded"></span> degraded
      <span class="swatch spof"></span> single point of failure
    </div>
  </div>

  <div class="activity-col">
    <ActivityLog />
  </div>
</div>

<style>
  .layout {
    display: grid;
    grid-template-columns: 11rem minmax(0, 1fr) 19rem;
    grid-template-areas: "palette stage activity";
    gap: 1rem;
    align-items: start;
  }
  .palette {
    grid-area: palette;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    position: sticky;
    top: 1rem;
  }
  .stage {
    grid-area: stage;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    min-width: 0;
  }
  .activity-col {
    grid-area: activity;
  }

  /* Tablet: activity log drops below the working area. */
  @media (max-width: 1100px) {
    .layout {
      grid-template-columns: 11rem minmax(0, 1fr);
      grid-template-areas:
        "palette stage"
        "activity activity";
    }
  }

  /* Phone: everything stacks; the palette becomes a wrapped button bar. */
  @media (max-width: 720px) {
    .layout {
      grid-template-columns: 1fr;
      grid-template-areas:
        "palette"
        "stage"
        "activity";
    }
    .palette {
      position: static;
      flex-direction: row;
      flex-wrap: wrap;
      align-items: center;
    }
    .palette .kind-list {
      flex-direction: row;
      flex-wrap: wrap;
      max-height: none;
      overflow: visible;
    }
    .palette hr {
      display: none;
    }
    .palette h2 {
      width: 100%;
      margin-top: 0.3rem;
    }
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
  .palette select {
    width: 100%;
  }
  .palette hr {
    border: none;
    border-top: 1px solid var(--line);
    margin: 0.4rem 0;
    width: 100%;
  }
  /* The Add list scrolls inside itself so a long catalogue never pushes
     Simulate / SPOF controls off the bottom of the sidebar. */
  .kind-list {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    max-height: 15rem;
    overflow-y: auto;
  }
  .stage-tools {
    display: flex;
    gap: 0.4rem;
    justify-content: flex-end;
  }
  .banner {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.4rem 0.75rem;
    border: 1px solid var(--line);
    border-radius: 8px;
    font-size: 0.85rem;
  }
  .banner button {
    margin-left: auto;
  }
  .canvas {
    width: 100%;
    height: clamp(20rem, 62vh, 44rem);
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
  .node-group[data-status="down"] .node {
    fill: #f8d7da;
    stroke: #c0392b;
    stroke-width: 2.5;
  }
  .node-group[data-status="degraded"] .node {
    fill: #fdf0d5;
    stroke: #d98324;
    stroke-width: 2.5;
  }
  .spof-ring {
    fill: none;
    stroke: #c0392b;
    stroke-width: 1.5;
    stroke-dasharray: 4 3;
  }
  .label {
    text-anchor: middle;
    font-size: 11px;
    font-weight: 600;
    fill: var(--fg);
  }
  .variant {
    text-anchor: middle;
    font-size: 8px;
    fill: var(--muted);
  }
  .badge {
    text-anchor: middle;
    font-size: 7.5px;
    fill: var(--muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
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
  .legend {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    font-size: 0.75rem;
    color: var(--muted);
  }
  .legend .swatch {
    width: 0.8rem;
    height: 0.8rem;
    border-radius: 3px;
    display: inline-block;
  }
  .legend .swatch:not(:first-child) {
    margin-left: 0.75rem;
  }
  .swatch.down { background: #f8d7da; border: 1.5px solid #c0392b; }
  .swatch.degraded { background: #fdf0d5; border: 1.5px solid #d98324; }
  .swatch.spof { background: transparent; border: 1.5px dashed #c0392b; }
</style>
