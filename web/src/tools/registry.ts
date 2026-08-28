import type { StudioStore } from "../lib/studio.svelte";
import type { WebMcpTool } from "../lib/webmcp-bridge";
import { addResourceTool } from "./add-resource";

/**
 * The live WebMCP toolset. Also drives the `/learn` route, so this list is the
 * single source of truth for "what can the agent do".
 */
export function buildToolRegistry(studio: StudioStore): WebMcpTool[] {
  return [addResourceTool(studio)];
}
