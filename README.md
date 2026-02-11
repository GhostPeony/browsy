# browsy

**Zero-render browser engine for AI agents.**

browsy converts web pages into a structured **Spatial DOM** — a flat list of interactive and text elements with bounding boxes, roles, and states — without rendering pixels. This gives AI agents a fast, accurate, token-efficient representation of any web page.

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

Traditional browser automation (Puppeteer, Playwright, Selenium) launches a full browser with a compositor, GPU process, and ~300MB of RAM per page. AI agents don't need pixels — they need to know what's on the page and where to click.

browsy gives agents exactly that:

| | Full Browser | browsy |
|---|---|---|
| **Memory** | ~300MB/page | ~5MB/page |
| **Startup** | 2-5s | instant |
| **Output** | screenshot + accessibility tree | Spatial DOM (JSON) |
| **Dependencies** | Chromium binary | Pure Rust, no binary deps |
| **Token cost** | ~4k tokens (screenshot) | ~200-800 tokens (compact format) |

## Features

### Spatial DOM Output
Every element the agent needs to know about — links, buttons, inputs, headings, text — is output with:
- **Bounding box** `[x, y, width, height]` in viewport pixels
- **Role** (link, button, textbox, heading, checkbox, etc.)
- **Text content** with smart deduplication
- **State** (disabled, checked, expanded, selected, required)
- **Label association** (`<label for="id">` linked to inputs)
- **Resolved URLs** (relative hrefs converted to absolute)

### Two output formats

**JSON** — full structured data for programmatic use:
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

**Compact** — minimal tokens for LLM context windows:
```
[5:input:search "Search..." @200,50 300x32]
```

### Output deduplication
Real-world HTML is full of wrapper elements: `<li><a>`, `<td><span>`, `<p><label>`. browsy detects these patterns and deduplicates, emitting only the meaningful child element. This reduces output by **34-42%** across real sites:

| Site | Raw elements | After dedup | Reduction |
|---|---|---|---|
| Hacker News | 633 | 381 | 40% |
| Wikipedia | 7,326 | 4,241 | 42% |
| httpbin.org | 38 | 25 | 34% |
| Craigslist | 1,134 | 676 | 40% |

### Delta output
After the first page load, subsequent changes can be expressed as deltas — only the added, changed, and removed elements. This dramatically reduces token usage for multi-step browsing sessions.

```
-[12,15,18]
[+19:a "New Item" ->https://example.com @100,200 150x20]
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

// Navigate back
session.back()?;
```

**Actions:**
- `goto(url)` — navigate to a URL
- `click(id)` — click an element (follows links, submits forms, triggers JS behaviors)
- `type_text(id, text)` — type into an input/textarea
- `select(id, value)` — select an option in a `<select>`
- `back()` — navigate back in history

**Helpers:**
- `find_by_text(text)` — find elements containing text
- `find_by_role(role)` — find elements by ARIA role
- `dom()` — get the current Spatial DOM
- `delta()` — get only what changed since the last call
- `behaviors()` — list detected interactive behaviors

### Pattern-based JavaScript inference
Instead of running a JS engine, browsy detects common JS patterns and simulates their effects:

- **onclick handlers**: `getElementById`, `classList.toggle`, jQuery `$().toggle()`, `location.href`
- **Bootstrap data attributes**: `data-toggle="collapse"`, `data-toggle="tab"`
- **ARIA patterns**: `aria-controls`, `aria-expanded`, `role="tab"`

When an agent clicks an element with a detected JS behavior, browsy applies the simulated effect (toggle visibility, switch tabs, toggle CSS classes) and returns the updated DOM.

