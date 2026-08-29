/**
 * The mutation surface shared by the UI and the WebMCP tools.
 *
 * The real implementation is `strata-wasm`'s `Studio` (see `wasm-core.ts`).
 * `createMemoryCore` is a lightweight stand-in used by tests and by `bun run dev`
 * until `bun run build:wasm` is wired into the build — it deliberately mirrors
 * only the surface the walking skeleton exercises, not the core's validation.
 */

export const RESOURCE_KINDS = [
  "compute",
  "database",
  "queue",
  "load-balancer",
  "object-store",
  "cache",
] as const;

export type ResourceKind = (typeof RESOURCE_KINDS)[number];

export interface Placement {
  region?: string;
  az?: string;
}

export interface ResourceNode {
  id: string;
  kind: string;
  label: string;
  x: number;
  y: number;
  variant?: string;
  placement?: Placement;
}

export interface EdgeLink {
  from: string;
  to: string;
}

export interface ArchitectureState {
  resources: ResourceNode[];
  edges: EdgeLink[];
}

/** Outcome of a simulated availability-zone failure (mirrors `analysis::BlastReport`). */
export interface BlastReport {
  target: string;
  down: string[];
  degraded: string[];
  healthy: string[];
  notes: string[];
}

/** A single point of failure and what it orphans (mirrors `analysis::Spof`). */
export interface Spof {
  id: string;
  orphans: string[];
}

export interface ProfileVariant {
  id: string;
  display_name: string;
  spof?: boolean;
  failover_seconds?: number;
}

/** The active provider profile (mirrors `profile::ProviderProfile`). */
export interface ProviderProfile {
  provider: string;
  display_name: string;
  regions: { id: string; azs: string[] }[];
  variants: Record<string, ProfileVariant[]>;
}

export interface StudioCore {
  addResource(kind: string, label: string, x: number, y: number): string;
  connect(from: string, to: string): void;
  /** Move a resource to a new canvas position. */
  move(id: string, x: number, y: number): void;
  removeResource(id: string): void;
  /** Set a resource's provider variant and/or placement. Empty strings = leave unchanged. */
  configure(id: string, variant?: string, region?: string, az?: string): void;
  /** Simulate losing an AZ; returns a JSON {@link BlastReport}. */
  simulateFailure(region: string, az: string): string;
  /** Current single points of failure; returns a JSON {@link Spof}`[]`. */
  findSpofs(): string;
  /** The active provider profile as JSON. */
  profileJson(): string;
  stateJson(): string;
  loadJson(json: string): void;
}

interface MemoryState {
  resources: ResourceNode[];
  edges: EdgeLink[];
  counters: Record<string, number>;
}

const EMPTY_REPORT = JSON.stringify({
  target: "",
  down: [],
  degraded: [],
  healthy: [],
  notes: [],
});

/**
 * In-memory stand-in for `strata-wasm`'s `Studio`. Mirrors the graph mutations
 * (used by the vitest contract tests); the failure-analysis methods are inert —
 * their correctness is covered by the Rust unit tests, and the tools that call
 * them are tested against a purpose-built stub core.
 */
export function createMemoryCore(): StudioCore {
  let state: MemoryState = { resources: [], edges: [], counters: {} };

  const has = (id: string) => state.resources.some((r) => r.id === id);
  const find = (id: string) => state.resources.find((r) => r.id === id);

  return {
    addResource(kind, label, x, y) {
      if (!RESOURCE_KINDS.includes(kind as ResourceKind)) {
        throw new Error(`unknown resource kind: ${kind}`);
      }
      const n = (state.counters[kind] ?? 0) + 1;
      state.counters[kind] = n;
      const id = `${kind}-${n}`;
      state.resources.push({ id, kind, label, x, y });
      return id;
    },
    connect(from, to) {
      if (from === to) throw new Error(`cannot connect ${from} to itself`);
      if (!has(from)) throw new Error(`no such resource: ${from}`);
      if (!has(to)) throw new Error(`no such resource: ${to}`);
      if (state.edges.some((e) => e.from === from && e.to === to)) {
        throw new Error(`${from} -> ${to} is already connected`);
      }
      state.edges.push({ from, to });
    },
    move(id, x, y) {
      const r = find(id);
      if (!r) throw new Error(`no such resource: ${id}`);
      r.x = x;
      r.y = y;
    },
    removeResource(id) {
      if (!has(id)) throw new Error(`no such resource: ${id}`);
      state.resources = state.resources.filter((r) => r.id !== id);
      state.edges = state.edges.filter((e) => e.from !== id && e.to !== id);
    },
    configure(id, variant, region, az) {
      const r = find(id);
      if (!r) throw new Error(`no such resource: ${id}`);
      if (variant) r.variant = variant;
      if (region) r.placement = { region, az: az || undefined };
    },
    simulateFailure() {
      return EMPTY_REPORT;
    },
    findSpofs() {
      return "[]";
    },
    profileJson() {
      return JSON.stringify({
        provider: "",
        display_name: "",
        regions: [],
        variants: {},
      });
    },
    stateJson() {
      return JSON.stringify(state);
    },
    loadJson(json) {
      const parsed = JSON.parse(json) as Partial<MemoryState>;
      state = {
        resources: parsed.resources ?? [],
        edges: parsed.edges ?? [],
        counters: parsed.counters ?? {},
      };
    },
  };
}
