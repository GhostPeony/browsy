# browsy

**Zero-render browser engine for AI agents.**

browsy converts web pages into a structured **Spatial DOM** -- a flat list of interactive and text elements with bounding boxes, roles, and states -- without rendering pixels. This gives AI agents a fast, accurate, token-efficient representation of any web page.

```
browsy fetch https://news.ycombinator.com
```
```
title: Hacker News
url: https://news.ycombinator.com
vp: 1920x1080
els: 381
---
[1:a "Hacker News" ->https://news.ycombinator.com @18,4 52x16]
[2:a "new" ->https://news.ycombinator.com/newest @77,4 19x16]
[3:a "past" ->https://news.ycombinator.com/front @102,4 21x16]
...
```

## Why browsy?

Traditional browser automation (Puppeteer, Playwright, Selenium) launches a full browser with a compositor, GPU process, and ~300MB of RAM per page. AI agents don't need pixels -- they need to know what's on the page and where to click.

browsy gives agents exactly that:

| | Full Browser | agent-browser | browsy |
|---|---|---|---|
| **Runtime** | Chromium process | Playwright wrapper | None (library) |
| **Memory** | ~300MB/page | ~200MB+ | ~5MB/page |
| **Latency** | 2-5s | 2-5s | <100ms |
| **Output** | screenshot | semantic tree | Spatial DOM (JSON) |
| **Token cost** | ~10k (screenshot) | ~1,400 tokens | ~200-800 tokens |
| **Binary size** | ~200MB | ~200MB | ~2MB crate |
| **JS support** | Full | Full | Hidden content exposure |

browsy is not a browser replacement. It's the **fast path** -- handles 70%+ of agent tasks (reading pages, filling forms, following links, searching) at a fraction of the cost. For JS-heavy SPAs, fall back to a real browser.

### Why browsy is faster (observed in benchmarks)

- **No pixel rendering**: no compositor/GPU, no screenshots, no vision model.
- **Semantic targeting**: actions use IDs, labels, roles, and form structure instead of screen coordinates.
- **Lower work per step**: many interactions operate on the in-memory DOM without refetching.
- **Lower token output**: compact Spatial DOM is smaller than screenshots or accessibility trees.

### What browsy is built for

- **Form-heavy workflows**: login, signup, search, contact forms, checkout steps.
- **High-throughput extraction**: many parallel agents per machine.
- **Predictable cost**: token-efficient output with explicit action IDs and guidance.

### Communicative by design

browsy surfaces **actionable guidance** when a page is blocked or challenged:
- Detects block signals (captcha, rate limits, Cloudflare/Turnstile, etc.)
- Returns structured recommendations (backoff, reduce scope, rotate UA)
- Flags when a **human is required** to proceed

This prevents agents from looping blindly and reduces wasted retries.
For shallow agents, browsy emits a single **next_step** hint (e.g., `backoff_and_retry`
or `ask_human_to_solve`) so they can act immediately without extra reasoning.

**Agent response playbook**
- `next_step: backoff_and_retry` → wait, then retry with a new UA and reduced scope
- `next_step: retry_with_guidance` → apply recommendations and retry once
- `next_step: ask_human_to_solve` → request human input, then resume

### Latest benchmark snapshot (2026-02-12)

- Parse i1: 823.67 ms (browsy)
- Parse i5: 1030.67 ms (browsy)
- Parse i10: 1468 ms (browsy)
- Fetch i1: 849.67 ms (browsy)
- Fetch i5: 1068.33 ms (browsy)
- Fetch i10: 1250.33 ms (browsy)
- Agent-browser i1: 7339.67 ms
- Agent-browser i5: 16408.67 ms
- Agent-browser i10: 25207 ms
- Playwright i1: 1752 ms
- Playwright i5: 2214.67 ms
- Playwright i10: 2205 ms

Live-site status (browsy):
- OK: news.ycombinator.com, duckduckgo.com, bbc.com/news, craigslist.org, news.ycombinator.com/login, httpbin.org/forms/post
- WARN (slow): github.com/login, accounts.google.com, github.com/anthropics/claude-code
- NEEDS_HUMAN: en.wikipedia.org/wiki/Main_Page, stackoverflow.com/questions, amazon.com, example.com, python.org

