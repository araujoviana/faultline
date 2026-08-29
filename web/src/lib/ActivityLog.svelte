<script lang="ts">
import { activity } from "./activity.svelte";

function time(ts: number) {
  return new Date(ts).toLocaleTimeString();
}
</script>

<aside class="activity">
  <h2>Agent activity</h2>
  {#if activity.entries.length === 0}
    <p class="empty">No tool calls yet.</p>
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
    border-radius: 12px;
    padding: 0.75rem;
    max-height: min(70vh, 34rem);
    overflow-y: auto;
  }
  @media (max-width: 1100px) {
    .activity {
      max-height: 22rem;
    }
  }
  h2 {
    font-size: 0.7rem;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--muted);
    margin: 0 0 0.5rem;
  }
  .empty {
    color: var(--muted);
    font-size: 0.85rem;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }
  li {
    border-left: 2px solid var(--node-stroke);
    padding-left: 0.5rem;
    font-size: 0.8rem;
  }
  li.err {
    border-left-color: #d94a7a;
  }
  .row {
    display: flex;
    justify-content: space-between;
    gap: 0.5rem;
  }
  .row time {
    color: var(--muted);
    font-size: 0.7rem;
  }
  .args {
    color: var(--muted);
    font-family: ui-monospace, monospace;
    font-size: 0.72rem;
    word-break: break-all;
  }
  .result {
    margin-top: 0.15rem;
  }
</style>
