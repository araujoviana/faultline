import { beforeEach, describe, expect, it } from "vitest";
import { activity, clearActivity } from "../lib/activity.svelte";
import { makeStubCore } from "../lib/stub-core";
import { createStudio } from "../lib/studio.svelte";
import { instrumentTool } from "../lib/webmcp-bridge";
import { generateIacTool } from "./generate-iac";

function setup(generateIac: (target: string) => string = () => 'resource "aws_s3_bucket" "x" {}') {
  const studio = createStudio(makeStubCore({ generateIac }));
  const tool = instrumentTool(generateIacTool(studio));
  return { studio, tool };
}

describe("generate-iac tool", () => {
  beforeEach(() => clearActivity());

  it("wraps the core HCL in a fenced hcl block", async () => {
    const { tool } = setup(() => 'resource "aws_s3_bucket" "x" {}');
    const result = await tool.execute({ target: "terraform" });
    const text = result.content[0].text;
    expect(text.startsWith("```hcl\n")).toBe(true);
    expect(text).toContain('resource "aws_s3_bucket" "x" {}');
    expect(text.trimEnd().endsWith("```")).toBe(true);
    expect(activity.entries[0]).toMatchObject({ tool: "generate-iac", ok: true });
  });

  it('defaults the target to "terraform" and passes it through', async () => {
    let seen = "";
    const { tool } = setup((target) => {
      seen = target;
      return "# ok";
    });
    await tool.execute({});
    expect(seen).toBe("terraform");
  });

  it("surfaces a core error as text without throwing", async () => {
    const { tool } = setup(() => {
      throw new Error("unknown target: pulumi. Supported: terraform");
    });
    const result = await tool.execute({ target: "pulumi" });
    expect(result.content[0].text).toContain("unknown target");
  });

  it("is read-only and hints untrusted output", () => {
    const { tool } = setup();
    expect(tool.annotations?.readOnlyHint).toBe(true);
    expect(tool.annotations?.untrustedContentHint).toBe(true);
  });

  it("advertises target as an enum and forbids extra properties", () => {
    const { tool } = setup();
    const schema = tool.inputSchema as {
      properties: { target: { enum: string[] } };
      additionalProperties: boolean;
    };
    expect(schema.properties.target.enum).toContain("terraform");
    expect(schema.additionalProperties).toBe(false);
  });
});
