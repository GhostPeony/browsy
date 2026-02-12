import { spawn, type ChildProcess } from "node:child_process";
import type { BrowsyConfig, BrowsyServerStatus, BrowsyServerInfo } from "./types.js";
import { BrowsyClient } from "./client.js";
import { findBrowsyBinary, isPortInUse } from "./process.js";

/**
 * Manages the lifecycle of a `browsy serve` child process.
 */
export class ServerManager {
  private process: ChildProcess | null = null;
  private status: BrowsyServerStatus = "stopped";
  private error: string | undefined;
  private config: BrowsyConfig;
  private client: BrowsyClient;

  constructor(config: BrowsyConfig) {
    this.config = config;
    this.client = new BrowsyClient(config.port);
  }

  /** Start the browsy server. No-op if already running. */
  async start(): Promise<void> {
    if (this.status === "running") return;

    // Check if something is already listening on the port
    if (await isPortInUse(this.config.port)) {
      // Verify it's actually browsy by hitting /health
      try {
        const res = await this.client.health();
        if (res.ok) {
          this.status = "running";
          return;
        }
      } catch {
        // Port in use but not browsy — error
      }
      this.status = "error";
      this.error = `Port ${this.config.port} is already in use by another process`;
      throw new Error(this.error);
    }

    const binary = findBrowsyBinary();
    if (!binary) {
      this.status = "error";
      this.error = "browsy binary not found in PATH";
      throw new Error(this.error);
    }

    this.status = "starting";
    this.error = undefined;

    const args = ["serve", "--port", String(this.config.port)];
    if (this.config.allowPrivateNetwork) {
      args.push("--allow-private-network");
    }

    this.process = spawn(binary, args, {
      stdio: "pipe",
      detached: false,
    });

    this.process.on("error", (err) => {
      this.status = "error";
      this.error = err.message;
    });

    this.process.on("exit", (code) => {
      if (this.status !== "error") {
        this.status = "stopped";
      }
      if (code !== null && code !== 0) {
        this.status = "error";
        this.error = `browsy exited with code ${code}`;
      }
      this.process = null;
    });

    await this.waitForReady();
  }

  /** Stop the browsy server. */
  async stop(): Promise<void> {
    if (!this.process) {
      this.status = "stopped";
      return;
    }

    this.process.kill();
    this.process = null;
    this.status = "stopped";
    this.error = undefined;
  }

  /** Whether the server is running. */
  isRunning(): boolean {
    return this.status === "running";
  }

  /** Get the current server status. */
  getStatus(): BrowsyServerInfo {
    return {
      status: this.status,
      port: this.config.port,
      pid: this.process?.pid,
      error: this.error,
    };
  }

  /** Poll /health until ready or timeout. */
  async waitForReady(): Promise<void> {
    const deadline = Date.now() + this.config.serverTimeout;
    const interval = 200;

    while (Date.now() < deadline) {
      try {
        const res = await this.client.health();
        if (res.ok) {
          this.status = "running";
          return;
        }
      } catch {
        // Server not ready yet
      }
      await new Promise((r) => setTimeout(r, interval));
    }

    this.status = "error";
    this.error = `browsy server did not become ready within ${this.config.serverTimeout}ms`;
    throw new Error(this.error);
  }
}