### Why we’re building it this way

Screenshot-based automation is slow, expensive, and brittle. Agents don’t need pixels — they need structure and intent. browsy optimizes for **speed, simplicity, and accuracy** for the majority of server-rendered or lightly dynamic pages, while allowing fallbacks to full browsers only when necessary.

## Features

### Hidden content exposure

Most "dynamic" content on server-rendered pages is already in the HTML -- just hidden via CSS (`display: none`, `visibility: hidden`, `aria-hidden`). Dropdown menus, accordion panels, modals, tab content, tooltips -- it's all there.

browsy includes hidden elements in the output with a `hidden: true` flag. The agent sees the full page without executing JavaScript:

```json
{"id": 12, "tag": "a", "text": "Profile", "href": "/profile", "hidden": true, "b": [0,0,0,0]}
{"id": 13, "tag": "a", "text": "Settings", "href": "/settings", "hidden": true, "b": [0,0,0,0]}
```

Use `dom.visible()` to filter to only visible elements, or `dom.els` to see everything.

### Spatial DOM output

Every element the agent needs -- links, buttons, inputs, headings, text, landmarks -- with:
- **Bounding box** `[x, y, width, height]` in viewport pixels
- **Role** (link, button, textbox, heading, navigation, banner, main, etc.)
- **Text content** with smart deduplication and fallback extraction
- **State** (disabled, checked, expanded, selected, required)
- **Label association** (`<label for="id">` linked to inputs)
- **Resolved URLs** (relative hrefs converted to absolute)
- **Hidden flag** (elements hidden by CSS are included, not discarded)
- **Landmark markers** (nav, header, footer, main — role only, no text blobs)

### Two output formats

**JSON** -- full structured data for programmatic use:
```json
{
  "id": 5,
  "tag": "input",
  "role": "textbox",
  "ph": "Search...",
  "label": "Search Query",
  "type": "search",
  "b": [200, 50, 300, 32]
}
```

**Compact** -- minimal tokens for LLM context windows:
```
[5:input:search "Search..." @200,50 300x32]
[!12:a "Profile" ->/profile @0,0 0x0]
```

Hidden elements are prefixed with `!` in compact format.

### Built-in web search

Search the web directly through browsy -- no separate API needed:

```rust
let mut session = Session::new()?;

// DuckDuckGo (default, most reliable)
let results = session.search("rust web frameworks")?;
// -> [{title: "...", url: "https://...", snippet: "..."}, ...]

// Google (also supported)
let results = session.search_with("query", SearchEngine::Google)?;

// Search and fetch top 3 result pages in one call
let pages = session.search_and_read("query", 3)?;
for page in &pages {
    println!("{} -> {} elements",
        page.result.title,
        page.dom.as_ref().map(|d| d.els.len()).unwrap_or(0)
    );
}
```

### Session API

Persistent browsing sessions with cookie jar, navigation history, and agent actions:

```rust
let mut session = Session::new()?;
session.goto("https://example.com")?;

// Find and interact with elements
let inputs = session.find_by_role("textbox");
session.type_text(inputs[0].id, "hello@example.com")?;
session.click(submit_button_id)?;

// O(1) element lookup by ID
let element = session.element(42);

// Navigate back
session.back()?;
```

**Actions:**
- `goto(url)` -- navigate to a URL
- `click(id)` -- click an element (follows links, submits forms, triggers detected behaviors)
- `type_text(id, text)` -- type into an input/textarea
- `select(id, value)` -- select an option in a `<select>`
- `back()` -- navigate back in history
- `search(query)` -- search the web via DuckDuckGo
- `search_with(query, engine)` -- search with a specific engine (DuckDuckGo or Google)
- `search_and_read(query, n)` -- search and fetch top N result pages

**Helpers:**
- `find_by_text(text)` -- find elements containing text
- `find_by_role(role)` -- find elements by ARIA role
- `element(id)` -- O(1) element lookup by ID
- `dom()` -- get the current Spatial DOM
- `delta()` -- get only what changed since the last navigation
- `behaviors()` -- list detected interactive behaviors

### Framework integrations

browsy integrates with popular Python AI frameworks via optional dependencies:

