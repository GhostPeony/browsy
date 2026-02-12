# Output Formats

browsy supports three output formats for the Spatial DOM: **JSON** (full fidelity), **compact** (minimal tokens), and **delta** (changes only). The choice depends on your token budget and whether you need machine-readable structure or LLM-friendly brevity.

## JSON format

The full `SpatialDom` serialized as JSON. Every field, every element, complete fidelity.

```rust
let json = serde_json::to_string_pretty(&dom)?;
```

```json
{
  "url": "https://example.com",
  "title": "Example",
  "vp": [1920.0, 1080.0],
  "scroll": [0.0, 0.0],
  "page_type": "Login",
  "suggested_actions": [
    {
      "action": "Login",
      "username_id": 19,
      "password_id": 21,
      "submit_id": 34
    }
  ],
  "els": [
    {
      "id": 1,
      "tag": "nav",
      "role": "navigation",
      "b": [0, 0, 1920, 60]
    },
    {
      "id": 19,
      "tag": "input",
      "role": "textbox",
      "ph": "Username or email address",
      "type": "text",
      "name": "login",
      "label": "Username or email address",
      "b": [480, 320, 960, 40]
    },
    {
      "id": 21,
      "tag": "input",
      "role": "textbox",
      "ph": "Password",
      "type": "password",
      "name": "password",
      "label": "Password",
      "b": [480, 380, 960, 40]
    },
    {
      "id": 34,
      "tag": "button",
      "role": "button",
      "text": "Sign in",
      "b": [480, 440, 960, 44]
    }
  ]
}
```

Optional fields (`text`, `href`, `ph`, `val`, `name`, `label`, `input_type`, `hidden`, `checked`, `disabled`, `expanded`, `selected`, `required`, `alert_type`) are omitted when absent, keeping the JSON compact. The `page_type` field is omitted when it is `Other`. The `captcha` field is omitted when no CAPTCHA is detected.

Use JSON when you need programmatic access to the full DOM structure, or when feeding the output to code rather than an LLM.

## Compact format

A one-line-per-element text format designed for minimal token usage. This is the default output format in the MCP server and CLI.

```rust
use browsy_core::output::to_compact_string;

let compact = to_compact_string(&dom);
```

Each element is rendered as a bracketed line:

```
[id:tag "text" ->href]
```

Full example output:

```
[1:nav]
[5:h1 "Welcome"]
[19:input [login] "Username or email address" wide]
[21:input:password [password] "Password" wide]
[!25:a "Forgot password?" ->/reset]
[34:button "Sign in" wide]
[40:a "Create an account" ->/signup @bot]
```

### Compact format rules

**Basic structure**: `[id:tag ...]` where `id` is the numeric element ID and `tag` is the HTML tag.

**Input types**: Non-text input types are appended after the tag: `[21:input:password ...]`, `[30:input:checkbox ...]`, `[35:input:email ...]`. Plain text inputs omit the type suffix.

**Text content**: Quoted strings show the element's text or placeholder: `"Sign in"`, `"Enter your email"`.

**Links**: Destinations shown with `->`: `[12:a "About" ->/about]`.

**Form field names**: Shown in square brackets: `[login]`, `[password]`, `[email]`.

**Checked state**: `[v]` indicates a checked checkbox or radio button.

**Required state**: `[*]` indicates a required field.

**Current value**: `[=value]` shows the current value of a form field.

**Hidden elements**: Prefixed with `!` to distinguish from visible elements: `[!25:a "Forgot password?"]`.

**Size hints**: Form elements (`input`, `button`, `textarea`, `select`) include a width classification relative to viewport:

| Hint | Meaning |
|---|---|
| `narrow` | Width < 15% of viewport |
| `wide` | Width > 50% of viewport |
| `full` | Width > 90% of viewport |

No hint is shown for elements between 15-50% of viewport width.

**Position disambiguation**: When multiple elements share the same `(tag, text)` tuple, a position tag is appended to disambiguate: `@top-L`, `@top`, `@top-R`, `@mid-L`, `@mid`, `@mid-R`, `@bot-L`, `@bot`, `@bot-R`, or `@below` (below the fold). Position tags are only added when needed -- unique elements have no position suffix.

The viewport is divided into a 3x3 grid for classification:

```
+--------+--------+--------+
| top-L  |  top   | top-R  |
+--------+--------+--------+
| mid-L  |  mid   | mid-R  |
+--------+--------+--------+
| bot-L  |  bot   | bot-R  |
+--------+--------+--------+
```

### Compact format header

When served through the MCP server or CLI, compact output includes a metadata header:

```
title: GitHub Login
url: https://github.com/login
els: 47
---
[1:nav]
[5:h1 "Sign in to GitHub"]
...
```

## Delta format

After the first page load, subsequent navigations can use delta output -- only the elements that changed. This dramatically reduces token usage for multi-step workflows.

```rust
use browsy_core::output::{diff, delta_to_compact_string};

let delta = diff(&old_dom, &new_dom);
let compact_delta = delta_to_compact_string(&delta);
```

The `DeltaDom` struct contains:

```rust
pub struct DeltaDom {
    pub changed: Vec<SpatialElement>,  // Added or modified elements
    pub removed: Vec<u32>,             // IDs of removed elements
    pub vp: [f32; 2],                  // Viewport for size hints
}
```

Compact delta format uses `+` for added/changed elements and `-` for removed IDs:

```
-[3,7,12,15]
[+19:input "Search" wide]
[+20:button "Go"]
[+21:h2 "Results"]
[+22:a "First result" ->https://example.com]
```

Matching between old and new elements is done by content similarity (tag + text + placeholder + href + input type + bounds), not by ID. IDs are assigned sequentially and may differ between page loads.

### Using delta in the Session API

```rust
let mut session = Session::new()?;
session.goto("https://example.com")?;
session.goto("https://example.com/about")?;

if let Some(delta) = session.delta() {
    let output = delta_to_compact_string(&delta);
    println!("{}", output);
}
```

## Token comparison

Compact format uses approximately **58 characters per element** on average, compared to 96-157 characters for JSON and accessibility-tree-based competitors. On a typical page with 80 elements:

| Format | Approximate tokens |
|---|---|
| Compact | ~1,200 |
| JSON | ~2,500 |
| Raw accessibility tree | ~4,000+ |

Delta format reduces this further on subsequent pages -- a navigation that changes 15 elements and removes 10 produces roughly 200 tokens instead of re-sending the full 1,200.

## Choosing a format

| Scenario | Format |
|---|---|
| Programmatic consumption (code, not LLM) | JSON |
| LLM agent with normal context | Compact |
| LLM agent with tight token budget | Compact + `filter_above_fold()` |
| Multi-step browsing workflow | Compact for first page, delta for subsequent |
| Debugging / inspection | JSON |
