import { describe, it, expect } from "vitest";
import { getTools } from "../src/langchain.js";
import { BrowsyContext } from "../src/context.js";
import { TOOL_DESCRIPTIONS } from "../src/schemas.js";

describe("LangChain integration", () => {
  it("getTools() returns 14 tools", () => {
    const ctx = new BrowsyContext({ autoStart: false });
    const tools = getTools(ctx);
    expect(tools).toHaveLength(14);
  });

  it("each tool has name and description", () => {
    const ctx = new BrowsyContext({ autoStart: false });
    const tools = getTools(ctx);
    for (const t of tools) {
      expect(t.name).toBeTruthy();
      expect(t.description).toBeTruthy();
    }
  });

  it("tool names match expected browsy tools", () => {
    const ctx = new BrowsyContext({ autoStart: false });
    const tools = getTools(ctx);
    const names = tools.map((t) => t.name);
    expect(names).toContain("browsy_browse");
    expect(names).toContain("browsy_click");
    expect(names).toContain("browsy_type_text");
    expect(names).toContain("browsy_search");
    expect(names).toContain("browsy_back");
  });

  it("tool descriptions match TOOL_DESCRIPTIONS", () => {
    const ctx = new BrowsyContext({ autoStart: false });
    const tools = getTools(ctx);
    for (const t of tools) {
      expect(t.description).toBe(
        TOOL_DESCRIPTIONS[t.name as keyof typeof TOOL_DESCRIPTIONS],
      );
    }
  });

  it("accepts BrowsyConfigInput directly", () => {
    const tools = getTools({ port: 9999, autoStart: false });
    expect(tools).toHaveLength(14);
  });
});
