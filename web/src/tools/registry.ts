import type { StudioStore } from "../lib/studio.svelte";
import type { WebMcpTool } from "../lib/webmcp-bridge";
import { addResourceTool } from "./add-resource";
import { configureResourceTool } from "./configure-resource";
import { connectTool } from "./connect";
import { describeArchitectureTool } from "./describe-architecture";
import { estimateCostTool } from "./estimate-cost";
import { explainTool } from "./explain";
import { findSpofsTool } from "./find-spofs";
import { generateIacTool } from "./generate-iac";
import { moveResourceTool } from "./move-resource";
import { proposeArchitectureTool } from "./propose-architecture";
import { removeResourceTool } from "./remove-resource";
import { resilienceLintTool } from "./resilience-lint";
import { simulateFailureTool } from "./simulate-failure";

/**
 * The live WebMCP toolset. Also drives the `/learn` route, so this list is the
 * single source of truth for "what can the agent do".
 */
export function buildToolRegistry(studio: StudioStore): WebMcpTool[] {
  return [
    proposeArchitectureTool(studio),
    describeArchitectureTool(studio),
    addResourceTool(studio),
    connectTool(studio),
    configureResourceTool(studio),
    moveResourceTool(studio),
    removeResourceTool(studio),
    simulateFailureTool(studio),
    findSpofsTool(studio),
    resilienceLintTool(studio),
    explainTool(studio),
    estimateCostTool(studio),
    generateIacTool(studio),
  ];
}
