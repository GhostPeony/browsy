# Page Types Reference

browsy classifies every page into a `PageType` to help agents decide what to do next. The classification is based on structural heuristics applied to the Spatial DOM -- no machine learning, no external services.

Page types are evaluated in priority order. The first match wins.

## PageType enum

```rust
pub enum PageType {
    Error,
    Captcha,
    Login,
    TwoFactorAuth,
    OAuthConsent,
    Inbox,
    EmailBody,
    Dashboard,
    Article,
    SearchResults,
    List,
    Search,
    Form,
    Other,          // default
}
```

## Detection criteria

| Page Type | Detection Criteria |
|-----------|-------------------|
| **Error** | Title contains HTTP error codes (`404`, `500`, `403`, `not found`, `error`) OR page has elements with `alert_type == "error"`. |
| **Captcha** | Title contains CAPTCHA keywords (`captcha`, `verify you're human`, `robot`, `security check`, `just a moment`, `attention required`) OR heading contains CAPTCHA phrases OR a CAPTCHA service (reCAPTCHA, hCaptcha, Turnstile, Cloudflare challenge) is detected in the HTML structure. |
| **Login** | Page has a visible `<input type="password">`. |
| **TwoFactorAuth** | Title or heading contains verification keywords (`verification`, `enter code`, `security code`, `2fa`, `two-factor`, `otp`, `one-time`, `passcode`) AND page has a visible text/number/tel input. No password field present (that would be Login). |
| **OAuthConsent** | Title or heading contains OAuth keywords (`authorize`, `allow access`, `grant permission`, `oauth`, `consent`). |
| **Inbox** | Title contains inbox keywords (`inbox`, `mail`, `messages`) AND page has 10+ visible links. |
| **EmailBody** | Page text contains 3+ of the email markers: `from:`, `to:`, `subject:`, `date:`. |
| **Dashboard** | Title or heading contains dashboard keywords (`dashboard`, `welcome back`, `overview`) AND page has both a `<nav>` and `<main>` landmark. |
| **Article** | Page has 3+ headings AND enough long paragraphs (>100 chars). When the page has 20+ links, the threshold is 10 long paragraphs (vs 2 for low-link pages). Pages with 15+ headings must have a paragraph-to-heading ratio of at least 0.8 to distinguish articles (Wikipedia) from heading-heavy list pages (BBC News). |
| **SearchResults** | Page has a search input (visible or hidden) AND 8+ links AND search context: title/heading contains search-result keywords (`search results`, `results for`, `search`) OR URL contains search query parameters (`?q=`, `?query=`, `?s=`, `?search=`, `/search`). |
| **List** | Page has 10+ visible links. Evaluated after Article and SearchResults. |
| **Search** | Page has a visible search input. Evaluated after List (many list pages have search bars in navigation). Also fires as a fallback when a page has fewer than 5 visible elements but has a hidden search input (common in JS-rendered search engines without JS execution). |
| **Form** | Page has 2+ visible data-entry inputs (excludes checkbox, radio, hidden, submit, button, and image inputs). |
| **Other** | Default when no heuristic matches. |

## Evaluation order

The order matters. For example:

- A login page with a search bar in the nav is classified as `Login` (password field check comes first), not `Search`.
- A search results page with many links is `SearchResults`, not `List`, because SearchResults is checked before List.
- An article with a search bar is `Article`, not `Search`, because Article is checked first.
- An error page with a login form is `Error`, because error checks come before Login.

## Accessing page type

### Rust

```rust
use browsy_core::output::PageType;

let dom = browsy_core::parse(html, 1920.0, 1080.0);
match dom.page_type {
    PageType::Login => println!("This is a login page"),
    PageType::Article => println!("This is an article"),
    _ => println!("Page type: {:?}", dom.page_type),
}
```

### Python

```python
page = browser.goto("https://example.com")
print(page.page_type())  # "Login", "Article", "Other", etc.
```

### MCP

The `page_info` tool returns `page_type` as a string. The `browse` tool includes it in the JSON output format.

## JSON serialization

`PageType` is serialized as a string. The field is omitted from JSON when the value is `Other` (via `skip_serializing_if`).

```json
{
  "page_type": "Login",
  "title": "Sign In",
  "url": "https://example.com/login"
}
```
