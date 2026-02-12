# Page Intelligence

Page intelligence is browsy's deterministic classification layer. Given a Spatial DOM, browsy computes a **page type** and a set of **suggested actions** (action recipes) -- each with concrete element IDs that agents can use directly. No LLM inference, no probabilistic guessing.

```rust
let dom = session.goto("https://github.com/login")?;

assert_eq!(dom.page_type, PageType::Login);
// dom.suggested_actions[0] == Login { username_id: 19, password_id: 21, submit_id: 34 }
```

## Page types

browsy classifies pages into one of 14 types, detected via priority-ordered heuristics applied to the Spatial DOM. The first matching rule wins.

| Page Type | Detection Signal |
|---|---|
| `Error` | Alert elements with `alert_type == "error"`, or title contains `404`, `500`, `403`, `not found`, `error` |
| `Captcha` | CAPTCHA service detected in HTML (reCAPTCHA, hCaptcha, Turnstile), or title/heading contains `captcha`, `verify you're human`, `just a moment` |
| `Login` | Visible password input field present |
| `TwoFactorAuth` | Title/heading contains verification keywords (`verification`, `2fa`, `otp`, `one-time`, `passcode`) AND a visible text/number/tel input exists |
| `OAuthConsent` | Title/heading contains `authorize`, `allow access`, `grant permission`, `oauth`, `consent` |
| `Inbox` | Title contains `inbox`, `mail`, `messages` AND page has 10+ links |
| `EmailBody` | 3+ email markers present in element text (`from:`, `to:`, `subject:`, `date:`) |
| `Dashboard` | Title/heading contains `dashboard`, `welcome back`, `overview` AND both `nav` and `main` landmarks exist |
| `Article` | 3+ headings AND 2+ long paragraphs (>100 chars). When link count >= 20, requires 10+ long paragraphs. Heading-heavy pages (15+ headings with low paragraph ratio) are excluded |
| `SearchResults` | Search input present AND 8+ links AND (title/heading contains `search results`/`results for` OR URL contains search query params like `?q=`) |
| `List` | 10+ visible links |
| `Search` | Visible search input (type `search`, role `searchbox`, name `q`, or placeholder/name containing `search`) |
| `Form` | 2+ visible data-entry inputs (excludes checkbox, radio, hidden, submit, button, image) |
| `Other` | No other type matched |

Detection order matters. A page with a password field and a search bar is classified as `Login`, not `Search`, because `Login` is checked first.

## Action recipes

Alongside page type, browsy detects **suggested actions** -- structured recipes telling the agent exactly what to do and which element IDs to use. Each action maps directly to Session API calls.

### Login

Detected when a visible password input exists near a text/email input.

```json
{
  "action": "Login",
  "username_id": 19,
  "password_id": 21,
  "submit_id": 34,
  "remember_me_id": 36
}
```

Agent usage: `session.type_text(19, "user@example.com")`, `session.type_text(21, "pass")`, `session.click(34)`. Or simply: `session.login("user@example.com", "pass")`.

### Register

Detected when a password field is accompanied by a confirm-password field or registration keywords in the title/heading. Login takes priority when both login and registration sections are present on the same page.

```json
{
  "action": "Register",
  "email_id": 12,
  "username_id": 14,
  "password_id": 16,
  "confirm_password_id": 18,
  "name_id": 10,
  "submit_id": 22
}
```

### EnterCode

Detected on verification/2FA pages with code-related keywords in the title or heading.

```json
{
  "action": "EnterCode",
  "input_id": 8,
  "submit_id": 12,
  "code_length": 6
}
```

`code_length` is set when the page uses separate narrow digit inputs (4-8 inputs each <60px wide).

### Search

Detected when an input has type `search`, role `searchbox`, name `q`, or a name/placeholder containing `search`.

```json
{
  "action": "Search",
  "input_id": 5,
  "submit_id": 7
}
```

### Consent

Detected on OAuth/authorization pages with approve/deny buttons.

```json
{
  "action": "Consent",
  "approve_ids": [15, 18],
  "deny_ids": [20]
}
```

