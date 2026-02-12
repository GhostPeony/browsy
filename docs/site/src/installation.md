# Installation

browsy is available as a Rust library, a CLI binary, a Python package, and an MCP server.

## Rust library

Add `browsy-core` to your project:

```bash
cargo add browsy-core
```

This enables the `fetch` feature by default, which includes HTTP fetching, session management, and web search via reqwest.

### Without networking

To use browsy as a pure HTML-to-Spatial-DOM parser with no network dependencies:

```bash
cargo add browsy-core --no-default-features
```

This disables the `fetch` feature. You get `browsy_core::parse(html, width, height)` and nothing else -- no `Session`, no HTTP, no reqwest. Useful for embedding browsy in contexts where you handle fetching yourself.

```rust
// Available without fetch feature
let dom = browsy_core::parse(html, 1920.0, 1080.0);

// Requires fetch feature (enabled by default)
use browsy_core::fetch::Session;
let mut session = Session::new()?;
```

### Feature flags

| Feature | Default | Description |
|---|---|---|
| `fetch` | Yes | HTTP fetching, `Session` API, web search, cookie persistence |

## CLI

Install the `browsy` CLI binary:

```bash
cargo install browsy
```

Usage:

```bash
# Fetch and parse a live page
browsy fetch https://example.com

# Parse local HTML from stdin
cat page.html | browsy parse

# JSON output
browsy fetch https://example.com --format json
```

## Python

browsy has PyO3 bindings published as the `browsy` package:

```bash
pip install browsy
```

```python
import browsy

# Parse HTML directly
dom = browsy.parse(html, 1920.0, 1080.0)
print(dom.page_type)
print(dom.suggested_actions)

# Session-based browsing
session = browsy.Session()
dom = session.goto("https://example.com")
session.type_text(19, "hello")
session.click(34)
```

The Python bindings expose the same `Session` API as the Rust library, including `login`, `search`, `enter_code`, and all form interaction methods.

### Requirements

- Python 3.9+
- No native dependencies (the compiled extension includes everything)

## MCP Server

browsy ships an MCP server that exposes the full Session API as tools. This works with Claude Code, Claude Desktop, and any MCP-compatible client.

### Install

```bash
cargo install browsy-mcp
```

### Configure for Claude Code

Add to your Claude Code MCP configuration (`.claude/mcp.json` or equivalent):

```json
{
  "mcpServers": {
    "browsy": {
      "command": "browsy-mcp",
      "args": []
    }
  }
}
```

### Configure for Claude Desktop

Add to your Claude Desktop config (`claude_desktop_config.json`):

```json
{
  "mcpServers": {
    "browsy": {
      "command": "browsy-mcp",
      "args": []
    }
  }
}
```

### Available MCP tools

The MCP server exposes these tools:

| Tool | Description |
|---|---|
| `browse` | Navigate to a URL, returns Spatial DOM |
| `click` | Click an element by ID |
| `type_text` | Type into an input field by ID |
| `check` / `uncheck` | Toggle checkboxes and radio buttons |
| `select` | Select a dropdown option |
| `get_page` | Get the current page DOM with form state |
| `back` | Go back in navigation history |
| `search` | Web search via DuckDuckGo or Google |
| `find` | Find elements by text or ARIA role |
| `login` | Fill and submit a login form |
| `enter_code` | Fill and submit a verification code |
| `tables` | Extract structured table data |
| `page_info` | Get page metadata, type, and suggested actions |

## Building from source

```bash
git clone https://github.com/GhostPeony/browsy
cd browsy

# Build everything (library + CLI + MCP server)
cargo build --release

# Run tests
cargo test -p browsy-core

# Install CLI and MCP server from local source
cargo install --path crates/cli
cargo install --path crates/mcp
```
