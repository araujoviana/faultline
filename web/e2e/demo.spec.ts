import { expect, type Page, test } from "@playwright/test";

/**
 * The cold-open demo, end to end, through the polyfilled `document.modelContext`.
 *
 * Mirrors the manual script in `web/TESTING.md`. Asserts the same UI states a
 * human would check: red / amber nodes, the blast-radius banner, the SPOF ring.
 * The demo is driven in stages so the intermediate canvas states can be checked
 * before the next tool call moves things on.
 */

const TOOL_NAMES = [
  "propose-architecture",
  "describe-architecture",
  "estimate-cost",
  "add-resource",
  "connect",
  "move-resource",
  "remove-resource",
  "configure-resource",
  "simulate-failure",
  "find-spofs",
  "resilience-lint",
  "explain",
  "generate-iac",
];

/** Collect real browser console errors + uncaught exceptions for the whole run. */
function trackErrors(page: Page): string[] {
  const errors: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() === "error") errors.push(msg.text());
  });
  page.on("pageerror", (err) => errors.push(String(err)));
  return errors;
}

/**
 * Install a `window.__demo(name, input)` helper that calls a WebMCP tool the way
 * Chrome does — input as a JSON *string*, not an object — and returns the tool's
 * text output. Waits for App.svelte's async tool registration first.
 */
async function installDriver(page: Page) {
  await page.evaluate(async (names: string[]) => {
    type Mc = {
      getTools(): Promise<Array<{ name: string }>>;
      executeTool(tool: unknown, input: string): Promise<unknown>;
    };
    const getMc = () => (document as unknown as { modelContext?: Mc }).modelContext;

    // App.svelte installs the dev polyfill and registers tools asynchronously in
    // onMount; poll for both to land.
    let mc: Mc | undefined;
    let tools: Array<{ name: string }> = [];
    for (let i = 0; i < 100; i++) {
      mc = getMc();
      if (mc) {
        tools = await mc.getTools();
        if (names.every((n) => tools.some((t) => t.name === n))) break;
      }
      await new Promise((r) => setTimeout(r, 100));
    }
    if (!mc) throw new Error("document.modelContext missing — dev polyfill did not install");
    if (!names.every((n) => tools.some((t) => t.name === n))) {
      throw new Error(`tools not registered: got ${tools.map((t) => t.name).join(", ")}`);
    }

    (window as unknown as Record<string, unknown>).__toolCount = tools.length;
    (window as unknown as Record<string, unknown>).__demo = async (
      name: string,
      input: Record<string, unknown>,
    ) => {
      const tool = tools.find((t) => t.name === name);
      if (!tool) throw new Error(`tool not registered: ${name}`);
      let r: unknown = await mc.executeTool(tool, JSON.stringify(input));
      // The dev polyfill sometimes hands back a JSON-encoded result string.
      if (typeof r === "string") {
        try {
          r = JSON.parse(r);
        } catch {
          return r;
        }
      }
      const content = (r as { content?: Array<{ text?: string }> }).content;
      if (Array.isArray(content)) return content.map((c) => c.text ?? "").join("\n");
      return typeof r === "string" ? r : JSON.stringify(r);
    };
  }, TOOL_NAMES);
}

function demo(page: Page, name: string, input: Record<string, unknown>): Promise<string> {
  return page.evaluate(
    ([n, i]) =>
      (window as unknown as { __demo: (n: string, i: unknown) => Promise<string> }).__demo(
        n as string,
        i,
      ),
    [name, input] as const,
  );
}

