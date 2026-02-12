import { zodToJsonSchema } from "./zod-to-json-schema.js";
import { BrowsyContext } from "./context.js";
import { TOOL_SCHEMAS, TOOL_DESCRIPTIONS } from "./schemas.js";
import type { BrowsyConfigInput } from "./types.js";

// ---------------------------------------------------------------------------
// Types matching OpenAI's ChatCompletionTool shape
// ---------------------------------------------------------------------------

export interface ChatCompletionTool {
  type: "function";
  function: {
    name: string;
    description: string;
    parameters: Record<string, unknown>;
    strict?: boolean;
  };
}

// ---------------------------------------------------------------------------
// Default context
// ---------------------------------------------------------------------------

let defaultContext: BrowsyContext | undefined;

function getDefaultContext(): BrowsyContext {
  if (!defaultContext) {
    defaultContext = new BrowsyContext();
  }
  return defaultContext;
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/**
 * Returns OpenAI-format tool definitions for all 14 browsy tools.
 */
export function getToolDefinitions(): ChatCompletionTool[] {
  return TOOL_SCHEMAS.map((entry) => ({
    type: "function" as const,
    function: {
      name: entry.name,
      description: TOOL_DESCRIPTIONS[entry.name as keyof typeof TOOL_DESCRIPTIONS],
      parameters: zodToJsonSchema(entry.schema),
      strict: true,
    },
  }));
}

/**
 * Dispatches a tool call by name and returns the string result.
 */
export async function handleToolCall(
  name: string,
  args: Record<string, unknown>,
  ctx?: BrowsyContext | BrowsyConfigInput,
): Promise<string> {
  const context = ctx instanceof BrowsyContext
    ? ctx
    : ctx
      ? new BrowsyContext(ctx)
      : getDefaultContext();

  const entry = TOOL_SCHEMAS.find((t) => t.name === name);
  if (!entry) {
    throw new Error(`Unknown browsy tool: ${name}`);
  }

  return context.executeToolCall(entry.method, args);
}

/**
 * Returns a bound handler function for dispatching tool calls.
 */
export function createToolCallHandler(ctx?: BrowsyContext | BrowsyConfigInput) {
  const context = ctx instanceof BrowsyContext
    ? ctx
    : ctx
      ? new BrowsyContext(ctx)
      : getDefaultContext();

  return async (name: string, args: Record<string, unknown>): Promise<string> => {
    return handleToolCall(name, args, context);
  };
}
