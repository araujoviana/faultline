import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { addResourceTool } from "../tools/add-resource";
import { clearActivity } from "./activity.svelte";
import { createMemoryCore } from "./core";
import { createStudio } from "./studio.svelte";
import { instrumentTool, registerTools } from "./webmcp-bridge";

describe("registerTools + @mcp-b/global polyfill", () => {
  beforeEach(() => clearActivity());

  // The polyfill defers its `toolchange` dispatch by a `setTimeout(0)`
  // (BrowserMcpServer.notifyProducerToolsChanged). Drain that macrotask before
  // the test ends so the dispatch runs inside the still-live jsdom realm rather
  // than surfacing as an unhandled rejection after teardown.
  afterEach(() => new Promise((resolve) => setTimeout(resolve, 20)));

  it("exposes the tool on document.modelContext and runs it end to end", async () => {
    const studio = createStudio(createMemoryCore());
    const controller = await registerTools([instrumentTool(addResourceTool(studio))]);

    const mc = document.modelContext;
    if (!mc) throw new Error("polyfill did not install document.modelContext");

    const tools = await mc.getTools();
    const tool = tools.find((t) => t.name === "add-resource");
    expect(tool).toBeTruthy();

    // Chrome's `executeTool` takes the input as a JSON string, not an object.
    const runner = mc as unknown as {
      executeTool(t: unknown, input: string): Promise<string | { content: unknown }>;
    };
    const result = await runner.executeTool(
      tool,
      JSON.stringify({ kind: "queue", label: "events" }),
    );

    expect(JSON.stringify(result)).toContain("queue");
    expect(studio.state.resources).toEqual([
      expect.objectContaining({ kind: "queue", label: "events", id: "queue-1" }),
    ]);

    controller.abort();
  });
});
