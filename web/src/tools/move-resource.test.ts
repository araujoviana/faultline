import { beforeEach, describe, expect, it } from "vitest";
import { activity, clearActivity } from "../lib/activity.svelte";
import { createMemoryCore } from "../lib/core";
import { createStudio } from "../lib/studio.svelte";
import { instrumentTool } from "../lib/webmcp-bridge";
import { moveResourceTool } from "./move-resource";

function setup() {
  const studio = createStudio(createMemoryCore());
  const tool = instrumentTool(moveResourceTool(studio));
  return { studio, tool };
}

describe("move-resource tool", () => {
  beforeEach(() => clearActivity());

  it("sets a resource's canvas position", async () => {
    const { studio, tool } = setup();
    const n = studio.addResource("compute", "api");

    const result = await tool.execute({ id: n, x: 120, y: 240 });

    expect(studio.state.resources.find((r) => r.id === n)).toMatchObject({ x: 120, y: 240 });
    expect(result.content[0].text).toBe(`Moved ${n} to (120, 240).`);
    expect(activity.entries[0]).toMatchObject({ tool: "move-resource", ok: true });
  });

  it("is one undo step", async () => {
    const { studio, tool } = setup();
    const n = studio.addResource("compute", "api");
    await tool.execute({ id: n, x: 300, y: 300 });

    studio.undo();

    expect(studio.state.resources.find((r) => r.id === n)?.x).not.toBe(300);
  });

  it("reports an unknown id without throwing", async () => {
    const { tool } = setup();
    const result = await tool.execute({ id: "ghost-1", x: 0, y: 0 });
    expect(result.content[0].text).toContain("ghost-1");
  });

  it("rejects non-numeric coordinates", async () => {
    const { studio, tool } = setup();
    const n = studio.addResource("compute", "api");
    const result = await tool.execute({ id: n, x: "over there", y: 10 });
    expect(result.content[0].text).toContain("number");
  });

  it("requires id, x, y and forbids extra properties", () => {
    const { tool } = setup();
    const schema = tool.inputSchema as {
      required: string[];
      additionalProperties: boolean;
    };
    expect(schema.required).toEqual(["id", "x", "y"]);
    expect(schema.additionalProperties).toBe(false);
  });
});
