import { beforeEach, describe, expect, it } from "vitest";
import { clearActivity } from "../lib/activity.svelte";
import { createMemoryCore } from "../lib/core";
import { makeStubCore } from "../lib/stub-core";
import { createStudio } from "../lib/studio.svelte";
import { instrumentTool } from "../lib/webmcp-bridge";
import { configureResourceTool } from "./configure-resource";

function setup() {
  const studio = createStudio(createMemoryCore());
  const tool = instrumentTool(configureResourceTool(studio));
  return { studio, tool };
}

describe("configure-resource tool", () => {
  beforeEach(() => clearActivity());

  it("sets the variant and placement of an existing resource", async () => {
    const { studio, tool } = setup();
    const db = studio.addResource("database", "orders");

    const result = await tool.execute({
      id: db,
      variant: "rds-multi-az",
      region: "us-east-1",
      az: "us-east-1a",
    });

    expect(studio.state.resources[0]).toMatchObject({
      variant: "rds-multi-az",
      placement: { region: "us-east-1", az: "us-east-1a" },
    });
    expect(result.content[0].text).toContain("rds-multi-az");
  });

  it("treats an omitted az as a regional deployment", async () => {
    const { studio, tool } = setup();
    const db = studio.addResource("database", "orders");

    await tool.execute({ id: db, variant: "rds-multi-az", region: "us-east-1" });

    expect(studio.state.resources[0].placement).toEqual({
      region: "us-east-1",
      az: undefined,
    });
  });

  it("reports an unknown id", async () => {
    const { tool } = setup();
    const result = await tool.execute({ id: "ghost-1", variant: "rds-multi-az" });
    expect(result.content[0].text).toContain("ghost-1");
  });

  it("lists the valid variants when the requested one is unknown", async () => {
    const core = makeStubCore({
      configure: () => {
        throw new Error("unknown database variant for Amazon Web Services: postgres");
      },
      stateJson: () =>
        JSON.stringify({
          resources: [{ id: "database-1", kind: "database", label: "d", x: 0, y: 0 }],
          edges: [],
        }),
      profileJson: () =>
        JSON.stringify({
          provider: "aws",
          display_name: "AWS",
          regions: [],
          variants: {
            database: [
              { id: "rds-multi-az", display_name: "RDS" },
              { id: "aurora", display_name: "Aurora" },
            ],
          },
        }),
    });
    const tool = instrumentTool(configureResourceTool(createStudio(core)));
    const result = await tool.execute({ id: "database-1", variant: "postgres" });
    expect(result.content[0].text).toContain("Valid database variants: rds-multi-az, aurora");
  });

  it("only requires id", () => {
    const { tool } = setup();
    const schema = tool.inputSchema as { required: string[] };
    expect(schema.required).toEqual(["id"]);
  });
});
