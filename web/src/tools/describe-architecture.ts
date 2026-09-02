import type { StudioStore } from "../lib/studio.svelte";
import type { WebMcpTool } from "../lib/webmcp-bridge";

/**
 * Read the whole canvas back as structured text — every resource with its
 * config, and every dependency edge. The agent's way to see what is already
 * there before it changes anything. Read-only.
 */
export function describeArchitectureTool(studio: StudioStore): WebMcpTool {
  return {
    name: "describe-architecture",
    title: "Describe the current architecture",
    description:
      "List every resource on the canvas (id, kind, label, provider variant, region/zone placement) " +
      "and every dependency edge. Call this first to see the current design before adding, connecting " +
      "or configuring anything. Read-only.",
    inputSchema: {
      type: "object",
      properties: {},
      additionalProperties: false,
    },
    annotations: { readOnlyHint: true, untrustedContentHint: true },
    async execute() {
      const { resources, edges } = studio.state;

      if (resources.length === 0) {
        return { content: [{ type: "text", text: "The canvas is empty." }] };
      }

      const lines = resources.map((r) => {
        const variant = r.variant ? r.variant : "no variant";
        const place = r.placement?.az ?? r.placement?.region ?? "unplaced";
        return `- ${r.id} (${r.kind}) "${r.label}" — ${variant}, ${place}`;
      });

      const edgeLines = edges.length ? edges.map((e) => `- ${e.from} → ${e.to}`) : ["- (none)"];

      return {
        content: [
          {
            type: "text",
            text:
              `${resources.length} resource(s):\n${lines.join("\n")}\n\n` +
              `${edges.length} dependency edge(s) (from depends on to):\n${edgeLines.join("\n")}`,
          },
        ],
      };
    },
  };
}
