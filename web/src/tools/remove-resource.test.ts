import { beforeEach, describe, expect, it } from "vitest";
import { clearActivity } from "../lib/activity.svelte";
import { createMemoryCore } from "../lib/core";
import { createStudio } from "../lib/studio.svelte";
import { instrumentTool } from "../lib/webmcp-bridge";
import { removeResourceTool } from "./remove-resource";

function setup() {
  const studio = createStudio(createMemoryCore());
  const tool = instrumentTool(removeResourceTool(studio));
  return { studio, tool };
}

describe("remove-resource tool", () => {
  beforeEach(() => clearActivity());

  it("deletes the resource and its edges, in one undo step", async () => {
    const { studio, tool } = setup();
    studio.addResource("compute", "api", 0, 0);
    studio.addResource("database", "orders", 0, 0);
    studio.connect("compute-1", "database-1");

    const result = await tool.execute({ id: "compute-1" });

    expect(result.content[0].text).toBe("Removed compute-1 and its edges.");
    expect(studio.state.resources.map((r) => r.id)).toEqual(["database-1"]);
    expect(studio.state.edges).toHaveLength(0);
    studio.undo();
    expect(studio.state.resources).toHaveLength(2);
  });

  it("surfaces an unknown id as an error message", async () => {
    const { tool } = setup();
    const result = await tool.execute({ id: "ghost-1" });
    expect(result.content[0].text).toContain("ghost-1");
  });

  it("is a write tool requiring an id", () => {
    const { tool } = setup();
    expect(tool.annotations?.readOnlyHint).toBe(false);
    expect(tool.inputSchema).toMatchObject({ required: ["id"] });
  });
});
