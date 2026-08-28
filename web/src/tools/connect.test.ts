import { beforeEach, describe, expect, it } from "vitest";
import { activity, clearActivity } from "../lib/activity.svelte";
import { createMemoryCore } from "../lib/core";
import { createStudio } from "../lib/studio.svelte";
import { instrumentTool } from "../lib/webmcp-bridge";
import { connectTool } from "./connect";

function setup() {
  const studio = createStudio(createMemoryCore());
  const tool = instrumentTool(connectTool(studio));
  return { studio, tool };
}

describe("connect tool", () => {
  beforeEach(() => clearActivity());

  it("adds a directed edge between two existing resources", async () => {
    const { studio, tool } = setup();
    const lb = studio.addResource("load-balancer", "edge");
    const api = studio.addResource("compute", "api");

    const result = await tool.execute({ from: lb, to: api });

    expect(studio.state.edges).toEqual([{ from: lb, to: api }]);
    expect(result.content[0].text).toBe(`Connected ${lb} -> ${api}.`);
    expect(activity.entries[0]).toMatchObject({ tool: "connect", ok: true });
  });

  it("reports an unknown endpoint without adding an edge", async () => {
    const { studio, tool } = setup();
    const api = studio.addResource("compute", "api");

    const result = await tool.execute({ from: api, to: "ghost-1" });

    expect(studio.state.edges).toHaveLength(0);
    expect(result.content[0].text).toContain("ghost-1");
  });

  it("requires both endpoints and forbids extra properties", () => {
    const { tool } = setup();
    const schema = tool.inputSchema as {
      required: string[];
      additionalProperties: boolean;
    };
    expect(schema.required).toEqual(["from", "to"]);
    expect(schema.additionalProperties).toBe(false);
  });
});
