# REST API

browsy includes a built-in HTTP server that exposes the full Session API as REST endpoints. This is the primary integration point for non-Rust, non-Python, and non-MCP clients.

## Starting the server

```bash
browsy serve --port 3847
```

The server listens on `http://localhost:3847` by default. See [CLI Usage](cli.md#serve) for all flags.

## Session management

The server manages multiple concurrent browsing sessions. Each session has its own cookie jar, navigation history, and form state.

Sessions are identified by the `X-Browsy-Session` header:

| Scenario | Behavior |
|----------|----------|
| No `X-Browsy-Session` header | Server creates a new session and returns the token in the response header |
| Valid token in header | Existing session is reused |
| Invalid or expired token | Server creates a new session and returns the new token |
| Session idle > 30 minutes | Session expires and is cleaned up |
| Server at capacity (default: 100 sessions) | Returns `503 Service Unavailable` |

Every response includes the `X-Browsy-Session` header. Clients should capture it from the first response and include it in all subsequent requests.

```bash
# First request -- capture the session token
TOKEN=$(curl -s -D- -o /dev/null http://localhost:3847/api/browse \
  -H "Content-Type: application/json" \
  -d '{"url": "https://example.com"}' | grep -i x-browsy-session | tr -d '\r' | cut -d' ' -f2)

# Subsequent requests -- reuse the session
curl http://localhost:3847/api/page-info -H "X-Browsy-Session: $TOKEN"
```

## CORS

The server sends CORS headers on all responses:

- `Access-Control-Allow-Origin: *`
- `Access-Control-Allow-Headers: Content-Type, X-Browsy-Session`
- `Access-Control-Expose-Headers: X-Browsy-Session`

This allows browser-based clients to call the API directly.

## Endpoint reference

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/api/browse` | Navigate to a URL |
| `POST` | `/api/click` | Click an element by ID |
| `POST` | `/api/type` | Type text into an input |
| `POST` | `/api/check` | Check a checkbox or radio |
| `POST` | `/api/uncheck` | Uncheck a checkbox or radio |
| `POST` | `/api/select` | Select a dropdown option |
| `POST` | `/api/search` | Web search |
| `POST` | `/api/login` | Fill and submit a login form |
| `POST` | `/api/enter-code` | Enter a verification code |
| `POST` | `/api/find` | Find elements by text or role |
| `POST` | `/api/back` | Go back in history |
| `GET` | `/api/page` | Get current page DOM |
| `GET` | `/api/page-info` | Get page metadata |
| `GET` | `/api/tables` | Extract table data |
| `GET` | `/health` | Health check |

All POST endpoints accept `Content-Type: application/json`.

## Endpoints

### POST /api/browse

Navigate to a URL and return the Spatial DOM.

**Request body:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `url` | string | yes | URL to navigate to |
| `format` | string | no | `"compact"` (default) or `"json"` |
| `scope` | string | no | `"all"` (default), `"visible"`, `"above_fold"`, or `"visible_above_fold"` |

```bash
curl http://localhost:3847/api/browse \
  -H "Content-Type: application/json" \
  -d '{"url": "https://example.com"}'
```

**Response:** The Spatial DOM in the requested format. Compact format returns plain text; JSON format returns the full structured DOM.

```bash
# JSON format with only visible elements
curl http://localhost:3847/api/browse \
  -H "Content-Type: application/json" \
  -d '{"url": "https://example.com", "format": "json", "scope": "visible"}'
```

### POST /api/click

Click an element by its ID. Links navigate to new pages; buttons submit forms.

**Request body:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `id` | integer | yes | Element ID to click |

```bash
curl http://localhost:3847/api/click \
  -H "Content-Type: application/json" \
  -H "X-Browsy-Session: $TOKEN" \
  -d '{"id": 3}'
```

**Response:** The resulting page DOM (after navigation or form submission).

### POST /api/type

Type text into an input field or textarea.

**Request body:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `id` | integer | yes | Element ID of the text input |
| `text` | string | yes | Text to type |

```bash
curl http://localhost:3847/api/type \
  -H "Content-Type: application/json" \
  -H "X-Browsy-Session: $TOKEN" \
  -d '{"id": 5, "text": "user@example.com"}'
```

**Response:** Confirmation. Use `GET /api/page` to see the updated form state.

### POST /api/check

Check a checkbox or radio button.

**Request body:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `id` | integer | yes | Element ID |

```bash
curl http://localhost:3847/api/check \
  -H "Content-Type: application/json" \
  -H "X-Browsy-Session: $TOKEN" \
  -d '{"id": 10}'
```

### POST /api/uncheck

Uncheck a checkbox or radio button.

**Request body:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `id` | integer | yes | Element ID |

```bash
curl http://localhost:3847/api/uncheck \
  -H "Content-Type: application/json" \
  -H "X-Browsy-Session: $TOKEN" \
  -d '{"id": 10}'
```

### POST /api/select

Select an option in a dropdown.

**Request body:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `id` | integer | yes | Element ID of the select element |
| `value` | string | yes | Value to select |

```bash
curl http://localhost:3847/api/select \
  -H "Content-Type: application/json" \
  -H "X-Browsy-Session: $TOKEN" \
  -d '{"id": 12, "value": "en-US"}'
```

### POST /api/search

Search the web and return structured results.

**Request body:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `query` | string | yes | Search query |
| `engine` | string | no | `"duckduckgo"` (default) or `"google"` |

```bash
curl http://localhost:3847/api/search \
  -H "Content-Type: application/json" \
  -d '{"query": "rust web framework"}'
```

**Response:**

```json
[
  {
    "title": "Actix Web - Rust Web Framework",
    "url": "https://actix.rs",
    "snippet": "A powerful, pragmatic, and fast web framework for Rust."
  }
]
```

### POST /api/login

Fill and submit a detected login form. Requires a page with a `Login` suggested action loaded in the session.

**Request body:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `username` | string | yes | Username or email |
| `password` | string | yes | Password |

```bash
# First navigate to the login page
curl http://localhost:3847/api/browse \
  -H "Content-Type: application/json" \
  -H "X-Browsy-Session: $TOKEN" \
  -d '{"url": "https://app.example.com/login"}'

# Then submit credentials
curl http://localhost:3847/api/login \
  -H "Content-Type: application/json" \
  -H "X-Browsy-Session: $TOKEN" \
  -d '{"username": "user@example.com", "password": "secretpassword"}'
```

**Response:** The resulting page DOM after login submission.

### POST /api/enter-code

Enter a verification or 2FA code. Requires a page with an `EnterCode` suggested action.

**Request body:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `code` | string | yes | Verification or 2FA code |

```bash
curl http://localhost:3847/api/enter-code \
  -H "Content-Type: application/json" \
  -H "X-Browsy-Session: $TOKEN" \
  -d '{"code": "847291"}'
```

**Response:** The resulting page DOM after code submission.

### POST /api/find

Find elements on the current page by text content or ARIA role.

**Request body:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `text` | string | no | Find elements containing this text |
| `role` | string | no | Find elements with this ARIA role |

At least one of `text` or `role` must be provided.

```bash
# Find by text
curl http://localhost:3847/api/find \
  -H "Content-Type: application/json" \
  -H "X-Browsy-Session: $TOKEN" \
  -d '{"text": "Sign In"}'

# Find by role
curl http://localhost:3847/api/find \
  -H "Content-Type: application/json" \
  -H "X-Browsy-Session: $TOKEN" \
  -d '{"role": "button"}'
```

**Response:** JSON array of matching elements.

### POST /api/back

Go back to the previous page in browsing history. No request body required.

```bash
curl -X POST http://localhost:3847/api/back \
  -H "X-Browsy-Session: $TOKEN"
```

**Response:** The previous page's DOM.

### GET /api/page

Get the current page DOM with form state overlaid. Use after `type`, `check`, `select`, or `uncheck` to see updated form values without re-fetching.

**Query parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `scope` | string | no | `"all"` (default), `"visible"`, `"above_fold"`, or `"visible_above_fold"` |
| `format` | string | no | `"compact"` (default) or `"json"` |

```bash
curl "http://localhost:3847/api/page?format=json&scope=visible" \
  -H "X-Browsy-Session: $TOKEN"
```

### GET /api/page-info

Get page metadata without the full element list. No parameters.

```bash
curl http://localhost:3847/api/page-info \
  -H "X-Browsy-Session: $TOKEN"
```

**Response:**

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

### GET /api/tables

Extract structured table data from the current page. No parameters.

```bash
curl http://localhost:3847/api/tables \
  -H "X-Browsy-Session: $TOKEN"
```

**Response:**

```json
[
  {
    "headers": ["Name", "Price", "Stock"],
    "rows": [
      ["Widget A", "$9.99", "In stock"],
      ["Widget B", "$14.99", "Out of stock"]
    ]
  }
]
```

### GET /health

Health check endpoint. No session required.

```bash
curl http://localhost:3847/health
```

**Response:**

```json
{
  "status": "ok"
}
```

## Scopes

The `scope` parameter controls which elements are included in the output:

| Scope | Description |
|-------|-------------|
| `all` | All elements including hidden ones (default) |
| `visible` | Only non-hidden elements |
| `above_fold` | Only elements with top edge within the viewport height |
| `visible_above_fold` | Non-hidden elements above the fold |

## Output formats

The `format` parameter controls the response format:

| Format | Content-Type | Description |
|--------|-------------|-------------|
| `compact` | `text/plain` | Minimal token-efficient text format (default) |
| `json` | `application/json` | Full structured Spatial DOM |

See [Output Formats](output-formats.md) for details on both formats.

## Error responses

Errors return JSON with an `error` field:

```json
{
  "error": "Element 999 not found"
}
```

| Status | Cause |
|--------|-------|
| `400` | Invalid request body or parameters |
| `404` | Element not found, no page loaded, or no matching action |
| `503` | Server at session capacity |

## Example: complete login flow

```bash
# Start the server
browsy serve --port 3847 &

# Browse to login page (captures session token)
TOKEN=$(curl -s -D- http://localhost:3847/api/browse \
  -H "Content-Type: application/json" \
  -d '{"url": "https://app.example.com/login"}' \
  | grep -i x-browsy-session | tr -d '\r' | cut -d' ' -f2)

# Check page type
curl -s http://localhost:3847/api/page-info \
  -H "X-Browsy-Session: $TOKEN" | jq .page_type
# "Login"

# Submit credentials
curl -s http://localhost:3847/api/login \
  -H "Content-Type: application/json" \
  -H "X-Browsy-Session: $TOKEN" \
  -d '{"username": "user@example.com", "password": "secret"}'

# Check if 2FA is needed
curl -s http://localhost:3847/api/page-info \
  -H "X-Browsy-Session: $TOKEN" | jq .page_type
# "TwoFactorAuth"

# Enter 2FA code
curl -s http://localhost:3847/api/enter-code \
  -H "Content-Type: application/json" \
  -H "X-Browsy-Session: $TOKEN" \
  -d '{"code": "847291"}'

# Now on the dashboard -- extract tables
curl -s http://localhost:3847/api/tables \
  -H "X-Browsy-Session: $TOKEN" | jq .
```
