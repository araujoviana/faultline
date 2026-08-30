<script lang="ts">
import ActivityLog from "./lib/ActivityLog.svelte";
import { RESOURCE_KINDS, type ResourceNode } from "./lib/core";
import type { StudioStore } from "./lib/studio.svelte";

let { studio }: { studio: StudioStore } = $props();

const NODE_W = 124;
const NODE_H = 54;

// SVG viewBox extent — the "world" size before pan/zoom is applied.
const VIEW_W = 700;
const VIEW_H = 620;
const MIN_K = 0.35;
const MAX_K = 3;

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

// ---- pointer drag: move a node, one undo step on release ----
let svgEl: SVGSVGElement | undefined = $state();
let worldEl: SVGGElement | undefined = $state();
let drag = $state<{ id: string; offX: number; offY: number } | null>(null);
let ghost = $state<{ id: string; x: number; y: number } | null>(null);

// Node positions live in the world group's local space, so map straight
// through its CTM — that folds in both the viewBox scale and the pan/zoom
// transform below.
function toSvgPoint(e: PointerEvent): { x: number; y: number } | null {
  const ctm = worldEl?.getScreenCTM();
  if (!ctm) return null;
  const p = new DOMPoint(e.clientX, e.clientY).matrixTransform(ctm.inverse());
  return { x: p.x, y: p.y };
}

// ---- pan & zoom: a translate/scale on the world group ----
const clampK = (k: number) => Math.min(MAX_K, Math.max(MIN_K, k));
let view = $state({ x: 0, y: 0, k: 1 });
let pan = $state<{ cx: number; cy: number; ox: number; oy: number } | null>(null);

// client px -> SVG viewport (viewBox) coords, honouring preserveAspectRatio.
function toViewport(clientX: number, clientY: number): { x: number; y: number } | null {
  const ctm = svgEl?.getScreenCTM();
  if (!ctm) return null;
  const p = new DOMPoint(clientX, clientY).matrixTransform(ctm.inverse());
  return { x: p.x, y: p.y };
}

function zoomAt(clientX: number, clientY: number, factor: number) {
  const v = toViewport(clientX, clientY);
  if (!v) return;
  const k = clampK(view.k * factor);
  const ratio = k / view.k;
  // keep the world point under the cursor fixed on screen
  view = { k, x: v.x - (v.x - view.x) * ratio, y: v.y - (v.y - view.y) * ratio };
}

function onWheel(e: WheelEvent) {
  e.preventDefault();
  zoomAt(e.clientX, e.clientY, Math.exp(-e.deltaY * 0.0016));
}

function zoomStep(dir: 1 | -1) {
  const r = svgEl?.getBoundingClientRect();
  if (!r) return;
  zoomAt(r.left + r.width / 2, r.top + r.height / 2, dir > 0 ? 1.25 : 0.8);
}

function resetView() {
  view = { x: 0, y: 0, k: 1 };
}

function startPan(e: PointerEvent) {
  if (e.button !== 0 || drag) return;
  pan = { cx: e.clientX, cy: e.clientY, ox: view.x, oy: view.y };
  svgEl?.setPointerCapture(e.pointerId);
}

function movePan(e: PointerEvent) {
  if (!pan) return;
  const a = toViewport(pan.cx, pan.cy);
  const b = toViewport(e.clientX, e.clientY);
  if (!a || !b) return;
  view = { ...view, x: pan.ox + (b.x - a.x), y: pan.oy + (b.y - a.y) };
}

function endPan(e: PointerEvent) {
  pan = null;
  if (svgEl?.hasPointerCapture(e.pointerId)) svgEl.releasePointerCapture(e.pointerId);
}

// Attach wheel non-passively so preventDefault() actually blocks page scroll.
$effect(() => {
  const el = svgEl;
  if (!el) return;
  el.addEventListener("wheel", onWheel, { passive: false });
  return () => el.removeEventListener("wheel", onWheel);
});

function nodePos(node: ResourceNode): { x: number; y: number } {
  return ghost && ghost.id === node.id ? ghost : { x: node.x, y: node.y };
}

function startDrag(e: PointerEvent, node: ResourceNode) {
  const p = toSvgPoint(e);
  if (!p) return;
  e.preventDefault();
  e.stopPropagation(); // don't let the canvas start a pan
  drag = { id: node.id, offX: p.x - node.x, offY: p.y - node.y };
  ghost = { id: node.id, x: node.x, y: node.y };
  (e.currentTarget as Element).setPointerCapture(e.pointerId);
}

