# JavaScript / TypeScript

The `browsy-ai` npm package provides a TypeScript SDK for the browsy REST API, plus ready-made integrations for LangChain.js, OpenAI, and Vercel AI SDK.

## Installation

```bash
npm install browsy-ai
```

The package uses ESM and requires Node.js 22+. Framework dependencies are optional peer dependencies — install only what you need.

## Core SDK

The core SDK manages the browsy server process, HTTP communication, and per-agent session isolation.

```typescript
import { BrowsyClient, BrowsyContext, ServerManager } from "browsy-ai";
```

### BrowsyContext

The simplest way to use browsy. `BrowsyContext` is a facade that coordinates the client, server manager, and session manager.

```typescript
import { BrowsyContext } from "browsy-ai";

const ctx = new BrowsyContext({ port: 3847 });

// Execute tool calls — server auto-starts, sessions auto-managed
const page = await ctx.executeToolCall("browse", { url: "https://example.com" });
console.log(page);

const info = await ctx.executeToolCall("pageInfo", {});
console.log(info);
```

### BrowsyClient

Lower-level HTTP client for direct API calls. Use this when you manage the server and sessions yourself.

```typescript
import { BrowsyClient } from "browsy-ai";

const client = new BrowsyClient(3847);

// Navigate
const res = await client.browse({ url: "https://example.com" });
console.log(res.body);

// Interact using the session from the response
await client.typeText({ id: 5, text: "hello" }, res.session);
await client.click({ id: 12 }, res.session);

// Extract data
const tables = await client.tables(res.session);
const info = await client.pageInfo(res.session);
```

### Configuration

```typescript
import { BrowsyContext } from "browsy-ai";

const ctx = new BrowsyContext({
  port: 3847,           // REST server port (default: 3847)
  autoStart: true,      // Auto-start browsy serve (default: true)
  allowPrivateNetwork: false,  // Allow private network URLs (default: false)
  serverTimeout: 10_000,      // Startup timeout in ms (default: 10000)
});
```

When `autoStart` is true, the SDK finds the `browsy` binary in your PATH (or via the `BROWSY_BIN` environment variable) and spawns `browsy serve --port <port>`.

### Session isolation

Each agent gets its own isolated browsing session with independent cookies, history, and form state:

```typescript
const ctx = new BrowsyContext();

// Different agents get different sessions
const page1 = await ctx.executeToolCall("browse", { url: "https://a.com" }, "agent-1");
const page2 = await ctx.executeToolCall("browse", { url: "https://b.com" }, "agent-2");
```

## LangChain.js

```bash
npm install browsy-ai @langchain/core
```

```typescript
import { getTools } from "browsy-ai/langchain";
```

### Quick start

```typescript
import { getTools } from "browsy-ai/langchain";
import { ChatOpenAI } from "@langchain/openai";
import { createReactAgent } from "@langchain/langgraph/prebuilt";

const tools = getTools({ port: 3847 });
const llm = new ChatOpenAI({ model: "gpt-4o" });
const agent = createReactAgent({ llm, tools });

const result = await agent.invoke({
  messages: [{ role: "user", content: "Go to news.ycombinator.com and list the top 5 stories" }],
});
```

### Custom context

Pass a `BrowsyContext` for full control:

```typescript
import { BrowsyContext } from "browsy-ai";
import { getTools } from "browsy-ai/langchain";

const ctx = new BrowsyContext({ port: 9000, autoStart: false });
const tools = getTools(ctx);
```

### Available tools

`getTools()` returns 14 LangChain tool instances:

| Tool name | Parameters | Description |
|-----------|-----------|-------------|
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

## OpenAI

```bash
npm install browsy-ai openai
```

```typescript
import { getToolDefinitions, handleToolCall } from "browsy-ai/openai";
```

### Quick start

```typescript
import OpenAI from "openai";
import { getToolDefinitions, handleToolCall, createToolCallHandler } from "browsy-ai/openai";

const client = new OpenAI();
const tools = getToolDefinitions();

const messages = [
  { role: "user" as const, content: "Go to example.com and tell me what's there." },
];

let response = await client.chat.completions.create({
  model: "gpt-4o",
  messages,
  tools,
});

// Tool call loop
while (response.choices[0].message.tool_calls?.length) {
  const msg = response.choices[0].message;
  messages.push(msg);

  for (const toolCall of msg.tool_calls!) {
    const args = JSON.parse(toolCall.function.arguments);
    const result = await handleToolCall(toolCall.function.name, args);

    messages.push({
      role: "tool" as const,
      tool_call_id: toolCall.id,
      content: result,
    });
  }

  response = await client.chat.completions.create({
    model: "gpt-4o",
    messages,
    tools,
  });
}

console.log(response.choices[0].message.content);
```

### Bound handler

Use `createToolCallHandler()` to get a pre-bound handler:

```typescript
import { getToolDefinitions, createToolCallHandler } from "browsy-ai/openai";

const tools = getToolDefinitions();
const handle = createToolCallHandler({ port: 3847 });

// In your tool call loop:
const result = await handle(toolCall.function.name, args);
```

## Vercel AI SDK

```bash
npm install browsy-ai ai
```

```typescript
import { browsyTools } from "browsy-ai/vercel-ai";
```

### Quick start

```typescript
import { generateText } from "ai";
import { openai } from "@ai-sdk/openai";
import { browsyTools } from "browsy-ai/vercel-ai";

const result = await generateText({
  model: openai("gpt-4o"),
  tools: browsyTools(),
  prompt: "Go to news.ycombinator.com and list the top 5 stories",
  maxSteps: 10,
});

console.log(result.text);
```

### Custom context

```typescript
import { BrowsyContext } from "browsy-ai";
import { browsyTools } from "browsy-ai/vercel-ai";

const ctx = new BrowsyContext({ port: 9000 });
const tools = browsyTools(ctx);
```

## Zod schemas

All tool parameter schemas are exported as Zod objects for use in custom integrations:

```typescript
import {
  BrowseParams,
  ClickParams,
  TypeTextParams,
  SearchParams,
  TOOL_DESCRIPTIONS,
  TOOL_SCHEMAS,
} from "browsy-ai";

// Use in your own tool definitions
const parsed = BrowseParams.parse({ url: "https://example.com" });

// Iterate over all tools
for (const { name, method, schema } of TOOL_SCHEMAS) {
  console.log(name, TOOL_DESCRIPTIONS[name]);
}
```

## Prerequisites

The SDK talks to a browsy REST server. You need the browsy CLI installed:

```bash
cargo install browsy
```

With `autoStart: true` (the default), the SDK starts the server automatically. With `autoStart: false`, start it manually:

```bash
browsy serve --port 3847
```