test("layout holds at phone and ultra-wide widths without horizontal overflow", async ({
  page,
}) => {
  const noHScroll = () =>
    page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth + 1);

  // Phone portrait: everything stacks, nothing bleeds off the right edge.
  await page.setViewportSize({ width: 375, height: 667 });
  await page.goto("/");
  await expect(page.locator("svg.canvas")).toBeVisible();
  await expect(page.locator("aside.palette")).toBeVisible();
  expect(await noHScroll(), "no horizontal scroll at 375px").toBe(true);

  await page.goto("/learn");
  await expect(page.locator("section.learn")).toBeVisible();
  expect(await noHScroll(), "no horizontal scroll on /learn at 375px").toBe(true);

  // Ultra-wide: the working column is capped, not stretched across the viewport.
  await page.setViewportSize({ width: 2560, height: 1440 });
  await page.goto("/");
  await expect(page.locator("svg.canvas")).toBeVisible();
  const mainWidth = await page.locator("main").evaluate((el) => el.getBoundingClientRect().width);
  expect(mainWidth, "main content is capped well under 2560px").toBeLessThan(1900);

  await expect(page.locator("footer a[href*='github.com']")).toBeVisible();
});

test("empty canvas names both ways in — manual and agent", async ({ page }) => {
  await page.goto("/");
  const empty = page.locator("svg.canvas g.empty");
  await expect(empty).toBeVisible();
  await expect(empty).toContainText("Design your architecture here");
  await expect(empty).toContainText("ask your agent");
});

test("a node can be dragged to a new position", async ({ page }) => {
  await page.goto("/");
  await installDriver(page);
  await demo(page, "add-resource", { kind: "compute", label: "api" });

  const node = page.locator("svg.canvas g.node-group").first();
  const before = await node.getAttribute("transform");
  const box = await node.boundingBox();
  if (!box) throw new Error("node has no bounding box");

  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
  await page.mouse.down();
  await page.mouse.move(box.x + 220, box.y + 140, { steps: 10 });
  await page.mouse.up();

  await expect(node).not.toHaveAttribute("transform", before ?? "");

  // The agent sees the same move surfaced as a tool it can also call.
  const moved = await demo(page, "move-resource", { id: "compute-1", x: 40, y: 40 });
  expect(moved).toContain("Moved compute-1 to (40, 40)");
  await expect(node).toHaveAttribute("transform", "translate(40 40)");
});

test("human parity: build, configure, connect, lint and generate IaC from the UI only", async ({
  page,
}) => {
  await page.goto("/");

  // 1. Add three resources from the palette (no agent).
  const palette = page.locator("aside.palette");
  for (const kind of ["load-balancer", "compute", "database"]) {
    await palette.getByRole("button", { name: `+ ${kind}`, exact: true }).click();
  }
  await expect(page.locator("svg.canvas g.node-group")).toHaveCount(3);

  // 2. Click a node to open its inspector, then configure it.
  const db = page.locator('g.node-group[data-id="database-1"]');
  await db.click();
  const inspector = palette.locator(".inspector");
  await expect(inspector).toContainText("database-1");
  await inspector.locator("select").nth(0).selectOption("rds-single-az"); // Variant
  await inspector.locator("select").nth(1).selectOption("us-east-1"); // Region
  await inspector.locator("select").nth(2).selectOption("us-east-1a"); // Zone (appears after region)
  // the <text> carries a <title> child (full string on hover), so match loosely
  await expect(db.locator("text.variant")).toContainText("RDS (Single-AZ)");
  await expect(db.locator("text.badge")).toContainText("us-east-1a");

  // 3. Connect nodes by dragging each node's port onto the next.
  async function link(fromId: string, toId: string) {
    await page.locator(`g.node-group[data-id="${fromId}"]`).click(); // select -> port visible
    const port = await page.locator(`g.node-group[data-id="${fromId}"] .port`).boundingBox();
    const target = await page.locator(`g.node-group[data-id="${toId}"]`).boundingBox();
    if (!port || !target) throw new Error("missing geometry");
    await page.mouse.move(port.x + port.width / 2, port.y + port.height / 2);
    await page.mouse.down();
    await page.mouse.move(target.x + target.width / 2, target.y + target.height / 2, { steps: 8 });
    await page.mouse.up();
  }
  await link("load-balancer-1", "compute-1");
  await link("compute-1", "database-1");
  await expect(page.locator("svg.canvas path.edge")).toHaveCount(2);

  // 4. Resilience lint from the UI — flags the single-AZ database, citing DDIA.
  await palette.getByRole("button", { name: "Resilience lint" }).click();
  const findings = page.locator(".findings");
  await expect(findings).toBeVisible();
  await expect(findings).toContainText("Replication");

  // 5. Generate Terraform from the UI — opens the dialog with the HCL.
  await palette.getByRole("button", { name: "Generate Terraform" }).click();
  const dialog = page.locator("dialog.iac");
  await expect(dialog).toBeVisible();
  await expect(dialog.locator("pre")).toContainText('resource "aws_db_instance" "database_1"');
});

