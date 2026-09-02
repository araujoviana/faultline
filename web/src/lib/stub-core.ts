import type { StudioCore } from "./core";

const EMPTY_STATE = JSON.stringify({ resources: [], edges: [] });
const EMPTY_REPORT = JSON.stringify({
  target: "",
  down: [],
  degraded: [],
  healthy: [],
  notes: [],
});
const EMPTY_COST = JSON.stringify({ total_monthly_usd: 0, lines: [], unpriced: [] });
const EMPTY_PROFILE = JSON.stringify({
  provider: "",
  display_name: "",
  regions: [],
  variants: {},
});

/**
 * A do-nothing {@link StudioCore} for tests that exercise a tool's *formatting*
 * of an analysis result rather than the analysis itself (which is covered by the
 * Rust unit tests). Override just the methods a test cares about.
 */
export function makeStubCore(overrides: Partial<StudioCore> = {}): StudioCore {
  return {
    propose: () => {},
    addResource: () => "resource-1",
    connect: () => {},
    move: () => {},
    removeResource: () => {},
    configure: () => {},
    simulateFailure: () => EMPTY_REPORT,
    findSpofs: () => "[]",
    lint: () => "[]",
    explain: (selection: string) =>
      JSON.stringify({
        subject: selection,
        selection_kind: selection.includes("->") ? "dependency" : "resource",
        summary: "",
        depends_on: [],
        depended_on_by: [],
        takes_down: [],
        notes: [],
      }),
    estimateCost: () => EMPTY_COST,
    generateIac: () => "# stub",
    profileJson: () => EMPTY_PROFILE,
    stateJson: () => EMPTY_STATE,
    loadJson: () => {},
    ...overrides,
  };
}
