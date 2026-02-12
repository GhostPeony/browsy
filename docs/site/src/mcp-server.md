# MCP Server (Claude Code)

browsy runs as a Model Context Protocol (MCP) server, exposing its browser engine as tools that Claude Code (or any MCP client) can call directly.

## Starting the server

```bash
browsy mcp
```

This launches browsy as a stdio-based MCP server. It creates a single persistent `Session` with cookie jar, navigation history, and form state.

## Claude Code configuration

Add browsy to your `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "browsy": {
      "command": "browsy",
      "args": ["mcp"]
    }
  }
}
```

The server advertises itself as `browsy-mcp` and exposes 14 tools.

## Available tools

### browse

Navigate to a URL and return the page content.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `url` | string | yes | URL to navigate to |
| `format` | string | no | `"compact"` (default) or `"json"` |
| `scope` | string | no | `"all"` (default), `"visible"`, `"above_fold"`, or `"visible_above_fold"` |

Returns the full Spatial DOM. In compact format, the output begins with a header block:

```
title: Example Domain
url: https://example.com
els: 12
---
[1:h1 "Example Domain"]
[2:p "This domain is for use in illustrative examples..."]
[3:a "More information..." ->https://www.iana.org/domains/example]
```

If a CAPTCHA is detected, a warning is prepended to the output:

```
CAPTCHA detected (ReCaptcha) -- this page requires human verification to proceed.
```

### click

Click an element by its ID. Links navigate to new pages, buttons submit forms.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `id` | u32 | yes | Element ID to click |

Returns the resulting page DOM. Link clicks trigger navigation (fetching the href). Button clicks submit the enclosing form with all typed values and checked states. If a CAPTCHA is detected on the resulting page, a warning is included.

### type_text

Type text into an input field or textarea by element ID.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `id` | u32 | yes | Element ID of the text input |
| `text` | string | yes | Text to type into the input |

This stores the value in session state. The value is included in form submissions and reflected in subsequent `get_page` calls. Only works on `<input>` and `<textarea>` elements.

### check

Check a checkbox or radio button by element ID.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `id` | u32 | yes | Element ID of the checkbox or radio button |

### uncheck

Uncheck a checkbox or radio button by element ID.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `id` | u32 | yes | Element ID of the checkbox or radio button |

### select

Select an option in a dropdown/select element.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `id` | u32 | yes | Element ID of the select element |
| `value` | string | yes | Value to select |

### get_page

Get the current page DOM with form state overlaid. Use after `type_text`, `check`, `select`, or `uncheck` to see the updated form values without re-fetching.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `format` | string | no | `"compact"` (default) or `"json"` |
| `scope` | string | no | `"all"` (default), `"visible"`, `"above_fold"`, or `"visible_above_fold"` |

### search

Search the web and return structured results with title, URL, and snippet.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `query` | string | yes | Search query |
| `engine` | string | no | `"duckduckgo"` (default) or `"google"` |

Returns a JSON array of search results, each with `title`, `url`, and `snippet` fields.

### back

Go back to the previous page in browsing history. No parameters. Returns the previous page's DOM.

### login

Fill in a detected login form and submit it. Requires a page with a `Login` suggested action.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `username` | string | yes | Username or email |
| `password` | string | yes | Password |

This is a compound action: it types the username into the detected username field, types the password into the password field, and clicks the submit button. Returns the resulting page DOM.

### enter_code

Enter a verification or 2FA code into the detected code input field. Requires a page with an `EnterCode` suggested action.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `code` | string | yes | Verification or 2FA code |

Types the code into the detected input and clicks submit. Returns the resulting page DOM.

### find

Find elements on the current page by text content or ARIA role.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `text` | string | no | Find elements containing this text |
| `role` | string | no | Find elements with this ARIA role |

At least one of `text` or `role` must be provided. Returns a JSON array of matching elements.

### tables

Extract structured table data from the current page. No parameters. Returns a JSON array of tables, each with `headers` (string array) and `rows` (array of string arrays).

### page_info

Get page metadata without the full element list. No parameters. Returns:

```json
{
  "title": "Sign In - Example",
  "url": "https://example.com/login",
  "page_type": "Login",
  "suggested_actions": [
    {
      "action": "Login",
      "username_id": 5,
      "password_id": 8,
      "submit_id": 12
    }
  ],
  "alerts": [],
  "pagination": null
}
```

When a CAPTCHA is detected, the response includes a `captcha` field with `captcha_type` and optional `sitekey`.

## Example conversation flow

A typical agent interaction with a login-protected site:

1. **browse** `https://app.example.com` -- page_type is `Login`, suggested_actions includes `Login` with field IDs.
2. **login** with username and password -- the agent calls `login` directly, which fills and submits the form.
3. The result page might be `TwoFactorAuth` with an `EnterCode` action.
4. **enter_code** with the 2FA code -- fills the code input and submits.
5. The result page is now `Dashboard` -- the agent can proceed with its task.

For pages without compound actions, the lower-level tools work:

1. **browse** the URL.
2. **type_text** to fill form fields by ID.
3. **check** or **select** for checkboxes and dropdowns.
4. **get_page** to verify the form state looks correct.
5. **click** the submit button to submit.

## CAPTCHA warnings

When `browse` or `click` returns a page detected as `Captcha`, a warning line is prepended to the output:

```
CAPTCHA detected (HCaptcha) -- this page requires human verification to proceed.
```

The `page_info` tool also surfaces CAPTCHA details in a structured `captcha` field. browsy cannot solve CAPTCHAs -- it detects and classifies them so the agent can decide how to proceed (request human help, use a third-party solver, or try a different approach).

## Output format

In compact mode (the default), elements are rendered as:

```
[id:tag "text"]
```

With additional annotations:

- `!id:tag` -- hidden element (display:none, visibility:hidden, aria-hidden, or hidden attribute)
- `[name]` -- HTML name attribute
- `[v]` -- checked checkbox/radio
- `[*]` -- required field
- `[=value]` -- current value
- `->url` -- href target
- `narrow` / `wide` / `full` -- size hint for form elements
- `@top-L` / `@mid` / `@bot-R` -- position hint (only shown to disambiguate duplicate elements)
