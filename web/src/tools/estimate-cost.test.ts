import { beforeEach, describe, expect, it } from "vitest";
import { clearActivity } from "../lib/activity.svelte";
import type { CostReport } from "../lib/core";
import { makeStubCore } from "../lib/stub-core";
import { createStudio } from "../lib/studio.svelte";
import { instrumentTool } from "../lib/webmcp-bridge";
import { estimateCostTool } from "./estimate-cost";

function setup(reports: CostReport[]) {
  let i = 0;
  const core = makeStubCore({
    estimateCost: () => JSON.stringify(reports[Math.min(i++, reports.length - 1)]),
  });
  const studio = createStudio(core);
  const tool = instrumentTool(estimateCostTool(studio));
  return { studio, tool };
}

const R1: CostReport = {
  total_monthly_usd: 163,
  lines: [
    { resource: "database-1", label: "orders", variant: "rds-single-az", monthly_usd: 78 },
    { resource: "compute-1", label: "api", variant: "ec2-asg", monthly_usd: 62 },
  ],
  unpriced: [],
};
const R2: CostReport = { ...R1, total_monthly_usd: 241, lines: R1.lines };

describe("estimate-cost tool", () => {
  beforeEach(() => clearActivity());

  it("reports the total and a per-resource breakdown", async () => {
    const { tool } = setup([R1]);
    const text = (await tool.execute({})).content[0].text;
    expect(text).toContain("Estimated $163.00/month");
    expect(text).toContain("database-1 (orders) — rds-single-az: $78.00/mo");
  });

  it("shows the delta from the second estimate on", async () => {
    const { tool } = setup([R1, R2]);
    await tool.execute({});
    const text = (await tool.execute({})).content[0].text;
    expect(text).toContain("+$78.00/mo since the last estimate");
  });

  it("names unpriced resources", async () => {
    const { tool } = setup([{ total_monthly_usd: 0, lines: [], unpriced: ["compute-1"] }]);
    const text = (await tool.execute({})).content[0].text;
    expect(text).toContain("Unpriced");
    expect(text).toContain("compute-1");
  });

  it("is read-only with no input", () => {
    const { tool } = setup([R1]);
    expect(tool.annotations?.readOnlyHint).toBe(true);
    expect(tool.inputSchema).toMatchObject({ additionalProperties: false });
  });
});
