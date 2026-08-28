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

export interface ResourceNode {
  id: string;
  kind: string;
  label: string;
  x: number;
  y: number;
}

export interface EdgeLink {
  from: string;
  to: string;
}

export interface ArchitectureState {
  resources: ResourceNode[];
  edges: EdgeLink[];
}

export interface StudioCore {
  addResource(kind: string, label: string, x: number, y: number): string;
  connect(from: string, to: string): void;
  removeResource(id: string): void;
  stateJson(): string;
  loadJson(json: string): void;
}

interface MemoryState {
  resources: ResourceNode[];
  edges: EdgeLink[];
  counters: Record<string, number>;
}

/** In-memory stand-in for `strata-wasm`'s `Studio`. */
export function createMemoryCore(): StudioCore {
  let state: MemoryState = { resources: [], edges: [], counters: {} };

  const has = (id: string) => state.resources.some((r) => r.id === id);

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
    removeResource(id) {
      if (!has(id)) throw new Error(`no such resource: ${id}`);
      state.resources = state.resources.filter((r) => r.id !== id);
      state.edges = state.edges.filter((e) => e.from !== id && e.to !== id);
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
