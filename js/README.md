# browsy-ai

Zero-render browser SDK for AI agents. Browse, interact with, and extract data from web pages without launching a browser.

browsy converts HTML into a **Spatial DOM** — a flat list of interactive elements with bounding boxes, roles, and states — at a fraction of the cost and latency of screenshot-based automation.

| | Screenshot-based | browsy |
|---|---|---|
| **Runtime** | Chromium process | None (Rust library) |
| **Memory** | ~300MB/page | ~5MB/page |
| **Latency** | 2-5s | <100ms |
| **Token cost** | ~10k+ | ~200-800 |

## Install

```bash
npm install browsy-ai
```

Framework dependencies are optional — install only what you use:

```bash
npm install browsy-ai @langchain/core    # + LangChain.js
npm install browsy-ai openai             # + OpenAI
npm install browsy-ai ai                 # + Vercel AI SDK
```

Requires **Node.js 22+** and the browsy CLI for the REST server:

```bash
cargo install browsy
```

## Quick Start

### Core SDK

```typescript
import { BrowsyContext } from "browsy-ai";

const ctx = new BrowsyContext({ port: 3847 });

const page = await ctx.executeToolCall("browse", { url: "https://example.com" });
console.log(page);

const info = await ctx.executeToolCall("pageInfo", {});
console.log(info);
```

### LangChain.js

```typescript
import { getTools } from "browsy-ai/langchain";
import { ChatOpenAI } from "@langchain/openai";
import { createReactAgent } from "@langchain/langgraph/prebuilt";

const tools = getTools();  // 14 LangChain tool instances
const llm = new ChatOpenAI({ model: "gpt-4o" });
const agent = createReactAgent({ llm, tools });

const result = await agent.invoke({
  messages: [{ role: "user", content: "Go to news.ycombinator.com and list the top 5 stories" }],
});
```

### OpenAI Function Calling

```typescript
import { getToolDefinitions, handleToolCall } from "browsy-ai/openai";

const tools = getToolDefinitions();
const result = await handleToolCall("browsy_browse", { url: "https://example.com" });
```

### Vercel AI SDK

```typescript
import { browsyTools } from "browsy-ai/vercel-ai";
import { generateText } from "ai";
import { openai } from "@ai-sdk/openai";

const result = await generateText({
  model: openai("gpt-4o"),
  tools: browsyTools(),
  prompt: "Go to example.com and summarize it",
  maxSteps: 10,
});
```

## Available Tools

All integrations expose these 14 browsy tools:

| Tool | Parameters | Description |
|------|-----------|-------------|
| `browsy_browse` | `url`, `format?`, `scope?` | Navigate to a URL |
| `browsy_click` | `id` | Click an element by ID |
| `browsy_type_text` | `id`, `text` | Type into an input field |
| `browsy_check` | `id` | Check a checkbox/radio |
| `browsy_uncheck` | `id` | Uncheck a checkbox/radio |
| `browsy_select` | `id`, `value` | Select a dropdown option |
| `browsy_search` | `query`, `engine?` | Web search |
| `browsy_login` | `username`, `password` | Log in using detected form |
| `browsy_enter_code` | `code` | Enter 2FA/verification code |
| `browsy_find` | `text?`, `role?` | Find elements by text or role |
| `browsy_get_page` | `format?`, `scope?` | Get current page with form state |
| `browsy_page_info` | — | Page metadata and suggested actions |
| `browsy_tables` | — | Extract structured table data |
| `browsy_back` | — | Go back in history |

## Session Isolation

Each agent gets its own browsing session with independent cookies, history, and form state:

```typescript
const ctx = new BrowsyContext();

const page1 = await ctx.executeToolCall("browse", { url: "https://a.com" }, "agent-1");
const page2 = await ctx.executeToolCall("browse", { url: "https://b.com" }, "agent-2");
```

## Configuration

```typescript
const ctx = new BrowsyContext({
  port: 3847,                  // REST server port (default: 3847)
  autoStart: true,             // Auto-start browsy serve (default: true)
  allowPrivateNetwork: false,  // Allow private network URLs (default: false)
  serverTimeout: 10_000,       // Startup timeout in ms (default: 10000)
});
```

## Zod Schemas

All tool parameter schemas are exported as Zod objects for custom integrations:

```typescript
import { BrowseParams, ClickParams, TOOL_DESCRIPTIONS, TOOL_SCHEMAS } from "browsy-ai";

const parsed = BrowseParams.parse({ url: "https://example.com" });

for (const { name, method, schema } of TOOL_SCHEMAS) {
  console.log(name, TOOL_DESCRIPTIONS[name]);
}
```

## Documentation

Full documentation at [browsy.dev](https://browsy.dev)

- [JavaScript / TypeScript Guide](https://ghostpeony.github.io/browsy/javascript.html)
- [Framework Integrations](https://ghostpeony.github.io/browsy/framework-integrations.html)
- [REST API Reference](https://ghostpeony.github.io/browsy/rest-api.html)

## License

MIT
