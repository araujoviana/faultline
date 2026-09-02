import type { StudioStore } from "../lib/studio.svelte";
import type { WebMcpTool } from "../lib/webmcp-bridge";

/**
 * Replace the canvas with a starting architecture built from a requirements
 * sentence (deterministic keyword matching in the Rust core). One undo step.
 */
export function proposeArchitectureTool(studio: StudioStore): WebMcpTool {
  return {
    name: "propose-architecture",
    title: "Propose a starting architecture",
    description:
      "Replace the canvas with a starting architecture built from a plain-language requirements " +
      "sentence. Understands intent keywords (read-heavy, cache, background jobs / async, static " +
      "assets, serverless, key-value / NoSQL, multi-region / survive a region outage, prototype / " +
      "cheap) and a region name (defaults to us-east-1). Every resource is configured, placed and " +
      "connected. This is a starting point — follow with resilience-lint. One undo step.",
    inputSchema: {
      type: "object",
      properties: {
        requirements: {
          type: "string",
          maxLength: 400,
          description:
            'One or two sentences describing the system, e.g. "public web app with a Postgres ' +
            'database and background email jobs, must survive an availability-zone outage".',
        },
      },
      required: ["requirements"],
      additionalProperties: false,
    },
    annotations: { readOnlyHint: false },
    async execute(input) {
      const requirements = String(input.requirements ?? "").trim();
      if (!requirements) {
        return { content: [{ type: "text", text: "A requirements sentence is required." }] };
      }

      studio.propose(requirements);

      const { resources, edges } = studio.state;
      const summary = resources
        .map((r) => `${r.id} (${r.label})${r.variant ? ` — ${r.variant}` : ""}`)
        .join("\n");
      return {
        content: [
          {
            type: "text",
            text:
              `Proposed a ${resources.length}-resource, ${edges.length}-edge architecture:\n${summary}\n\n` +
              "Adjust it on the canvas or run resilience-lint to review it.",
          },
        ],
      };
    },
  };
}
