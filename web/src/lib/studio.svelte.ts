import type {
  ArchitectureState,
  BlastReport,
  CostReport,
  Explanation,
  Finding,
  ProviderProfile,
  ResilienceScore,
  Spof,
  StudioCore,
} from "./core";

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
  readonly findings: Finding[];
  /** 0–100 resilience score for the current design; set by {@link lint}. */
  readonly score: ResilienceScore | null;
  readonly explanation: Explanation | null;
  readonly cost: CostReport | null;
  /** Change in total monthly cost since the previous estimate, or null if this is the first. */
  readonly costDelta: number | null;
  /** Replace the design with a starting architecture from a requirements sentence (one undo step). */
  propose(requirements: string): void;
  addResource(kind: string, label: string, x?: number, y?: number): string;
  connect(from: string, to: string): void;
  /** Move a resource to a new canvas position (one undo step). */
  move(id: string, x: number, y: number): void;
  removeResource(id: string): void;
  configure(id: string, variant?: string, region?: string, az?: string): void;
  /** Simulate losing an AZ (`az` set) or a whole region (`az` omitted). Read-only view. */
  simulateFailure(region: string, az?: string): BlastReport;
  findSpofs(): Spof[];
  /** Rule-based resilience findings + the 0–100 score. Read-only view, not an undo step. */
  lint(): Finding[];
  /** Explain one resource id, or an edge written `"from->to"`. Read-only view, not an undo step. */
  explain(selection: string): Explanation;
  /** Rough monthly cost estimate. Read-only view; keeps the previous total so {@link costDelta} works. */
  estimateCost(): CostReport;
  /** Emit the architecture as infrastructure-as-code. Read-only: no mutation, no undo step. */
  generateIac(target?: string): string;
  clearAnalysis(): void;
  reset(): void;
  undo(): void;
}

const EMPTY = JSON.stringify({ resources: [], edges: [] });

/**
 * Deterministic non-overlapping placement for a freshly added resource: walk a
 * fixed grid and take the first cell that no existing node sits on. Keeps a
 * brand-new architecture readable before anyone (human or agent) lays it out.
 */
const GRID_COLS = 4;
const GRID_DX = 166;
const GRID_DY = 92;
const GRID_X0 = 34;
const GRID_Y0 = 26;

function gridCell(i: number): { x: number; y: number } {
  return {
    x: GRID_X0 + (i % GRID_COLS) * GRID_DX,
    y: GRID_Y0 + Math.floor(i / GRID_COLS) * GRID_DY,
  };
}

function freeCell(taken: Array<{ x: number; y: number }>): { x: number; y: number } {
  for (let i = 0; i < 48; i++) {
    const cell = gridCell(i);
    const clash = taken.some((t) => Math.abs(t.x - cell.x) < 90 && Math.abs(t.y - cell.y) < 56);
    if (!clash) return cell;
  }
  return gridCell(taken.length);
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
  let findings = $state<Finding[]>([]);
  let score = $state<ResilienceScore | null>(null);
  let explanation = $state<Explanation | null>(null);
  let cost = $state<CostReport | null>(null);
  let costDelta = $state<number | null>(null);

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
    get findings() {
      return findings;
    },
    get score() {
      return score;
    },
    get explanation() {
      return explanation;
    },
    get cost() {
      return cost;
    },
    get costDelta() {
      return costDelta;
    },
    propose(requirements) {
      checkpoint();
      core.propose(requirements);
      lastReport = null;
      spofs = [];
      findings = [];
      score = null;
      explanation = null;
      cost = null;
      costDelta = null;
      refresh();
    },
    addResource(kind, label, x, y) {
      checkpoint();
      const spot = x === undefined || y === undefined ? freeCell(snapshot.resources) : { x, y };
      const id = core.addResource(kind, label, spot.x, spot.y);
      refresh();
      return id;
    },
    connect(from, to) {
      checkpoint();
      core.connect(from, to);
      refresh();
    },
    move(id, x, y) {
      checkpoint();
      core.move(id, x, y);
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
    simulateFailure(region, az = "") {
      const report = JSON.parse(core.simulateFailure(region, az)) as BlastReport;
      lastReport = report;
      return report;
    },
    findSpofs() {
      const found = JSON.parse(core.findSpofs()) as Spof[];
      spofs = found;
      return found;
    },
    lint() {
      const found = JSON.parse(core.lint()) as Finding[];
      findings = found;
      score = JSON.parse(core.resilienceScore()) as ResilienceScore;
      return found;
    },
    explain(selection) {
      const result = JSON.parse(core.explain(selection)) as Explanation;
      explanation = result;
      return result;
    },
    estimateCost() {
      const next = JSON.parse(core.estimateCost()) as CostReport;
      costDelta = cost
        ? Math.round((next.total_monthly_usd - cost.total_monthly_usd) * 100) / 100
        : null;
      cost = next;
      return next;
    },
    generateIac(target = "terraform") {
      return core.generateIac(target);
    },
    clearAnalysis() {
      lastReport = null;
      spofs = [];
      findings = [];
      score = null;
      explanation = null;
      cost = null;
      costDelta = null;
    },
    reset() {
      checkpoint();
      core.loadJson(EMPTY);
      lastReport = null;
      spofs = [];
      findings = [];
      score = null;
      explanation = null;
      cost = null;
      costDelta = null;
      refresh();
    },
    undo() {
      const previous = history.pop();
      if (previous === undefined) return;
      core.loadJson(previous);
      lastReport = null;
      spofs = [];
      findings = [];
      score = null;
      explanation = null;
      cost = null;
      costDelta = null;
      refresh();
    },
  };
}
