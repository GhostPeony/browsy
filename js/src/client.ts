import type { BrowsyResponse } from "./types.js";

/**
 * HTTP client for the browsy REST API.
 * Uses Node.js built-in fetch(). Passes and extracts X-Browsy-Session header.
 */
export class BrowsyClient {
  private baseUrl: string;

  constructor(port: number) {
    this.baseUrl = `http://127.0.0.1:${port}`;
  }

  /** GET /health */
  async health(): Promise<BrowsyResponse> {
    return this.request("GET", "/health");
  }

  /** POST /api/browse */
  async browse(params: {
    url: string;
    format?: string;
    scope?: string;
  }, session?: string): Promise<BrowsyResponse> {
    return this.request("POST", "/api/browse", params, session);
  }

  /** POST /api/click */
  async click(params: { id: number }, session?: string): Promise<BrowsyResponse> {
    return this.request("POST", "/api/click", params, session);
  }

  /** POST /api/type */
  async typeText(params: { id: number; text: string }, session?: string): Promise<BrowsyResponse> {
    return this.request("POST", "/api/type", params, session);
  }

  /** POST /api/check */
  async check(params: { id: number }, session?: string): Promise<BrowsyResponse> {
    return this.request("POST", "/api/check", params, session);
  }

  /** POST /api/uncheck */
  async uncheck(params: { id: number }, session?: string): Promise<BrowsyResponse> {
    return this.request("POST", "/api/uncheck", params, session);
  }

  /** POST /api/select */
  async select(params: { id: number; value: string }, session?: string): Promise<BrowsyResponse> {
    return this.request("POST", "/api/select", params, session);
  }

  /** POST /api/search */
  async search(params: { query: string; engine?: string }, session?: string): Promise<BrowsyResponse> {
    return this.request("POST", "/api/search", params, session);
  }

  /** POST /api/login */
  async login(params: { username: string; password: string }, session?: string): Promise<BrowsyResponse> {
    return this.request("POST", "/api/login", params, session);
  }

  /** POST /api/enter-code */
  async enterCode(params: { code: string }, session?: string): Promise<BrowsyResponse> {
    return this.request("POST", "/api/enter-code", params, session);
  }

  /** POST /api/find */
  async find(params: { text?: string; role?: string }, session?: string): Promise<BrowsyResponse> {
    return this.request("POST", "/api/find", params, session);
  }

  /** GET /api/page */
  async getPage(params?: { format?: string; scope?: string }, session?: string): Promise<BrowsyResponse> {
    const query = new URLSearchParams();
    if (params?.format) query.set("format", params.format);
    if (params?.scope) query.set("scope", params.scope);
    const qs = query.toString();
    return this.request("GET", `/api/page${qs ? `?${qs}` : ""}`, undefined, session);
  }

  /** GET /api/page-info */
  async pageInfo(session?: string): Promise<BrowsyResponse> {
    return this.request("GET", "/api/page-info", undefined, session);
  }

  /** GET /api/tables */
  async tables(session?: string): Promise<BrowsyResponse> {
    return this.request("GET", "/api/tables", undefined, session);
  }

  /** POST /api/back */
  async back(session?: string): Promise<BrowsyResponse> {
    return this.request("POST", "/api/back", undefined, session);
  }

  // ---------------------------------------------------------------------------
  // Internal
  // ---------------------------------------------------------------------------

  private async request(
    method: string,
    path: string,
    body?: unknown,
    session?: string,
  ): Promise<BrowsyResponse> {
    const headers: Record<string, string> = {};
    if (session) {
      headers["X-Browsy-Session"] = session;
    }
    if (body !== undefined) {
      headers["Content-Type"] = "application/json";
    }

    const res = await fetch(`${this.baseUrl}${path}`, {
      method,
      headers,
      body: body !== undefined ? JSON.stringify(body) : undefined,
    });

    const responseSession = res.headers.get("X-Browsy-Session") ?? session ?? "";
    const text = await res.text();

    let json: unknown;
    try {
      json = JSON.parse(text);
    } catch {
      // Response is plain text, not JSON
    }

    return {
      ok: res.ok,
      status: res.status,
      session: responseSession,
      body: text,
      json,
    };
  }
}
