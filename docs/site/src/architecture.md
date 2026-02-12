# Architecture

browsy is a zero-render browser engine. It converts raw HTML into a flat list of interactive and text elements with bounding boxes, page type classification, and suggested actions -- without rendering pixels or executing JavaScript.

## Pipeline

```
HTML
 │
 ├──────────────────────────────────────────────────────────────────┐
 v                                                                  │
DOM Parser (html5ever)                                              │
 │                                                                  │
 v                                                                  │
DomNode tree ──> External CSS fetch (reqwest) ──> merged CSS text   │
 │                                                                  │
 v                                                                  │
CSS Engine (browsy)                                                 │
 ├── Selector matching (tag, class, ID, attribute, combinators)     │
 ├── Property parsing (var(), calc(), shorthands)                   │
 ├── @media query evaluation                                        │
 └── Specificity + cascade ordering                                 │
 │                                                                  │
 v                                                                  │
StyledNode tree (LayoutStyle per element)                           │
 │                                                                  │
 v                                                                  │
Layout Engine (Taffy)                                               │
 ├── Flexbox                                                        │
 ├── CSS Grid                                                       │
 └── Block flow                                                     │
 │                                                                  │
 v                                                                  │
LayoutNode tree (with bounding boxes)                               │
 │                                                                  │
 v                                                                  │
Spatial DOM Generator (browsy)                                      │
 ├── Element emission (interactive + text + landmark + img)         │
 ├── CAPTCHA detection (from tree scan)                             │
 ├── Deduplication (wrapper skip)                                   │
 ├── Hidden content preservation                                    │
 ├── Text fallback chain (aria-label > title > img alt > svg title) │
 ├── Label association (<label for="id">)                           │
 ├── URL resolution (relative -> absolute)                          │
 ├── Page type classification                                       │
 └── Suggested action detection                                     │
 │                                                                  │
 v                                                                  │
SpatialDom                                                          │
 ├── els: Vec<SpatialElement>  (flat list with IDs + bounds)        │
 ├── page_type: PageType                                            │
 ├── suggested_actions: Vec<SuggestedAction>                        │
 ├── captcha: Option<CaptchaInfo>                                   │
 └── title, url, viewport, scroll                                   │
```

## Entry point

The primary entry point is `browsy_core::parse`:

```rust
pub fn parse(html: &str, viewport_width: f32, viewport_height: f32) -> SpatialDom {
    let dom_tree = dom::parse_html(html);
    let styled = css::compute_styles_with_viewport(&dom_tree, viewport_width, viewport_height);
    let laid_out = layout::compute_layout(&styled, viewport_width, viewport_height);
    output::generate_spatial_dom(&laid_out, viewport_width, viewport_height)
}
```

For network-aware usage, `Session::goto()` fetches the HTML, resolves external CSS, and runs the full pipeline.

## Project structure

```
crates/
  core/                 browsy-core library (the engine)
    src/
      lib.rs              Entry point: parse(html, w, h) -> SpatialDom
      dom/mod.rs           HTML -> DomNode tree (thin wrapper around html5ever)
      css/
        mod.rs              Style computation, CSS variable inheritance
        selector.rs         CSS selector matching engine
        properties.rs       CSS property parsing, var() resolution, calc()
      layout/mod.rs        Style tree -> Taffy -> bounding boxes
      output/mod.rs        SpatialDom generation, page type, actions, CAPTCHA
      js/mod.rs            Behavior detection from HTML attributes
      fetch/
        mod.rs              HTTP fetching, form extraction, resource blocking
        session.rs          Session API, search, navigation, form interaction
    tests/
      css_layout.rs        CSS + layout integration tests
      output.rs            Spatial DOM output tests
      benchmark.rs         Detection accuracy benchmark runner
      corpus/              HTML snapshots with ground truth labels

  cli/                  browsy CLI binary
    src/main.rs           fetch and parse commands

  mcp/                  browsy MCP server
    src/
      lib.rs              MCP tool definitions (14 tools)
      main.rs             stdio server entry point

  python/               Python bindings (PyO3)
    src/lib.rs            Browser, Page, Element classes
    browsy/__init__.py    Python module
```

## What is ours vs external

browsy depends on two external crates for foundational work:

| Crate | Role | What it does |
|-------|------|-------------|
| **html5ever** (Mozilla/Servo) | HTML parsing | Converts raw HTML into a DOM tree. Handles malformed HTML, character encoding, and the full HTML5 parsing algorithm. |
| **Taffy** (Dioxus) | Layout computation | Computes bounding boxes from a style tree. Handles Flexbox, CSS Grid, and block layout. |

Everything else is built from scratch in browsy:

