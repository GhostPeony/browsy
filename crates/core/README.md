# browsy-core

Zero-render browser engine for AI agents. Converts HTML into a **Spatial DOM** — a flat list of interactive elements with bounding boxes, roles, and states — without rendering pixels.

## Usage

```rust
use browsy_core::fetch::{Session, SearchEngine};

let mut session = Session::new()?;

// Browse
session.goto("https://example.com")?;
let dom = session.dom().unwrap();

for el in dom.visible() {
    if el.role.as_deref() == Some("link") {
        println!("{}: {}", el.text.as_deref().unwrap_or(""), el.href.as_deref().unwrap_or(""));
    }
}

// Search
let results = session.search("rust web frameworks")?;
for r in &results {
    println!("{} -> {}", r.title, r.url);
}
```

## Without networking

Disable the `fetch` feature for a pure HTML-to-Spatial-DOM parser:

```toml
[dependencies]
browsy-core = { version = "0.1", default-features = false }
```

```rust
let dom = browsy_core::parse(html, 1920.0, 1080.0);
```

## Features

- **Page intelligence** — 12 page types detected automatically, 13 action recipes with element IDs
- **CAPTCHA detection** — reCAPTCHA, hCaptcha, Cloudflare Turnstile, image grids
- **Hidden content exposure** — dropdowns, modals, accordions included with `hidden: true`
- **Session API** — navigate, click, type, select, search with cookie persistence
- **Built-in web search** — DuckDuckGo and Google
- **Smart deduplication** — 34-42% element reduction on real sites
- **CSS engine** — flexbox, grid, variables, calc(), @media queries
- **6MB binary** — zero runtime dependencies

## Documentation

- [Full docs](https://ghostpeony.github.io/browsy/)
- [browsy.dev](https://browsy.dev)

## License

MIT
