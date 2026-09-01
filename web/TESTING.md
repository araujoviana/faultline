# Manual browser test checklist — Faultline WebMCP

The automated `e2e/demo.spec.ts` (Playwright, Chromium) drives this exact flow through the
`@mcp-b/global` polyfill. This document is for verifying the same flow in a **real** WebMCP
runtime, which only a human on real hardware can do (ChatGPT Desktop especially).

> **Tool surface as of this revision: 8 tools** — `add-resource`, `connect`, `move-resource`,
> `configure-resource`, `simulate-failure`, `find-spofs`, `resilience-lint`, `generate-iac`.
> `web/src/tools/registry.ts` is the source of truth; `/learn` renders it live. If `/learn` shows a
> different count, this doc is stale — trust `/learn`.

---

## 0. Which build exposes WebMCP

| Context | `document.modelContext` comes from | Notes |
|---|---|---|
| `bun run dev` (localhost) | `@mcp-b/global` **dev polyfill**, loaded lazily by `webmcp-bridge.ts` when `import.meta.env.DEV` | No browser flag needed. Use this for quick local checks. |
| Production build (`bun run build` / deployed site) | the **browser itself** | The polyfill is compiled out of the prod bundle on purpose (`ensureModelContext` is behind `import.meta.env.DEV`). A prod build has **no** `modelContext` unless the browser provides one. |

So: to test the deployed URL you need a WebMCP-capable browser (below). To test the polyfilled
path, run the dev server or the Playwright spec.

---

## 1. Enabling WebMCP per browser

### Chrome 149+  (`chrome://version` to check)
1. Open `chrome://flags/#enable-webmcp-testing`
2. Set to **Enabled**
3. **Relaunch** Chrome
4. Load the app; open DevTools console and confirm:
   ```js
   typeof document.modelContext   // "object"
   ```
   (Alternatively, if the site carries a WebMCP **origin-trial token** in a `<meta http-equiv="origin-trial">`
   tag, no flag is needed — we do not ship one yet.)

### Edge 150+  (`edge://version` to check)
1. Open `edge://flags/#enable-webmcp-testing`
2. Set to **Enabled**
3. **Relaunch** Edge
4. Same console check as Chrome.

