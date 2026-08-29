import type { StudioStore } from "../lib/studio.svelte";
import type { WebMcpTool } from "../lib/webmcp-bridge";

/**
 * Reposition a resource on the canvas. Lets the agent lay a diagram out so the
 * human can read it — the same edit a human makes by dragging a node.
 */
export function moveResourceTool(studio: StudioStore): WebMcpTool {
  return {
    name: "move-resource",
    title: "Move a resource",
    description:
      "Set an existing resource's position on the canvas (top-left origin). Use it to lay out the diagram so the dependency flow reads clearly.",
    inputSchema: {
      type: "object",
      properties: {
        id: { type: "string", description: 'Resource id, e.g. "compute-1".' },
        x: { type: "number", description: "Horizontal position; 0 is the left edge." },
        y: { type: "number", description: "Vertical position; 0 is the top edge." },
      },
      required: ["id", "x", "y"],
      additionalProperties: false,
    },
    annotations: { readOnlyHint: false },
    async execute(input) {
      const id = String(input.id ?? "");
      const x = Number(input.x);
      const y = Number(input.y);

      if (!Number.isFinite(x) || !Number.isFinite(y)) {
        return { content: [{ type: "text", text: "x and y must be numbers" }] };
      }

      try {
        studio.move(id, x, y);
      } catch (error) {
        return {
          content: [{ type: "text", text: error instanceof Error ? error.message : String(error) }],
        };
      }

      return {
        content: [{ type: "text", text: `Moved ${id} to (${Math.round(x)}, ${Math.round(y)}).` }],
      };
    },
  };
}
