# Session API

The Session API provides stateful web browsing with cookie persistence, form interaction, navigation history, and built-in web search. It is the primary interface for agents interacting with the web through browsy.

```rust
use browsy_core::fetch::Session;

let mut session = Session::new()?;
let dom = session.goto("https://example.com")?;
```

Requires the `fetch` feature (enabled by default).

## Creating a session

### `Session::new()`

Creates a session with default configuration (1920x1080 viewport, 30s timeout, CSS fetching enabled).

```rust
let mut session = Session::new()?;
```

### `Session::with_config(config)`

Creates a session with custom configuration.

```rust
use browsy_core::fetch::{Session, SessionConfig};

let config = SessionConfig {
    viewport_width: 1366.0,
    viewport_height: 768.0,
    timeout_secs: 15,
    fetch_css: false,  // Skip external CSS for speed
    ..Default::default()
};
let mut session = Session::with_config(config)?;
```

## SessionConfig fields

| Field | Type | Default | Description |
|---|---|---|---|
| `viewport_width` | `f32` | `1920.0` | Viewport width in pixels. Affects layout computation and fold detection |
| `viewport_height` | `f32` | `1080.0` | Viewport height in pixels. Defines the fold line |
| `user_agent` | `String` | Chrome-like UA | HTTP User-Agent header |
| `timeout_secs` | `u64` | `30` | HTTP request timeout |
| `fetch_css` | `bool` | `true` | Whether to fetch external CSS stylesheets. Disabling speeds up parsing but reduces layout accuracy |
| `blocked_patterns` | `Vec<String>` | Analytics/tracking URLs | URL patterns to block (analytics, ads, tracking pixels) |
| `max_response_bytes` | `usize` | `5MB` | Maximum HTML response size |
| `max_css_bytes_total` | `usize` | `2MB` | Maximum total CSS bytes across all stylesheets |
| `max_css_bytes_per_file` | `usize` | `512KB` | Maximum size per individual CSS file |
| `max_redirects` | `usize` | `10` | Maximum HTTP redirect chain length |
| `allow_private_network` | `bool` | `false` | Whether to allow requests to private/internal IPs |
| `allow_non_http` | `bool` | `false` | Whether to allow non-HTTP(S) schemes |

## Navigation

### `goto(url) -> Result<SpatialDom, FetchError>`

Navigate to a URL. Fetches the page, parses HTML, optionally fetches external CSS, computes layout, and returns the Spatial DOM. Cookies are persisted automatically.

```rust
let dom = session.goto("https://news.ycombinator.com")?;
println!("Title: {}", dom.title);
println!("Elements: {}", dom.els.len());
```

### `back() -> Result<SpatialDom, FetchError>`

Navigate to the previous page in history. Returns an error if there is no history.

```rust
session.goto("https://example.com")?;
session.goto("https://example.com/about")?;
let dom = session.back()?;  // Back to example.com
```

### `url() -> Option<&str>`

Returns the current page URL.

```rust
if let Some(url) = session.url() {
    println!("Currently at: {}", url);
}
```

## Interaction

### `click(id) -> Result<SpatialDom, FetchError>`

Click an element by ID. Behavior depends on the element type:

- **Links (`<a>`)** -- navigates to the `href` URL. Skips `javascript:`, `mailto:`, `tel:`, and anchor-only (`#`) links.
- **Buttons / submit inputs** -- submits the parent form with all current form values.
- **Elements with JS behaviors** -- simulated. `onclick` handlers with `window.location` trigger navigation. Toggle/show/hide behaviors modify the DOM.

```rust
let dom = session.goto("https://news.ycombinator.com")?;
// Click the first link
let dom = session.click(3)?;
```

### `type_text(id, text) -> Result<(), FetchError>`

Type text into an input or textarea. The value is stored in the session and overlaid onto the DOM. When a form is submitted via `click`, these values are included in the form data.

```rust
session.type_text(19, "user@example.com")?;
session.type_text(21, "hunter2")?;
```

Returns an error if the element is not an `input` or `textarea`.

### `check(id) -> Result<(), FetchError>`

