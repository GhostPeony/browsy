# CSS Engine

browsy includes a CSS engine built from scratch in Rust. It handles selector matching, property parsing, variable resolution, `calc()` expressions, `@media` queries, and specificity ordering. The engine computes the subset of CSS properties needed for layout -- approximately 40 properties that affect bounding box computation.

## Architecture

```
HTML ──> DomNode tree
          │
          ├── <style> blocks ──> parse_stylesheet() ──> Vec<CssRule>
          ├── External <link> CSS ──> fetched + parse_stylesheet()
          ├── Inline style="" ──> parse_inline_style_with_vars()
          │
          └── compute_styles() ──> StyledNode tree (LayoutStyle per node)
                │
                └── Taffy layout ──> bounding boxes
```

Style computation walks the DOM tree, matching each element against all CSS rules by specificity. Inline styles override stylesheet rules. CSS custom properties (`--var`) inherit through the tree.

## Selector matching

The selector engine supports these selector types:

| Selector | Example | Description |
|----------|---------|-------------|
| Tag | `div`, `button` | Matches element tag name |
| Class | `.nav-item` | Matches class attribute |
| ID | `#header` | Matches id attribute |
| Universal | `*` | Matches any element |
| Descendant | `div p` | Matches `p` inside any `div` ancestor |
| Child | `div > p` | Matches `p` that is a direct child of `div` |
| Pseudo-class | `:hover`, `:first-child` | Parsed but ignored for layout (no interaction state) |
| Attribute (exists) | `[disabled]` | Element has the attribute |
| Attribute (exact) | `[type="submit"]` | Attribute equals value |
| Attribute (word) | `[class~="active"]` | Whitespace-separated word match |
| Attribute (prefix) | `[href^="/"]` | Attribute starts with value |
| Attribute (suffix) | `[src$=".png"]` | Attribute ends with value |
| Attribute (contains) | `[class*="btn"]` | Attribute contains substring |
| Attribute (hyphen-prefix) | `[lang\|="en"]` | Exact match or prefix with hyphen |
| Comma-separated | `h1, h2, h3` | Union of selectors |

### Specificity

Selectors are ordered by CSS specificity rules:

- ID selectors: weight 100
- Class selectors, attribute selectors, pseudo-classes: weight 10
- Tag selectors, universal: weight 1

Higher specificity rules override lower specificity rules. Equal specificity resolves by source order (later wins). Inline styles always win over stylesheet rules.

## Property parsing

### Supported properties

The engine parses approximately 40 layout-affecting CSS properties:

| Category | Properties |
|----------|-----------|
| **Box model** | `display`, `box-sizing`, `width`, `height`, `min-width`, `min-height`, `max-width`, `max-height` |
| **Spacing** | `margin` (+ sides), `padding` (+ sides), `border-width` (+ sides) |
| **Position** | `position`, `top`, `right`, `bottom`, `left` |
| **Flexbox** | `flex-direction`, `flex-wrap`, `flex-grow`, `flex-shrink`, `flex-basis`, `align-items`, `align-self`, `justify-content`, `gap` |
| **Grid** | `grid-template-columns`, `grid-template-rows`, `grid-column`, `grid-row` |
| **Typography** | `font-size`, `line-height` |
| **Visibility** | `visibility`, `overflow` |

Shorthand properties are expanded: `margin: 10px 20px` expands to `margin-top`, `margin-right`, `margin-bottom`, `margin-left`. Similarly for `padding`, `border-width`, `flex`, and `gap`.

### Dimension types

```rust
pub enum Dimension {
    Px(f32),           // Absolute pixels
    Percent(f32),      // Percentage of parent
    Calc(f32, f32),    // calc() result: (px_component, percent_component)
    Auto,              // Auto sizing
}
```

The engine resolves `em` values against the element's computed `font-size` and `rem` values against the root font size (16px default).

### var() resolution

CSS custom properties are collected during style computation and inherited through the DOM tree:

```css
:root {
  --primary-color: #333;
  --spacing: 16px;
}

.container {
  padding: var(--spacing);
  color: var(--primary-color);
}

.card {
  margin: var(--spacing-large, 24px);  /* fallback value */
}
```

The `var()` resolver supports:
- Simple references: `var(--name)`
- Fallback values: `var(--name, fallback)`
- Nested var() references in fallbacks

### calc() expressions

The `calc()` parser handles full arithmetic expressions with mixed units:

```css
.element {
  width: calc(100% - 32px);
  margin: calc(16px + 1em);
  padding: calc(2 * var(--spacing));
}
```

Supported operators: `+`, `-`, `*`, `/`. The parser respects operator precedence and handles parenthesized sub-expressions. Mixed `px` and `%` units are preserved as a `Calc(px, percent)` dimension and resolved during layout.

## @media queries

The engine evaluates `@media` queries against the viewport dimensions provided at parse time:

```css
@media (max-width: 768px) {
  .sidebar { display: none; }
}

@media screen and (min-width: 1024px) {
  .container { max-width: 1200px; }
}
```

### Supported media features

| Feature | Example | Description |
|---------|---------|-------------|
| `min-width` | `(min-width: 768px)` | Viewport width >= value |
| `max-width` | `(max-width: 1024px)` | Viewport width <= value |
| `min-height` | `(min-height: 600px)` | Viewport height >= value |
| `max-height` | `(max-height: 900px)` | Viewport height <= value |
| `width` | `(width: 1920px)` | Exact viewport width |
| `height` | `(height: 1080px)` | Exact viewport height |
| `orientation` | `(orientation: portrait)` | Portrait or landscape |
| `screen` | `screen` | Always matches |
| `print` | `print` | Never matches |
| `all` | `all` | Always matches |

Multiple conditions joined with `and` are evaluated conjunctively. The `screen and` / `all and` prefix is stripped before evaluating conditions.

## External stylesheets

When using the `fetch` feature (enabled by default), browsy automatically fetches external CSS linked via `<link rel="stylesheet">` tags. Fetched CSS is parsed and merged with inline `<style>` blocks during style computation.

Resource limits prevent abuse:
- Maximum total CSS bytes (across all external stylesheets)
- Maximum bytes per individual stylesheet
- Blocked URL patterns (analytics, tracking, ad-related CSS)
- Private network and non-HTTP URL blocking

## Layout engine

After style computation, browsy feeds the styled tree into Taffy (from the Dioxus project) for layout computation. Taffy handles:

- **Flexbox**: All flex container and flex item properties
- **CSS Grid**: Template columns/rows, explicit placement
- **Block layout**: Standard block flow with margins, padding, borders

Taffy returns bounding boxes (x, y, width, height) for every element, which browsy uses to build the Spatial DOM.

## What is NOT supported

The CSS engine focuses on properties that affect element position and size. The following are intentionally not implemented:

- **Visual properties**: `color`, `background`, `border-color`, `border-radius`, `box-shadow`, `opacity`, `z-index`
- **Transforms**: `transform`, `translate`, `rotate`, `scale`
- **Animations**: `animation`, `transition`, `@keyframes`
- **Pseudo-elements**: `::before`, `::after`, `::placeholder` (no content generation)
- **Advanced selectors**: `:nth-child()`, `:not()`, `~` (general sibling), `+` (adjacent sibling)
- **Advanced grid**: `grid-auto-flow`, `grid-auto-rows`, named grid areas, `minmax()` in some contexts
- **Columns**: `column-count`, `column-width`
- **Table layout**: `table-layout`, `border-collapse`

These omissions are by design. browsy computes where elements are and how large they are, not what they look like. The Spatial DOM output contains position and size data; color and visual styling are irrelevant for agent interaction.
