import type { StudioStore } from "../lib/studio.svelte";
import type { WebMcpTool } from "../lib/webmcp-bridge";

/**
 * Simulate the loss of one availability zone and report the blast radius. The
 * result also drives the canvas overlay (down = red, degraded = amber).
 */
export function simulateFailureTool(studio: StudioStore): WebMcpTool {
  return {
    name: "simulate-failure",
    title: "Simulate an AZ failure",
    description:
      "Knock out one availability zone and report which resources go down, which degrade, and why.",
    inputSchema: {
      type: "object",
      properties: {
        region: { type: "string", description: 'Region id, e.g. "us-east-1".' },
        az: { type: "string", description: 'Availability zone to fail, e.g. "us-east-1a".' },
      },
      required: ["region", "az"],
      additionalProperties: false,
    },
    annotations: { readOnlyHint: true },
    async execute(input) {
      const region = String(input.region ?? "");
      const az = String(input.az ?? "");

      let report: ReturnType<StudioStore["simulateFailure"]>;
      try {
        report = studio.simulateFailure(region, az);
      } catch (error) {
        return {
          content: [{ type: "text", text: error instanceof Error ? error.message : String(error) }],
        };
      }

      const lines: string[] = [];
      if (report.down.length === 0 && report.degraded.length === 0) {
        lines.push(`AZ ${az} failure — no impact on the current design.`);
      } else {
        lines.push(
          `AZ ${az} failure — ${report.down.length} down` +
            (report.down.length ? ` (${report.down.join(", ")})` : "") +
            `, ${report.degraded.length} degraded` +
            (report.degraded.length ? ` (${report.degraded.join(", ")})` : "") +
            ".",
        );
      }
      for (const note of report.notes) lines.push(`- ${note}`);
      return { content: [{ type: "text", text: lines.join("\n") }] };
    },
  };
}
