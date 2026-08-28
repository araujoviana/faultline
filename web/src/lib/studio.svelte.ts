import type { ArchitectureState, StudioCore } from "./core";

/**
 * Reactive wrapper around a {@link StudioCore}. Holds a `$state` snapshot of the
 * architecture, refreshed after every mutation, plus a single undo history stack
 * that covers both human and agent edits.
 */
export interface StudioStore {
  readonly state: ArchitectureState;
  readonly canUndo: boolean;
  addResource(kind: string, label: string, x?: number, y?: number): string;
  connect(from: string, to: string): void;
  removeResource(id: string): void;
  reset(): void;
  undo(): void;
}

const EMPTY = JSON.stringify({ resources: [], edges: [] });

function scatter(): number {
  return 60 + Math.round(Math.random() * 520);
}

export function createStudio(core: StudioCore): StudioStore {
  let snapshot = $state<ArchitectureState>(read());
  const history: string[] = $state([]);

  function read(): ArchitectureState {
    const parsed = JSON.parse(core.stateJson()) as Partial<ArchitectureState>;
    return { resources: parsed.resources ?? [], edges: parsed.edges ?? [] };
  }

  function checkpoint() {
    history.push(core.stateJson());
  }

  function refresh() {
    snapshot = read();
  }

  return {
    get state() {
      return snapshot;
    },
    get canUndo() {
      return history.length > 0;
    },
    addResource(kind, label, x = scatter(), y = scatter()) {
      checkpoint();
      const id = core.addResource(kind, label, x, y);
      refresh();
      return id;
    },
    connect(from, to) {
      checkpoint();
      core.connect(from, to);
      refresh();
    },
    removeResource(id) {
      checkpoint();
      core.removeResource(id);
      refresh();
    },
    reset() {
      checkpoint();
      core.loadJson(EMPTY);
      refresh();
    },
    undo() {
      const previous = history.pop();
      if (previous === undefined) return;
      core.loadJson(previous);
      refresh();
    },
  };
}
