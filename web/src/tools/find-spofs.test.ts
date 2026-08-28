import { beforeEach, describe, expect, it } from "vitest";
import { clearActivity } from "../lib/activity.svelte";
import type { Spof } from "../lib/core";
import { makeStubCore } from "../lib/stub-core";
import { createStudio } from "../lib/studio.svelte";
import { instrumentTool } from "../lib/webmcp-bridge";
import { findSpofsTool } from "./find-spofs";

function setup(spofs: Spof[]) {
  const core = makeStubCore({ findSpofs: () => JSON.stringify(spofs) });
  const studio = createStudio(core);
  const tool = instrumentTool(findSpofsTool(studio));
  return { studio, tool };
}

describe("find-spofs tool", () => {
  beforeEach(() => clearActivity());

  it("lists each SPOF and what it orphans", async () => {
    const { studio, tool } = setup([
      { id: "database-1", orphans: ["compute-1", "load-balancer-1"] },
    ]);

    const result = await tool.execute({});

    expect(result.content[0].text).toContain("database-1 — orphans compute-1, load-balancer-1");
    expect(studio.spofs).toHaveLength(1);
  });

  it("reports a clean design", async () => {
    const { tool } = setup([]);
    const result = await tool.execute({});
    expect(result.content[0].text).toBe("No single points of failure in the current design.");
  });

  it("is marked read-only with no required input", () => {
    const { tool } = setup([]);
    expect(tool.annotations?.readOnlyHint).toBe(true);
    expect(tool.inputSchema).toMatchObject({ additionalProperties: false });
  });
});
