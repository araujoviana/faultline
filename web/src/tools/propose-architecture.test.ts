import { beforeEach, describe, expect, it } from "vitest";
import { clearActivity } from "../lib/activity.svelte";
import { createMemoryCore } from "../lib/core";
import { createStudio } from "../lib/studio.svelte";
import { instrumentTool } from "../lib/webmcp-bridge";
import { proposeArchitectureTool } from "./propose-architecture";

function setup() {
  const studio = createStudio(createMemoryCore());
  const tool = instrumentTool(proposeArchitectureTool(studio));
  return { studio, tool };
}

describe("propose-architecture tool", () => {
  beforeEach(() => clearActivity());

  it("replaces the canvas with a connected, configured architecture", async () => {
    const { studio, tool } = setup();
    studio.addResource("cdn", "leftover", 0, 0);

    const result = await tool.execute({ requirements: "public web app with a database" });

    expect(studio.state.resources.length).toBeGreaterThanOrEqual(3);
    expect(studio.state.resources.every((r) => r.variant)).toBe(true);
    expect(studio.state.edges.length).toBeGreaterThan(0);
    expect(studio.state.resources.some((r) => r.label === "leftover")).toBe(false);
    expect(result.content[0].text).toContain("Proposed a");
  });

  it("is undoable in one step", async () => {
    const { studio, tool } = setup();
    await tool.execute({ requirements: "web app" });
    expect(studio.canUndo).toBe(true);
    studio.undo();
    expect(studio.state.resources).toHaveLength(0);
  });

  it("requires a non-empty requirements string", async () => {
    const { tool } = setup();
    const result = await tool.execute({ requirements: "   " });
    expect(result.content[0].text).toContain("required");
  });

  it("advertises requirements as the one required input and is not read-only", () => {
    const { tool } = setup();
    expect(tool.annotations?.readOnlyHint).toBe(false);
    expect(tool.inputSchema).toMatchObject({ required: ["requirements"] });
  });
});
