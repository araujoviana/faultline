import type { StudioStore } from "../lib/studio.svelte";
import type { WebMcpTool } from "../lib/webmcp-bridge";

/**
 * Explain one thing on the canvas — a resource or a dependency edge — in plain
 * language: its role in the design, what it depends on and what depends on it,
 * what its loss would take down, and one architectural principle that applies
 * (cited to *Designing Data-Intensive Applications*). Read-only: it teaches, it
 * changes nothing.
 */
export function explainTool(studio: StudioStore): WebMcpTool {
  return {
    name: "explain",
    title: "Explain a resource or dependency",
    description:
      "Explain a selection in plain language: its role in the design, its dependencies and " +
      "dependents, what its failure would take down, and a relevant principle from Designing " +
      'Data-Intensive Applications (2nd ed.). Pass a resource id (e.g. "database-1") or an edge ' +
      'as "from->to" (e.g. "compute-1->database-1"). Read-only: reports, changes nothing.',
    inputSchema: {
      type: "object",
      properties: {
        selection: {
          type: "string",
          description:
            'A resource id ("database-1") or a dependency edge ("compute-1->database-1").',
        },
      },
      required: ["selection"],
      additionalProperties: false,
    },
    annotations: { readOnlyHint: true },
    async execute(input) {
      const selection = String(input.selection ?? "").trim();
      if (!selection) {
        return {
          content: [{ type: "text", text: "A selection (resource id or edge) is required." }],
        };
      }

      const e = studio.explain(selection);
      const lines: string[] = [`${e.subject} — ${e.summary}`];
      if (e.depends_on.length) lines.push(`Depends on: ${e.depends_on.join(", ")}`);
      if (e.depended_on_by.length) lines.push(`Depended on by: ${e.depended_on_by.join(", ")}`);
      if (e.takes_down.length) lines.push(`Its loss takes down: ${e.takes_down.join(", ")}`);
      for (const note of e.notes) lines.push(`- ${note}`);

      return { content: [{ type: "text", text: lines.join("\n") }] };
    },
  };
}