function moveDrag(e: PointerEvent) {
  if (!drag) return;
  const p = toSvgPoint(e);
  if (!p) return;
  ghost = { id: drag.id, x: p.x - drag.offX, y: p.y - drag.offY };
}

function endDrag() {
  if (drag && ghost) studio.move(ghost.id, Math.round(ghost.x), Math.round(ghost.y));
  drag = null;
  ghost = null;
}

/**
 * Manhattan path between two node boxes, anchored to the box edges. Routes
 * vertically or horizontally depending on which way the two nodes mostly lie,
 * with a single bend at the midpoint. Good enough until overlap-avoiding
 * routing lands (see docs/canvas-constraints.md, decision 7).
 */
function edgePath(a: ResourceNode, b: ResourceNode): string {
  const acx = a.x + NODE_W / 2;
  const acy = a.y + NODE_H / 2;
  const bcx = b.x + NODE_W / 2;
  const bcy = b.y + NODE_H / 2;
  const dx = bcx - acx;
  const dy = bcy - acy;

  if (Math.abs(dy) >= Math.abs(dx)) {
    const y1 = dy > 0 ? a.y + NODE_H : a.y;
    const y2 = dy > 0 ? b.y : b.y + NODE_H;
    if (Math.abs(acx - bcx) < 2) return `M${acx} ${y1} L${bcx} ${y2}`;
    const my = (y1 + y2) / 2;
    return `M${acx} ${y1} L${acx} ${my} L${bcx} ${my} L${bcx} ${y2}`;
  }

  const x1 = dx > 0 ? a.x + NODE_W : a.x;
  const x2 = dx > 0 ? b.x : b.x + NODE_W;
  if (Math.abs(acy - bcy) < 2) return `M${x1} ${acy} L${x2} ${bcy}`;
  const mx = (x1 + x2) / 2;
  return `M${x1} ${acy} L${mx} ${acy} L${mx} ${bcy} L${x2} ${bcy}`;
}

function dimEdge(from: string, to: string): boolean {
  return status(from) === "down" || status(to) === "down";
}

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

<!--
  Neutral glyph set — one per ResourceKind, drawn on a 16x16 grid with
  stroke = currentColor so a glyph inherits the kind tint or a status colour.
  Rendered inline (not via a child component or {@html}) so it stays in the
  SVG namespace. See docs/canvas-constraints.md.
