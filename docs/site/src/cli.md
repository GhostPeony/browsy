# CLI Usage

The browsy CLI provides three commands: `fetch` for URLs, `parse` for local HTML files, and `serve` for the REST API server.

## Installation

```bash
cargo install browsy
```

## Commands

### fetch

Fetch a URL, compute the Spatial DOM, and print the result.

```bash
browsy fetch <URL> [OPTIONS]
```

| Flag | Description |
|------|-------------|
| `--json` | Output as JSON instead of compact format |
| `--viewport <WxH>` | Viewport size (default: `1920x1080`) |
| `--no-css` | Skip fetching external CSS stylesheets |
| `--visible-only` | Only include visible (non-hidden) elements |
| `--above-fold` | Only include elements above the viewport fold |

**Examples:**

```bash
# Compact output (default)
browsy fetch https://example.com

# JSON output
browsy fetch https://example.com --json

# Mobile viewport
browsy fetch https://example.com --viewport 375x812

# Skip external CSS for faster parsing
browsy fetch https://example.com --no-css

# Only visible above-fold elements
browsy fetch https://example.com --visible-only --above-fold
```

### parse

Parse a local HTML file and print the Spatial DOM. No network requests are made (external stylesheets are not fetched).

```bash
browsy parse <FILE> [OPTIONS]
```

| Flag | Description |
|------|-------------|
| `--json` | Output as JSON instead of compact format |
| `--viewport <WxH>` | Viewport size (default: `1920x1080`) |

Use `-` to read from stdin:

```bash
echo '<h1>Hello</h1>' | browsy parse -
curl -s https://example.com | browsy parse -
```

**Examples:**

```bash
# Parse a local file
browsy parse index.html

# Parse with JSON output
browsy parse index.html --json

# Parse from stdin
cat page.html | browsy parse -
```

### serve

Start the REST API + A2A server.

```bash
browsy serve [OPTIONS]
```

| Flag | Description |
|------|-------------|
| `--port <PORT>` | Port to listen on (default: `3847`) |
| `--allow-private-network` | Allow fetching private/LAN addresses |

**Examples:**

```bash
# Start on default port
browsy serve

# Custom port
browsy serve --port 8080

# Allow local development server access
browsy serve --allow-private-network
```

The server exposes a REST API and A2A protocol endpoints. See [REST API](rest-api.md) and [A2A Protocol](a2a.md).

## Output formats

### Compact format (default)

The compact format is designed for minimal token usage in LLM contexts:

```
title: Example Domain
url: https://example.com
vp: 1920x1080
els: 3
---
[1:h1 "Example Domain"]
[2:p "This domain is for use in illustrative examples in documents."]
[3:a "More information..." ->https://www.iana.org/domains/example]
```

The header shows the page title, URL, viewport dimensions, and element count. Each element line follows the pattern `[id:tag "text"]` with optional annotations:

- `!id:tag` -- hidden element
- `id:input:password` -- input type (when not "text")
- `[name]` -- HTML name attribute
- `[v]` -- checked
- `[*]` -- required
- `[=value]` -- current value
- `->url` -- href
- `narrow` / `wide` / `full` -- width relative to viewport
- `@region` -- position (only when needed to disambiguate duplicates)

### JSON format

The JSON format includes the full `SpatialDom` structure with all element properties. See the [Architecture](architecture.md) page for the complete schema.

## MCP server mode

browsy also runs as an MCP server for use with Claude Code and other MCP clients. See [MCP Server](mcp-server.md) for details.

```bash
browsy mcp
```
