import type { BrowsyConfigInput, BrowsyResponse, BrowsyServerInfo } from "./types.js";
import { parseConfig } from "./config.js";
import { BrowsyClient } from "./client.js";
import { ServerManager } from "./server-manager.js";
import { SessionManager } from "./session-manager.js";

import type { BrowsyConfig } from "./types.js";

/**
 * Central facade holding config, client, server manager, and session manager.
 * One instance per SDK lifetime.
 */
export class BrowsyContext {
  readonly config: BrowsyConfig;
  readonly client: BrowsyClient;
  readonly serverManager: ServerManager;
  readonly sessionManager: SessionManager;

  /** Agent ID to use when no agent context is available */
  private defaultAgentId = "__default__";

  constructor(configInput?: BrowsyConfigInput) {
    this.config = parseConfig(configInput);
    this.client = new BrowsyClient(this.config.port);
    this.serverManager = new ServerManager(this.config);
    this.sessionManager = new SessionManager();
  }

  /** Ensure the browsy server is running. */
  async ensureServer(): Promise<void> {
    if (!this.serverManager.isRunning()) {
      await this.serverManager.start();
    }
  }

  /** Get current server status. */
  getStatus(): BrowsyServerInfo {
    return this.serverManager.getStatus();
  }

  /**
   * Execute a browsy tool call.
   * Ensures server is running, manages sessions, calls the client, and returns results.
   */
  async executeToolCall(
    method: string,
    params: Record<string, unknown>,
    agentId?: string,
  ): Promise<string> {
    await this.ensureServer();

    const aid = agentId ?? this.defaultAgentId;
    const session = this.sessionManager.getOrCreate(aid);
    const token = session.token || undefined;

    let response: BrowsyResponse;

    switch (method) {
      case "browse":
        response = await this.client.browse(
          params as { url: string; format?: string; scope?: string },
          token,
        );
        break;
      case "click":
        response = await this.client.click(params as { id: number }, token);
        break;
      case "typeText":
        response = await this.client.typeText(
          params as { id: number; text: string },
          token,
        );
        break;
      case "check":
        response = await this.client.check(params as { id: number }, token);
        break;
      case "uncheck":
        response = await this.client.uncheck(params as { id: number }, token);
        break;
      case "select":
        response = await this.client.select(
          params as { id: number; value: string },
          token,
        );
        break;
      case "search":
        response = await this.client.search(
          params as { query: string; engine?: string },
          token,
        );
        break;
      case "login":
        response = await this.client.login(
          params as { username: string; password: string },
          token,
        );
        break;
      case "enterCode":
        response = await this.client.enterCode(
          params as { code: string },
          token,
        );
        break;
      case "find":
        response = await this.client.find(
          params as { text?: string; role?: string },
          token,
        );
        break;
      case "pageInfo":
        response = await this.client.pageInfo(token);
        break;
      case "tables":
        response = await this.client.tables(token);
        break;
      case "getPage":
        response = await this.client.getPage(
          params as { format?: string; scope?: string },
          token,
        );
        break;
      case "back":
        response = await this.client.back(token);
        break;
      default:
        throw new Error(`Unknown browsy method: ${method}`);
    }

    // Update session token from response
    if (response.session) {
      this.sessionManager.update(aid, response.session);
    }

    // Return error message on failure, body on success
    let result = response.body;
    if (!response.ok) {
      const errorJson = response.json as { error?: string } | undefined;
      result = errorJson?.error ?? response.body;
    }

    return result;
  }
}