Check a checkbox or radio button.

```rust
session.check(36)?;  // Check "Remember me"
```

### `uncheck(id) -> Result<(), FetchError>`

Uncheck a checkbox or radio button.

```rust
session.uncheck(36)?;
```

### `toggle(id) -> Result<(), FetchError>`

Toggle a checkbox or radio button based on its current effective state (considering session overrides and HTML defaults).

```rust
session.toggle(36)?;  // If checked, unchecks. If unchecked, checks.
```

### `select(id, value) -> Result<(), FetchError>`

Select an option in a `<select>` element by value.

```rust
session.select(15, "california")?;
```

## Reading page state

### `dom() -> Option<SpatialDom>`

Returns the current Spatial DOM with form state overlaid. Typed values, checked/unchecked states from `type_text`, `check`, and `uncheck` are reflected in the returned DOM.

```rust
session.type_text(19, "hello")?;
let dom = session.dom().unwrap();
let el = dom.get(19).unwrap();
assert_eq!(el.val.as_deref(), Some("hello"));
```

### `dom_ref() -> Option<&SpatialDom>`

Returns a reference to the raw Spatial DOM without form state overlay. Reflects the page as parsed, ignoring any `type_text`/`check`/`uncheck` calls.

```rust
let raw = session.dom_ref().unwrap();
```

### `delta() -> Option<DeltaDom>`

Returns the diff between the current and previous page. Only available after at least two navigations.

```rust
session.goto("https://example.com")?;
session.goto("https://example.com/about")?;
if let Some(delta) = session.delta() {
    println!("Added/changed: {}", delta.changed.len());
    println!("Removed IDs: {:?}", delta.removed);
}
```

### `element(id) -> Option<&SpatialElement>`

O(1) element lookup by ID.

```rust
if let Some(el) = session.element(42) {
    println!("{}: {}", el.tag, el.text.as_deref().unwrap_or(""));
}
```

## Finding elements

### `find_by_text(text) -> Vec<&SpatialElement>`

Exact substring match on element text (case-sensitive).

```rust
let results = session.find_by_text("Sign in");
```

### `find_by_text_fuzzy(text) -> Vec<&SpatialElement>`

Case-insensitive substring match on element text.

```rust
let results = session.find_by_text_fuzzy("sign in");
// Matches "Sign In", "SIGN IN", "Please sign in", etc.
```

### `find_by_role(role) -> Vec<&SpatialElement>`

Find all elements with a specific ARIA role.

```rust
let headings = session.find_by_role("heading");
let links = session.find_by_role("link");
let buttons = session.find_by_role("button");
```

### `find_input_by_purpose(purpose) -> Option<&SpatialElement>`

Find an input element by its semantic purpose. Matches on input type, name, label, and placeholder.

```rust
use browsy_core::fetch::InputPurpose;

let password = session.find_input_by_purpose(InputPurpose::Password);
let email = session.find_input_by_purpose(InputPurpose::Email);
let username = session.find_input_by_purpose(InputPurpose::Username);
let code = session.find_input_by_purpose(InputPurpose::VerificationCode);
let search = session.find_input_by_purpose(InputPurpose::Search);
let phone = session.find_input_by_purpose(InputPurpose::Phone);
```

| Purpose | Matching logic |
|---|---|
| `Password` | `input[type="password"]` |
| `Email` | `input[type="email"]` or name/label contains `email` |
| `Username` | Text/email input with name/label containing `user` or `login` |
| `VerificationCode` | Text/number/tel input with name/label/placeholder containing `code`, `otp`, or `verify` |
| `Search` | `input[type="search"]`, role `searchbox`, or name containing `search` |
| `Phone` | `input[type="tel"]` or name/label containing `phone` |

### `find_nearest_button(input_id) -> Option<&SpatialElement>`

Find the nearest submit button to a given input element. Prefers buttons below the input, scored by Manhattan distance with Y weighted 2x.

```rust
if let Some(btn) = session.find_nearest_button(19) {
    println!("Submit button: {} (id: {})", btn.text.as_deref().unwrap_or(""), btn.id);
}
```

