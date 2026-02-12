import { describe, it, expect, beforeAll, afterAll } from "vitest";
import { getToolDefinitions, handleToolCall, createToolCallHandler } from "../src/openai.js";
import { BrowsyContext } from "../src/context.js";
import { createMockBrowsyServer } from "./fixtures/mock-browsy-server.js";

describe("OpenAI integration", () => {
  describe("getToolDefinitions()", () => {
    it("returns 14 tool definitions", () => {
      const defs = getToolDefinitions();
      expect(defs).toHaveLength(14);
    });

    it("each definition has correct shape", () => {
      const defs = getToolDefinitions();
      for (const def of defs) {
        expect(def.type).toBe("function");
        expect(def.function.name).toBeTruthy();
        expect(def.function.description).toBeTruthy();
        expect(def.function.parameters).toBeDefined();
        expect(def.function.parameters.type).toBe("object");
        expect(def.function.strict).toBe(true);
      }
    });

    it("browsy_browse has required url parameter", () => {
      const defs = getToolDefinitions();
      const browse = defs.find((d) => d.function.name === "browsy_browse")!;
      expect(browse.function.parameters.required).toContain("url");
      const props = browse.function.parameters.properties as Record<string, unknown>;
      expect(props.url).toBeDefined();
    });

    it("browsy_page_info has no required parameters", () => {
      const defs = getToolDefinitions();
      const info = defs.find((d) => d.function.name === "browsy_page_info")!;
      expect(info.function.parameters.required).toBeUndefined();
    });
  });

  describe("handleToolCall()", () => {
    let port: number;
    const mock = createMockBrowsyServer();
    let ctx: BrowsyContext;

    beforeAll(async () => {
      port = await mock.start();
      ctx = new BrowsyContext({ port, autoStart: false });
      await ctx.serverManager.waitForReady();
    });

    afterAll(async () => {
      await mock.stop();
    });

    it("dispatches browsy_browse correctly", async () => {
      const result = await handleToolCall(
        "browsy_browse",
        { url: "https://example.com" },
        ctx,
      );
      expect(result).toContain("Example Page");
    });

    it("dispatches browsy_search correctly", async () => {
      const result = await handleToolCall(
        "browsy_search",
        { query: "test" },
        ctx,
      );
      expect(result).toContain("Example Domain");
    });

    it("throws on unknown tool name", async () => {
      await expect(
        handleToolCall("nonexistent_tool", {}, ctx),
      ).rejects.toThrow("Unknown browsy tool");
    });
  });

  describe("createToolCallHandler()", () => {
    let port: number;
    const mock = createMockBrowsyServer();
    let ctx: BrowsyContext;

    beforeAll(async () => {
      port = await mock.start();
      ctx = new BrowsyContext({ port, autoStart: false });
      await ctx.serverManager.waitForReady();
    });

    afterAll(async () => {
      await mock.stop();
    });

    it("returns a callable handler", async () => {
      const handler = createToolCallHandler(ctx);
      const result = await handler("browsy_browse", { url: "https://example.com" });
      expect(result).toContain("Example Page");
    });
  });
});
