import { logActivity } from "./activity.svelte";

/** MCP-shaped tool result. */
export interface ToolResult {
  content: Array<{ type: "text"; text: string }>;
}

export type ToolExecute = (
  input: Record<string, unknown>,
  options?: { signal?: AbortSignal },
) => Promise<ToolResult>;

/** One WebMCP tool: `{ descriptor, schema, execute }` collapsed into one object. */
export interface WebMcpTool {
  name: string;
  title?: string;
  description: string;
  inputSchema: Record<string, unknown>;
  annotations?: { readOnlyHint?: boolean; untrustedContentHint?: boolean };
  execute: ToolExecute;
}

/** Wrap `execute` so every call (success or failure) lands in the activity log. */
export function instrumentTool(tool: WebMcpTool): WebMcpTool {
  return {
    ...tool,
    async execute(input, options) {
      try {
        const result = await tool.execute(input, options);
        logActivity({
          tool: tool.name,
          args: input,
          result: result.content.map((c) => c.text).join(" "),
          ok: true,
        });
        return result;
      } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        logActivity({ tool: tool.name, args: input, result: message, ok: false });
        throw error;
      }
    },
  };
}

let polyfillInstalled = false;

/** True once `document.modelContext` is usable, installing the dev polyfill if needed. */
async function ensureModelContext(): Promise<boolean> {
  if (typeof document !== "undefined" && document.modelContext) return true;
  if (import.meta.env.DEV && !polyfillInstalled) {
    const { initializeWebModelContext } = await import("@mcp-b/global");
    initializeWebModelContext();
    polyfillInstalled = true;
  }
  return typeof document !== "undefined" && !!document.modelContext;
}

/**
 * Register every tool on `document.modelContext`. Returns an `AbortController`
 * whose `abort()` unregisters them all (call it on teardown).
 */
export async function registerTools(tools: WebMcpTool[]): Promise<AbortController> {
  const controller = new AbortController();
  if (!(await ensureModelContext())) {
    console.warn("[faultline] document.modelContext unavailable — tools not registered");
    return controller;
  }
  // The polyfill's `registerTool` overloads are heavily generic; a structural
  // type keeps the call site readable.
  const mc = document.modelContext as unknown as {
    registerTool(tool: unknown, options?: { signal?: AbortSignal }): Promise<void>;
  };
  for (const tool of tools) {
    await mc.registerTool(
      {
        name: tool.name,
        title: tool.title,
        description: tool.description,
        inputSchema: tool.inputSchema,
        annotations: tool.annotations,
        execute: (input: Record<string, unknown>, options?: { signal?: AbortSignal }) =>
          tool.execute(input, options),
      },
      { signal: controller.signal },
    );
  }
  return controller;
}
