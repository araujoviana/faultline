import type { StudioStore } from "../lib/studio.svelte";
import type { WebMcpTool } from "../lib/webmcp-bridge";

/**
 * Delete a resource and any edges touching it. The same edit a human makes with
 * the inspector's Delete button; lets the agent undo its own mistakes granularly.
 * One undo step.
 */
export function removeResourceTool(studio: StudioStore): WebMcpTool {
  return {
    name: "remove-resource",
    title: "Remove a resource",
    description:
      "Delete a resource from the canvas by id. Any dependency edges to or from it are removed too. One undo step.",
    inputSchema: {
      type: "object",
      properties: {
        id: { type: "string", description: 'Resource id, e.g. "cache-1".' },
      },
      required: ["id"],
      additionalProperties: false,
    },
    annotations: { readOnlyHint: false },
    async execute(input) {
      const id = String(input.id ?? "");

      try {
        studio.removeResource(id);
      } catch (error) {
        return {
          content: [{ type: "text", text: error instanceof Error ? error.message : String(error) }],
        };
      }

      return { content: [{ type: "text", text: `Removed ${id} and its edges.` }] };
    },
  };
}
