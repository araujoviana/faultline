import type { StudioStore } from "../lib/studio.svelte";
import type { WebMcpTool } from "../lib/webmcp-bridge";

/**
 * Simulate the loss of one availability zone (`az` set) or a whole region (`az`
 * omitted) and report the blast radius. Also drives the canvas overlay
 * (down = red, degraded = amber).
 */
export function simulateFailureTool(studio: StudioStore): WebMcpTool {
  return {
    name: "simulate-failure",
    title: "Simulate a zone or region failure",
    description:
      "Knock out one availability zone (pass az) or a whole region (omit az) and report which " +
      "resources go down, which degrade, and why.",
    inputSchema: {
      type: "object",
      properties: {
        region: { type: "string", description: 'Region id, e.g. "us-east-1".' },
        az: {
          type: "string",
          description:
            'Availability zone to fail, e.g. "us-east-1a". Omit to fail the entire region.',
        },
      },
      required: ["region"],
      additionalProperties: false,
    },
    // untrustedContentHint: the notes embed resource labels, which are free text
    // a human or an earlier agent turn supplied (core `analysis.rs` formats
    // `"<label> may briefly fail over ..."`).
    annotations: { readOnlyHint: true, untrustedContentHint: true },
    async execute(input) {
      const region = String(input.region ?? "");
      const az = input.az == null ? "" : String(input.az);
      const target = az ? `AZ ${az}` : `region ${region}`;

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
        lines.push(`${target} failure — no impact on the current design.`);
      } else {
        lines.push(
          `${target} failure — ${report.down.length} down` +
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
