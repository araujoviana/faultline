<script lang="ts">
import { activity } from "./activity.svelte";

function time(ts: number) {
  return new Date(ts).toLocaleTimeString();
}
</script>

<aside class="activity">
  <h2>Agent activity</h2>
  {#if activity.entries.length === 0}
    <p class="empty">No tool calls yet. When your agent acts, every call lands here.</p>
  {:else}
    <ul>
      {#each activity.entries as entry (entry.id)}
        <li class:err={!entry.ok}>
          <div class="row">
            <code>{entry.tool}</code>
            <time>{time(entry.ts)}</time>
          </div>
          <div class="args">{JSON.stringify(entry.args)}</div>
          <div class="result">{entry.result}</div>
        </li>
      {/each}
    </ul>
  {/if}
</aside>

<style>
  .activity {
    border: 1px solid var(--line);
    border-radius: var(--radius-lg);
    padding: var(--space-3);
    max-height: min(70vh, 34rem);
    overflow-y: auto;
    background: var(--bg-sunken);
  }
  @media (max-width: 1100px) {
    .activity {
      max-height: 22rem;
    }
  }
  h2 {
    font-size: var(--text-2xs);
    text-transform: uppercase;
    letter-spacing: 0.09em;
    color: var(--muted);
    margin: 0 0 var(--space-3);
  }
  .empty {
    color: var(--muted);
    font-size: var(--text-sm);
    line-height: 1.5;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }
  li {
    border-left: 2px solid var(--node-stroke);
    padding-left: var(--space-2);
    font-size: var(--text-xs);
  }
  li.err {
    border-left-color: var(--status-down);
  }
  .row {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    gap: var(--space-2);
  }
  .row code {
    font-weight: 600;
    color: var(--fg);
  }
  .row time {
    color: var(--muted);
    font-size: var(--text-2xs);
    font-variant-numeric: tabular-nums;
    flex-shrink: 0;
  }
  .args {
    color: var(--muted);
    font-family: var(--font-mono);
    font-size: var(--text-2xs);
    word-break: break-all;
    margin-top: 0.1rem;
  }
  .result {
    margin-top: 0.2rem;
    line-height: 1.45;
  }
  li.err .result {
    color: var(--status-down);
  }
</style>
