import { beforeEach, describe, expect, it } from "vitest";
import { activity, clearActivity, logActivity } from "./activity.svelte";

describe("activity log", () => {
  beforeEach(() => clearActivity());

  it("gives every entry a unique id, even within one millisecond", () => {
    for (let i = 0; i < 5; i++) {
      logActivity({ tool: "add-resource", args: {}, result: "ok", ok: true });
    }
    const ids = activity.entries.map((e) => e.id);
    expect(new Set(ids).size).toBe(5);
  });

  it("keeps newest first", () => {
    logActivity({ tool: "a", args: {}, result: "", ok: true });
    logActivity({ tool: "b", args: {}, result: "", ok: true });
    expect(activity.entries[0].tool).toBe("b");
  });
});