**LangChain:**
```bash
pip install browsy[langchain]
```
```python
from browsy.langchain import get_tools
tools = get_tools()  # -> [BrowsyBrowseTool, BrowsyClickTool, ...]
```

**CrewAI:**
```bash
pip install browsy[crewai]
```
```python
from browsy.crewai import BrowsyTool
tool = BrowsyTool()  # Single tool with all actions
```

**OpenAI function calling:**
```bash
pip install browsy[openai]
```
```python
from browsy.openai import get_tool_definitions, handle_tool_call
tools = get_tool_definitions()
result = handle_tool_call("browsy_browse", {"url": "https://example.com"})
```

**AutoGen:**
```bash
pip install browsy[autogen]
```
```python
from browsy.autogen import BrowsyBrowser
browser = BrowsyBrowser()  # AutoGen-compatible agent tool
```

**Smolagents:**
```bash
pip install browsy[smolagents]
```
```python
from browsy.smolagents import BrowsyTool
tool = BrowsyTool()  # HuggingFace smolagents tool
```

Install all integrations at once: `pip install browsy[all]`

**OpenClaw / SimpleClaw (TypeScript):**
```bash
npm install @openclaw/browsy
```
```typescript
import { register } from "@openclaw/browsy";
export default { register };
// Agents automatically get 14 browsy tools — browse, click, type, search, login, etc.
```

