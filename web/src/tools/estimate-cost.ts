import type { StudioStore } from "../lib/studio.svelte";
import type { WebMcpTool } from "../lib/webmcp-bridge";

/**
 * Rough monthly cost estimate for the current design, from the bundled pricing
 * snapshot in the active provider profile. Read-only. Reports the total, a
 * per-resource breakdown worst-first, any unpriced resources, and — from the
 * second call on — the change since the previous estimate, so the agent can
 * show the cost delta of a design change.
 */
export function estimateCostTool(studio: StudioStore): WebMcpTool {
  return {
    name: "estimate-cost",
    title: "Estimate monthly cost",
    description:
      "Order-of-magnitude monthly cost of the current design (USD), from the bundled pricing " +
      "snapshot — not a live quote. Returns the total, a per-resource breakdown, unpriced " +
      "resources, and the delta since the previous estimate. Read-only.",
    inputSchema: {
      type: "object",
      properties: {},
      additionalProperties: false,
    },
    annotations: { readOnlyHint: true, untrustedContentHint: true },
    async execute() {
      const report = studio.estimateCost();
      const money = (n: number) => `$${n.toFixed(2)}`;

      if (report.lines.length === 0 && report.unpriced.length === 0) {
        return { content: [{ type: "text", text: "Nothing to price yet — the canvas is empty." }] };
      }

      const lines = report.lines.map(
        (l) => `- ${l.resource} (${l.label}) — ${l.variant}: ${money(l.monthly_usd)}/mo`,
      );
      const parts = [`Estimated ${money(report.total_monthly_usd)}/month`];
      const delta = studio.costDelta;
      if (delta !== null && delta !== 0) {
        parts.push(
          `(${delta > 0 ? "+" : "−"}${money(Math.abs(delta))}/mo since the last estimate)`,
        );
      }
      let text = `${parts.join(" ")}\n${lines.join("\n")}`;
      if (report.unpriced.length) {
        text += `\nUnpriced (no variant set, or not in the profile): ${report.unpriced.join(", ")}`;
      }
      return { content: [{ type: "text", text }] };
    },
  };
}