| Component | Description |
|-----------|-------------|
| CSS selector matching | Tag, class, ID, attribute selectors (7 operator types), descendant/child combinators, specificity ordering |
| CSS property parsing | ~40 layout properties, shorthand expansion, `var()` resolution with fallbacks, `calc()` with full expression parser |
| CSS variables | Custom property collection, inheritance through DOM tree |
| @media queries | min-width, max-width, min-height, max-height, orientation, screen/print |
| Spatial DOM output | Element emission, deduplication, landmark markers, text fallback chains, hidden content exposure, alert detection, table extraction |
| Page intelligence | Page type classification (14 types), suggested action detection (12 action types), CAPTCHA detection (7 CAPTCHA types), pagination detection, verification code extraction |
| Session API | Cookie persistence, navigation history, form state overlay, form submission, compound actions (login, enter_code) |
| Web search | DuckDuckGo and Google result parsing |
| Behavior detection | onclick/ARIA/Bootstrap pattern inference from HTML attributes |

## Key design decisions

### Hidden content exposure

Elements with `display:none`, `visibility:hidden`, `aria-hidden="true"`, or the `hidden` attribute are NOT discarded. They appear in the Spatial DOM with `hidden: true`. This is intentional -- agents need to see dropdown menus, accordion panels, modal dialogs, tab content, and other JS-toggled content that is present in the HTML but not visible without JavaScript execution.

### Landmark markers

HTML5 landmarks (`<nav>`, `<header>`, `<footer>`, `<main>`, `<aside>`, `<section>`, `<form>`) and elements with explicit landmark ARIA roles are emitted as structural markers with their role only -- no recursive text collection. Their children carry the actual content. This prevents a `<nav>` from emitting a giant concatenated string of all its link texts.

### Text fallback chain

Interactive elements (links, buttons) that contain no text but only images or icons get their text from a fallback chain:

1. `aria-label` attribute
2. `title` attribute
3. Child `<img alt>` text
4. Child `<svg><title>` text

This ensures that icon-only buttons like a hamburger menu or close button have accessible text in the Spatial DOM.

### SVG handling

SVG child elements are not emitted (they are visual, not semantic). However, `<svg><title>` text is extracted and stored as the SVG element's `aria-label`, making it available through the text fallback chain.

### Deduplication

Wrapper elements that only wrap a single interactive child (like `<li><a>...`, `<td><span>...`, `<p><a>...`) are skipped. Only the meaningful child element is emitted. This prevents duplicate text in the output. When a wrapper has its own text that would not be captured by the child, it is emitted with only its own text.

### Zero-size skip

Visible elements with zero width and height are skipped as layout artifacts. Hidden elements are always preserved regardless of size.

### Element ID assignment

Element IDs are assigned sequentially (1, 2, 3, ...) during a single parse. IDs are NOT stable across page loads -- they are positional, not content-based. The delta diff system uses content keys (tag + text + href + bounds) rather than IDs to match elements across page transitions.

## Testing

### Integration tests

Tests live in `crates/core/tests/` as integration tests:

```bash
cargo test -p browsy-core                        # all tests
cargo test -p browsy-core --test css_layout      # CSS + layout
cargo test -p browsy-core --test output          # Spatial DOM output
```

### Detection benchmark

The `crates/core/tests/corpus/` directory contains HTML snapshots of real websites with ground truth labels in `manifest.json`. The benchmark runner parses every snapshot and verifies:

- Correct page type classification
- Correct suggested action detection
- Valid element IDs in all actions (referencing real elements)
- Verification code extraction accuracy

```bash
cargo test -p browsy-core --test benchmark -- --nocapture
```

Adding a new test case:

1. Harvest an HTML snapshot with `HARVEST_URL` and `HARVEST_NAME` environment variables.
2. Add the expected labels to `corpus/manifest.json`.
3. Run the benchmark to confirm the failure.
4. Fix the heuristics in `output/mod.rs`.
5. Re-run the benchmark to confirm the fix with no regressions.

## Output formats

### JSON

Full structured output via `serde_json`. All optional fields use `skip_serializing_if` to keep the JSON compact.

### Compact text

A minimal text format designed for LLM token efficiency:

```
[1:h1 "Page Title"]
[!2:div "Hidden content"]
[3:input:email [email] [*] "Enter email" wide]
[4:button "Submit" full]
[5:a "Link" ->https://example.com @top-R]
```

Each element is one line: `[id:tag "text"]` with annotations for type, name, state, size, href, and position.

### Delta format

For page transitions, the delta format shows only what changed:

```
-[3,5,7]
[+8:h1 "New Heading"]
[+9:a "New Link" ->https://example.com]
```

Removed element IDs are prefixed with `-`, added/changed elements with `+`.