test("propose-architecture lays down a connected stack the human can adjust", async ({ page }) => {
  await page.goto("/");
  await installDriver(page);

  const out = await demo(page, "propose-architecture", {
    requirements: "read-heavy public web app with background jobs, survive an AZ outage",
  });
  expect(out).toContain("Proposed a");

  // Cache + queue tiers were pulled in by the keywords; everything is wired.
  await expect(page.locator("svg.canvas g.node-group")).toHaveCount(6);
  await expect(page.locator("svg.canvas path.edge")).toHaveCount(6);

  // describe-architecture reads the canvas back so the agent can see it.
  const described = await demo(page, "describe-architecture", {});
  expect(described).toContain("6 resource(s):");
  expect(described).toMatch(/cache-1 \(cache\)/);
  expect(described).toContain("→");

  // remove-resource deletes a node and its edges; undo restores it.
  const removed = await demo(page, "remove-resource", { id: "cache-1" });
  expect(removed).toContain("Removed cache-1");
  await expect(page.locator("svg.canvas g.node-group")).toHaveCount(5);

  // A human can immediately do the same from the palette input.
  await page.locator("aside.palette .propose-in").fill("serverless api with a key-value store");
  await page.locator("aside.palette").getByRole("button", { name: "Propose" }).click();
  await expect(
    page.locator('svg.canvas g.node-group:has(rect[data-kind="api-gateway"])'),
  ).toHaveCount(1);
});