### ChatGPT Desktop browser
1. No flag. Open the app URL directly in the ChatGPT Desktop in-app browser.
2. The agent surface injects `document.modelContext`; the same console check should pass.
3. Drive the demo below by **talking to the agent** ("build me an ALB → EC2 → RDS stack, then
   simulate an AZ failure, lint it, and generate Terraform"), not by hand.

---

## 2. The `executeTool` JSON-string gotcha

Chrome's `document.modelContext.executeTool(tool, input)` takes `input` as a **JSON string**, not
an object:

```js
const tools = await document.modelContext.getTools();
const t = tools.find(t => t.name === "add-resource");

// ✅ works in Chrome
await document.modelContext.executeTool(t, JSON.stringify({ kind: "database", label: "orders" }));

// ❌ silently mis-parses / throws
await document.modelContext.executeTool(t, { kind: "database", label: "orders" });
```

The polyfill is lenient about this; real Chrome is not. If a manual `executeTool` call "does
nothing", check that the input was stringified.

---

## 3. Tool reference (source of truth: `web/src/tools/`, rendered live at `/learn`)

| Tool | Required input | Optional | Read-only | Effect |
|---|---|---|---|---|
| `add-resource` | `kind`, `label` | — | no | Adds a node. `kind` ∈ `compute`, `database`, `queue`, `load-balancer`, `object-store`, `cache`, `cdn`, `dns`, `functions`, `api-gateway`. Id is auto-assigned as `<kind>-<n>`. |
| `connect` | `from`, `to` | — | no | Directed edge `from → to` = "`from` depends on / calls `to`". Both ids must exist. |
| `move-resource` | `id`, `x`, `y` | — | no | Repositions a node on the canvas (one undo step). Mirrors a human drag. |
| `configure-resource` | `id` | `variant`, `region`, `az` | no | Sets the provider variant and/or placement. Omitted fields unchanged. `region` set + `az` omitted = regional (multi-AZ). |
| `simulate-failure` | `region`, `az` | — | **yes** | Knocks out one AZ; drives the canvas overlay + banner. |
| `find-spofs` | *(none)* | — | **yes** | Rings single-point-of-failure nodes; lists what each orphans. |
| `resilience-lint` | *(none)* | — | **yes** | Rule-based resilience/misconfig checks over the graph; each finding cites a *Designing Data-Intensive Applications* (2nd ed.) chapter/section. |
| `generate-iac` | *(none)* | `target` (`"terraform"`) | **yes** | Emits the architecture as Terraform HCL for review. Returns a fenced ```hcl block. Changes nothing. |

AWS profile variants (from `/learn` → `configure-resource`, and `profiles/aws.json`):
`compute`: `ec2-asg` · `database`: `rds-single-az` (SPOF), `rds-multi-az` (~90s failover), `aurora`
(~30s), `dynamodb` · `load-balancer`: `alb` · `object-store`: `s3` · `cache`: `elasticache` (~60s) ·
`queue`: `sqs` · `cdn`: `cloudfront` · `dns`: `route53` (~60s) · `functions`: `lambda` ·
`api-gateway`: `apigw-http`.
Regions: `us-east-1`, `eu-west-1`, each with `…a` / `…b` / `…c` AZs.

---

## 4. Cold-open demo script

Run in order. "Agent call" = the tool + input the agent issues. Check both the **tool text**
(agent-visible) and the **canvas** (human-visible). This mirrors `e2e/demo.spec.ts`.

> **Fast path:** paste `e2e/manual-console-check.js` into the DevTools console (with the app open
> in a WebMCP browser). It runs this whole script through the real `document.modelContext`, checks
> every tool's text output, and prints a pass/fail table + p50/p95 latency for the write-up. You
> still watch the canvas yourself at the 👁 markers. Reload the page before re-running.

### Step 1 — build `alb → ec2-asg → rds-single-az`, DB in `us-east-1a`, wired together

| Agent call | Expected tool text | Expected canvas |
|---|---|---|
| `add-resource {kind:"load-balancer", label:"alb"}` | `Added load-balancer "alb" as load-balancer-1.` | node `alb` appears |
| `add-resource {kind:"compute", label:"api"}` | `Added compute "api" as compute-1.` | node `api` appears |
| `add-resource {kind:"database", label:"orders"}` | `Added database "orders" as database-1.` | node `orders` appears |
| `configure-resource {id:"load-balancer-1", variant:"alb"}` | `Configured load-balancer-1: variant alb.` | subtitle → "Application Load Balancer" |
| `configure-resource {id:"compute-1", variant:"ec2-asg"}` | `Configured compute-1: variant ec2-asg.` | subtitle → "EC2 + Auto Scaling Group" |
| `configure-resource {id:"database-1", variant:"rds-single-az", region:"us-east-1", az:"us-east-1a"}` | `Configured database-1: variant rds-single-az, us-east-1/us-east-1a.` | subtitle → "RDS (Single-AZ)", badge → `us-east-1a` |
| `connect {from:"load-balancer-1", to:"compute-1"}` | `Connected load-balancer-1 -> compute-1.` | edge lb→compute |
| `connect {from:"compute-1", to:"database-1"}` | `Connected compute-1 -> database-1.` | edge compute→db |

**Checkpoint:** 3 nodes, 2 edges on the canvas.

### Step 2 — `simulate-failure {region:"us-east-1", az:"us-east-1a"}`

- **Tool text:** `AZ us-east-1a failure — 3 down (compute-1, database-1, load-balancer-1), 0 degraded.`
- **Canvas:** all 3 nodes render **red** (`data-status="down"`).
- **Banner:** `AZ us-east-1a — 3 down, 0 degraded`.

### Step 3 — `find-spofs {}`

- **Tool text:**
  ```
  1 single point(s) of failure:
  - database-1 — orphans compute-1, load-balancer-1
  ```
- **Canvas:** dashed red **ring** around `database-1`; `compute-1` and `load-balancer-1` named as
  its orphans in the tool text.

### Step 4 — `resilience-lint {}`

- **Tool text:** `N finding(s):` then one bullet per finding, format
  `- [SEVERITY] <rule-id>: <title> (<resource>) — <detail> — DDIA — <Chapter> §"<section>"`, e.g.:
  ```
  2 finding(s):
  - [HIGH] single-az-datastore: … (database-1) — … — DDIA — Ch 6 Replication §"Handling Node Outages"
  - [LOW] unbuffered-write-path: … (compute-1) — … — DDIA — Ch 7 Sharding §"Skewed Workloads and Relieving Hot Spots"
  ```
- Agent-visible text only (no canvas requirement); every finding carries a DDIA citation. Confirm
  the `single-az-datastore` finding (HIGH) is present here.

### Step 5 — harden: `configure-resource {id:"database-1", variant:"rds-multi-az", region:"us-east-1"}` then re-run `simulate-failure {region:"us-east-1", az:"us-east-1a"}` and `resilience-lint {}`

- **configure tool text:** `Configured database-1: variant rds-multi-az, us-east-1.`
  (DB is now regional — the `us-east-1a` badge becomes `us-east-1`.)
- **re-simulate tool text:**
  ```
  AZ us-east-1a failure — 0 down, 1 degraded (database-1).
  - orders may briefly fail over (~90s)
  ```
- **Canvas:** `database-1` **amber** (`data-status="degraded"`); `compute-1` and
  `load-balancer-1` **not** red/amber (healthy).
- **Banner:** `AZ us-east-1a — 0 down, 1 degraded`.
- **re-lint tool text:** the `single-az-datastore` finding is **gone** (other lower-severity
  findings may remain).

### Step 6 — `generate-iac {}`

- **Tool text:** a fenced ```hcl block containing `resource "aws_db_instance" "database_1"` with
  `multi_az` set, and the ALB target-group wiring (`target_group_arns`).
- Read-only: the canvas does not change.

### Step 7 — `move-resource {id:"load-balancer-1", x:40, y:40}`

- **Tool text:** `Moved load-balancer-1 to (40, 40).`
- **Canvas:** the `alb` node jumps to the new position; edges follow. One undo reverts it.

### Step 8 — `/learn`

- Navigate to `/learn`. All **8** tools render: `add-resource`, `connect`, `move-resource`,
  `configure-resource`, `simulate-failure`, `find-spofs`, `resilience-lint`, `generate-iac` — each
  with its description, the `read-only` tag where applicable, and a formatted JSON input schema.
- **Hard-refresh `/learn`** (Cmd/Ctrl-R on the route directly) — it must survive (SPA fallback via
  `_redirects`).

---

## 5. Per-browser sign-off

Repeat §4 in each runtime. Tick when the tool text **and** canvas match at every step.
Also record, for the Devpost write-up: task-success rate, tool-selection accuracy, round-trips,
error-recovery behaviour, and p50/p95 tool latency.

### Chrome 149+ (flag enabled)
- [ ] `document.modelContext` present after enabling the flag
- [ ] Step 1 — build: 3 nodes, 2 edges
- [ ] Step 2 — simulate: 3 red, banner "3 down"
- [ ] Step 3 — find-spofs: ring on `database-1`, orphans named
- [ ] Step 4 — resilience-lint: `single-az-datastore` finding present, DDIA citation shown
- [ ] Step 5 — harden + re-simulate + re-lint: DB amber "~90s", compute + LB healthy, banner "1 degraded", single-AZ finding cleared
- [ ] Step 6 — generate-iac: fenced hcl, `aws_db_instance` + `multi_az` + `target_group_arns`
- [ ] Step 7 — move-resource: node repositions, undo reverts
- [ ] Step 8 — `/learn` lists all 8 tools with schemas; hard-refresh survives
- [ ] No console errors during the run

### Edge 150+ (flag enabled)
- [ ] `document.modelContext` present after enabling the flag
- [ ] Step 1 — build: 3 nodes, 2 edges
- [ ] Step 2 — simulate: 3 red, banner "3 down"
- [ ] Step 3 — find-spofs: ring on `database-1`, orphans named
- [ ] Step 4 — resilience-lint: `single-az-datastore` finding present, DDIA citation shown
- [ ] Step 5 — harden + re-simulate + re-lint: DB amber "~90s", compute + LB healthy, banner "1 degraded", single-AZ finding cleared
- [ ] Step 6 — generate-iac: fenced hcl, `aws_db_instance` + `multi_az` + `target_group_arns`
- [ ] Step 7 — move-resource: node repositions, undo reverts
- [ ] Step 8 — `/learn` lists all 8 tools with schemas; hard-refresh survives
- [ ] No console errors during the run

### ChatGPT Desktop browser
- [ ] `document.modelContext` present on opening the URL (no flag)
- [ ] Agent discovers all 8 tools (ask it to list what it can do here)
- [ ] Step 1 — build: 3 nodes, 2 edges
- [ ] Step 2 — simulate: 3 red, banner "3 down"
- [ ] Step 3 — find-spofs: ring on `database-1`, orphans named
- [ ] Step 4 — resilience-lint: `single-az-datastore` finding present, DDIA citation shown
- [ ] Step 5 — harden + re-simulate + re-lint: DB amber "~90s", compute + LB healthy, banner "1 degraded", single-AZ finding cleared
- [ ] Step 6 — generate-iac: fenced hcl, `aws_db_instance` + `multi_az` + `target_group_arns`
- [ ] Step 7 — move-resource: node repositions, undo reverts
- [ ] Step 8 — `/learn` lists all 8 tools with schemas; hard-refresh survives
- [ ] `executeTool` input passed as a JSON string (agent-side; verify calls land)
- [ ] No console errors during the run

### Record for the submission form
- [ ] Clients/agents tested + versions (`chrome://version`, Edge version, ChatGPT Desktop build)
- [ ] Any quirks or failures per client
- [ ] Task-success rate / tool-selection accuracy / round-trips / error-recovery / p50-p95 latency
