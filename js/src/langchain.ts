import { tool } from "@langchain/core/tools";
import { BrowsyContext } from "./context.js";
import { TOOL_SCHEMAS, TOOL_DESCRIPTIONS } from "./schemas.js";
import type { BrowsyConfigInput } from "./types.js";

let defaultContext: BrowsyContext | undefined;

function getDefaultContext(): BrowsyContext {
  if (!defaultContext) {
    defaultContext = new BrowsyContext();
  }
  return defaultContext;
}

/**
 * Returns an array of 14 LangChain tool instances for all browsy operations.
 * Lazily initializes a default BrowsyContext if none is provided.
 */
export function getTools(ctx?: BrowsyContext | BrowsyConfigInput) {
  const context = ctx instanceof BrowsyContext
    ? ctx
    : ctx
      ? new BrowsyContext(ctx)
      : getDefaultContext();

  return TOOL_SCHEMAS.map((entry) =>
    tool(
      async (params) => context.executeToolCall(entry.method, params),
      {
        name: entry.name,
        description: TOOL_DESCRIPTIONS[entry.name as keyof typeof TOOL_DESCRIPTIONS],
        schema: entry.schema,
      },
    ),
  );
}
