import type { StudioStore } from "../lib/studio.svelte";
import type { WebMcpTool } from "../lib/webmcp-bridge";

/**
 * Map a neutral resource onto a concrete provider service and/or place it in the
 * topology. Used to turn a rough sketch into something failure analysis can
 * reason about (e.g. "this database is RDS Multi-AZ in us-east-1").
 */
export function configureResourceTool(studio: StudioStore): WebMcpTool {
  return {
    name: "configure-resource",
    title: "Configure a resource",
    description:
      "Set an existing resource's provider variant and/or its region/availability-zone placement. Omitted fields are left unchanged.",
    inputSchema: {
      type: "object",
      properties: {
        id: { type: "string", description: 'Resource id, e.g. "database-1".' },
        variant: {
          type: "string",
          description: 'Provider service, e.g. "rds-multi-az". See the active profile on /learn.',
        },
        region: { type: "string", description: 'Region id, e.g. "us-east-1".' },
        az: {
          type: "string",
          description:
            'Availability zone, e.g. "us-east-1a". Omit for a regional (multi-AZ) deployment.',
        },
      },
      required: ["id"],
      additionalProperties: false,
    },
    annotations: { readOnlyHint: false },
    async execute(input) {
      const id = String(input.id ?? "");
      const variant = input.variant == null ? undefined : String(input.variant);
      const region = input.region == null ? undefined : String(input.region);
      const az = input.az == null ? undefined : String(input.az);

      try {
        studio.configure(id, variant, region, az);
      } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        // On an unknown-variant error, tell the agent which ones are valid for
        // this kind — the variant ids aren't an enum in the schema.
        const kind = studio.state.resources.find((r) => r.id === id)?.kind;
        const valid = kind ? (studio.profile.variants[kind] ?? []).map((v) => v.id) : [];
        const hint =
          message.includes("variant") && valid.length
            ? ` Valid ${kind} variants: ${valid.join(", ")}.`
            : "";
        return { content: [{ type: "text", text: message + hint }] };
      }

      const parts: string[] = [];
      if (variant) parts.push(`variant ${variant}`);
      if (region) parts.push(az ? `${region}/${az}` : region);
      const what = parts.length ? parts.join(", ") : "no changes";
      return { content: [{ type: "text", text: `Configured ${id}: ${what}.` }] };
    },
  };
}
