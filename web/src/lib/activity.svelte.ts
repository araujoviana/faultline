/** A running log of every WebMCP tool call, newest first. Screenshot-worthy. */

export interface ActivityEntry {
  /** Monotonic, unique per entry — used as the render key. `ts` collides when
   *  several tool calls land in the same millisecond. */
  id: number;
  ts: number;
  tool: string;
  args: unknown;
  result: string;
  ok: boolean;
}

export const activity = $state<{ entries: ActivityEntry[] }>({ entries: [] });

let nextId = 0;

export function logActivity(entry: Omit<ActivityEntry, "ts" | "id">): void {
  activity.entries.unshift({ ...entry, id: nextId++, ts: Date.now() });
}

export function clearActivity(): void {
  activity.entries = [];
}
