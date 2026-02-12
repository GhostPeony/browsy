import { describe, it, expect } from "vitest";
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
  TOOL_SCHEMAS,
} from "../src/schemas.js";

describe("Zod schemas", () => {
  it("BrowseParams parses valid input", () => {
    const result = BrowseParams.parse({ url: "https://example.com" });
    expect(result.url).toBe("https://example.com");
  });

  it("BrowseParams accepts optional fields", () => {
    const result = BrowseParams.parse({ url: "https://example.com", format: "json", scope: "visible" });
    expect(result.format).toBe("json");
    expect(result.scope).toBe("visible");
  });

  it("BrowseParams rejects missing url", () => {
    expect(() => BrowseParams.parse({})).toThrow();
  });

  it("ClickParams parses id", () => {
    const result = ClickParams.parse({ id: 42 });
    expect(result.id).toBe(42);
  });

  it("TypeTextParams parses id and text", () => {
    const result = TypeTextParams.parse({ id: 1, text: "hello" });
    expect(result.id).toBe(1);
    expect(result.text).toBe("hello");
  });

  it("CheckParams parses id", () => {
    const result = CheckParams.parse({ id: 3 });
    expect(result.id).toBe(3);
  });

  it("UncheckParams parses id", () => {
    const result = UncheckParams.parse({ id: 3 });
    expect(result.id).toBe(3);
  });

  it("SelectParams parses id and value", () => {
    const result = SelectParams.parse({ id: 5, value: "opt1" });
    expect(result.id).toBe(5);
    expect(result.value).toBe("opt1");
  });

  it("SearchParams parses query", () => {
    const result = SearchParams.parse({ query: "test" });
    expect(result.query).toBe("test");
  });

  it("SearchParams accepts optional engine", () => {
    const result = SearchParams.parse({ query: "test", engine: "google" });
    expect(result.engine).toBe("google");
  });

  it("LoginParams parses username and password", () => {
    const result = LoginParams.parse({ username: "user", password: "pass" });
    expect(result.username).toBe("user");
    expect(result.password).toBe("pass");
  });

  it("EnterCodeParams parses code", () => {
    const result = EnterCodeParams.parse({ code: "123456" });
    expect(result.code).toBe("123456");
  });

  it("FindParams parses optional text and role", () => {
    expect(FindParams.parse({})).toEqual({});
    expect(FindParams.parse({ text: "hello" })).toEqual({ text: "hello" });
    expect(FindParams.parse({ role: "button" })).toEqual({ role: "button" });
  });

  it("GetPageParams parses optional fields", () => {
    expect(GetPageParams.parse({})).toEqual({});
    expect(GetPageParams.parse({ format: "json" })).toEqual({ format: "json" });
  });

  it("PageInfoParams accepts empty object", () => {
    expect(PageInfoParams.parse({})).toEqual({});
  });

  it("TablesParams accepts empty object", () => {
    expect(TablesParams.parse({})).toEqual({});
  });

  it("BackParams accepts empty object", () => {
    expect(BackParams.parse({})).toEqual({});
  });
});

describe("TOOL_DESCRIPTIONS", () => {
  it("has 14 tool descriptions", () => {
    expect(Object.keys(TOOL_DESCRIPTIONS)).toHaveLength(14);
  });

  it("all descriptions are non-empty strings", () => {
    for (const [name, desc] of Object.entries(TOOL_DESCRIPTIONS)) {
      expect(typeof desc).toBe("string");
      expect(desc.length).toBeGreaterThan(0);
    }
  });
});

describe("TOOL_SCHEMAS", () => {
  it("has 14 tool schemas", () => {
    expect(TOOL_SCHEMAS).toHaveLength(14);
  });

  it("each entry has name, method, and schema", () => {
    for (const entry of TOOL_SCHEMAS) {
      expect(entry.name).toBeTruthy();
      expect(entry.method).toBeTruthy();
      expect(entry.schema).toBeDefined();
    }
  });

  it("all names match TOOL_DESCRIPTIONS keys", () => {
    const descKeys = new Set(Object.keys(TOOL_DESCRIPTIONS));
    for (const entry of TOOL_SCHEMAS) {
      expect(descKeys.has(entry.name)).toBe(true);
    }
  });
});
