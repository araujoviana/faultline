/**
 * Manual real-browser check for Faultline's WebMCP tools.
 *
 * Paste this whole file into the DevTools console on a WebMCP-capable browser
 * (Chrome 149+ / Edge 150+ with the flag, or ChatGPT Desktop) with the app open
 * at https://faultline-studio.pages.dev — see web/TESTING.md §1.
 *
 * It drives the cold-open demo through the *real* `document.modelContext`
 * exactly as an agent would (input as a JSON string), checks each tool's text
 * output, and prints a pass/fail table plus latency telemetry for the Devpost
 * write-up. It does NOT verify the canvas — watch the diagram yourself at the
 * points called out with 👁.
 *
 * Re-run: reload the page first (it mutates the live document).
 */
(async () => {
  const mc = document.modelContext;
  if (!mc) {
    console.error(
      "❌ document.modelContext is undefined — WebMCP is not active in this browser. See web/TESTING.md §1.",
    );
    return;
  }

  const EXPECTED = [
    "add-resource",
    "connect",
    "move-resource",
    "configure-resource",
    "simulate-failure",
    "find-spofs",
    "resilience-lint",
    "generate-iac",
  ];

  // Wait for async tool registration (App.svelte registers in onMount).
  let tools = [];
  for (let i = 0; i < 50; i++) {
    tools = await mc.getTools();
    if (EXPECTED.every((n) => tools.some((t) => t.name === n))) break;
    await new Promise((r) => setTimeout(r, 100));
  }
  const got = tools.map((t) => t.name);
  const missing = EXPECTED.filter((n) => !got.includes(n));
  console.log(`%cTools registered: ${got.length}`, "font-weight:bold", got);
  if (missing.length) {
    console.error("❌ missing tools:", missing);
    return;
  }

  const results = [];
  const timings = [];

  async function call(name, input, expectSubstrings, note) {
    const tool = tools.find((t) => t.name === name);
    const t0 = performance.now();
    let text = "";
    let ok = false;
    try {
      let r = await mc.executeTool(tool, JSON.stringify(input));
      if (typeof r === "string") {
        try {
          r = JSON.parse(r);
        } catch {
          /* plain string */
        }
      }
      text = Array.isArray(r?.content)
        ? r.content.map((c) => c.text ?? "").join("\n")
        : typeof r === "string"
          ? r
          : JSON.stringify(r);
      ok = expectSubstrings.every((s) => text.includes(s));
    } catch (e) {
      text = `THREW: ${e?.message ?? e}`;
    }
    const ms = Math.round(performance.now() - t0);
    timings.push(ms);
    results.push({
      step: results.length + 1,
      tool: name,
      ms,
      ok: ok ? "✅" : "❌",
      note: note ?? "",
    });
    console.log(`${ok ? "✅" : "❌"} [${ms}ms] ${name}`, input, "→", text.slice(0, 200));
    return text;
  }

  console.log(
    "%c— Step 1: build alb → ec2-asg → rds-single-az —",
    "color:#8b5cf6;font-weight:bold",
  );
  await call("add-resource", { kind: "load-balancer", label: "alb" }, ["as load-balancer-1"]);
  await call("add-resource", { kind: "compute", label: "api" }, ["as compute-1"]);
  await call("add-resource", { kind: "database", label: "orders" }, ["as database-1"]);
  await call("configure-resource", { id: "load-balancer-1", variant: "alb" }, ["variant alb"]);
  await call("configure-resource", { id: "compute-1", variant: "ec2-asg" }, ["variant ec2-asg"]);
  await call(
    "configure-resource",
    { id: "database-1", variant: "rds-single-az", region: "us-east-1", az: "us-east-1a" },
    ["rds-single-az", "us-east-1/us-east-1a"],
  );
  await call("connect", { from: "load-balancer-1", to: "compute-1" }, [
    "load-balancer-1 -> compute-1",
  ]);
  await call("connect", { from: "compute-1", to: "database-1" }, ["compute-1 -> database-1"]);
  console.log(
    "%c👁 canvas: 3 nodes, 2 directed edges, DB subtitle 'RDS (Single-AZ)' + badge 'us-east-1a'",
    "color:#0aa",
  );

  console.log("%c— Step 2: simulate-failure —", "color:#8b5cf6;font-weight:bold");
  await call("simulate-failure", { region: "us-east-1", az: "us-east-1a" }, ["3 down"]);
  console.log(
    "%c👁 canvas: all 3 nodes RED; banner 'AZ us-east-1a — 3 down, 0 degraded'",
    "color:#0aa",
  );

  console.log("%c— Step 3: find-spofs —", "color:#8b5cf6;font-weight:bold");
  await call("find-spofs", {}, ["database-1", "compute-1", "load-balancer-1"]);
  console.log("%c👁 canvas: dashed red ring around database-1", "color:#0aa");

  console.log("%c— Step 4: resilience-lint —", "color:#8b5cf6;font-weight:bold");
  await call("resilience-lint", {}, ["single-az-datastore", "DDIA"], "must cite DDIA");

  console.log(
    "%c— Step 5: harden → Multi-AZ, re-simulate, re-lint —",
    "color:#8b5cf6;font-weight:bold",
  );
  await call(
    "configure-resource",
    { id: "database-1", variant: "rds-multi-az", region: "us-east-1" },
    ["variant rds-multi-az", "us-east-1"],
  );
  await call("simulate-failure", { region: "us-east-1", az: "us-east-1a" }, [
    "0 down",
    "1 degraded",
    "~90s",
  ]);
  const relint = await call("resilience-lint", {}, [], "single-az finding should be GONE");
  results[results.length - 1].ok = relint.includes("single-az-datastore") ? "❌" : "✅";
  console.log(
    "%c👁 canvas: database-1 AMBER, compute-1 + load-balancer-1 healthy; banner '0 down, 1 degraded'",
    "color:#0aa",
  );

  console.log("%c— Step 6: generate-iac —", "color:#8b5cf6;font-weight:bold");
  await call("generate-iac", {}, ["```hcl", "aws_db_instance", "multi_az", "target_group_arns"]);

  console.log("%c— Step 7: move-resource —", "color:#8b5cf6;font-weight:bold");
  await call("move-resource", { id: "load-balancer-1", x: 40, y: 40 }, [
    "Moved load-balancer-1 to (40, 40)",
  ]);
  console.log("%c👁 canvas: alb node jumps to top-left; edge follows", "color:#0aa");

  // Summary
  const pass = results.filter((r) => r.ok === "✅").length;
  const sorted = [...timings].sort((a, b) => a - b);
  const p = (q) => sorted[Math.min(sorted.length - 1, Math.floor(q * sorted.length))];
  console.log("%c\n=== SUMMARY ===", "font-weight:bold;font-size:14px");
  console.table(results);
  console.log(`task-success: ${pass}/${results.length} tool calls asserted OK`);
  console.log(`round-trips: ${results.length} calls for the full loop`);
  console.log(`latency: p50 ${p(0.5)}ms · p95 ${p(0.95)}ms · max ${sorted[sorted.length - 1]}ms`);
  console.log(
    "Now record: browser + version, any quirks, and whether the 👁 canvas states matched.",
  );
})();