### CookieConsent

Detected when a substantial text block mentions cookies/GDPR and accept/reject buttons are present.

```json
{
  "action": "CookieConsent",
  "accept_id": 42,
  "reject_id": 44
}
```

### Contact

Detected on pages with contact-related keywords and a visible textarea for the message body.

```json
{
  "action": "Contact",
  "name_id": 5,
  "email_id": 7,
  "message_id": 9,
  "submit_id": 11
}
```

### FillForm

Generic form detection. Emitted when visible form fields exist and no more specific action (Login, Register, Contact) matched. Includes labeled field metadata.

```json
{
  "action": "FillForm",
  "fields": [
    {"id": 10, "label": "First Name", "name": "first_name", "type": "text"},
    {"id": 12, "label": "Email", "name": "email", "type": "email"}
  ],
  "submit_id": 20
}
```

### SelectFromList

Detected when 5+ links are arranged in distinct vertical rows (list-like layout).

```json
{
  "action": "SelectFromList",
  "items": [3, 8, 13, 18, 23]
}
```

### Paginate

Detected when next/previous navigation links are found (text matching `next`, `previous`, `>`, `>>`, etc.).

```json
{
  "action": "Paginate",
  "next_id": 95,
  "prev_id": 91
}
```

### Download

Detected when links point to downloadable file types.

```json
{
  "action": "Download",
  "items": [{"id": 30, "text": "Report Q4 2024", "href": "/files/report.pdf"}]
}
```

### CaptchaChallenge

Detected when a CAPTCHA service is found in the HTML structure.

```json
{
  "action": "CaptchaChallenge",
  "captcha_type": "ReCaptcha",
  "sitekey": "6LcXxxAAAABBBCCC...",
  "submit_id": 50
}
```

## CAPTCHA detection

browsy identifies CAPTCHA services by scanning the HTML structure for known markers:

| Type | Detection |
|---|---|
| `ReCaptcha` | `g-recaptcha` class, `data-sitekey` attr, reCAPTCHA script URLs |
| `HCaptcha` | `h-captcha` class, hCaptcha script URLs |
| `Turnstile` | `cf-turnstile` class, Turnstile script URLs |
| `CloudflareChallenge` | Cloudflare "Just a moment..." challenge page pattern |
| `ImageGrid` | Custom image-grid CAPTCHA (select matching images) |
| `TextCaptcha` | Text-based CAPTCHA (type characters from an image) |
| `Unknown` | CAPTCHA detected but service not identified |

CAPTCHA info is available at `dom.captcha`:

```rust
if let Some(captcha) = &dom.captcha {
    println!("Type: {:?}", captcha.captcha_type);     // CaptchaType::ReCaptcha
    println!("Sitekey: {:?}", captcha.sitekey);        // Some("6Lc...")
}
```

## How detection works

All detection is **deterministic, heuristic-based, priority-ordered**. No machine learning models, no token costs. The same HTML always produces the same page type and action set.

The detection pipeline:

1. Parse HTML into the Spatial DOM (element list with bounding boxes and roles)
2. Scan for CAPTCHA markers in the layout tree
3. Run `detect_page_type` -- walks through page type checks in priority order, returns the first match
4. Run `detect_suggested_actions` -- runs all action detectors independently, collecting all that match

Multiple actions can coexist. A login page might have both `Login` and `CookieConsent` actions. A search results page might have `Search`, `SelectFromList`, and `Paginate`.

## Example flow

```rust
use browsy_core::fetch::Session;
use browsy_core::output::PageType;

let mut session = Session::new()?;
let dom = session.goto("https://example.com/login")?;

match dom.page_type {
    PageType::Login => {
        // Use the Login action recipe directly
        session.login("user@example.com", "hunter2")?;
    }
    PageType::TwoFactorAuth => {
        session.enter_code("847291")?;
    }
    PageType::Captcha => {
        let info = session.captcha_info();
        // Report to the caller -- browsy cannot solve CAPTCHAs
    }
    _ => {
        // Read the page content, follow links, etc.
    }
}
```