test("cold-open demo: build, simulate, find SPOFs, harden", async ({ page }) => {
  const errors = trackErrors(page);

  // 1. App loads clean.
  await page.goto("/");
  await expect(page.locator("svg.canvas")).toBeVisible();

  // 2. /learn lists every tool with its schema.
  await page.goto("/learn");
  for (const name of TOOL_NAMES) {
    await expect(page.locator("code.name", { hasText: new RegExp(`^${name}$`) })).toBeVisible();
  }
  await expect(page.locator("section.learn article pre")).toHaveCount(TOOL_NAMES.length);

  // 3. Drive the demo through document.modelContext.
  await page.goto("/");
  await installDriver(page);
  expect(
    await page.evaluate(() => (window as unknown as { __toolCount: number }).__toolCount),
  ).toBe(TOOL_NAMES.length);

  // Build alb -> ec2-asg -> rds-single-az, DB in us-east-1a, wired together.
  await demo(page, "add-resource", { kind: "load-balancer", label: "alb" });
  await demo(page, "add-resource", { kind: "compute", label: "api" });
  await demo(page, "add-resource", { kind: "database", label: "orders" });
  await demo(page, "configure-resource", { id: "load-balancer-1", variant: "alb" });
  await demo(page, "configure-resource", { id: "compute-1", variant: "ec2-asg" });
  await demo(page, "configure-resource", {
    id: "database-1",
    variant: "rds-single-az",
    region: "us-east-1",
    az: "us-east-1a",
  });
  await demo(page, "connect", { from: "load-balancer-1", to: "compute-1" });
  await demo(page, "connect", { from: "compute-1", to: "database-1" });

  await expect(page.locator("svg.canvas g.node-group")).toHaveCount(3);
  await expect(page.locator("svg.canvas g.node-group g.glyph")).toHaveCount(3);
  await expect(page.locator("svg.canvas path.edge")).toHaveCount(2);
  // edges are directed: from -> to, marked with an arrowhead.
  await expect(page.locator("svg.canvas path.edge").first()).toHaveAttribute(
    "marker-end",
    /edge-arrow/,
  );

  // simulate-failure -> all three down, banner "3 down".
  const outage = await demo(page, "simulate-failure", { region: "us-east-1", az: "us-east-1a" });
  expect(outage).toContain("3 down");
  await expect(page.locator('svg.canvas g.node-group[data-status="down"]')).toHaveCount(3);
  await expect(page.locator(".banner")).toContainText("3 down");

  // find-spofs -> database is ringed, names its orphans.
  const spofs = await demo(page, "find-spofs", {});
  expect(spofs).toContain("database-1");
  expect(spofs).toContain("compute-1");
  expect(spofs).toContain("load-balancer-1");
  await expect(
    page.locator('svg.canvas g.node-group:has(rect[data-kind="database"]) .spof-ring'),
  ).toHaveCount(1);

  // resilience-lint -> flags the single-AZ datastore, citing DDIA, with a score.
  const lint1 = await demo(page, "resilience-lint", {});
  expect(lint1).toContain("single-az-datastore");
  expect(lint1).toMatch(/DDIA|Replication/);
  expect(lint1).toMatch(/Resilience score: \d+\/100/);
  await expect(page.locator(".score-badge")).toBeVisible();
  const score1 = Number((lint1.match(/score: (\d+)\/100/) as RegExpMatchArray)[1]);

  // explain -> teaches the datastore's role and names its blast radius.
  const why = await demo(page, "explain", { selection: "database-1" });
  expect(why).toContain("system of record");
  expect(why).toContain("takes down");
  await expect(page.locator(".explain-panel")).toContainText("orders");

  // estimate-cost -> a dollar figure for the current (cheap, single-AZ) design.
  const cost1 = await demo(page, "estimate-cost", {});
  expect(cost1).toContain("/month");
  await expect(page.locator(".cost-panel")).toContainText("/month");

  // Harden -> Multi-AZ, then re-simulate: DB degrades (~90s), compute + LB healthy.
  await demo(page, "configure-resource", {
    id: "database-1",
    variant: "rds-multi-az",
    region: "us-east-1",
  });

  // The high-severity finding clears once the datastore is Multi-AZ.
  const lint2 = await demo(page, "resilience-lint", {});
  expect(lint2).not.toContain("single-az-datastore");
  const score2 = Number((lint2.match(/score: (\d+)\/100/) as RegExpMatchArray)[1]);
  expect(score2).toBeGreaterThan(score1);

  // estimate-cost again -> the agent shows the cost delta of the harden decision.
  const cost2 = await demo(page, "estimate-cost", {});
  expect(cost2).toMatch(/\+\$\d/);
  await expect(page.locator(".cost-panel .cost-delta")).toContainText("+$");
  const failover = await demo(page, "simulate-failure", { region: "us-east-1", az: "us-east-1a" });
  expect(failover).toContain("~90s");
  await expect(
    page.locator('svg.canvas g.node-group:has(rect[data-kind="database"])'),
  ).toHaveAttribute("data-status", "degraded");
  await expect(page.locator('svg.canvas g.node-group[data-status="down"]')).toHaveCount(0);
  await expect(page.locator(".banner")).toContainText("1 degraded");

  // generate-iac -> the agent gets reviewable Terraform straight from the diagram.
  const hcl = await demo(page, "generate-iac", { target: "terraform" });
  expect(hcl).toContain("```hcl");
  expect(hcl).toContain('resource "aws_db_instance" "database_1"');
  expect(hcl).toContain("multi_az");
  expect(hcl).toContain("target_group_arns");

  // A whole-region loss is a different failure domain: Multi-AZ does not survive
  // it — every resource in us-east-1 goes down.
  const regionOut = await demo(page, "simulate-failure", { region: "us-east-1" });
  expect(regionOut).toContain("region us-east-1 failure");
  expect(regionOut).toContain("3 down");
  await expect(page.locator('svg.canvas g.node-group[data-status="down"]')).toHaveCount(3);
  await expect(page.locator(".banner")).toContainText("region us-east-1");

  expect(errors).toEqual([]);
});
