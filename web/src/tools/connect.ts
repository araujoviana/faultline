import type { StudioStore } from "../lib/studio.svelte";
import type { WebMcpTool } from "../lib/webmcp-bridge";

/**
 * Draw a dependency edge between two existing resources.
 *
 * Direction matters: `from` depends on / sends traffic to `to`. Failure analysis
 * propagates along these edges.
 */
export function connectTool(studio: StudioStore): WebMcpTool {
  return {
    name: "connect",
    title: "Connect two resources",
    description:
      "Add a dependency edge — `from` depends on / calls `to`. Both resources must already exist.",
    inputSchema: {
      type: "object",
      properties: {
        from: {
          type: "string",
          description: 'Id of the dependent resource, e.g. "compute-1".',
        },
        to: {
          type: "string",
          description: 'Id of the depended-on resource, e.g. "database-1".',
        },
      },
      required: ["from", "to"],
      additionalProperties: false,
    },
    annotations: { readOnlyHint: false },
    async execute(input) {
      const from = String(input.from ?? "");
      const to = String(input.to ?? "");
      try {
        studio.connect(from, to);
      } catch (error) {
        return {
          content: [{ type: "text", text: error instanceof Error ? error.message : String(error) }],
        };
      }
      return { content: [{ type: "text", text: `Connected ${from} -> ${to}.` }] };
    },
  };
}
