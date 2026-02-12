# Spatial DOM

The Spatial DOM is the primary output of browsy. It converts an HTML document into a flat list of `SpatialElement` structs -- each representing an interactive element, text block, or structural landmark -- with bounding boxes, ARIA roles, and form state. No tree traversal, no pixel rendering.

```rust
use browsy_core::parse;

let dom = parse(html, 1920.0, 1080.0);
// dom.els: Vec<SpatialElement> -- flat, ordered, ready for agent consumption
```

## SpatialElement fields

Every element in the Spatial DOM is a `SpatialElement` with these fields:

| Field | Type | Description |
|---|---|---|
| `id` | `u32` | Stable numeric ID, assigned sequentially. Used for all interactions (`click`, `type_text`, etc.) |
| `tag` | `String` | HTML tag name (`a`, `button`, `input`, `p`, `h1`, etc.) |
| `role` | `Option<String>` | ARIA role -- explicit from `role` attr or implicit from tag. `link`, `button`, `textbox`, `heading`, `navigation`, etc. |
| `text` | `Option<String>` | Visible text content. For images, this is the `alt` text |
| `href` | `Option<String>` | Link destination (resolved to absolute URL when parsed via Session) |
| `b` | `[i32; 4]` | Bounding box: `[x, y, width, height]` in pixels relative to the document |
| `hidden` | `Option<bool>` | `Some(true)` if the element is hidden. Absent (`None`) when visible |
| `name` | `Option<String>` | HTML `name` attribute (form fields only: `input`, `textarea`, `select`) |
| `val` | `Option<String>` | Current value from the HTML `value` attribute |
| `ph` | `Option<String>` | Placeholder text |
| `label` | `Option<String>` | Associated `<label>` text (resolved via `<label for="id">`) |
| `input_type` | `Option<String>` | Input type (`text`, `password`, `email`, `checkbox`, `radio`, `search`, etc.). Serializes as `type` in JSON |
| `checked` | `Option<bool>` | Whether a checkbox/radio is checked |
| `disabled` | `Option<bool>` | Whether the element is disabled |
| `expanded` | `Option<bool>` | ARIA expanded state (dropdowns, accordions) |
| `selected` | `Option<bool>` | ARIA selected state (tabs, options) |
| `required` | `Option<bool>` | Whether the field is required |
| `alert_type` | `Option<String>` | Alert classification: `"alert"`, `"status"`, `"error"`, `"success"`, `"warning"` |

All `Option` fields use `skip_serializing_if` -- absent fields are omitted from JSON output to keep payloads compact.

## Hidden content exposure

Elements with `display: none`, `visibility: hidden`, `aria-hidden="true"`, or the `hidden` attribute are **not discarded**. They appear in the Spatial DOM with `hidden: Some(true)`.

This is a deliberate design decision. Without JavaScript execution, browsy cannot toggle visibility. By including hidden elements, agents can see:

- **Dropdown menus** -- `<ul>` inside a nav that only appears on hover
- **Modal dialogs** -- login forms, cookie consent, popups
- **Accordion panels** -- FAQ content behind collapsed sections
- **Tab content** -- inactive tab panels
- **Off-canvas navigation** -- mobile menus hidden at desktop widths

```rust
// All elements including hidden
let all = &dom.els;

// Only visible elements
let visible = dom.visible();

// Hidden elements are distinguishable
for el in &dom.els {
    if el.hidden == Some(true) {
        // This element is hidden in the rendered page
    }
}
```

Hidden elements always have a zero-size exemption -- they are preserved regardless of bounding box dimensions. Visible elements with zero width and height are skipped as layout artifacts.

## Deduplication

HTML commonly wraps interactive elements in container tags that carry no additional meaning:

```html
<li><a href="/about">About</a></li>
<td><span><button>Submit</button></span></td>
```

browsy collapses these wrappers. When a wrapper tag (`li`, `td`, `th`, `span`, `p`, `dt`, `dd`) contains only interactive children and no meaningful text of its own, the wrapper is skipped. Only the inner interactive element is emitted.

This produces a 34-42% element reduction on real sites without losing any semantic content.

## Landmark markers

HTML5 landmark elements (`nav`, `header`, `footer`, `main`, `aside`, `section`, `form`) and elements with explicit landmark ARIA roles (`navigation`, `banner`, `contentinfo`, `complementary`, `region`, `main`, `form`) emit as **role-only structural markers**.

A landmark element appears in the output with its role but **no recursive text**. Its children carry the actual content as separate elements. This prevents the entire navigation bar's text from being duplicated into a single massive `nav` element.

```json
{"id": 1, "tag": "nav", "role": "navigation", "b": [0, 0, 1920, 60]},
{"id": 2, "tag": "a", "role": "link", "text": "Home", "href": "/", "b": [20, 10, 80, 40]},
{"id": 3, "tag": "a", "role": "link", "text": "About", "href": "/about", "b": [120, 10, 80, 40]}
```

## Element lookup

The `SpatialDom` maintains an internal `HashMap<u32, usize>` index for O(1) element lookup by ID:

```rust
// O(1) -- does not scan the element list
let element = dom.get(42);
```

The index is built automatically during parsing and can be rebuilt after mutation:

```rust
dom.els.push(new_element);
dom.rebuild_index();
```

## Filtering

```rust
// Only visible (non-hidden) elements
let visible: Vec<&SpatialElement> = dom.visible();

// Elements whose top edge is within the viewport
let above: Vec<&SpatialElement> = dom.above_fold();

// Elements whose top edge is below the viewport
let below: Vec<&SpatialElement> = dom.below_fold();

// New SpatialDom containing only above-fold elements (for token-limited contexts)
let trimmed: SpatialDom = dom.filter_above_fold();
```

The fold line is determined by `dom.vp[1]` (viewport height, default 1080px).

## Tables

`dom.tables()` extracts structured table data by grouping `th` and `td` elements by their Y coordinates:

```rust
let tables: Vec<TableData> = dom.tables();
for table in &tables {
    println!("Headers: {:?}", table.headers);   // Vec<String>
    for row in &table.rows {
        println!("Row: {:?}", row);              // Vec<String>
    }
}
```

Elements within 5px of the same Y coordinate are grouped into the same row. Cells are sorted left-to-right by X position within each row.

## Alerts

`dom.alerts()` returns elements with a detected `alert_type`:

```rust
let alerts: Vec<&SpatialElement> = dom.alerts();
for alert in &alerts {
    println!("{}: {}", alert.alert_type.as_deref().unwrap(), alert.text.as_deref().unwrap_or(""));
    // "error: Invalid password"
    // "success: Account created"
}
```

Alert types are detected from ARIA `role` attributes (`alert`, `status`) and CSS class patterns (`alert-error`, `msg-danger`, `flash-success`, etc.). Only compound class patterns are matched -- a bare `error` class is too ambiguous.

## Verification codes

`dom.find_codes()` extracts 4-8 digit verification codes from page text:

```rust
let codes: Vec<String> = dom.find_codes();
// ["847291"] -- extracted from "Your verification code is 847291"
```

Codes are found near keyword context (`verification code`, `security code`, `your code`, `otp`, `passcode`, `one-time`). Year-like 4-digit numbers (1900-2099) are filtered out. Proximity matching also checks nearby elements within 100px Y distance for keyword context.

## Text fallback chain

For interactive elements (links, buttons) that contain no direct text -- only images or icons -- browsy walks a fallback chain to find meaningful text:

1. `aria-label` attribute
2. `title` attribute
3. Child `<img>` `alt` text
4. Child `<svg>` `<title>` text

This ensures that icon-only buttons and image links always have text for the agent to read.
