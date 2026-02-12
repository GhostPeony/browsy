# Action Recipes Reference

browsy detects structured action patterns on each page and emits them as `SuggestedAction` variants. Each action provides element IDs that an agent can use directly with `click`, `type_text`, `check`, and `select` operations.

Actions are detected after page type classification. Multiple actions can coexist on a single page (a login page might also have a Search action for the nav bar and a CookieConsent action for a banner).

## SuggestedAction enum

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum SuggestedAction {
    Login { ... },
    Register { ... },
    Contact { ... },
    FillForm { ... },
    Search { ... },
    EnterCode { ... },
    Download { ... },
    CaptchaChallenge { ... },
    CookieConsent { ... },
    Consent { ... },
    SelectFromList { ... },
    Paginate { ... },
}
```

All actions are serialized with a `"action"` tag field for easy pattern matching.

---

## Login

Detected when the page has a visible password input, a nearby text/email input, and a submit button.

```json
{
  "action": "Login",
  "username_id": 5,
  "password_id": 8,
  "submit_id": 12,
  "remember_me_id": 10
}
```

| Field | Type | Description |
|-------|------|-------------|
| `username_id` | u32 | Text or email input nearest to the password field (within 500px Y) |
| `password_id` | u32 | The `<input type="password">` element |
| `submit_id` | u32 | Nearest submit button below the password field |
| `remember_me_id` | Option\<u32\> | Checkbox with "remember" in its label or name |

**When it fires:** Page has a visible password input and a nearby username/email input. Does NOT fire if the page also has registration context (confirm password + registration keywords) -- Register takes priority in that case. When a page has both login and registration sections (like Hacker News), Login takes priority over Register.

**Usage:** The MCP `login` tool and Python `browser.login()` use this action internally. They type into `username_id` and `password_id`, then click `submit_id`.

---

## Register

Detected on registration pages: password field plus either a confirm password field or registration keywords in the title/heading.

```json
{
  "action": "Register",
  "email_id": 3,
  "username_id": 4,
  "password_id": 7,
  "confirm_password_id": 9,
  "name_id": 2,
  "submit_id": 11
}
```

| Field | Type | Description |
|-------|------|-------------|
| `email_id` | Option\<u32\> | Email input |
| `username_id` | Option\<u32\> | Username text input |
| `password_id` | u32 | Primary password input |
| `confirm_password_id` | Option\<u32\> | Second password input (confirm) |
| `name_id` | Option\<u32\> | Full name text input |
| `submit_id` | u32 | Submit button |

**When it fires:** Page has a visible password field AND either (a) two or more password fields (confirm password pattern) or (b) title/heading contains registration keywords (`register`, `sign up`, `signup`, `create account`, `join`, `new account`). Does not fire when login keywords are present alongside confirm password (dual login/register pages prefer Login).

---

## Contact

Detected on contact forms: a textarea (message body) plus contact-related context in the title or headings.

```json
{
  "action": "Contact",
  "name_id": 2,
  "email_id": 4,
  "message_id": 6,
  "submit_id": 8
}
```

| Field | Type | Description |
|-------|------|-------------|
| `name_id` | Option\<u32\> | Name input |
| `email_id` | Option\<u32\> | Email input |
| `message_id` | u32 | Textarea element |
| `submit_id` | u32 | Submit button |

**When it fires:** Page has a visible textarea AND title/heading contains contact keywords (`contact us`, `contact form`, `get in touch`, `reach out`, `send us a message`, `inquiry`).

---

## FillForm

Generic form action for pages classified as `Form` that don't match a more specific pattern (Login, Register, Contact, Search).

```json
{
  "action": "FillForm",
  "fields": [
    { "id": 3, "label": "First Name", "name": "first_name", "type": "text" },
    { "id": 5, "label": "Email Address", "name": "email", "type": "email" },
    { "id": 7, "label": "Phone", "name": "phone", "type": "tel" }
  ],
  "submit_id": 10
}
```

| Field | Type | Description |
|-------|------|-------------|
| `fields` | Vec\<FormField\> | Visible data-entry fields with labels |
| `submit_id` | u32 | Submit button |

Each `FormField` contains:

| Field | Type | Description |
|-------|------|-------------|
| `id` | u32 | Element ID |
| `label` | Option\<String\> | Associated label text (from `<label>` or placeholder) |
| `name` | Option\<String\> | HTML name attribute |
| `input_type` | Option\<String\> | Input type attribute |

**When it fires:** Page type is `Form` (2+ data-entry inputs) AND no more specific form action (Login, Register, Contact, Search) was already detected.

---

## Search

Detected when a search input is present on the page.

```json
{
  "action": "Search",
  "input_id": 15,
  "submit_id": 16
}
```

| Field | Type | Description |
|-------|------|-------------|
| `input_id` | u32 | Search input element |
| `submit_id` | u32 | Submit button |

**When it fires:** Page has an input matching search criteria: `type="search"`, `role="searchbox"`, `name="q"`, name contains "search", or placeholder contains "search". Prefers visible inputs but falls back to hidden ones (for JS-rendered search engines).

---

## EnterCode

Detected on verification/2FA pages with code-related context.

```json
{
  "action": "EnterCode",
  "input_id": 4,
  "submit_id": 6,
  "code_length": 6
}
```

| Field | Type | Description |
|-------|------|-------------|
| `input_id` | u32 | Code input element (first input if multiple narrow digit inputs) |
| `submit_id` | u32 | Submit button |
| `code_length` | Option\<usize\> | Expected code length (set when 4-8 narrow inputs are detected) |

**When it fires:** Title or heading contains verification keywords AND the page has a visible text/number/tel input. Does not fire if a password field is present (that is Login). Detects separate-digit inputs (width < 60px, 4-8 inputs) and reports the code length.

**Usage:** The MCP `enter_code` tool and Python `browser.enter_code()` use this action internally.

---

## Download

Detected when the page has links or buttons with download-related text or file extension hrefs.

```json
{
  "action": "Download",
  "items": [
    { "id": 20, "text": "Download v2.1.0", "href": "https://example.com/release.zip" },
    { "id": 22, "text": "Download PDF", "href": "https://example.com/guide.pdf" }
  ]
}
```

| Field | Type | Description |
|-------|------|-------------|
| `items` | Vec\<DownloadItem\> | Downloadable links/buttons |

Each `DownloadItem` contains:

| Field | Type | Description |
|-------|------|-------------|
| `id` | u32 | Element ID |
| `text` | Option\<String\> | Link/button text |
| `href` | Option\<String\> | Download URL |

**When it fires:** Page has visible links or buttons where the text starts with "download" (and is short) or the href ends with a known file extension (`.zip`, `.tar.gz`, `.dmg`, `.exe`, `.msi`, `.deb`, `.rpm`, `.pkg`, `.appimage`, `.pdf`, `.csv`, `.xlsx`).

---

## CaptchaChallenge

Detected when a CAPTCHA service is found in the HTML structure or the page is classified as Captcha.

```json
{
  "action": "CaptchaChallenge",
  "captcha_type": "ReCaptcha",
  "sitekey": "6Le-wvkSAAAAABx7...",
  "submit_id": 15
}
```

| Field | Type | Description |
|-------|------|-------------|
| `captcha_type` | CaptchaType | Type of CAPTCHA detected |
| `sitekey` | Option\<String\> | Site key from `data-sitekey` attribute |
| `submit_id` | Option\<u32\> | Submit/verify button |

**When it fires:** Page has a `captcha` field set (detected CAPTCHA service in HTML) OR page type is `Captcha`. See [CAPTCHA Detection](ref-captcha.md) for details.

---

## CookieConsent

Detected when the page has a cookie notice with accept/reject buttons.

```json
{
  "action": "CookieConsent",
  "accept_id": 50,
  "reject_id": 52
}
```

| Field | Type | Description |
|-------|------|-------------|
| `accept_id` | u32 | Accept/agree button |
| `reject_id` | Option\<u32\> | Reject button (not always present) |

**When it fires:** Page has a substantial text block (>30 chars) mentioning cookies/GDPR AND a button with accept-related text (`accept all`, `accept cookies`, `allow cookies`, `allow all`, `agree`, `got it`, `i understand`, `i agree`).

---

## Consent

Detected on OAuth/authorization consent pages with approve/deny buttons.

```json
{
  "action": "Consent",
  "approve_ids": [30],
  "deny_ids": [32]
}
```

| Field | Type | Description |
|-------|------|-------------|
| `approve_ids` | Vec\<u32\> | Approve/allow/authorize buttons |
| `deny_ids` | Vec\<u32\> | Deny/cancel/decline buttons |

**When it fires:** Title or heading contains OAuth keywords (`authorize`, `allow access`, `grant permission`, `oauth`, `consent`) AND the page has buttons with approve or deny text.

---

## SelectFromList

Detected on pages with many links arranged in a list-like pattern.

```json
{
  "action": "SelectFromList",
  "items": [10, 14, 18, 22, 26]
}
```

| Field | Type | Description |
|-------|------|-------------|
| `items` | Vec\<u32\> | One link ID per row (the first link in each row group) |

**When it fires:** Page has 5+ visible links that form 5+ distinct rows (links within 30px Y are grouped into the same row). The action provides the first link ID from each row as representative items.

---

## Paginate

Detected when the page has next/previous navigation links or numbered page links.

```json
{
  "action": "Paginate",
  "next_id": 100,
  "prev_id": 98
}
```

| Field | Type | Description |
|-------|------|-------------|
| `next_id` | Option\<u32\> | Next page link |
| `prev_id` | Option\<u32\> | Previous page link |

**When it fires:** Page has links with pagination text (`next`, `prev`, `previous`, `>`, `>>`, `<`, `<<`, and Unicode equivalents).

---

## Detection order

Actions are detected in this order:

1. Register (or Login if no registration context)
2. EnterCode
3. Consent
4. Contact
5. Search
6. SelectFromList
7. CookieConsent
8. Paginate
9. FillForm (only if no more specific form action exists)
10. Download
11. CaptchaChallenge

Multiple actions can coexist. A login page with a cookie banner and nav search bar will have Login, CookieConsent, and Search actions simultaneously.
