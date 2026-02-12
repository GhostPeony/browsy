# CAPTCHA Detection

browsy detects CAPTCHAs from HTML structure alone -- no rendering, no image analysis, no JavaScript execution. Detection works by scanning the raw DOM tree for known CAPTCHA service indicators before the Spatial DOM is generated.

## CaptchaType enum

```rust
pub enum CaptchaType {
    ReCaptcha,           // Google reCAPTCHA v2 or v3
    HCaptcha,            // hCaptcha
    Turnstile,           // Cloudflare Turnstile
    CloudflareChallenge, // Cloudflare JS challenge ("Just a moment...")
    ImageGrid,           // Custom image-grid CAPTCHA ("select all images containing...")
    TextCaptcha,         // Text-based CAPTCHA (type characters from an image)
    Unknown,             // CAPTCHA detected but type not identified
}
```

## Detection signals

browsy scans the layout tree for these patterns:

### Script sources

| Pattern | Detected as |
|---------|-------------|
| `src` contains `recaptcha` or `google.com/recaptcha` | ReCaptcha |
| `src` contains `hcaptcha.com` | HCaptcha |
| `src` contains `challenges.cloudflare.com/turnstile` | Turnstile |

### Iframe sources

| Pattern | Detected as |
|---------|-------------|
| `src` contains `recaptcha` or `google.com/recaptcha` | ReCaptcha |
| `src` contains `hcaptcha.com` or `newassets.hcaptcha.com` | HCaptcha |

### Div classes

| Pattern | Detected as |
|---------|-------------|
| Class contains `g-recaptcha` | ReCaptcha |
| Class contains `h-captcha` | HCaptcha |
| Class contains `cf-turnstile` | Turnstile |

### Div IDs

| Pattern | Detected as |
|---------|-------------|
| ID contains `challenge-running` or `cf-challenge` | CloudflareChallenge |

### Site key

Any element with a `data-sitekey` attribute has its value captured. This attribute is used by reCAPTCHA, hCaptcha, and Turnstile to embed the site key.

### Title and heading keywords

Page type detection checks title and headings for CAPTCHA-related phrases. These trigger `PageType::Captcha` even without a known CAPTCHA service:

**Title keywords:** `captcha`, `verify you're human`, `verify you are human`, `robot`, `security check`, `challenge`, `just a moment`, `attention required`, `are you human`

**Heading keywords:** `captcha`, `verify you're human`, `security check`, `are you human`, `complete the challenge`, `human verification`

## CaptchaInfo struct

```rust
pub struct CaptchaInfo {
    pub captcha_type: CaptchaType,
    pub sitekey: Option<String>,
}
```

The `sitekey` is populated when a `data-sitekey` attribute is found. It is the value needed by third-party CAPTCHA solving services.

## CaptchaChallenge action

When a CAPTCHA is detected, the `CaptchaChallenge` suggested action is emitted:

```rust
SuggestedAction::CaptchaChallenge {
    captcha_type: CaptchaType,
    sitekey: Option<String>,
    submit_id: Option<u32>,
}
```

The `submit_id` is the nearest verify/submit/continue button, if one exists. When no known CAPTCHA service is detected but the page is classified as `Captcha`, browsy infers the type:

- 4+ image buttons on the page: `ImageGrid`
- Otherwise: `Unknown`

## Session methods

### Rust

```rust
let mut session = Session::new()?;
let dom = session.goto("https://example.com")?;

// Check if the current page is a CAPTCHA
if session.is_captcha() {
    println!("CAPTCHA detected!");
}

// Get CAPTCHA details
if let Some(info) = session.captcha_info() {
    println!("Type: {:?}", info.captcha_type);
    if let Some(ref key) = info.sitekey {
        println!("Site key: {}", key);
    }
}
```

### Python

```python
browser = Browser()
page = browser.goto("https://example.com")

if page.page_type() == "Captcha":
    for action in page.suggested_actions():
        if action["action"] == "CaptchaChallenge":
            print(f"Type: {action['captcha_type']}")
            print(f"Site key: {action.get('sitekey')}")
```

## MCP behavior

When the `browse` or `click` tools return a page detected as `Captcha`, the output is prefixed with a warning:

```
CAPTCHA detected (ReCaptcha) -- this page requires human verification to proceed.
```

The `page_info` tool includes the full CAPTCHA information:

```json
{
  "page_type": "Captcha",
  "captcha": {
    "captcha_type": "ReCaptcha",
    "sitekey": "6Le-wvkSAAAA..."
  },
  "suggested_actions": [
    {
      "action": "CaptchaChallenge",
      "captcha_type": "ReCaptcha",
      "sitekey": "6Le-wvkSAAAA...",
      "submit_id": 15
    }
  ]
}
```

## What browsy cannot do

browsy detects and classifies CAPTCHAs. It does not solve them. When a CAPTCHA is encountered, the agent has several options:

1. **Human-in-the-loop:** Surface the CAPTCHA to a human operator.
2. **Third-party solver:** Pass the `captcha_type` and `sitekey` to a CAPTCHA solving service (2captcha, Anti-Captcha, etc.), receive the solution token, and inject it.
3. **Alternative approach:** Try a different URL, use an API instead of the web interface, or skip the blocked resource.
4. **Wait and retry:** Some Cloudflare challenges resolve after a delay.

The `sitekey` in the `CaptchaInfo` is the value that third-party solving services typically require.

## Detection pipeline

CAPTCHA detection happens at two stages:

1. **Tree scan** (`detect_captcha_from_tree`): Before the Spatial DOM is generated, the layout tree is scanned for CAPTCHA service indicators (script/iframe sources, div classes/IDs, data-sitekey). This produces the `CaptchaInfo` stored on `SpatialDom.captcha`.

2. **Page type classification** (`detect_page_type`): After the Spatial DOM is built, the page type heuristic checks for CAPTCHA signals: title keywords, heading keywords, and the presence of `captcha` on the SpatialDom. If any signal matches, the page is classified as `PageType::Captcha`.

3. **Action detection** (`detect_captcha_challenge_action`): If `captcha` is set or the page type is `Captcha`, the `CaptchaChallenge` action is emitted with the type, sitekey, and submit button.
