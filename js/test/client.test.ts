import { describe, it, expect, beforeAll, afterAll } from "vitest";
import { BrowsyClient } from "../src/client.js";
import { createMockBrowsyServer } from "./fixtures/mock-browsy-server.js";

describe("BrowsyClient", () => {
  let port: number;
  const mock = createMockBrowsyServer();
  let client: BrowsyClient;

  beforeAll(async () => {
    port = await mock.start();
    client = new BrowsyClient(port);
  });

  afterAll(async () => {
    await mock.stop();
  });

  it("health() returns ok", async () => {
    const res = await client.health();
    expect(res.ok).toBe(true);
    expect(res.body).toBe("ok");
  });

  it("browse() navigates and returns page content", async () => {
    const res = await client.browse({ url: "https://example.com" });
    expect(res.ok).toBe(true);
    expect(res.session).toBeTruthy();
    expect(res.body).toContain("Example Page");
  });

  it("browse() with session reuses session", async () => {
    const res1 = await client.browse({ url: "https://example.com" });
    const res2 = await client.browse({ url: "https://example.com/2" }, res1.session);
    expect(res2.session).toBe(res1.session);
  });

  it("click() returns page content", async () => {
    const browse = await client.browse({ url: "https://example.com" });
    const res = await client.click({ id: 5 }, browse.session);
    expect(res.ok).toBe(true);
    expect(res.body).toContain("Example Page");
  });

  it("typeText() returns confirmation", async () => {
    const browse = await client.browse({ url: "https://example.com" });
    const res = await client.typeText({ id: 3, text: "hello" }, browse.session);
    expect(res.ok).toBe(true);
    expect(res.json).toMatchObject({ ok: true });
  });

  it("check() returns confirmation", async () => {
    const browse = await client.browse({ url: "https://example.com" });
    const res = await client.check({ id: 7 }, browse.session);
    expect(res.ok).toBe(true);
    expect(res.json).toMatchObject({ ok: true });
  });

  it("uncheck() returns confirmation", async () => {
    const browse = await client.browse({ url: "https://example.com" });
    const res = await client.uncheck({ id: 7 }, browse.session);
    expect(res.ok).toBe(true);
    expect(res.json).toMatchObject({ ok: true });
  });

  it("select() returns confirmation", async () => {
    const browse = await client.browse({ url: "https://example.com" });
    const res = await client.select({ id: 4, value: "opt1" }, browse.session);
    expect(res.ok).toBe(true);
    expect(res.json).toMatchObject({ ok: true });
  });

  it("search() returns results array", async () => {
    const res = await client.search({ query: "example" });
    expect(res.ok).toBe(true);
    expect(res.json).toBeInstanceOf(Array);
    expect((res.json as unknown[]).length).toBeGreaterThan(0);
  });

  it("login() returns page content", async () => {
    const browse = await client.browse({ url: "https://example.com/login" });
    const res = await client.login({ username: "user", password: "pass" }, browse.session);
    expect(res.ok).toBe(true);
    expect(res.body).toContain("Example Page");
  });

  it("enterCode() returns page content", async () => {
    const browse = await client.browse({ url: "https://example.com/2fa" });
    const res = await client.enterCode({ code: "123456" }, browse.session);
    expect(res.ok).toBe(true);
  });

  it("find() returns elements", async () => {
    const browse = await client.browse({ url: "https://example.com" });
    const res = await client.find({ text: "Submit" }, browse.session);
    expect(res.ok).toBe(true);
    expect(res.json).toBeInstanceOf(Array);
  });

  it("getPage() returns current page", async () => {
    const browse = await client.browse({ url: "https://example.com" });
    const res = await client.getPage(undefined, browse.session);
    expect(res.ok).toBe(true);
    expect(res.body).toContain("Example Page");
  });

  it("pageInfo() returns metadata", async () => {
    const browse = await client.browse({ url: "https://example.com" });
    const res = await client.pageInfo(browse.session);
    expect(res.ok).toBe(true);
    expect(res.json).toMatchObject({ title: "Example Page" });
  });

  it("tables() returns structured data", async () => {
    const browse = await client.browse({ url: "https://example.com" });
    const res = await client.tables(browse.session);
    expect(res.ok).toBe(true);
    expect(res.json).toBeInstanceOf(Array);
  });

  it("back() returns page content", async () => {
    const browse = await client.browse({ url: "https://example.com" });
    const res = await client.back(browse.session);
    expect(res.ok).toBe(true);
    expect(res.body).toContain("Example Page");
  });
});
