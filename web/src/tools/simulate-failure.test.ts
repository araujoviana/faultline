import { beforeEach, describe, expect, it } from "vitest";
import { activity, clearActivity } from "../lib/activity.svelte";
import type { BlastReport } from "../lib/core";
import { makeStubCore } from "../lib/stub-core";
import { createStudio } from "../lib/studio.svelte";
import { instrumentTool } from "../lib/webmcp-bridge";
import { simulateFailureTool } from "./simulate-failure";

function setup(report: Partial<BlastReport> & { throws?: string } = {}) {
  const core = makeStubCore({
    simulateFailure: () => {
      if (report.throws) throw new Error(report.throws);
      return JSON.stringify({
        target: "AZ us-east-1a",
        down: report.down ?? [],
        degraded: report.degraded ?? [],
        healthy: report.healthy ?? [],
        notes: report.notes ?? [],
      });
    },
  });
  const studio = createStudio(core);
  const tool = instrumentTool(simulateFailureTool(studio));
  return { studio, tool };
}

describe("simulate-failure tool", () => {
  beforeEach(() => clearActivity());

  it("summarises the blast report and lists notes", async () => {
    const { studio, tool } = setup({
      down: ["compute-1", "database-1"],
      degraded: ["load-balancer-1"],
      notes: ["edge lost 1 of 2 compute dependencies"],
    });

    const result = await tool.execute({ region: "us-east-1", az: "us-east-1a" });

    const text = result.content[0].text;
    expect(text).toContain("2 down (compute-1, database-1)");
    expect(text).toContain("1 degraded (load-balancer-1)");
    expect(text).toContain("- edge lost 1 of 2 compute dependencies");
    expect(studio.lastReport?.down).toEqual(["compute-1", "database-1"]);
    expect(activity.entries[0]).toMatchObject({ tool: "simulate-failure", ok: true });
  });

  it("says so when nothing is affected", async () => {
    const { tool } = setup({});
    const result = await tool.execute({ region: "us-east-1", az: "us-east-1a" });
    expect(result.content[0].text).toContain("no impact");
  });

  it("surfaces a bad zone as an error message", async () => {
    const { tool } = setup({ throws: "us-east-9z is not an availability zone of us-east-1" });
    const result = await tool.execute({ region: "us-east-1", az: "us-east-9z" });
    expect(result.content[0].text).toContain("not an availability zone");
  });

  it("is marked read-only, hints untrusted output, and requires region + az", () => {
    const { tool } = setup();
    expect(tool.annotations?.readOnlyHint).toBe(true);
    expect(tool.annotations?.untrustedContentHint).toBe(true);
    expect((tool.inputSchema as { required: string[] }).required).toEqual(["region", "az"]);
  });
});
