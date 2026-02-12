# A2A Protocol

browsy implements Google's [Agent-to-Agent (A2A) protocol](https://google.github.io/A2A/), enabling agent discovery and task delegation over HTTP. Any A2A-compatible agent can discover browsy's capabilities and delegate web browsing tasks to it.

## Overview

A2A is a standard for agents to find and communicate with each other. browsy's A2A support consists of two parts:

1. **Agent card** -- a JSON manifest at a well-known URL describing browsy's capabilities.
2. **Task execution** -- an endpoint that accepts goals, executes them as browsing tasks, and streams status events back via SSE.

Both are served automatically by `browsy serve`.

```bash
browsy serve --port 3847
```

## Agent card

The agent card is served at `GET /.well-known/agent.json` and describes browsy's identity and capabilities.

```bash
curl http://localhost:3847/.well-known/agent.json
```

**Response:**

```json
{
  "name": "browsy",
  "description": "Zero-render browser engine for AI agents. Navigates, extracts, and interacts with web pages without rendering pixels.",
  "url": "http://localhost:3847",
  "version": "1.0",
  "capabilities": {
    "streaming": true,
    "pushNotifications": false
  },
  "skills": [
    {
      "id": "web-browse",
      "name": "Web Browsing",
      "description": "Navigate to URLs, interact with pages, extract content, fill forms, and search the web.",
      "tags": ["browse", "scrape", "extract", "search", "login", "forms"]
    }
  ]
}
```

Agents discover browsy by fetching this card and inspecting the `skills` array. The `streaming: true` capability indicates that task responses are delivered as Server-Sent Events (SSE).

## Task execution

### POST /a2a/tasks

Submit a task for browsy to execute. The response is an SSE event stream with status updates.

**Request body:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `goal` | string | yes | Natural language description of the task |
| `params` | object | no | Structured parameters (see below) |

**Params fields:**

| Field | Type | Description |
|-------|------|-------------|
| `url` | string | Target URL to browse |
| `credentials` | object | `{ "username": "...", "password": "..." }` for login tasks |
| `search_query` | string | Query string for search tasks |
| `extract` | string | What to extract from the page (e.g., `"tables"`, `"links"`, `"text"`) |

browsy infers the task intent from the `goal` text and `params` fields. Explicit params take priority over goal parsing.

## Intent detection

browsy maps each task to one of these intents:

| Intent | Trigger | Behavior |
|--------|---------|----------|
| `Search` | `search_query` param, or goal contains "search" | Performs a web search, returns results |
| `Login` | `credentials` param, or goal contains "login"/"sign in" | Navigates to URL, fills login form, submits |
| `Extract` | `extract` param (not "tables"), or goal contains "extract"/"scrape" | Navigates to URL, returns page content |
| `ExtractTables` | `extract: "tables"`, or goal contains "table" | Navigates to URL, extracts structured table data |
| `FillForm` | Goal contains "fill"/"form"/"submit" | Navigates to URL, interacts with form elements |
| `Browse` | Default fallback | Navigates to URL, returns the Spatial DOM |

## SSE event stream

The response uses `Content-Type: text/event-stream`. Each event is a JSON object with the following structure:

```
data: {"id":"task_abc123","status":"working","steps":[{"description":"Navigating to https://example.com"}]}

data: {"id":"task_abc123","status":"completed","steps":[{"description":"Navigating to https://example.com"},{"description":"Page loaded: Example Domain (3 elements)"}],"result":{"page_type":"Other","title":"Example Domain","elements":3}}
```

**Event fields:**

| Field | Type | Description |
|-------|------|-------------|
| `id` | string | Unique task identifier |
| `status` | string | `"working"`, `"completed"`, or `"failed"` |
| `steps` | array | List of `{ "description": "..." }` objects showing progress |
| `result` | object | Present when `status` is `"completed"`. Contains extracted data |
| `error` | string | Present when `status` is `"failed"`. Describes what went wrong |

The stream always ends with a terminal event (`"completed"` or `"failed"`).

## Examples

### Browse a page

```bash
curl -N http://localhost:3847/a2a/tasks \
  -H "Content-Type: application/json" \
  -d '{
    "goal": "Browse the Hacker News front page",
    "params": { "url": "https://news.ycombinator.com" }
  }'
```

**Event stream:**

```
data: {"id":"task_1","status":"working","steps":[{"description":"Navigating to https://news.ycombinator.com"}]}

data: {"id":"task_1","status":"completed","steps":[{"description":"Navigating to https://news.ycombinator.com"},{"description":"Page loaded: Hacker News (120 elements)"}],"result":{"page_type":"List","title":"Hacker News","elements":120}}
```

### Search the web

```bash
curl -N http://localhost:3847/a2a/tasks \
  -H "Content-Type: application/json" \
  -d '{
    "goal": "Search for Rust web frameworks",
    "params": { "search_query": "rust web framework 2026" }
  }'
```

### Login to a site

```bash
curl -N http://localhost:3847/a2a/tasks \
  -H "Content-Type: application/json" \
  -d '{
    "goal": "Login to the application",
    "params": {
      "url": "https://app.example.com/login",
      "credentials": { "username": "user@example.com", "password": "secret" }
    }
  }'
```

**Event stream:**

```
data: {"id":"task_3","status":"working","steps":[{"description":"Navigating to https://app.example.com/login"}]}

data: {"id":"task_3","status":"working","steps":[{"description":"Navigating to https://app.example.com/login"},{"description":"Login page detected, submitting credentials"}]}

data: {"id":"task_3","status":"completed","steps":[{"description":"Navigating to https://app.example.com/login"},{"description":"Login page detected, submitting credentials"},{"description":"Login successful, redirected to Dashboard"}],"result":{"page_type":"Dashboard","title":"Dashboard - App"}}
```

### Extract table data

```bash
curl -N http://localhost:3847/a2a/tasks \
  -H "Content-Type: application/json" \
  -d '{
    "goal": "Extract the pricing table",
    "params": {
      "url": "https://example.com/pricing",
      "extract": "tables"
    }
  }'
```

### Extract page content

```bash
curl -N http://localhost:3847/a2a/tasks \
  -H "Content-Type: application/json" \
  -d '{
    "goal": "Extract the main article text",
    "params": {
      "url": "https://example.com/blog/post",
      "extract": "text"
    }
  }'
```

### Fill a form

```bash
curl -N http://localhost:3847/a2a/tasks \
  -H "Content-Type: application/json" \
  -d '{
    "goal": "Fill out the contact form with name John and email john@example.com",
    "params": { "url": "https://example.com/contact" }
  }'
```

## Task status polling

A stub endpoint exists for polling task status by ID:

```
GET /a2a/tasks/{task_id}
```

```bash
curl http://localhost:3847/a2a/tasks/task_abc123
```

This returns the last known state of the task. Since tasks execute synchronously over SSE, polling is primarily useful for checking whether a task completed after a disconnection.

## Error handling

When a task fails, the final SSE event includes an `error` field:

```
data: {"id":"task_5","status":"failed","steps":[{"description":"Navigating to https://invalid.example"}],"error":"Network error: DNS resolution failed"}
```

Common failure causes:

| Error | Cause |
|-------|-------|
| Network error | DNS failure, connection refused, timeout |
| CAPTCHA detected | Target page requires human verification |
| No login form found | Login intent but page has no detected login action |
| Element not found | Form interaction referenced a nonexistent element |
