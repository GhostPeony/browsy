import type { BrowsyConfig, BrowsyConfigInput } from "./types.js";

// ---------------------------------------------------------------------------
// Defaults
// ---------------------------------------------------------------------------

export function defaultConfig(): BrowsyConfig {
  return {
    port: 3847,
    autoStart: true,
    allowPrivateNetwork: false,
    serverTimeout: 10_000,
  };
}

// ---------------------------------------------------------------------------
// Config parsing
// ---------------------------------------------------------------------------

/**
 * Merge partial user input onto defaults, producing a complete BrowsyConfig.
 */
export function parseConfig(input?: BrowsyConfigInput): BrowsyConfig {
  const base = defaultConfig();
  if (!input) return base;

  return {
    port: input.port ?? base.port,
    autoStart: input.autoStart ?? base.autoStart,
    allowPrivateNetwork: input.allowPrivateNetwork ?? base.allowPrivateNetwork,
    serverTimeout: input.serverTimeout ?? base.serverTimeout,
  };
}
