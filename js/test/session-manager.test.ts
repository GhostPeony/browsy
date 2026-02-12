import { describe, it, expect } from "vitest";
import { SessionManager } from "../src/session-manager.js";

describe("SessionManager", () => {
  it("creates a new session for unknown agent", () => {
    const mgr = new SessionManager();
    const session = mgr.getOrCreate("agent-1");
    expect(session.agentId).toBe("agent-1");
    expect(session.token).toBe("");
    expect(session.createdAt).toBeTruthy();
  });

  it("returns existing session for known agent", () => {
    const mgr = new SessionManager();
    const s1 = mgr.getOrCreate("agent-1");
    mgr.update("agent-1", "tok-abc");
    const s2 = mgr.getOrCreate("agent-1");
    expect(s2.token).toBe("tok-abc");
    expect(s2).toBe(s1); // Same reference
  });

  it("update() creates session if not exists", () => {
    const mgr = new SessionManager();
    mgr.update("agent-new", "tok-xyz");
    const session = mgr.get("agent-new");
    expect(session).toBeDefined();
    expect(session!.token).toBe("tok-xyz");
  });

  it("remove() deletes a session", () => {
    const mgr = new SessionManager();
    mgr.getOrCreate("agent-1");
    expect(mgr.remove("agent-1")).toBe(true);
    expect(mgr.get("agent-1")).toBeUndefined();
    expect(mgr.count()).toBe(0);
  });

  it("remove() returns false for unknown agent", () => {
    const mgr = new SessionManager();
    expect(mgr.remove("ghost")).toBe(false);
  });

  it("listSessions() returns all sessions", () => {
    const mgr = new SessionManager();
    mgr.getOrCreate("a");
    mgr.getOrCreate("b");
    mgr.getOrCreate("c");
    expect(mgr.listSessions()).toHaveLength(3);
  });

  it("count() reflects session count", () => {
    const mgr = new SessionManager();
    expect(mgr.count()).toBe(0);
    mgr.getOrCreate("a");
    expect(mgr.count()).toBe(1);
    mgr.getOrCreate("b");
    expect(mgr.count()).toBe(2);
    mgr.remove("a");
    expect(mgr.count()).toBe(1);
  });

  it("isolates sessions between agents", () => {
    const mgr = new SessionManager();
    mgr.update("agent-1", "tok-1");
    mgr.update("agent-2", "tok-2");
    expect(mgr.get("agent-1")!.token).toBe("tok-1");
    expect(mgr.get("agent-2")!.token).toBe("tok-2");
  });
});
