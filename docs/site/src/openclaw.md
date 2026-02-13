# OpenClaw Integration

browsy integrates with [OpenClaw](https://openclaw.dev) as a first-class plugin, giving every agent fast, zero-render browsing capabilities without Playwright or Chromium.

## Why use browsy in OpenClaw?

OpenClaw's built-in browser uses Playwright + CDP: ~300MB RAM, 2-5s per page. browsy handles 70%+ of agent browsing tasks at **10x speed** and **60x less memory**. The plugin auto-starts a browsy server and injects 14 browsing tools into every agent.

| | Built-in Browser | browsy Plugin |
|---|---|---|
| **Engine** | Chromium via Playwright | Zero-render Spatial DOM |
| **Memory** | ~300MB/page | ~5MB/page |
| **Latency** | 2-5s/page | <100ms/page |
| **JS support** | Full | Hidden content exposure |
| **Setup** | Bundled | `npm install openclaw-browsy` + browsy CLI |

## Installation

```bash
# Install the OpenClaw plugin
npm install openclaw-browsy

# Install the browsy CLI (needed for the server)
cargo install browsy
```

## Configuration

Add to your OpenClaw config:

```json
{
  "plugins": {
    "openclaw-browsy": {
      "port": 3847,
      "autoStart": true,
      "allowPrivateNetwork": false,
      "preferBrowsy": true,
      "serverTimeout": 10000
    }
  }
}
```

| Option | Default | Description |
|--------|---------|-------------|
| `port` | `3847` | Port for the browsy REST server |
| `autoStart` | `true` | Start `browsy serve` automatically on plugin init |
| `allowPrivateNetwork` | `false` | Allow fetching private/internal network URLs |
| `preferBrowsy` | `true` | Intercept built-in browser tool calls and redirect through browsy |
| `serverTimeout` | `10000` | Timeout (ms) waiting for server startup |

## Plugin registration

```typescript
// openclaw.config.ts
import { register } from "openclaw-browsy";
export default { register };
```

The plugin registers four components following OpenClaw's standard pattern:

1. **`preToolExecution` hook** — intercepts built-in browser tools (`browser`, `web_browser`, `playwright_browser`, `browse_web`) and redirects them through browsy when `preferBrowsy` is enabled
2. **`agent:bootstrap` hook** — injects 14 browsy tools into every agent's toolset at startup
3. **`browsy-server` service** — manages the `browsy serve` process lifecycle (auto-start, health polling, shutdown)
4. **Gateway methods + CLI commands** — `browsy.status`, `browsy.restart`, `/browsy-status`, `/browsy-sessions`

## Available tools

Every agent gets these 14 tools automatically:

| Tool | Parameters | Description |
|------|-----------|-------------|
| `browsy_browse` | `url`, `format?`, `scope?` | Navigate to a URL |
| `browsy_click` | `id` | Click an element by ID |
| `browsy_type_text` | `id`, `text` | Type text into an input field |
| `browsy_check` | `id` | Check a checkbox or radio button |
| `browsy_uncheck` | `id` | Uncheck a checkbox or radio button |
| `browsy_select` | `id`, `value` | Select a dropdown option |
| `browsy_search` | `query`, `engine?` | Search the web (DuckDuckGo or Google) |
| `browsy_login` | `username`, `password` | Log in using detected form fields |
| `browsy_enter_code` | `code` | Enter a verification or 2FA code |
| `browsy_find` | `text?`, `role?` | Find elements by text or ARIA role |
| `browsy_get_page` | `format?`, `scope?` | Get current page DOM with form state |
| `browsy_page_info` | — | Get page metadata and suggested actions |
| `browsy_tables` | — | Extract structured table data |
| `browsy_back` | — | Go back in browsing history |

## How it works

The plugin is a pure proxy — it talks to browsy's REST API via `fetch()` and manages sessions:

```
Agent → browsy_browse("https://example.com")
  → Plugin ensures browsy server is running
  → Plugin gets/creates session for this agent
  → POST /api/browse with X-Browsy-Session header
  → browsy fetches, parses, and returns Spatial DOM
  → Plugin updates session token
  → Agent receives page content
```

Each agent gets its own isolated session with independent cookies, history, and form state.

## SimpleClaw and other OpenClaw-compatible frameworks

The `openclaw-browsy` plugin works with any framework that implements the OpenClaw plugin API. This includes [SimpleClaw](https://simpleclaw.dev) and other lightweight agent orchestrators built on the OpenClaw standard.

### SimpleClaw quick start

```typescript
import { SimpleClaw } from "simpleclaw";
import { register } from "openclaw-browsy";

const claw = new SimpleClaw({
  plugins: [{ register }],
  config: {
    "openclaw-browsy": {
      port: 3847,
      preferBrowsy: true,
    },
  },
});

// Agents automatically get browsy tools
const agent = claw.createAgent({
  name: "researcher",
  instructions: "You browse the web and extract information.",
});

const result = await agent.run("Search for 'Rust web frameworks' and summarize the top 3 results");
```

### Standalone usage (no framework)

You can also use the browsy client directly without OpenClaw:

```typescript
import { BrowsyContext } from "openclaw-browsy";

const ctx = new BrowsyContext({ port: 3847, autoStart: false });

// Assumes browsy serve is already running
const page = await ctx.executeToolCall("browse", { url: "https://example.com" });
console.log(page);

const info = await ctx.executeToolCall("pageInfo", {});
console.log(info);
```

### Custom agent orchestrators

Any orchestrator that implements the four-method `OpenClawPluginApi` interface can use the plugin:

```typescript
interface OpenClawPluginApi {
  registerHook(name: string, handler: (...args: unknown[]) => unknown): void;
  registerService(name: string, service: { start?: () => void | Promise<void>; stop?: () => void | Promise<void> }): void;
  registerGatewayMethod(name: string, handler: (...args: unknown[]) => unknown): void;
  registerCommand(name: string, handler: (...args: unknown[]) => unknown): void;
}
```

Call `register(api)` with your implementation and browsy tools become available to your agents.

## preferBrowsy mode

When `preferBrowsy` is enabled (the default), the plugin intercepts calls to built-in browser tools and aborts them with a message directing the agent to use `browsy_browse` instead. This provides a transparent speed upgrade for agents that were previously using Playwright.

The intercepted tool names are:
- `browser`
- `web_browser`
- `playwright_browser`
- `browse_web`

To disable interception and run browsy alongside the built-in browser:

```json
{
  "openclaw-browsy": {
    "preferBrowsy": false
  }
}
```

## When to fall back to a full browser

browsy handles server-rendered pages, forms, search, and data extraction. Fall back to Playwright for:

- **JS-heavy SPAs** — React, Vue, Angular apps that render entirely client-side
- **Screenshots** — when you need visual/pixel-level inspection
- **Complex JS interactions** — drag-and-drop, infinite scroll, WebSocket-driven UIs
- **PDF generation** — print-to-PDF workflows

With `preferBrowsy: false`, both browsy and the built-in browser are available. Agents can choose the right tool for each task.

## Bundled skills

The plugin includes three runtime skills for common browsing patterns:

### browse-and-extract

Navigate to a URL and extract data, automatically handling cookie consent and login walls.

### web-research

Search the web, visit multiple pages, and compile a research summary with source attribution.

### form-filler

Detect form fields using browsy's page intelligence, fill them with provided data, and submit.
