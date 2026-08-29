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
  "add-resource",
  "connect",
  "configure-resource",
  "simulate-failure",
  "find-spofs",
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
      const r = await mc.executeTool(tool, JSON.stringify(input));
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

  // Harden -> Multi-AZ, then re-simulate: DB degrades (~90s), compute + LB healthy.
  await demo(page, "configure-resource", {
    id: "database-1",
    variant: "rds-multi-az",
    region: "us-east-1",
  });
  const failover = await demo(page, "simulate-failure", { region: "us-east-1", az: "us-east-1a" });
  expect(failover).toContain("~90s");
  await expect(
    page.locator('svg.canvas g.node-group:has(rect[data-kind="database"])'),
  ).toHaveAttribute("data-status", "degraded");
  await expect(page.locator('svg.canvas g.node-group[data-status="down"]')).toHaveCount(0);
  await expect(page.locator(".banner")).toContainText("1 degraded");

  expect(errors).toEqual([]);
});
