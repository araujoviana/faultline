/** A running log of every WebMCP tool call, newest first. Screenshot-worthy. */

export interface ActivityEntry {
  ts: number;
  tool: string;
  args: unknown;
  result: string;
  ok: boolean;
}

export const activity = $state<{ entries: ActivityEntry[] }>({ entries: [] });

export function logActivity(entry: Omit<ActivityEntry, "ts">): void {
  activity.entries.unshift({ ...entry, ts: Date.now() });
}

export function clearActivity(): void {
  activity.entries = [];
}
