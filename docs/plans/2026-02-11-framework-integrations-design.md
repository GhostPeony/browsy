# browsy Framework Integrations & Protocol Support

**Date:** 2026-02-11
**Status:** Draft

---

## 1. Summary

Expand browsy from its current MCP-native integration into a universal agent browser. This design adds:

- **Unified HTTP server** — REST API + A2A protocol on a single port (`browsy serve`)
- **Python extras** — Framework integrations as optional dependencies in the `browsy` PyPI package
- **A2A skill provider** — Google Agent-to-Agent protocol support for autonomous browsing delegation
- **Framework coverage** — LangChain, CrewAI, OpenAI, AutoGen, Smolagents (upgraded from examples to proper modules)

### What stays the same

- `browsy-core` Rust crate is untouched
- MCP server (`crates/mcp/`) continues as-is
- PyO3 bindings remain the core Python interface
- CLI (`browsy fetch`, `browsy parse`) unchanged

---

## 2. Current State

| Integration | Type | Location | Status |
|---|---|---|---|
| MCP | Native stdio server, 14 tools | `crates/mcp/` | Production |
| Python bindings | PyO3 native extension | `crates/python/` | Production |
| LangChain | Example script | `examples/langchain_tool.py` | Example |
| CrewAI | Example script | `examples/crewai_tool.py` | Example |
| OpenAI functions | Example script | `examples/openai_functions.py` | Example |
| CLI | Subprocess | `crates/cli/` | Production |

---

## 3. Python Package Restructuring

### 3.1 Directory Layout

```
crates/python/
  browsy/
    __init__.py              # Core: Browser, Page, Element (PyO3, unchanged)
    _integrations/
      __init__.py
      langchain.py           # from browsy.langchain import get_tools
      crewai.py              # from browsy.crewai import BrowsyTool
      openai.py              # from browsy.openai import get_tool_definitions
      autogen.py             # from browsy.autogen import BrowsyBrowser
      smolagents.py          # from browsy.smolagents import BrowsyTool
  pyproject.toml             # extras_require definitions
```

### 3.2 Install Patterns

```bash
pip install browsy                    # Core only (PyO3 bindings)
pip install browsy[langchain]         # + langchain dependency
pip install browsy[crewai]            # + crewai dependency
pip install browsy[autogen]           # + pyautogen dependency
pip install browsy[smolagents]        # + smolagents dependency
pip install browsy[openai]            # + openai dependency
pip install browsy[all]               # Everything
```

### 3.3 Import Surface

```python
from browsy.langchain import get_tools
from browsy.crewai import BrowsyTool
from browsy.openai import get_tool_definitions, handle_tool_call
from browsy.autogen import BrowsyBrowser
from browsy.smolagents import BrowsyTool
```

Each module lazily imports its framework dependency and raises a clear `ImportError` with install instructions if missing.

### 3.4 Shared Browser Instance

All integrations share a single `Browser()` instance by default (preserving cookies/session across tool calls) but accept an injected instance for isolation:

```python
# Default: shared session
tools = get_tools()

# Custom: isolated session
browser = Browser(viewport_width=1440, viewport_height=900)
tools = get_tools(browser=browser)
```

### 3.5 Performance Path

Every Python framework integration calls PyO3 bindings directly — no CLI subprocess, no REST API, no serialization layer:

```
agent.tool_call()                    # Framework overhead: <5ms
  -> browsy.Browser.goto(url)        # PyO3 FFI: microseconds
    -> Rust Session::goto()          # Direct in-process call
      -> HTTP fetch                  # 100-500ms (dominant cost)
      -> html5ever + CSS + Taffy     # ~100ms parse
    -> PyO3 data conversion          # <5ms
  -> page.to_compact()              # <2ms string formatting
```

Total framework wrapper overhead: **<2%** of operation time. browsy's speed advantage persists through all integrations.

---

## 4. Unified HTTP Server (`crates/server/`)

### 4.1 New Rust Crate

A new crate `crates/server/` using `axum` provides both the REST API and A2A protocol on one port.

**Dependencies:** `axum`, `tokio`, `serde_json`, `browsy-core` (with `fetch` feature)

### 4.2 Launch

