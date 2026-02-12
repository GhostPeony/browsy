import { tool } from "ai";
import { BrowsyContext } from "./context.js";
import {
  BrowseParams,
  ClickParams,
  TypeTextParams,
  CheckParams,
  UncheckParams,
  SelectParams,
  SearchParams,
  LoginParams,
  EnterCodeParams,
  FindParams,
  GetPageParams,
  PageInfoParams,
  TablesParams,
  BackParams,
  TOOL_DESCRIPTIONS,
} from "./schemas.js";
import type { BrowsyConfigInput } from "./types.js";

let defaultContext: BrowsyContext | undefined;

function getDefaultContext(): BrowsyContext {
  if (!defaultContext) {
    defaultContext = new BrowsyContext();
  }
  return defaultContext;
}

/**
 * Returns an object map of browsy tool name → Vercel AI SDK tool instance.
 * Ready for use with `generateText({ tools: browsyTools() })`.
 */
export function browsyTools(ctx?: BrowsyContext | BrowsyConfigInput) {
  const context = ctx instanceof BrowsyContext
    ? ctx
    : ctx
      ? new BrowsyContext(ctx)
      : getDefaultContext();

  return {
    browsy_browse: tool({
      description: TOOL_DESCRIPTIONS.browsy_browse,
      parameters: BrowseParams,
      execute: async (params) => context.executeToolCall("browse", params),
    }),
    browsy_click: tool({
      description: TOOL_DESCRIPTIONS.browsy_click,
      parameters: ClickParams,
      execute: async (params) => context.executeToolCall("click", params),
    }),
    browsy_type_text: tool({
      description: TOOL_DESCRIPTIONS.browsy_type_text,
      parameters: TypeTextParams,
      execute: async (params) => context.executeToolCall("typeText", params),
    }),
    browsy_check: tool({
      description: TOOL_DESCRIPTIONS.browsy_check,
      parameters: CheckParams,
      execute: async (params) => context.executeToolCall("check", params),
    }),
    browsy_uncheck: tool({
      description: TOOL_DESCRIPTIONS.browsy_uncheck,
      parameters: UncheckParams,
      execute: async (params) => context.executeToolCall("uncheck", params),
    }),
    browsy_select: tool({
      description: TOOL_DESCRIPTIONS.browsy_select,
      parameters: SelectParams,
      execute: async (params) => context.executeToolCall("select", params),
    }),
    browsy_search: tool({
      description: TOOL_DESCRIPTIONS.browsy_search,
      parameters: SearchParams,
      execute: async (params) => context.executeToolCall("search", params),
    }),
    browsy_login: tool({
      description: TOOL_DESCRIPTIONS.browsy_login,
      parameters: LoginParams,
      execute: async (params) => context.executeToolCall("login", params),
    }),
    browsy_enter_code: tool({
      description: TOOL_DESCRIPTIONS.browsy_enter_code,
      parameters: EnterCodeParams,
      execute: async (params) => context.executeToolCall("enterCode", params),
    }),
    browsy_find: tool({
      description: TOOL_DESCRIPTIONS.browsy_find,
      parameters: FindParams,
      execute: async (params) => context.executeToolCall("find", params),
    }),
    browsy_get_page: tool({
      description: TOOL_DESCRIPTIONS.browsy_get_page,
      parameters: GetPageParams,
      execute: async (params) => context.executeToolCall("getPage", params),
    }),
    browsy_page_info: tool({
      description: TOOL_DESCRIPTIONS.browsy_page_info,
      parameters: PageInfoParams,
      execute: async () => context.executeToolCall("pageInfo", {}),
    }),
    browsy_tables: tool({
      description: TOOL_DESCRIPTIONS.browsy_tables,
      parameters: TablesParams,
      execute: async () => context.executeToolCall("tables", {}),
    }),
    browsy_back: tool({
      description: TOOL_DESCRIPTIONS.browsy_back,
      parameters: BackParams,
      execute: async () => context.executeToolCall("back", {}),
    }),
  };
}
