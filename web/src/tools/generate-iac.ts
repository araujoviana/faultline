import type { StudioStore } from "../lib/studio.svelte";
import type { WebMcpTool } from "../lib/webmcp-bridge";

/**
 * Emit the current architecture as Terraform HCL for the human to review.
 * Read-only: it produces text and changes nothing on the canvas. Covers the
 * configured AWS variant and placement of every resource; network (VPC,
 * subnets, security groups) and IAM are referenced as input variables, not
 * generated.
 */
export function generateIacTool(studio: StudioStore): WebMcpTool {
  return {
    name: "generate-iac",
    title: "Generate Terraform (HCL)",
    description:
      "Emit the current architecture as Terraform HCL for the human to review. Read-only: " +
      "produces text, changes nothing. Each configured resource becomes one or more `resource` " +
      "blocks with its AWS variant and placement; an alb -> compute edge wires target groups, " +
      "other edges become depends_on. Network and IAM are var.* inputs, not generated. Returns a " +
      "fenced hcl code block.",
    inputSchema: {
      type: "object",
      properties: {
        target: {
          type: "string",
          enum: ["terraform"],
          description: 'Output format. Only "terraform" is supported today.',
        },
      },
      additionalProperties: false,
    },
    annotations: { readOnlyHint: true, untrustedContentHint: true },
    async execute(input) {
      const target = input.target == null ? "terraform" : String(input.target);
      try {
        const hcl = studio.generateIac(target);
        return { content: [{ type: "text", text: `\`\`\`hcl\n${hcl}\n\`\`\`` }] };
      } catch (error) {
        return {
          content: [{ type: "text", text: error instanceof Error ? error.message : String(error) }],
        };
      }
    },
  };
}
