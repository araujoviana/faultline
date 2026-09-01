import { beforeEach, describe, expect, it } from "vitest";
import { clearActivity } from "../lib/activity.svelte";
import type { Finding } from "../lib/core";
import { makeStubCore } from "../lib/stub-core";
import { createStudio } from "../lib/studio.svelte";
import { instrumentTool } from "../lib/webmcp-bridge";
import { resilienceLintTool } from "./resilience-lint";

function setup(findings: Finding[]) {
  const core = makeStubCore({ lint: () => JSON.stringify(findings) });
  const studio = createStudio(core);
  const tool = instrumentTool(resilienceLintTool(studio));
  return { studio, tool };
}

const SINGLE_AZ: Finding = {
  rule: "single-az-datastore",
  severity: "high",
  resource: "database-1",
  title: "orders has no cross-zone redundancy",
  detail: "database-1 (database) is a single-zone datastore.",
  citation: {
    source: "Designing Data-Intensive Applications, 2nd ed.",
    chapter: "Chapter 6: Replication",
    section: "Handling Node Outages",
  },
};

const SINGLE_REGION: Finding = {
  rule: "single-region",
  severity: "medium",
  resource: null,
  title: "The whole system lives in one region",
  detail: "Every placed resource is in us-east-1.",
  citation: {
    source: "Designing Data-Intensive Applications, 2nd ed.",
    chapter: "Chapter 6: Replication",
    section: "Multi-Region Operation",
  },
};

describe("resilience-lint tool", () => {
  beforeEach(() => clearActivity());

  it("formats each finding with its severity, rule id, resource and DDIA citation", async () => {
    const { studio, tool } = setup([SINGLE_AZ]);

    const result = await tool.execute({});
    const text = result.content[0].text;

    expect(text).toContain("1 finding(s):");
    expect(text).toContain("[HIGH] single-az-datastore:");
    expect(text).toContain("(database-1)");
    expect(text).toContain('DDIA — Chapter 6: Replication §"Handling Node Outages"');
    expect(studio.findings).toHaveLength(1);
  });

  it("omits the resource suffix for a whole-design finding", async () => {
    const { tool } = setup([SINGLE_REGION]);
    const text = (await tool.execute({})).content[0].text;
    expect(text).toContain("[MEDIUM] single-region: The whole system lives in one region —");
    expect(text).not.toContain("(null)");
  });

  it("reports a clean design", async () => {
    const { tool } = setup([]);
    const result = await tool.execute({});
    expect(result.content[0].text).toBe(
      "No resilience findings — the design has no known anti-patterns.",
    );
  });

  it("is marked read-only with no required input", () => {
    const { tool } = setup([]);
    expect(tool.annotations?.readOnlyHint).toBe(true);
    expect(tool.inputSchema).toMatchObject({ additionalProperties: false });
  });
});
