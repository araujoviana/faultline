import type { ArchitectureState, BlastReport, ProviderProfile, Spof, StudioCore } from "./core";

/**
 * Reactive wrapper around a {@link StudioCore}. Holds a `$state` snapshot of the
 * architecture, refreshed after every mutation, plus a single undo history stack
 * that covers both human and agent edits. Failure-analysis results are held
 * separately and are *not* part of the undo history (they are read-only views).
 */
export interface StudioStore {
  readonly state: ArchitectureState;
  readonly canUndo: boolean;
  readonly profile: ProviderProfile;
  readonly lastReport: BlastReport | null;
  readonly spofs: Spof[];
  addResource(kind: string, label: string, x?: number, y?: number): string;
  connect(from: string, to: string): void;
  removeResource(id: string): void;
  configure(id: string, variant?: string, region?: string, az?: string): void;
  simulateFailure(region: string, az: string): BlastReport;
  findSpofs(): Spof[];
  clearAnalysis(): void;
  reset(): void;
  undo(): void;
}

const EMPTY = JSON.stringify({ resources: [], edges: [] });

function scatter(): number {
  return 60 + Math.round(Math.random() * 520);
}

const EMPTY_PROFILE: ProviderProfile = {
  provider: "",
  display_name: "",
  regions: [],
  variants: {},
};

export function createStudio(core: StudioCore): StudioStore {
  let snapshot = $state<ArchitectureState>(read());
  const history: string[] = $state([]);
  let lastReport = $state<BlastReport | null>(null);
  let spofs = $state<Spof[]>([]);

  let profile: ProviderProfile = EMPTY_PROFILE;
  try {
    profile = JSON.parse(core.profileJson()) as ProviderProfile;
  } catch {
    profile = EMPTY_PROFILE;
  }

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
    get profile() {
      return profile;
    },
    get lastReport() {
      return lastReport;
    },
    get spofs() {
      return spofs;
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
    configure(id, variant, region, az) {
      checkpoint();
      core.configure(id, variant, region, az);
      refresh();
    },
    simulateFailure(region, az) {
      const report = JSON.parse(core.simulateFailure(region, az)) as BlastReport;
      lastReport = report;
      return report;
    },
    findSpofs() {
      const found = JSON.parse(core.findSpofs()) as Spof[];
      spofs = found;
      return found;
    },
    clearAnalysis() {
      lastReport = null;
      spofs = [];
    },
    reset() {
      checkpoint();
      core.loadJson(EMPTY);
      lastReport = null;
      spofs = [];
      refresh();
    },
    undo() {
      const previous = history.pop();
      if (previous === undefined) return;
      core.loadJson(previous);
      lastReport = null;
      spofs = [];
      refresh();
    },
  };
}
