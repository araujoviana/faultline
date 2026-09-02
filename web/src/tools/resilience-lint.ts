import type { StudioStore } from "../lib/studio.svelte";
import type { WebMcpTool } from "../lib/webmcp-bridge";

/**
 * Lint the current design for resilience anti-patterns — single-zone datastores,
 * unmanaged compute, synchronous service coupling, single-region deployments,
 * unbuffered write paths — each finding citing the principle from *Designing
 * Data-Intensive Applications* (2nd ed.) that justifies it. Read-only: it
 * reports, it changes nothing.
 */
export function resilienceLintTool(studio: StudioStore): WebMcpTool {
  return {
    name: "resilience-lint",
    title: "Lint the architecture for resilience anti-patterns",
    description:
      "Run rule-based resilience checks over the current design. Each finding has a severity " +
      "(high / medium / low), the resource it concerns, a plain-language explanation of the risk " +
      "and the fix, and a citation to the architectural principle in Designing Data-Intensive " +
      "Applications (2nd ed.) behind it. Read-only: reports, changes nothing on the canvas.",
    inputSchema: {
      type: "object",
      properties: {},
      additionalProperties: false,
    },
    annotations: { readOnlyHint: true },
    async execute() {
      const findings = studio.lint();
      const score = studio.score;
      const header = score ? `Resilience score: ${score.value}/100 (${score.grade}).` : "";

      if (findings.length === 0) {
        return {
          content: [
            {
              type: "text",
              text: `${header} No resilience findings — the design has no known anti-patterns.`.trim(),
            },
          ],
        };
      }
      const lines = findings.map((f) => {
        const where = f.resource ? ` (${f.resource})` : "";
        const cite = `DDIA — ${f.citation.chapter} §"${f.citation.section}"`;
        return `- [${f.severity.toUpperCase()}] ${f.rule}: ${f.title}${where} — ${f.detail} — ${cite}`;
      });
      return {
        content: [
          {
            type: "text",
            text: `${header} ${findings.length} finding(s):\n${lines.join("\n")}`.trim(),
          },
        ],
      };
    },
  };
}
