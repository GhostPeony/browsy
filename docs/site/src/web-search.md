# Web Search

browsy includes built-in web search via DuckDuckGo and Google. No API keys or external services required -- it fetches search result pages directly and parses the HTML.

## Search engines

| Engine | Endpoint | Reliability |
|--------|----------|-------------|
| DuckDuckGo | `https://html.duckduckgo.com/html/` | High. Uses the HTML-only endpoint, no JavaScript needed. |
| Google | `https://www.google.com/search` | Variable. Google may return CAPTCHAs or block automated requests. |

DuckDuckGo is the default and recommended engine.

## Rust API

### Basic search

```rust
use browsy_core::fetch::{Session, SearchEngine};

let mut session = Session::new()?;
let results = session.search("rust web scraping")?;

for r in &results {
    println!("{}: {} -- {}", r.title, r.url, r.snippet);
}
```

### Choosing a search engine

```rust
let results = session.search_with("rust web scraping", SearchEngine::Google)?;
```

### Search and read

Search and automatically fetch the top N result pages:

```rust
let pages = session.search_and_read("rust web scraping", 3)?;

for page in &pages {
    println!("--- {} ---", page.result.title);
    if let Some(ref dom) = page.dom {
        println!("  Page type: {:?}", dom.page_type);
        println!("  Elements: {}", dom.els.len());
    } else {
        println!("  (fetch failed)");
    }
}
```

Each `SearchPage` contains the original `SearchResult` (title, URL, snippet) and an `Option<SpatialDom>` for the fetched page. Pages that fail to fetch have `dom: None`.

```rust
let pages = session.search_and_read_with(
    "rust web scraping",
    5,
    SearchEngine::DuckDuckGo,
)?;
```

## Python API

```python
from browsy import Browser

browser = Browser()

# Basic search (DuckDuckGo)
results = browser.search("python asyncio tutorial")
for r in results:
    print(r["title"], r["url"])
```

Search results are returned as a list of dictionaries, each with `title`, `url`, and `snippet` keys.

## MCP API

The `search` tool accepts a query and optional engine:

```json
{
  "query": "browsy zero-render browser",
  "engine": "duckduckgo"
}
```

Returns a JSON array of results:

```json
[
  {
    "title": "browsy - Zero-render browser engine",
    "url": "https://example.com/browsy",
    "snippet": "A browser engine for AI agents..."
  }
]
```

## SearchResult struct

```rust
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
}
```

## How it works

### DuckDuckGo

browsy fetches `https://html.duckduckgo.com/html/?q=<query>`, which returns a pure HTML page with no JavaScript. Results are extracted by finding `<div class="result">` containers and parsing the title link (`result__a`), URL (`result__url`), and snippet (`result__snippet`). Redirect URLs are decoded from the `uddg` query parameter.

### Google

browsy fetches `https://www.google.com/search?q=<query>&num=10`. Results are extracted using a structural pattern: anchor tags containing an `<h3>` descendant. The title comes from the h3 text, the URL from the anchor href (with `/url?q=` redirect decoding), and snippets from nearby div elements. The parser targets the `#rso` results container to skip ads and navigation.

Google results may be less reliable because Google actively detects and blocks automated requests. DuckDuckGo's HTML endpoint is specifically designed for non-JavaScript clients and is the recommended default.
