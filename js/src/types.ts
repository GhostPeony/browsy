/**
 * Shared types for the browsy SDK.
 */

// ---------------------------------------------------------------------------
// Browsy Configuration
// ---------------------------------------------------------------------------

export interface BrowsyConfig {
  /** Port for the browsy server (default: 3847) */
  port: number;
  /** Auto-start the browsy server on init (default: true) */
  autoStart: boolean;
  /** Allow fetching private/internal network URLs (default: false) */
  allowPrivateNetwork: boolean;
  /** Timeout in ms waiting for browsy server readiness (default: 10000) */
  serverTimeout: number;
}

export interface BrowsyConfigInput {
  port?: number;
  autoStart?: boolean;
  allowPrivateNetwork?: boolean;
  serverTimeout?: number;
}

// ---------------------------------------------------------------------------
// Browsy Server
// ---------------------------------------------------------------------------

export type BrowsyServerStatus = "stopped" | "starting" | "running" | "error";

export interface BrowsyServerInfo {
  status: BrowsyServerStatus;
  port: number;
  pid?: number;
  error?: string;
}

// ---------------------------------------------------------------------------
// Browsy Session
// ---------------------------------------------------------------------------

export interface BrowsySession {
  agentId: string;
  token: string;
  createdAt: string;
}

// ---------------------------------------------------------------------------
// Browsy HTTP Client
// ---------------------------------------------------------------------------

export interface BrowsyResponse {
  ok: boolean;
  status: number;
  session: string;
  body: string;
  json?: unknown;
}
