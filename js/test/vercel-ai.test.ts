import { describe, it, expect } from "vitest";
import { browsyTools } from "../src/vercel-ai.js";
import { BrowsyContext } from "../src/context.js";

describe("Vercel AI SDK integration", () => {
  it("browsyTools() returns object with 14 tools", () => {
    const ctx = new BrowsyContext({ autoStart: false });
    const tools = browsyTools(ctx);
    expect(Object.keys(tools)).toHaveLength(14);
  });

  it("all expected tool names are present", () => {
    const ctx = new BrowsyContext({ autoStart: false });
    const tools = browsyTools(ctx);
    const names = Object.keys(tools);
    expect(names).toContain("browsy_browse");
    expect(names).toContain("browsy_click");
    expect(names).toContain("browsy_type_text");
    expect(names).toContain("browsy_check");
    expect(names).toContain("browsy_uncheck");
    expect(names).toContain("browsy_select");
    expect(names).toContain("browsy_search");
    expect(names).toContain("browsy_login");
    expect(names).toContain("browsy_enter_code");
    expect(names).toContain("browsy_find");
    expect(names).toContain("browsy_get_page");
    expect(names).toContain("browsy_page_info");
    expect(names).toContain("browsy_tables");
    expect(names).toContain("browsy_back");
  });

  it("each tool has description and parameters", () => {
    const ctx = new BrowsyContext({ autoStart: false });
    const tools = browsyTools(ctx);
    for (const [name, t] of Object.entries(tools)) {
      expect(t.description).toBeTruthy();
      expect(t.parameters).toBeDefined();
    }
  });

  it("accepts BrowsyConfigInput directly", () => {
    const tools = browsyTools({ port: 9999, autoStart: false });
    expect(Object.keys(tools)).toHaveLength(14);
  });
});
