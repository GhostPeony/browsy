# browsy

Zero-render browser engine for AI agents. Converts HTML into a Spatial DOM (flat list of interactive/text elements with bounding boxes) without rendering pixels.

## What's ours vs external

**External foundations:**
- **html5ever** (Mozilla/Servo) — HTML parsing only. We feed it HTML, it gives us a DOM tree.
- **Taffy** (Dioxus) — Flexbox + CSS Grid layout computation only. We build the style tree, it computes bounding boxes.

**Built from scratch in browsy:**
- CSS engine: selector matching, property parsing, `var()` resolution, `calc()`, `@media` queries, specificity, inheritance
- Spatial DOM output: element emission, deduplication, landmarks, text fallbacks, hidden content exposure, delta diffing
- Session API: navigation, form interaction, cookie jar, search (DuckDuckGo + Google)
- Behavior detection: onclick/ARIA/Bootstrap pattern inference

## Project structure

```
crates/
  core/           browsy-core library (the main crate)
    src/
      lib.rs        Entry point: parse(html, w, h) -> SpatialDom
      dom/mod.rs    HTML → DomNode tree (thin wrapper around html5ever)
      css/
        mod.rs        Style computation, CSS variable inheritance (ours)
        selector.rs   CSS selector matching engine (ours)
        properties.rs CSS property parsing, var() resolution, calc() (ours)
      layout/mod.rs Style tree → Taffy → bounding boxes (ours + Taffy)
      output/mod.rs SpatialDom generation, dedup, landmarks, text fallbacks (ours)
      js/mod.rs     Behavior detection from HTML attributes (ours)
      fetch/
        mod.rs        HTTP fetching, form extraction, resource blocking (ours)
        session.rs    Session API, search, navigation (ours)
  cli/            browsy CLI binary
```

## Build and test

```bash
cargo test -p browsy-core         # run all tests
cargo test -p browsy-core --test css_layout  # just CSS tests
cargo test -p browsy-core --test output      # just output tests
cargo test -p browsy-core --test benchmark -- --nocapture  # detection benchmark
cargo build -p browsy-core        # build library only
cargo build                       # build everything (library + CLI)
```

The `fetch` feature (enabled by default) adds HTTP fetching via reqwest. Disable it for a pure HTML parser with no network dependencies.

## Key design decisions

- **Hidden content exposure**: Elements with `display:none`, `visibility:hidden`, `aria-hidden`, or `hidden` attribute are NOT discarded. They're included in the output with `hidden: true`. This lets agents see dropdown menus, accordion panels, modals, and tab content without JS execution.

- **Landmark markers**: `<nav>`, `<header>`, `<footer>`, `<main>`, `<aside>`, `<section>`, `<form>` (and elements with explicit landmark ARIA roles) emit as structural markers with role only, no recursive text. Children carry the actual content.

- **Text fallback chain**: For text-less links/buttons (containing only images or icons), text is extracted from: aria-label -> title attr -> child img alt -> child SVG title.

- **SVG handling**: SVG children are discarded (not semantic content), but `<title>` text is extracted and stored as `aria-label` on the SVG node for accessibility.

- **CSS variables**: Custom properties (`--var: value`) are collected during style computation and resolve via `var(--name, fallback)`. They inherit through the DOM tree.

- **Deduplication**: Wrapper elements (`<li><a>`, `<td><span>`) that only wrap interactive children are skipped — only the meaningful child element is emitted.

- **Zero-size skip**: Visible elements with zero width and height are skipped (layout artifacts), but hidden elements are always preserved regardless of size.

## Architecture

```
HTML -> dom::parse_html (html5ever) -> DomNode tree
  -> css::compute_styles (browsy) -> StyledNode tree (with CSS variables)
  -> layout::compute_layout (browsy + Taffy) -> LayoutNode tree (with bounding boxes)
  -> output::generate_spatial_dom (browsy) -> SpatialDom (flat element list)
```

## Coding conventions

- Rust 2021 edition
- No `unwrap()` in library code paths that handle user input; `unwrap()` is acceptable in tests and for internal invariants
- Tests are in `crates/core/tests/` as integration tests, not unit tests
- Feature-gated code uses `#[cfg(feature = "fetch")]`
- `pub(crate)` for internal helpers that cross module boundaries
- Serde skip_serializing_if for optional fields to keep JSON compact

## Detection benchmark corpus

The `crates/core/tests/corpus/` directory contains HTML snapshots with ground truth labels for page detection accuracy. The benchmark runner (`benchmark.rs`) parses all snapshots and checks page type, suggested actions, verification codes, and action ID validity.

**Adding a new problematic site:**
```bash
# 1. Harvest the HTML snapshot
HARVEST_URL="https://broken-site.com" HARVEST_NAME="broken-site" \
  cargo test -p browsy-core --test harvest harvest_single -- --ignored --nocapture

# 2. Copy the printed manifest entry into corpus/manifest.json
#    Change page_type to the CORRECT expected value

# 3. Run benchmark — confirms the failure
cargo test -p browsy-core --test benchmark -- --nocapture

# 4. Fix heuristics in output/mod.rs

# 5. Re-run benchmark — confirms fix + no regressions
cargo test -p browsy-core --test benchmark -- --nocapture
```