```bash
browsy serve                          # default :3847
browsy serve --port 8080              # custom port
browsy serve --allow-private-network  # allow fetching localhost/LAN
```

### 4.3 REST API Endpoints

```
POST /api/browse          { url, scope?, format? }
POST /api/click           { id }
POST /api/type            { id, text }
POST /api/check           { id }
POST /api/uncheck         { id }
POST /api/select          { id, value }
POST /api/search          { query, engine? }
POST /api/login           { username, password }
POST /api/enter-code      { code }
POST /api/find            { text?, role? }
GET  /api/page            ?scope=&format=
GET  /api/page-info
GET  /api/tables
POST /api/back
```

### 4.4 Session Model

Each client gets an isolated session via `X-Browsy-Session` header:

1. First request: server creates session, returns token in `X-Browsy-Session`
2. Subsequent requests: client passes token back, server routes to correct session
3. Each session has its own cookie jar, history, and DOM state
4. Sessions expire after 30 minutes of inactivity (configurable)

### 4.5 Response Format

- **JSON** (default): `Content-Type: application/json`
- **Compact text**: `Accept: text/plain` returns the compact format
- Same scope filtering as MCP: `visible`, `above_fold`, `visible_above_fold`

---

## 5. A2A Protocol Integration

### 5.1 Role: Skill Provider

Browsy operates as an autonomous skill provider in A2A. Orchestrating agents describe a browsing goal in natural language; browsy handles the multi-step navigation internally using its page intelligence (page type detection, suggested actions, form detection).

This is the right abstraction for A2A — agents delegate browsing tasks rather than micromanaging clicks. The MCP integration already covers low-level tool mode.

### 5.2 Agent Card Discovery

```
GET /.well-known/agent.json
```

```json
{
  "name": "browsy",
  "description": "Zero-render browser for AI agents. Navigates websites, fills forms, extracts structured data without rendering pixels.",
  "url": "http://localhost:3847",
  "version": "0.x.x",
  "capabilities": {
    "streaming": true,
    "pushNotifications": false
  },
  "skills": [
    {
      "id": "web-browse",
      "name": "Browse & Extract",
      "description": "Navigate to a URL, interact with the page, and extract content. Handles multi-step flows like login, search, and form submission autonomously.",
      "tags": ["web", "browsing", "scraping", "forms"],
      "examples": [
        "Go to example.com and extract the pricing table",
        "Search for 'browsy' on DuckDuckGo and return the top 5 results",
        "Log in to this dashboard with these credentials and download the report"
      ]
    }
  ],
  "defaultInputModes": ["text/plain"],
  "defaultOutputModes": ["text/plain", "application/json"]
}
```

### 5.3 Task Flow

```
Orchestrator                          browsy A2A
    |                                     |
    |  POST /a2a/tasks                    |
    |  { "goal": "extract pricing..." }   |
    | ----------------------------------> |
    |                                     |  browse(url)
    |  SSE: status=working                |  detect page type
    | <---------------------------------- |  click/type as needed
    |                                     |  extract content
    |  SSE: status=completed              |
    |  { result: { tables: [...] } }      |
    | <---------------------------------- |
```

### 5.4 Internal Planner

The A2A skill provider uses browsy's existing page intelligence to map goals to actions:

1. Parse the goal text for URL, action intent, and target data
2. `browse()` to the URL
3. Read `page_type` and `suggested_actions` from the Spatial DOM
4. Execute the appropriate action sequence (login flow, search flow, form fill, etc.)
5. Extract requested content (tables, text, links) from final page state
6. Return structured result

No LLM needed inside browsy — the heuristics are deterministic. The page intelligence system already detects login pages, search pages, forms, articles, etc. and suggests the right actions.

---

## 6. Framework Integration Details

### 6.1 LangChain (`browsy[langchain]`)

Upgrade from `examples/langchain_tool.py` to `browsy/_integrations/langchain.py`.

Six `BaseTool` subclasses matching the MCP tool surface:
- `BrowsyBrowseTool` — navigate to URL
- `BrowsyClickTool` — click element by ID
- `BrowsyTypeTextTool` — type into input
- `BrowsySearchTool` — web search
- `BrowsyLoginTool` — compound login action
- `BrowsyPageInfoTool` — page metadata

