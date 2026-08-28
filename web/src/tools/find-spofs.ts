import type { StudioStore } from "../lib/studio.svelte";
import type { WebMcpTool } from "../lib/webmcp-bridge";

/**
 * Report single points of failure in the current design — resources whose
 * provider variant has no built-in redundancy — and what each would orphan.
 * Also rings the offending nodes on the canvas.
 */
export function findSpofsTool(studio: StudioStore): WebMcpTool {
  return {
    name: "find-spofs",
    title: "Find single points of failure",
    description:
      "List resources that are single points of failure by construction, with the resources each would take down.",
    inputSchema: {
      type: "object",
      properties: {},
      additionalProperties: false,
    },
    annotations: { readOnlyHint: true },
    async execute() {
      const found = studio.findSpofs();
      if (found.length === 0) {
        return {
          content: [{ type: "text", text: "No single points of failure in the current design." }],
        };
      }
      const lines = found.map(
        (s) => `${s.id} — orphans ${s.orphans.length ? s.orphans.join(", ") : "nothing else"}`,
      );
      return {
        content: [
          {
            type: "text",
            text: `${found.length} single point(s) of failure:\n${lines.map((l) => `- ${l}`).join("\n")}`,
          },
        ],
      };
    },
  };
}