-->
{#snippet glyph(kind: string)}
  {#if kind === "compute"}
    <rect x="2.5" y="2.5" width="11" height="11" rx="1.6" />
    <rect x="6" y="6" width="4" height="4" rx="0.5" />
    <path d="M8 1v1.5M8 13.5V15M1 8h1.5M13.5 8H15" />
  {:else if kind === "database"}
    <ellipse cx="8" cy="4" rx="5" ry="2.1" />
    <path d="M3 4v8c0 1.15 2.24 2.1 5 2.1s5-.95 5-2.1V4" />
    <path d="M3 8c0 1.15 2.24 2.1 5 2.1s5-.95 5-2.1" />
  {:else if kind === "queue"}
    <rect x="1.5" y="5" width="3.4" height="6" rx="1" />
    <rect x="6.3" y="5" width="3.4" height="6" rx="1" />
    <rect x="11.1" y="5" width="3.4" height="6" rx="1" />
  {:else if kind === "load-balancer"}
    <circle cx="8" cy="3" r="1.7" />
    <circle cx="3" cy="13" r="1.7" />
    <circle cx="13" cy="13" r="1.7" />
    <path d="M8 4.7 4 11.3M8 4.7l4 6.6" />
  {:else if kind === "object-store"}
    <path d="M2.5 4.5h11l-1 8.2a1.4 1.4 0 0 1-1.4 1.3H4.9a1.4 1.4 0 0 1-1.4-1.3z" />
    <path d="M1.6 4.5h12.8" />
    <path d="M5.7 4.5c0-2.3 4.6-2.3 4.6 0" />
  {:else if kind === "cache"}
    <circle cx="8" cy="9.3" r="5.2" />
    <path d="M8 9.3V6M6.3 1.6h3.4M8 1.6v2M12.4 4.9l1-1" />
  {:else if kind === "cdn"}
    <circle cx="8" cy="8" r="6" />
    <ellipse cx="8" cy="8" rx="2.6" ry="6" />
    <path d="M2 8h12M3 4.5h10M3 11.5h10" />
  {:else if kind === "dns"}
    <rect x="2" y="3.5" width="12" height="9" rx="1.5" />
    <circle cx="5.4" cy="8" r="1.5" />
    <path d="M8.6 6.4h3.4M8.6 9.6h3.4" />
  {:else if kind === "functions"}
    <path d="M3.5 13.5c2 0 2.6-1 3.4-3l2.2-5.6c.5-1.2 1.1-1.9 2.4-1.9h1" />
    <path d="M5.5 6.5 10.5 13.5" />
  {:else if kind === "api-gateway"}
    <path d="M6.2 2.5C4.4 2.5 4.9 6.7 3 8c1.9 1.3 1.4 5.5 3.2 5.5" />
    <path d="M9.8 2.5c1.8 0 1.3 4.2 3.2 5.5-1.9 1.3-1.4 5.5-3.2 5.5" />
  {/if}
{/snippet}

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

    <div class="canvas-wrap">
    <svg
      bind:this={svgEl}
      class="canvas"
      class:panning={!!pan}
      viewBox="0 0 {VIEW_W} {VIEW_H}"
      preserveAspectRatio="xMidYMid meet"
      role="img"
      aria-label="Architecture canvas — scroll to zoom, drag to pan"
      onpointerdown={startPan}
      onpointermove={movePan}
      onpointerup={endPan}
      onpointercancel={endPan}
    >
      <defs>
        <marker
          id="edge-arrow"
          viewBox="0 0 10 10"
          refX="8.5"
          refY="5"
          markerWidth="6"
          markerHeight="6"
          orient="auto-start-reverse"
        >
          <path d="M0 0 L10 5 L0 10 z" class="edge-head" />
        </marker>
        <pattern
          id="canvas-grid"
          width="24"
          height="24"
          patternUnits="userSpaceOnUse"
          patternTransform="translate({view.x} {view.y}) scale({view.k})"
        >
          <circle cx="1" cy="1" r="1" class="grid-dot" />
        </pattern>
      </defs>

      <rect
        class="grid-bg"
        x="0"
        y="0"
        width={VIEW_W}
        height={VIEW_H}
        fill="url(#canvas-grid)"
        pointer-events="none"
      />

      <g bind:this={worldEl} transform="translate({view.x} {view.y}) scale({view.k})">
      {#each studio.state.edges as edge (edge.from + "->" + edge.to)}
        {@const a = nodeById(edge.from)}
        {@const b = nodeById(edge.to)}
        {#if a && b}
          <path
            d={edgePath(a, b)}
            class="edge"
            class:dim={dimEdge(edge.from, edge.to)}
            marker-end="url(#edge-arrow)"
          />
        {/if}
      {/each}

      {#each studio.state.resources as node (node.id)}
        {@const pos = nodePos(node)}
        <g
          transform="translate({pos.x} {pos.y})"
          class="node-group"
          class:dragging={drag?.id === node.id}
          data-status={status(node.id)}
          style="--kc: var(--k-{node.kind})"
          role="button"
          tabindex="-1"
          aria-label="{node.label} — drag to reposition"
          onpointerdown={(e) => startDrag(e, node)}
          onpointermove={moveDrag}
          onpointerup={endDrag}
          onpointercancel={endDrag}
        >
          {#if spofIds.has(node.id)}
            <path
              class="spof-ring"
              d="M{6} {8} h{NODE_W} l{-6} {NODE_H} h{-NODE_W} z"
            />
          {/if}

          <rect width={NODE_W} height={NODE_H} rx="10" class="node" data-kind={node.kind} />

          <line class="strata-line" x1="9" x2={NODE_W - 9} y1="16" y2="16" />
          <line class="strata-line" x1="9" x2={NODE_W - 9} y1="19.5" y2="19.5" />
          <line class="strata-line" x1="9" x2={NODE_W - 9} y1="23" y2="23" />

          <rect class="spine" x="0" y="0" width="4" height={NODE_H} rx="2" />

          <rect class="chip" x="9" y="9" width="21" height="21" rx="6" />
          <g class="glyph" transform="translate(12 12)">
            {@render glyph(node.kind)}
          </g>

          <text class="label" x="38" y="22">{node.label}</text>
          <text class="variant" x="38" y="34">{variantName(node) || node.kind}</text>
          <text class="badge" x="10" y={NODE_H - 8}>{placementBadge(node)}</text>
        </g>
      {/each}

      {#if studio.state.resources.length === 0}
        <g class="empty" text-anchor="middle">
          <text class="empty-title" x="350" y="284">Design your architecture here</text>
          <text class="empty-body" x="350" y="311">Add a resource from the palette —</text>
          <text class="empty-body" x="350" y="329">or ask your agent to propose one.</text>
          <text class="empty-hint" x="350" y="361">
            “propose a resilient web stack in us-east-1”
          </text>
        </g>
      {/if}
      </g>
    </svg>

      <div class="view-tools">
        <button aria-label="Zoom out" onclick={() => zoomStep(-1)}>−</button>
        <button aria-label="Reset view" onclick={resetView}>{Math.round(view.k * 100)}%</button>
        <button aria-label="Zoom in" onclick={() => zoomStep(1)}>+</button>
      </div>
    </div>

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
    max-height: 26rem;
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
  .canvas-wrap {
    position: relative;
  }
  .canvas {
    display: block;
    width: 100%;
    height: clamp(20rem, 62vh, 44rem);
    border: 1px solid var(--line);
    border-radius: 12px;
    background: var(--bg);
    cursor: grab;
    touch-action: none;
    overflow: hidden;
  }
  .canvas.panning {
    cursor: grabbing;
  }
  .grid-dot {
    fill: var(--line);
  }

  .view-tools {
    position: absolute;
    right: 0.6rem;
    bottom: 0.6rem;
    display: flex;
    gap: 0.25rem;
  }
  .view-tools button {
    padding: 0.2rem 0.5rem;
    min-width: 2rem;
    background: var(--bg);
    line-height: 1.2;
  }

  /* ---- dependency edges: Manhattan, arrow points from -> to ---- */
  .edge {
    fill: none;
    stroke: var(--edge);
    stroke-width: 1.5;
    opacity: 0.8;
  }
  .edge.dim {
    opacity: 0.3;
  }
  .edge-head {
    fill: var(--edge);
  }

  /* ---- Strata node ---- */
  .node-group {
    cursor: grab;
    touch-action: none;
  }
  .node-group.dragging {
    cursor: grabbing;
  }
  .node-group.dragging .node {
    stroke: var(--kc);
    stroke-width: 1.5;
  }
  .node {
    fill: var(--node-fill);
    stroke: var(--line);
    stroke-width: 1;
  }
  .node-group[data-status="down"] .node {
    fill: var(--status-down-bg);
  }
  .node-group[data-status="degraded"] .node {
    fill: var(--status-degraded-bg);
  }

  .strata-line {
    stroke: var(--kc);
    stroke-width: 1;
    opacity: 0.13;
  }

  .spine {
    fill: var(--node-stroke);
  }
  .node-group[data-status="down"] .spine {
    fill: var(--status-down);
  }
  .node-group[data-status="degraded"] .spine {
    fill: var(--status-degraded);
  }

  .chip {
    fill: var(--kc);
    opacity: 0.16;
  }
  .glyph {
    fill: none;
    stroke: var(--kc);
    stroke-width: 1.35;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .node-group[data-status="down"] .glyph {
    stroke: var(--status-down);
  }
  .node-group[data-status="degraded"] .glyph {
    stroke: var(--status-degraded);
  }

  /* SPOF: the card throws an offset, sheared duplicate — a slip. */
  .spof-ring {
    fill: var(--spof);
    opacity: 0.16;
  }

  .label {
    text-anchor: start;
    font-size: 11px;
    font-weight: 600;
    fill: var(--fg);
  }
  .variant {
    text-anchor: start;
    font-size: 8px;
    fill: var(--muted);
  }
  .badge {
    text-anchor: start;
    font-size: 7.5px;
    fill: var(--muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .empty-title {
    fill: var(--fg);
    font-size: 15px;
    font-weight: 700;
  }
  .empty-body {
    fill: var(--muted);
    font-size: 11.5px;
  }
  .empty-hint {
    fill: var(--muted);
    font-size: 10.5px;
    font-family: ui-monospace, "SF Mono", Menlo, monospace;
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
  .swatch.down {
    background: var(--status-down-bg);
    border: 1.5px solid var(--status-down);
  }
  .swatch.degraded {
    background: var(--status-degraded-bg);
    border: 1.5px solid var(--status-degraded);
  }
  .swatch.spof {
    background: var(--spof);
    opacity: 0.35;
  }
</style>
