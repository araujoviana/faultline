import { RESOURCE_KINDS, type ResourceKind } from "../lib/core";
import type { StudioStore } from "../lib/studio.svelte";
import type { WebMcpTool } from "../lib/webmcp-bridge";

/**
 * Add a single cloud resource to the canvas.
 *
 * Intentionally coarse: the agent names *what* to add, the studio decides
 * placement and id. All logic lives in the store (shared with the UI).
 */
export function addResourceTool(studio: StudioStore): WebMcpTool {
  return {
    name: "add-resource",
    title: "Add a resource",
    description:
      "Add one cloud building block to the architecture canvas. Call repeatedly to sketch a design.",
    inputSchema: {
      type: "object",
      properties: {
        kind: {
          type: "string",
          enum: [...RESOURCE_KINDS],
          description: "Which vendor-neutral building block to add.",
        },
        label: {
          type: "string",
          maxLength: 40,
          description: 'Short human-readable name, e.g. "orders API".',
        },
      },
      required: ["kind", "label"],
      additionalProperties: false,
    },
    annotations: { readOnlyHint: false },
    async execute(input) {
      const kind = String(input.kind ?? "");
      const label = String(input.label ?? "")
        .slice(0, 40)
        .trim();

      if (!RESOURCE_KINDS.includes(kind as ResourceKind)) {
        return {
          content: [
            {
              type: "text",
              text: `Unknown kind "${kind}". Valid kinds: ${RESOURCE_KINDS.join(", ")}.`,
            },
          ],
        };
      }
      if (!label) {
        return { content: [{ type: "text", text: "A non-empty label is required." }] };
      }

      const id = studio.addResource(kind, label);
      return { content: [{ type: "text", text: `Added ${kind} "${label}" as ${id}.` }] };
    },
  };
}
