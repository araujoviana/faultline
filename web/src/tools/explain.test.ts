import { beforeEach, describe, expect, it } from "vitest";
import { clearActivity } from "../lib/activity.svelte";
import type { Explanation } from "../lib/core";
import { makeStubCore } from "../lib/stub-core";
import { createStudio } from "../lib/studio.svelte";
import { instrumentTool } from "../lib/webmcp-bridge";
import { explainTool } from "./explain";

function setup(explanation: Explanation) {
  const core = makeStubCore({ explain: () => JSON.stringify(explanation) });
  const studio = createStudio(core);
  const tool = instrumentTool(explainTool(studio));
  return { studio, tool };
}

const RESOURCE: Explanation = {
  subject: "database-1 (orders)",
  selection_kind: "resource",
  summary: "The system of record.",
  depends_on: [],
  depended_on_by: ["compute-1 (api)"],
  takes_down: ["compute-1 (api)", "load-balancer-1 (alb)"],
  notes: ["Its variant is a single point of failure.", "DDIA Ch 6."],
};

describe("explain tool", () => {
  beforeEach(() => clearActivity());

  it("renders subject, summary, dependents and blast radius", async () => {
    const { studio, tool } = setup(RESOURCE);
    const result = await tool.execute({ selection: "database-1" });
    const text = result.content[0].text;
    expect(text).toContain("database-1 (orders) — The system of record.");
    expect(text).toContain("Depended on by: compute-1 (api)");
    expect(text).toContain("Its loss takes down: compute-1 (api), load-balancer-1 (alb)");
    expect(text).toContain("- DDIA Ch 6.");
    expect(studio.explanation).toEqual(RESOURCE);
  });

  it("requires a non-empty selection", async () => {
    const { tool } = setup(RESOURCE);
    const result = await tool.execute({ selection: "  " });
    expect(result.content[0].text).toContain("selection");
  });

  it("is read-only and requires the selection input", () => {
    const { tool } = setup(RESOURCE);
    expect(tool.annotations?.readOnlyHint).toBe(true);
    expect(tool.inputSchema).toMatchObject({ required: ["selection"] });
  });
});
