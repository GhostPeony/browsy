# browsy-mcp

MCP server for browsy — use the zero-render browser engine from Claude Code, Claude Desktop, or any MCP-compatible client.

## Install

```bash
cargo install browsy-mcp
```

## Configure for Claude Code

Add to `.claude/mcp.json`:

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

## Configure for Claude Desktop

Add to `claude_desktop_config.json`:

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

## Available tools

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
| `page_info` | Get page metadata and suggested actions |

## Documentation

- [Full docs](https://ghostpeony.github.io/browsy/)
- [MCP Server guide](https://ghostpeony.github.io/browsy/mcp-server.html)
- [browsy.dev](https://browsy.dev)

## License

MIT
