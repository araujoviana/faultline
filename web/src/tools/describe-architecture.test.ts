import { beforeEach, describe, expect, it } from "vitest";
import { clearActivity } from "../lib/activity.svelte";
import { createMemoryCore } from "../lib/core";
import { createStudio } from "../lib/studio.svelte";
import { instrumentTool } from "../lib/webmcp-bridge";
import { describeArchitectureTool } from "./describe-architecture";

function setup() {
  const studio = createStudio(createMemoryCore());
  const tool = instrumentTool(describeArchitectureTool(studio));
  return { studio, tool };
}

describe("describe-architecture tool", () => {
  beforeEach(() => clearActivity());

  it("reports an empty canvas", async () => {
    const { tool } = setup();
    const result = await tool.execute({});
    expect(result.content[0].text).toBe("The canvas is empty.");
  });

  it("lists resources with config and the dependency edges", async () => {
    const { studio, tool } = setup();
    studio.addResource("load-balancer", "edge", 0, 0);
    studio.addResource("database", "orders", 0, 0);
    studio.configure("database-1", "rds-single-az", "us-east-1", "us-east-1a");
    studio.connect("load-balancer-1", "database-1");

    const text = (await tool.execute({})).content[0].text;
    expect(text).toContain('load-balancer-1 (load-balancer) "edge" — no variant, unplaced');
    expect(text).toContain('database-1 (database) "orders" — rds-single-az, us-east-1a');
    expect(text).toContain("load-balancer-1 → database-1");
  });

  it("is read-only, hints untrusted output, and takes no input", () => {
    const { tool } = setup();
    expect(tool.annotations?.readOnlyHint).toBe(true);
    expect(tool.annotations?.untrustedContentHint).toBe(true);
    expect(tool.inputSchema).toMatchObject({ additionalProperties: false });
  });
});