## Compound actions

These methods combine multiple interactions into a single call, using the page intelligence action recipes.

### `login(username, password) -> Result<SpatialDom, FetchError>`

Detects the login form from `suggested_actions`, fills in credentials, and submits. Returns the resulting page.

```rust
let dom = session.goto("https://github.com/login")?;
let result = session.login("user@example.com", "hunter2")?;
```

Returns an error if no `Login` action recipe was detected on the current page.

### `enter_code(code) -> Result<SpatialDom, FetchError>`

Fills in a verification code and submits the form, using the `EnterCode` action recipe.

```rust
let result = session.enter_code("847291")?;
```

### `find_verification_code() -> Option<String>`

Extracts a verification code from the current page text (4-8 digit sequences near code-related keywords).

```rust
// On a page that says "Your verification code is 847291"
if let Some(code) = session.find_verification_code() {
    session.enter_code(&code)?;
}
```

## CAPTCHA detection

### `is_captcha() -> bool`

Returns `true` if the current page is classified as a CAPTCHA challenge.

```rust
if session.is_captcha() {
    println!("CAPTCHA detected -- cannot proceed automatically");
}
```

### `captcha_info() -> Option<&CaptchaInfo>`

Returns CAPTCHA details if detected: `captcha_type` (ReCaptcha, HCaptcha, Turnstile, CloudflareChallenge, ImageGrid, TextCaptcha, Unknown) and optional `sitekey`.

```rust
if let Some(info) = session.captcha_info() {
    match info.captcha_type {
        CaptchaType::ReCaptcha => {
            println!("reCAPTCHA sitekey: {:?}", info.sitekey);
        }
        CaptchaType::CloudflareChallenge => {
            println!("Cloudflare challenge -- wait and retry");
        }
        _ => {}
    }
}
```

## Web search

### `search(query) -> Result<Vec<SearchResult>, FetchError>`

Search the web using DuckDuckGo. Returns structured results with title, URL, and snippet.

```rust
let results = session.search("rust programming language")?;
for r in &results {
    println!("{}: {} -- {}", r.title, r.url, r.snippet);
}
```

### `search_with(query, engine) -> Result<Vec<SearchResult>, FetchError>`

Search with a specific engine.

```rust
use browsy_core::fetch::SearchEngine;

let results = session.search_with("browsy", SearchEngine::Google)?;
```

Available engines: `SearchEngine::DuckDuckGo` (default, most reliable) and `SearchEngine::Google` (may return CAPTCHAs for automated requests).

### `search_and_read(query, n) -> Result<Vec<SearchPage>, FetchError>`

Search and fetch the top N results, returning each page's Spatial DOM alongside the search result metadata.

```rust
let pages = session.search_and_read("rust web scraping", 3)?;
for page in &pages {
    println!("{}:", page.result.title);
    if let Some(ref dom) = page.dom {
        println!("  {} elements, page_type: {:?}", dom.els.len(), dom.page_type);
    }
}
```

## Behaviors

### `behaviors() -> Vec<JsBehavior>`

Detects JavaScript behaviors from HTML attributes (onclick, data-toggle, data-bs-toggle, etc.). Returns trigger element IDs and inferred actions.

```rust
let behaviors = session.behaviors();
for b in &behaviors {
    println!("Element {} triggers {:?}", b.trigger_id, b.action);
}
```

## Error handling

All fallible methods return `Result<_, FetchError>`. Error variants:

| Variant | Cause |
|---|---|
| `FetchError::InvalidUrl(msg)` | URL could not be parsed |
| `FetchError::BlockedUrl(url)` | URL matched a blocked pattern or is a private network address |
| `FetchError::Network(msg)` | HTTP request failed (timeout, DNS, connection refused) |
| `FetchError::HttpError(status)` | Non-2xx HTTP status code |
| `FetchError::ResponseTooLarge(size, max)` | Response exceeded `max_response_bytes` |
| `FetchError::ActionError(msg)` | Invalid interaction (element not found, wrong element type, no page loaded) |