The OpenClaw plugin auto-starts a browsy server, manages per-agent sessions, and can intercept built-in Playwright browser tools for a transparent 10x speed upgrade. Works with any OpenClaw-compatible framework including SimpleClaw. See the [full integration guide](https://browsy.dev/openclaw.html).

### REST API server

browsy includes a REST API server for language-agnostic access:

```bash
browsy serve --port 3847
```

Endpoints:

| Method | Path | Description |
|--------|------|-------------|
| POST | `/api/browse` | Navigate to a URL |
| POST | `/api/click` | Click an element by ID |
| POST | `/api/type` | Type into an input field |
| POST | `/api/search` | Web search |
| POST | `/api/login` | Fill and submit a login form |
| GET | `/api/page` | Get current page DOM |
| GET | `/api/page-info` | Page metadata and suggested actions |
| GET | `/api/tables` | Extract structured table data |

Sessions are managed via the `X-Browsy-Session` header. The server creates a session on first request and returns the token in the response header.

### A2A protocol

browsy implements Google's [Agent-to-Agent (A2A) protocol](https://google.github.io/A2A/) for agent discovery and task delegation:

- **Agent card**: `GET /.well-known/agent.json` -- describes browsy's capabilities
- **Task execution**: `POST /a2a/tasks` -- accepts a goal in natural language, streams progress via SSE

```bash
# Discover the agent
curl http://localhost:3847/.well-known/agent.json

# Execute a task
curl -X POST http://localhost:3847/a2a/tasks \
  -H "Content-Type: application/json" \
  -d '{"goal": "Search for browsy on DuckDuckGo and return the top results"}'
```

### Viewport filtering

Reduce tokens further by filtering to what's visible:

```rust
let dom = session.dom()?;
dom.above_fold()        // elements in the viewport
dom.below_fold()        // elements below the viewport
dom.filter_above_fold() // new SpatialDom with only above-fold elements
dom.visible()           // only non-hidden elements
dom.get(id)             // O(1) element lookup
```

### Output deduplication

Real-world HTML is full of wrapper elements: `<li><a>`, `<td><span>`, `<p><label>`. browsy detects these patterns and deduplicates, emitting only the meaningful child element:

| Site | Raw elements | After dedup | Reduction |
|---|---|---|---|
| Hacker News | 633 | 381 | 40% |
| Wikipedia | 7,326 | 4,270 | 42% |
| Craigslist | 1,134 | 677 | 40% |

### Delta output

After the first page load, subsequent changes are expressed as deltas -- only the added, changed, and removed elements:

```
-[12,15,18]
[+19:a "New Item" ->https://example.com @100,200 150x20]
```

### Landmark structure

Landmark elements (`<nav>`, `<header>`, `<footer>`, `<main>`, `<aside>`, `<section>`) are emitted as **structural markers** with their ARIA role but no text. This gives agents page structure awareness without duplicating content already in child elements:

```
[1:header @0,0 1920x50]          ← role: banner (no text blob)
[2:a "Logo" ->/ @10,10 80x30]
[3:nav @0,50 1920x40]            ← role: navigation (no text blob)
[4:a "Home" ->/home @10,55 50x30]
[5:a "About" ->/about @70,55 55x30]
```

Works for both implicit landmarks (`<nav>`) and explicit ARIA roles (`<div role="navigation">`).

### Smart text extraction

Links and buttons that contain only images, SVGs, or icons get their text from a fallback chain:

1. `aria-label` attribute
2. `title` attribute
3. Child `<img alt="...">` text
4. Child `<svg><title>...</title></svg>` text

No more text-less links in the output.

### CSS engine

browsy computes layout without rendering, using [Taffy](https://github.com/DioxusLabs/taffy) for flexbox and CSS grid:

- **CSS variables**: `var(--name)`, `var(--name, fallback)`, inheritance, and variable chains
- **Flexbox**: direction, wrap, grow/shrink/basis, align-items, justify-content, gap
- **CSS Grid**: template columns/rows, `fr` units, `repeat()`, placement
- **Box model**: margin (1-4 value shorthand), padding (1-4 value shorthand), border-width, box-sizing
- **Positioning**: static, relative, absolute, fixed
- **Units**: px, %, em, rem, calc() with mixed units
- **calc()**: full expression parser with `+`, `-`, `*`, `/` and precedence
- **@media queries**: min-width, max-width, min-height, max-height, orientation, screen/print
- **Selectors**: tag, class, ID, descendant, child, universal, pseudo-classes, attribute selectors (`[attr]`, `[attr="val"]`, `[attr^="val"]`, `[attr$="val"]`, `[attr*="val"]`, `[attr~="val"]`, `[attr|="val"]`)
- **Inheritance**: font-size cascades correctly through the tree
- **External stylesheets**: fetched and applied automatically
- **Responsive**: viewport dimensions flow through the entire CSS pipeline

### Behavior detection

browsy detects common interactive patterns from HTML attributes and reports available interactions:

- **onclick handlers**: `getElementById`, `classList.toggle`, jQuery `$().toggle()`, `location.href`
- **Bootstrap/jQuery patterns**: `data-toggle="collapse"`, `data-toggle="tab"`
- **ARIA patterns**: `aria-controls`, `aria-expanded`, `role="tab"`

Combined with hidden content exposure, agents can see what's behind dropdowns and tabs without clicking.

### Smart resource blocking

By default, browsy blocks requests to known ad/tracking/font/media domains to reduce latency and bandwidth. The block list is configurable.

## Installation

### As a library

```toml
[dependencies]
browsy-core = "0.1"
```

```rust
use browsy_core::fetch::{Session, SearchEngine};

let mut session = Session::new()?;

// Browse
session.goto("https://example.com")?;
let dom = session.dom().unwrap();

for el in dom.visible() {
    if el.role.as_deref() == Some("link") {
        println!("{}: {}", el.text.as_deref().unwrap_or(""), el.href.as_deref().unwrap_or(""));
    }
}

// Search
let results = session.search("rust web frameworks")?;
for r in &results {
    println!("{} -> {}", r.title, r.url);
}
```

### As a CLI

```bash
cargo install browsy
browsy fetch https://example.com
browsy fetch https://example.com --json
browsy fetch https://example.com --json-meta  # include domain_memory metadata
browsy fetch https://example.com --viewport 375x812  # mobile
browsy parse index.html

# Start the REST API + A2A server
browsy serve
browsy serve --port 8080
```

### Without networking

Disable the `fetch` feature for a pure HTML-to-Spatial-DOM parser with zero network dependencies:

```toml
[dependencies]
browsy-core = { version = "0.1", default-features = false }
```

```rust
let dom = browsy_core::parse(html, 1920.0, 1080.0);
```

### As a Python package

```bash
pip install browsy

# With framework integrations
pip install browsy[langchain]   # LangChain tools
pip install browsy[crewai]      # CrewAI tool
pip install browsy[openai]      # OpenAI function calling
pip install browsy[autogen]     # AutoGen integration
pip install browsy[smolagents]  # HuggingFace smolagents
pip install browsy[all]         # All integrations
```

## Architecture

browsy is built on two battle-tested open source foundations and adds everything else on top:

- **[html5ever](https://github.com/servo/html5ever)** (Mozilla/Servo) -- HTML parsing. The same parser that powered Firefox Servo. Handles real-world HTML with all its quirks.
- **[Taffy](https://github.com/DioxusLabs/taffy)** (Dioxus) -- Flexbox and CSS Grid layout computation.

Everything else is built from scratch in browsy:

- **CSS engine** -- selector matching, property parsing, `var()` resolution, `calc()` evaluation, `@media` queries, specificity ordering, style inheritance
- **Spatial DOM output** -- element emission, deduplication, landmark markers, text fallback extraction, hidden content exposure, delta diffing
- **Session API** -- navigation, form interaction, cookie jar, search (DuckDuckGo + Google), history
- **Behavior detection** -- onclick/ARIA/Bootstrap pattern inference from HTML attributes

```
HTML string
    |
    v
+----------+     +--------------+     +----------+     +-------------+
|  Parser  |---->|  CSS Engine  |---->|  Layout  |---->| Spatial DOM |
| html5ever|     |   (browsy)   |     |  (Taffy) |     |   (browsy)  |
|  (Servo) |     |              |     |          |     |             |
+----------+     +--------------+     +----------+     +-------------+
                                                             |
                                                   +---------+---------+
                                                   v         v         v
                                                 JSON    Compact    Delta
```

## Testing

88+ integration tests across 6 test modules:

```bash
cargo test -p browsy-core
cargo test -p browsy-server
```

| Module | Tests | Covers |
|---|---|---|
| `css_layout` | 31 | Flexbox, grid, selectors, calc(), @media, attribute selectors, CSS variables |
| `output` | 16 | Spatial DOM, ARIA, deltas, fold filtering, hidden content, landmarks, text fallbacks |
| `session` | 12 | Navigation, forms, interactions, search (DuckDuckGo + Google) |
| `js` | 6 | Behavior detection, toggle simulation, Bootstrap patterns |
| `realworld` | 4 | Live site testing (Hacker News, Wikipedia, GitHub, Craigslist) |
| `server` | 19 | REST API endpoints, CORS, sessions, A2A protocol |

## Real-world results

Tested against live sites:

| Site | Total elements | Visible | Hidden | Links | Parse time |
|---|---|---|---|---|---|
| Hacker News | 381 | 381 | 0 | 229 | <50ms |
| Wikipedia | 4,270 | 4,231 | 39 | 1,908 | ~200ms |
| GitHub | 615 | 525 | 90 | 214 | ~150ms |
| Craigslist | 677 | 667 | 10 | 455 | <100ms |

Parse times are for HTML -> Spatial DOM conversion only (excludes network fetch).

## How it compares

| Tool | Approach | Tokens/page | Latency | JS support |
|---|---|---|---|---|
| **browsy** | Zero-render (Rust library) | 200-800 | <100ms | Hidden content exposure |
| agent-browser | Playwright wrapper | ~1,400 | 2-5s | Full |
| Browser Use | Playwright + vision | ~15,000+ | 5-10s | Full |
| Playwright MCP | Screenshot + a11y tree | ~7,800+ | 3-8s | Full |
| Computer Use | Full OS + vision model | ~10,000+ | 10s+ | Full |

browsy handles server-rendered content (docs, forms, search results, news) at a fraction of the cost. For JS-heavy SPAs, fall back to a real browser.

## Dependencies

All dependencies are MIT/Apache-2.0 licensed:

**Foundations** (browsy builds on these):
- **[html5ever](https://github.com/servo/html5ever)** -- HTML parsing (Mozilla/Servo)
- **[taffy](https://github.com/DioxusLabs/taffy)** -- Flexbox + Grid layout engine (Dioxus)

**Utilities:**
- **reqwest** -- HTTP client (optional, behind `fetch` feature)
- **url** -- URL parsing and resolution
- **serde** / **serde_json** -- Serialization
- **clap** -- CLI argument parsing (CLI crate only)

No Chromium. No WebKit. No V8. No GPU. No binary dependencies.

## License

MIT
