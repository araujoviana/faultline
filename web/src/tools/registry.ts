import type { StudioStore } from "../lib/studio.svelte";
import type { WebMcpTool } from "../lib/webmcp-bridge";
import { addResourceTool } from "./add-resource";
import { configureResourceTool } from "./configure-resource";
import { connectTool } from "./connect";
import { findSpofsTool } from "./find-spofs";
import { simulateFailureTool } from "./simulate-failure";

/**
 * The live WebMCP toolset. Also drives the `/learn` route, so this list is the
 * single source of truth for "what can the agent do".
 */
export function buildToolRegistry(studio: StudioStore): WebMcpTool[] {
  return [
    addResourceTool(studio),
    connectTool(studio),
    configureResourceTool(studio),
    simulateFailureTool(studio),
    findSpofsTool(studio),
  ];
}