### CSS engine
browsy computes layout without rendering, using [Taffy](https://github.com/DioxusLabs/taffy) for flexbox and CSS grid:

- **Flexbox**: direction, wrap, grow/shrink/basis, align-items, justify-content, gap
- **CSS Grid**: template columns/rows, `fr` units, `repeat()`, placement
- **Box model**: margin, padding, border-width, box-sizing
- **Positioning**: static, relative, absolute, fixed
- **Units**: px, %, em (context-aware), rem (root-relative)
- **Inheritance**: font-size cascades correctly through the tree
- **Selectors**: tag, class, ID, descendant, child, attribute, pseudo-classes (`:first-child`, `:last-child`, `:nth-child()`, `:not()`)
- **External stylesheets**: fetched and applied automatically
- **Inline styles**: parsed with `!important` support

### HTML parsing
Built on [html5ever](https://github.com/servo/html5ever) — the same HTML parser used by Firefox Servo. Handles real-world HTML with all its quirks.

### Smart resource blocking
By default, browsy blocks requests to known ad/tracking/font/media domains to reduce latency and bandwidth. The block list is configurable.

## Installation

### As a library

```toml
[dependencies]
browsy-core = "0.1"
```

```rust
use browsy_core::fetch::Session;

let mut session = Session::new()?;
session.goto("https://example.com")?;
let dom = session.dom()?;

for el in &dom.els {
    if el.role.as_deref() == Some("link") {
        println!("{}: {}", el.text.as_deref().unwrap_or(""), el.href.as_deref().unwrap_or(""));
    }
}
```

### As a CLI

```bash
cargo install browsy
browsy fetch https://example.com
browsy fetch https://example.com --json
browsy fetch https://example.com --viewport 375x812  # mobile
browsy parse index.html
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

## Architecture

```
HTML string
    │
    ▼
┌──────────┐     ┌──────────────┐     ┌──────────┐     ┌─────────────┐
│  Parser  │────▶│  CSS Engine  │────▶│  Layout  │────▶│ Spatial DOM │
│ html5ever│     │  + selectors │     │  (Taffy)  │     │   Output    │
└──────────┘     └──────────────┘     └──────────┘     └─────────────┘
                                                              │
                                                    ┌────────┼────────┐
                                                    ▼        ▼        ▼
                                                  JSON   Compact   Delta
```

**Crate structure:**

| Module | Lines | Purpose |
|---|---|---|
| `css/mod.rs` | 1,033 | CSS property parsing, style computation, inheritance |
| `css/selector.rs` | 452 | CSS selector matching (tag, class, ID, combinators) |
| `layout/mod.rs` | 419 | Taffy layout tree construction and bounds computation |
| `output/mod.rs` | 614 | Spatial DOM generation, dedup, URL resolution, delta |
| `fetch/mod.rs` | 709 | HTTP sessions, cookies, forms, agent actions |
| `js/mod.rs` | 510 | Pattern-based JS inference and DOM simulation |
| `dom/mod.rs` | 148 | HTML parsing via html5ever into DOM tree |
| `lib.rs` | 18 | Public API surface |
| **Total** | **~4,000** | **(excluding tests)** |

## Testing

32 integration tests covering:
- Layout computation (flexbox, grid, percentage widths, font inheritance)
- CSS features (selectors, style tags, display:none, visibility)
- Output accuracy (spatial DOM, roles, ARIA attributes, forms, e-commerce)
- Session API (navigation, typing, selecting, delta, JS click)
- JavaScript inference (onclick, data-toggle, aria-controls, tabs)
- URL resolution (relative to absolute, special schemes preserved)
- Label-input association (`<label for="id">`)

```bash
cargo test -p browsy-core
```

## Real-world performance

Tested against live sites (numbers from stress testing):

| Site | Elements | Parse time |
|---|---|---|
| Hacker News | 381 | <50ms |
| Wikipedia (main page) | 4,241 | ~200ms |
| Craigslist | 676 | <100ms |
| httpbin.org | 25 | <10ms |

Parse times are for HTML→Spatial DOM conversion only (excludes network fetch).

## Dependencies

All dependencies are MIT/Apache-2.0 licensed:

- **html5ever** — HTML parsing (Mozilla/Servo)
- **taffy** — Flexbox + Grid layout engine (Dioxus)
- **reqwest** — HTTP client (optional, behind `fetch` feature)
- **url** — URL parsing and resolution
- **serde** / **serde_json** — Serialization
- **clap** — CLI argument parsing (CLI crate only)

No Chromium. No WebKit. No V8. No GPU. No binary dependencies.

## License

MIT
