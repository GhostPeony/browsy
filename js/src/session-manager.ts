import type { BrowsySession } from "./types.js";

/**
 * Maps agentId -> browsy session token.
 * Each agent gets its own isolated browsing session.
 */
export class SessionManager {
  private sessions = new Map<string, BrowsySession>();

  /** Get existing session or create a placeholder for the agent. */
  getOrCreate(agentId: string): BrowsySession {
    const existing = this.sessions.get(agentId);
    if (existing) return existing;

    const session: BrowsySession = {
      agentId,
      token: "", // Will be populated from server response
      createdAt: new Date().toISOString(),
    };
    this.sessions.set(agentId, session);
    return session;
  }

  /** Update the session token (from X-Browsy-Session response header). */
  update(agentId: string, token: string): void {
    const session = this.sessions.get(agentId);
    if (session) {
      session.token = token;
    } else {
      this.sessions.set(agentId, {
        agentId,
        token,
        createdAt: new Date().toISOString(),
      });
    }
  }

  /** Remove an agent's session. */
  remove(agentId: string): boolean {
    return this.sessions.delete(agentId);
  }

  /** Get a session by agentId. */
  get(agentId: string): BrowsySession | undefined {
    return this.sessions.get(agentId);
  }

  /** List all active sessions. */
  listSessions(): BrowsySession[] {
    return Array.from(this.sessions.values());
  }

  /** Number of active sessions. */
  count(): number {
    return this.sessions.size;
  }
}
