# Manual browser test checklist — Faultline WebMCP

The automated `e2e/demo.spec.ts` (Playwright, Chromium) drives this exact flow through the
`@mcp-b/global` polyfill. This document is for verifying the same flow in a **real** WebMCP
runtime, which only a human on real hardware can do (ChatGPT Desktop especially).

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
   simulate an AZ failure…"), not by hand.

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

| Tool | Required input | Optional | Effect |
|---|---|---|---|
| `add-resource` | `kind`, `label` | — | Adds a node. `kind` ∈ `compute`, `database`, `queue`, `load-balancer`, `object-store`, `cache`. Id is auto-assigned as `<kind>-<n>`. |
| `connect` | `from`, `to` | — | Directed edge `from → to` = "`from` depends on / calls `to`". Both ids must exist. |
| `configure-resource` | `id` | `variant`, `region`, `az` | Sets the provider variant and/or placement. Omitted fields unchanged. `region` set + `az` omitted = regional (multi-AZ). |
| `simulate-failure` | `region`, `az` | — | Read-only. Knocks out one AZ; drives the canvas overlay + banner. |
| `find-spofs` | *(none)* | — | Read-only. Rings single-point-of-failure nodes; lists what each orphans. |

AWS profile variants (from `/learn` → `configure-resource`, and `profiles/aws.json`):
`compute`: `ec2-asg` · `database`: `rds-single-az` (SPOF), `rds-multi-az` (~90s failover), `aurora`
(~30s) · `load-balancer`: `alb` · `object-store`: `s3` · `cache`: `elasticache`.
Regions: `us-east-1`, `eu-west-1`, each with `…a` / `…b` / `…c` AZs.

---

## 4. Cold-open demo script

Run in order. "Agent call" = the tool + input the agent issues. Check both the **tool text**
(agent-visible) and the **canvas** (human-visible).

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

### Step 4 — harden: `configure-resource {id:"database-1", variant:"rds-multi-az", region:"us-east-1"}` then re-run `simulate-failure {region:"us-east-1", az:"us-east-1a"}`

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

### Step 5 — `/learn`

- Navigate to `/learn`. All **5** tools render: `add-resource`, `connect`, `configure-resource`,
  `simulate-failure`, `find-spofs`, each with its description, read-only tag where applicable, and
  a formatted JSON input schema.

---

## 5. Per-browser sign-off

Repeat §4 in each runtime. Tick when the tool text **and** canvas match at every step.

### Chrome 149+ (flag enabled)
- [ ] `document.modelContext` present after enabling the flag
- [ ] Step 1 — build: 3 nodes, 2 edges
- [ ] Step 2 — simulate: 3 red, banner "3 down"
- [ ] Step 3 — find-spofs: ring on `database-1`, orphans named
- [ ] Step 4 — harden + re-simulate: DB amber "~90s", compute + LB healthy, banner "1 degraded"
- [ ] Step 5 — `/learn` lists all 5 tools with schemas
- [ ] No console errors during the run

### Edge 150+ (flag enabled)
- [ ] `document.modelContext` present after enabling the flag
- [ ] Step 1 — build: 3 nodes, 2 edges
- [ ] Step 2 — simulate: 3 red, banner "3 down"
- [ ] Step 3 — find-spofs: ring on `database-1`, orphans named
- [ ] Step 4 — harden + re-simulate: DB amber "~90s", compute + LB healthy, banner "1 degraded"
- [ ] Step 5 — `/learn` lists all 5 tools with schemas
- [ ] No console errors during the run

### ChatGPT Desktop browser
- [ ] `document.modelContext` present on opening the URL (no flag)
- [ ] Agent discovers all 5 tools (ask it to list what it can do here)
- [ ] Step 1 — build: 3 nodes, 2 edges
- [ ] Step 2 — simulate: 3 red, banner "3 down"
- [ ] Step 3 — find-spofs: ring on `database-1`, orphans named
- [ ] Step 4 — harden + re-simulate: DB amber "~90s", compute + LB healthy, banner "1 degraded"
- [ ] Step 5 — `/learn` lists all 5 tools with schemas
- [ ] `executeTool` input passed as a JSON string (agent-side; verify calls land)
- [ ] No console errors during the run
