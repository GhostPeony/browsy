# Quickstart

This guide covers the core browsy-core workflow: parse HTML, fetch live pages, read page intelligence, and interact with forms.

## 1. Install

```bash
cargo add browsy-core
```

This pulls in the `fetch` feature by default, which includes HTTP fetching via reqwest. See [Installation](./installation.md) for other installation methods.

## 2. Parse HTML

The simplest entry point is `browsy_core::parse`. Pass it an HTML string and a viewport size, and it returns a `SpatialDom` -- a flat list of elements with bounding boxes, roles, and states.

```rust
let html = r#"
<html>
  <body>
    <h1>Hello, world</h1>
    <a href="/about">About</a>
    <input type="text" placeholder="Search..." />
  </body>
</html>
"#;

let dom = browsy_core::parse(html, 1920.0, 1080.0);

// Iterate over elements
for el in &dom.els {
    println!("[{}:{} {:?}]", el.id, el.tag, el.text);
}
```

The viewport dimensions (1920x1080 here) affect layout computation -- elements get positioned and sized as they would in a real browser at that resolution.

`SpatialDom` serializes to JSON via serde:

```rust
let json = serde_json::to_string_pretty(&dom).unwrap();
println!("{}", json);
```

## 3. Fetch and parse a live page

The `Session` API handles HTTP fetching, cookie persistence, and page interaction. It requires the `fetch` feature (enabled by default).

```rust
use browsy_core::fetch::Session;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut session = Session::new()?;
    let dom = session.goto("https://example.com")?;

    println!("Title: {}", dom.title);
    println!("Elements: {}", dom.els.len());

    // Elements are accessible by ID
    if let Some(el) = dom.get(1) {
        println!("First element: {} {:?}", el.tag, el.text);
    }

    // Filter to visible-only or above-the-fold
    let visible = dom.visible();
    let above_fold = dom.above_fold();

    Ok(())
}
```

Sessions persist cookies across navigations. Each call to `goto` returns a fresh `SpatialDom` for the new page.

## 4. Read page intelligence

Every `SpatialDom` includes two forms of page intelligence: a detected **page type** and a list of **suggested actions** with stable element IDs.

```rust
use browsy_core::fetch::Session;
use browsy_core::output::{PageType, SuggestedAction};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut session = Session::new()?;
    let dom = session.goto("https://github.com/login")?;

    // Page type: Login, Search, Article, Form, List, Dashboard, etc.
    println!("Page type: {:?}", dom.page_type);

    // Suggested actions tell the agent exactly what to do
    for action in &dom.suggested_actions {
        match action {
            SuggestedAction::Login { username_id, password_id, submit_id, .. } => {
                println!("Login form found:");
                println!("  Username field: element {}", username_id);
                println!("  Password field: element {}", password_id);
                println!("  Submit button: element {}", submit_id);
            }
            SuggestedAction::Search { input_id, submit_id } => {
                println!("Search: input={}, submit={}", input_id, submit_id);
            }
            SuggestedAction::EnterCode { input_id, submit_id, code_length } => {
                println!("2FA code: input={}, submit={}, length={:?}",
                    input_id, submit_id, code_length);
            }
            _ => println!("Action: {:?}", action),
        }
    }

    Ok(())
}
```

### Page types

browsy detects 12 page types automatically:

| PageType | Meaning |
|---|---|
| `Login` | Login form with username/password fields |
| `TwoFactorAuth` | Verification code entry (2FA, email confirmation) |
| `OAuthConsent` | OAuth authorization prompt |
| `Captcha` | CAPTCHA challenge page |
| `Search` | Search page (empty query state) |
| `SearchResults` | Search results page |
| `Inbox` | Email or message inbox |
| `EmailBody` | Single email or message view |
| `Dashboard` | Dashboard or admin panel |
| `Form` | Generic form (registration, contact, settings) |
| `Article` | Article, blog post, documentation page |
| `List` | List or catalog page (products, directory) |
| `Error` | Error page (404, 500, access denied) |
| `Other` | No specific type detected |

### CAPTCHA detection

When browsy detects a CAPTCHA, it sets `page_type` to `Captcha` and populates `captcha` with details:

```rust
if dom.page_type == PageType::Captcha {
    if let Some(captcha) = &dom.captcha {
        println!("CAPTCHA type: {:?}", captcha.captcha_type);
        // ReCaptcha, HCaptcha, Turnstile, CloudflareChallenge, ImageGrid, TextCaptcha
        if let Some(sitekey) = &captcha.sitekey {
            println!("Site key: {}", sitekey);
        }
    }
}
```

Or use the session convenience methods:

```rust
if session.is_captcha() {
    println!("CAPTCHA: {:?}", session.captcha_info());
}
```

## 5. Log in to a site

browsy provides two ways to interact with login forms: manual (using element IDs) and automatic (using `session.login`).

### Manual login

Use the element IDs from `SuggestedAction::Login` to type credentials and submit:

```rust
use browsy_core::fetch::Session;
use browsy_core::output::SuggestedAction;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut session = Session::new()?;
    let dom = session.goto("https://github.com/login")?;

    // Find the login action
    for action in &dom.suggested_actions {
        if let SuggestedAction::Login { username_id, password_id, submit_id, .. } = action {
            session.type_text(*username_id, "user@example.com")?;
            session.type_text(*password_id, "my-password")?;
            let result = session.click(*submit_id)?;
            println!("After login: {:?}", result.page_type);
            break;
        }
    }

    Ok(())
}
```

### Automatic login

`session.login` detects the login form from `suggested_actions` and fills it in one call:

```rust
let mut session = Session::new()?;
session.goto("https://github.com/login")?;

let result = session.login("user@example.com", "my-password")?;
println!("After login: {:?}", result.page_type);
```

This fails with `FetchError::ActionError` if no `SuggestedAction::Login` is detected on the current page.

### 2FA / verification codes

If the login redirects to a 2FA page, use `enter_code`:

```rust
if result.page_type == PageType::TwoFactorAuth {
    let final_page = session.enter_code("123456")?;
    println!("After 2FA: {:?}", final_page.page_type);
}
```

## 6. Search the web

browsy has built-in web search via DuckDuckGo and Google. No API keys required.

### Get search results

```rust
use browsy_core::fetch::{Session, SearchEngine};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut session = Session::new()?;

    // DuckDuckGo (default)
    let results = session.search("rust web frameworks")?;
    for r in &results {
        println!("{}: {}", r.title, r.url);
        println!("  {}", r.snippet);
    }

    // Google
    let results = session.search_with("rust web frameworks", SearchEngine::Google)?;

    Ok(())
}
```

### Search and read pages

`search_and_read` fetches the top N results and returns each page's `SpatialDom`:

```rust
let pages = session.search_and_read("browsy browser engine", 3)?;

for page in &pages {
    println!("--- {} ---", page.result.title);
    if let Some(dom) = &page.dom {
        println!("  Page type: {:?}", dom.page_type);
        println!("  Elements: {}", dom.els.len());
    } else {
        println!("  (fetch failed)");
    }
}
```

## Next steps

- [Spatial DOM](./spatial-dom.md) -- understand the output format in detail
- [Page Intelligence](./page-intelligence.md) -- all 13 action recipes explained
- [Session API](./session-api.md) -- full reference for navigation, forms, and interaction
- [MCP Server](./mcp-server.md) -- use browsy from Claude Code