Plus a convenience function:
```python
def get_tools(browser: Browser | None = None) -> list[BaseTool]:
    """Return all browsy tools, optionally with a custom Browser instance."""
```

### 6.2 CrewAI (`browsy[crewai]`)

Single `BrowsyTool` extending `BaseTool` with command-based input. CrewAI agents prefer fewer, chunkier tools:

```python
class BrowsyTool(BaseTool):
    name = "browsy"
    description = "Web browser. Commands: browse <url>, click <id>, type <id> <text>, search <query>, login <user> <pass>, info, back"

    def _run(self, command: str) -> str:
        ...
```

### 6.3 OpenAI Function Calling (`browsy[openai]`)

Framework-agnostic tool definitions + handler. Works with raw OpenAI SDK, Assistants API, or anything using OpenAI-format tool schemas:

```python
def get_tool_definitions() -> list[dict]:
    """Return OpenAI-format tool definitions for all browsy actions."""

def handle_tool_call(name: str, arguments: dict, browser: Browser | None = None) -> str:
    """Execute a tool call and return the result string."""
```

### 6.4 AutoGen (`browsy[autogen]`)

New integration. A `ConversableAgent` subclass that registers browsing functions:

```python
class BrowsyBrowser(ConversableAgent):
    """AutoGen agent that can browse the web using browsy."""

    def __init__(self, browser: Browser | None = None, **kwargs):
        ...
        self.register_function({"browse": self._browse, "click": self._click, ...})
```

Can be added to AutoGen group chats — other agents ask it to browse.

### 6.5 Smolagents (`browsy[smolagents]`)

New integration. HuggingFace's lightweight agent framework uses a `Tool` base class:

```python
class BrowsyTool(Tool):
    name = "web_browser"
    description = "Browse websites and interact with web pages."
    inputs = {"action": {"type": "string", "description": "Command to execute..."}}
    output_type = "string"

    def forward(self, action: str) -> str:
        ...
```

---

## 7. Testing Strategy

### 7.1 Python Integration Tests

Each framework module gets a test file using local HTML fixtures (no network dependency):

```python
# tests/test_langchain.py
def test_browse_tool():
    browser = Browser()
    page = browser.load_html('<form><input name="user"></form>', "http://test")
    tools = get_tools(browser=browser)
    result = tools[0].invoke({"url": "http://test"})
    assert "user" in result
```

### 7.2 REST API Tests

Rust integration tests in `crates/server/tests/`:
- Endpoint correctness (each endpoint returns expected shape)
- Session isolation (two clients, different state)
- Scope filtering (visible, above_fold filters work via HTTP)
- Error handling (invalid session tokens, bad requests)

### 7.3 A2A Conformance Tests

- Agent Card discovery returns valid JSON matching spec
- Task creation returns task ID and initial status
- Status streaming delivers working/completed events
- Result payload contains extracted content
- Error tasks report failure with message

### 7.4 CI Matrix

```yaml
- cargo test -p browsy-core            # Core engine (existing)
- cargo test -p browsy-mcp             # MCP server (existing)
- cargo test -p browsy-server          # REST + A2A (new)
- pip install browsy[all] && pytest    # All Python integrations (new)
```

---

## 8. Rollout Order

| Phase | Work | New Crate/Module |
|---|---|---|
| 1 | REST API + A2A server | `crates/server/` |
| 2 | Package restructuring | `browsy/_integrations/` + `pyproject.toml` extras |
| 3 | Upgrade LangChain, CrewAI, OpenAI | Move from `examples/` to modules with tests |
| 4 | New integrations: AutoGen, Smolagents | New modules with tests |
| 5 | Documentation | Integration guides in mdbook site |

---

## 9. What We're Not Doing

- **JS/npm package** — Not now. Python bindings + REST API cover JS ecosystem needs via HTTP. WASM compilation is a large effort for later.
- **Semantic Kernel** — Python SDK too immature. REST API covers Microsoft-ecosystem users.
- **Vercel AI SDK** — Covered by REST API. No dedicated package.
- **LLM inside browsy** — A2A skill provider uses deterministic page intelligence, not an embedded model.
- **Breaking changes to existing APIs** — MCP server, Python bindings, and CLI remain backwards-compatible.
