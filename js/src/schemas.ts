import { z } from "zod";

// ---------------------------------------------------------------------------
// Tool parameter schemas (shared across all integrations)
// ---------------------------------------------------------------------------

export const BrowseParams = z.object({
  url: z.string().describe("URL to navigate to"),
  format: z.string().optional().describe("Output format: 'compact' (default) or 'json'"),
  scope: z.string().optional().describe("Scope: 'all' (default), 'visible', 'above_fold', or 'visible_above_fold'"),
});

export const ClickParams = z.object({
  id: z.number().describe("Element ID to click"),
});

export const TypeTextParams = z.object({
  id: z.number().describe("Element ID of the text input"),
  text: z.string().describe("Text to type into the input"),
});

export const CheckParams = z.object({
  id: z.number().describe("Element ID of the checkbox or radio button"),
});

export const UncheckParams = z.object({
  id: z.number().describe("Element ID of the checkbox or radio button"),
});

export const SelectParams = z.object({
  id: z.number().describe("Element ID of the select element"),
  value: z.string().describe("Value to select"),
});

export const SearchParams = z.object({
  query: z.string().describe("Search query"),
  engine: z.string().optional().describe("Search engine: 'duckduckgo' (default) or 'google'"),
});

export const LoginParams = z.object({
  username: z.string().describe("Username or email"),
  password: z.string().describe("Password"),
});

export const EnterCodeParams = z.object({
  code: z.string().describe("Verification or 2FA code"),
});

export const FindParams = z.object({
  text: z.string().optional().describe("Find elements containing this text"),
  role: z.string().optional().describe("Find elements with this ARIA role"),
});

export const GetPageParams = z.object({
  format: z.string().optional().describe("Output format: 'compact' (default) or 'json'"),
  scope: z.string().optional().describe("Scope: 'all' (default), 'visible', 'above_fold', or 'visible_above_fold'"),
});

export const PageInfoParams = z.object({});

export const TablesParams = z.object({});

export const BackParams = z.object({});

// ---------------------------------------------------------------------------
// Tool descriptions
// ---------------------------------------------------------------------------

export const TOOL_DESCRIPTIONS = {
  browsy_browse: "Navigate to a URL and return the page content. Use this to browse websites.",
  browsy_click: "Click an element by its ID. Links navigate to new pages, buttons submit forms.",
  browsy_type_text: "Type text into an input field or textarea by element ID.",
  browsy_check: "Check a checkbox or radio button by element ID.",
  browsy_uncheck: "Uncheck a checkbox or radio button by element ID.",
  browsy_select: "Select an option in a dropdown/select element by element ID and value.",
  browsy_search: "Search the web and return structured results with title, URL, and snippet.",
  browsy_login: "Log in using detected login form fields. Requires a page with a login form loaded.",
  browsy_enter_code: "Enter a verification or 2FA code into the detected code input field.",
  browsy_find: "Find elements on the current page by text content or ARIA role.",
  browsy_get_page: "Get the current page DOM with form state (typed values, checked states). Use after type_text/check/select to see the updated form.",
  browsy_page_info: "Get page metadata: page type, suggested actions (login/search/consent), alerts, pagination, title, and URL.",
  browsy_tables: "Extract structured table data from the current page. Returns headers and rows.",
  browsy_back: "Go back to the previous page in browsing history.",
} as const;

// ---------------------------------------------------------------------------
// Schema + method mapping (used by integrations to iterate)
// ---------------------------------------------------------------------------

export const TOOL_SCHEMAS = [
  { name: "browsy_browse", method: "browse", schema: BrowseParams },
  { name: "browsy_click", method: "click", schema: ClickParams },
  { name: "browsy_type_text", method: "typeText", schema: TypeTextParams },
  { name: "browsy_check", method: "check", schema: CheckParams },
  { name: "browsy_uncheck", method: "uncheck", schema: UncheckParams },
  { name: "browsy_select", method: "select", schema: SelectParams },
  { name: "browsy_search", method: "search", schema: SearchParams },
  { name: "browsy_login", method: "login", schema: LoginParams },
  { name: "browsy_enter_code", method: "enterCode", schema: EnterCodeParams },
  { name: "browsy_find", method: "find", schema: FindParams },
  { name: "browsy_get_page", method: "getPage", schema: GetPageParams },
  { name: "browsy_page_info", method: "pageInfo", schema: PageInfoParams },
  { name: "browsy_tables", method: "tables", schema: TablesParams },
  { name: "browsy_back", method: "back", schema: BackParams },
] as const;
