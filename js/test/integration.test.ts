import { describe, it, expect, beforeAll, afterAll } from "vitest";
import { BrowsyContext } from "../src/context.js";
import { BrowsyClient } from "../src/client.js";
import { createMockBrowsyServer } from "./fixtures/mock-browsy-server.js";

describe("Integration: browse flow", () => {
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

  it("full navigate → interact → extract flow", async () => {
    const client = new BrowsyClient(port);

    // Step 1: Navigate
    const browseRes = await client.browse({ url: "https://example.com" });
    expect(browseRes.ok).toBe(true);
    expect(browseRes.session).toBeTruthy();
    const session = browseRes.session;

    // Step 2: Find elements
    const findRes = await client.find({ text: "Submit" }, session);
    expect(findRes.ok).toBe(true);
    const elements = findRes.json as Array<{ id: number; tag: string; text: string }>;
    expect(elements.length).toBeGreaterThan(0);

    // Step 3: Type into a field
    const typeRes = await client.typeText({ id: 3, text: "test input" }, session);
    expect(typeRes.ok).toBe(true);

    // Step 4: Click submit
    const clickRes = await client.click({ id: 5 }, session);
    expect(clickRes.ok).toBe(true);

    // Step 5: Check page info
    const infoRes = await client.pageInfo(session);
    expect(infoRes.ok).toBe(true);
    expect(infoRes.json).toMatchObject({ title: "Example Page" });

    // Step 6: Extract tables
    const tablesRes = await client.tables(session);
    expect(tablesRes.ok).toBe(true);
    const tables = tablesRes.json as Array<{ headers: string[]; rows: string[][] }>;
    expect(tables.length).toBeGreaterThan(0);
    expect(tables[0]!.headers).toContain("Name");

    // Step 7: Go back
    const backRes = await client.back(session);
    expect(backRes.ok).toBe(true);
  });

  it("session isolation between agents", async () => {
    const client = new BrowsyClient(port);

    // Agent 1 browses
    const res1 = await client.browse({ url: "https://example.com" });
    expect(res1.session).toBeTruthy();

    // Agent 2 browses (no session header — gets new session)
    const res2 = await client.browse({ url: "https://other.com" });
    expect(res2.session).toBeTruthy();

    // Sessions are different
    expect(res1.session).not.toBe(res2.session);
  });

  it("executeToolCall dispatches correctly", async () => {
    const result = await ctx.executeToolCall("browse", { url: "https://example.com" });
    expect(result).toContain("Example Page");
  });

  it("executeToolCall throws on unknown method", async () => {
    await expect(
      ctx.executeToolCall("nonexistent", {}),
    ).rejects.toThrow("Unknown browsy method");
  });

  it("search + browse workflow", async () => {
    const client = new BrowsyClient(port);

    // Search
    const searchRes = await client.search({ query: "example" });
    expect(searchRes.ok).toBe(true);
    const results = searchRes.json as Array<{ url: string }>;
    expect(results.length).toBeGreaterThan(0);

    // Browse first result
    const browseRes = await client.browse({ url: results[0]!.url }, searchRes.session);
    expect(browseRes.ok).toBe(true);
  });

  it("login flow", async () => {
    const client = new BrowsyClient(port);

    // Navigate to login page
    const browseRes = await client.browse({ url: "https://example.com/login" });
    const session = browseRes.session;

    // Login
    const loginRes = await client.login(
      { username: "user@test.com", password: "secret" },
      session,
    );
    expect(loginRes.ok).toBe(true);
    expect(loginRes.body).toContain("Example Page");
  });
});
