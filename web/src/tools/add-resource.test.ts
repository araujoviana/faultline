import { beforeEach, describe, expect, it } from "vitest";
import { activity, clearActivity } from "../lib/activity.svelte";
import { createMemoryCore } from "../lib/core";
import { createStudio } from "../lib/studio.svelte";
import { instrumentTool } from "../lib/webmcp-bridge";
import { addResourceTool } from "./add-resource";

function setup() {
  const studio = createStudio(createMemoryCore());
  const tool = instrumentTool(addResourceTool(studio));
  return { studio, tool };
}

describe("add-resource tool", () => {
  beforeEach(() => clearActivity());

  it("adds a resource of the requested kind and label to the store", async () => {
    const { studio, tool } = setup();

    const result = await tool.execute({ kind: "database", label: "orders db" });

    expect(studio.state.resources).toEqual([
      expect.objectContaining({ id: "database-1", kind: "database", label: "orders db" }),
    ]);
    expect(result).toEqual({
      content: [{ type: "text", text: 'Added database "orders db" as database-1.' }],
    });
  });

  it("records every call in the activity log", async () => {
    const { tool } = setup();

    await tool.execute({ kind: "compute", label: "api" });

    expect(activity.entries).toHaveLength(1);
    expect(activity.entries[0]).toMatchObject({ tool: "add-resource", ok: true });
  });

  it("rejects an unknown kind without mutating the store", async () => {
    const { studio, tool } = setup();

    const result = await tool.execute({ kind: "kubernetes", label: "cluster" });

    expect(studio.state.resources).toHaveLength(0);
    expect(result.content[0].text).toContain('Unknown kind "kubernetes"');
  });

  it("advertises kind as an enum and caps the label length", () => {
    const { tool } = setup();
    const schema = tool.inputSchema as {
      properties: { kind: { enum: string[] }; label: { maxLength: number } };
      required: string[];
    };

    expect(schema.properties.kind.enum).toContain("load-balancer");
    expect(schema.properties.label.maxLength).toBe(40);
    expect(schema.required).toEqual(["kind", "label"]);
  });
});
