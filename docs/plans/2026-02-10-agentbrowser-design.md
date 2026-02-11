# agentbrowser — Design Specification

**Date:** 2026-02-10
**Status:** Approved
**License:** MIT (100% — all dependencies MIT or Apache-2.0)

---

## 1. Vision & Core Architecture

agentbrowser is a zero-render browser engine purpose-built for AI agents. It processes HTML/CSS/JS and outputs a Spatial DOM — a structured representation containing every element's exact bounding box, semantic role, text content, and interaction state — without ever producing pixels.

### Core Principle

Browsers compute layout (where things are) before they paint (what things look like). agentbrowser keeps the layout, throws away the paint. The result: 10-20x fewer tokens, exact click coordinates, zero text misreads, and faster execution than any screenshot-based approach.

### Architecture

```
+-----------------------------------------------------+
|                  agentbrowser-core (Rust)            |
|                                                     |
|  +----------+  +----------------+  +--------------+ |
|  | html5ever|->| CSS Extractor  |->|    taffy     | |
|  | (parse)  |  | (layout props) |  |  (flex/grid) | |
|  +----------+  +----------------+  +--------------+ |
|        |              |                |             |
|  +----------------------------------------------+   |
|  |         Layout Engine (block, position,       |   |
|  |         text measurement, overflow)           |   |
|  +----------------------------------------------+   |
|        |                                             |
|  +--------------+  +----------+  +--------------+   |
|  | Spatial DOM  |  |   V8     |  | Auth/Stealth |   |
|  | Output       |  | (JS)     |  | Layer        |   |
|  +--------------+  +----------+  +--------------+   |
|                                                     |
+-----------------------------------------------------+
|  agentbrowser-node (napi-rs)  |  agentbrowser-py    |
|  TypeScript SDK               |  Python SDK (PyO3)  |
+-----------------------------------------------------+
```

### Target Output Per Page

- **500-2,000 tokens** for the Spatial DOM (vs 1,300+ for a single screenshot)
- **Exact coordinates** for every interactive element (vs +-30-50px error from vision models)
- **Zero text misreads** (extracted directly from DOM, not OCR'd from pixels)

---

## 2. Core Engine — Minimal Build

### Dependencies (3 crates, nothing else)

| Crate | License | Purpose |
|---|---|---|
| `html5ever` | MIT/Apache-2.0 | HTML parsing — spec-compliant, non-negotiable for real websites |
| `taffy` | MIT/Apache-2.0 | Flexbox + CSS Grid layout — covers ~90% of modern layouts |
| `rusty_v8` | MIT | V8 JavaScript engine — ~95% of sites need JS to build their DOM |

### CSS Property Handling

No full CSS parser. A minimal extractor that only understands layout-affecting properties:

```
KEPT (~40 properties):              IGNORED (~510 properties):
display                             color, background, border-color
width, height, min-width, etc.      font-family, font-style
margin, padding                     box-shadow, text-shadow
position, top/right/bottom/left     transform (visual only)
flex-direction, flex-wrap, etc.     animation, transition
grid-template-*, grid-column, etc.  opacity (visual only)
overflow                            cursor, outline
visibility                          all pseudo-element styling
z-index                             everything decorative
font-size (for text metrics)
line-height
```

~40 properties out of ~550. 93% of CSS ignored.

### Pipeline (4 steps)

```
HTML string
    |
1. html5ever  ->  DOM tree
    |
2. CSS extract ->  layout properties per element (custom, minimal)
    |
3. taffy       ->  bounding boxes (x, y, w, h) for every element
    |
4. Output      ->  Spatial DOM (flat list, indexed, minimal tokens)
```

V8 runs in parallel — executes JS, mutates the DOM, pipeline re-runs on changes.

### Targets

- Binary size: under 15MB without V8, ~30MB with V8 (vs Chromium at 200MB+)
- Memory: under 50MB per page session (vs Chromium at 200-500MB)

---

## 3. Spatial DOM Output Format

Two formats: full (first load) and delta (on changes).

### Full Output

```json
{
  "url": "https://example.com/login",
  "title": "Sign In",
  "vp": [1920, 1080],
  "scroll": [0, 0],
  "els": [
    { "id": 1, "tag": "input", "role": "textbox", "ph": "Email", "b": [560, 120, 800, 48] },
    { "id": 2, "tag": "input", "role": "textbox", "ph": "Password", "type": "password", "b": [560, 184, 800, 48] },
    { "id": 3, "tag": "button", "text": "Sign In", "b": [560, 260, 800, 48] },
    { "id": 4, "tag": "a", "text": "Forgot password?", "href": "/reset", "b": [560, 324, 140, 20] },
    { "id": 5, "tag": "a", "text": "Sign up", "href": "/register", "b": [1160, 324, 60, 20] }
  ]
}
```

Key design decisions:
- `b` = bounds: `[x, y, width, height]` — array not object, saves tokens
- `ph` = placeholder, `text` = visible text content
- Only interactive + text-bearing elements included. Decorative divs, wrappers, spacers are excluded.
- Flat list, no nesting. Position tells the agent everything hierarchy would.

### Delta Output (after JS mutations / actions)

```json
{
  "delta": true,
  "added": [
    { "id": 6, "tag": "span", "role": "alert", "text": "Invalid email", "b": [560, 92, 200, 20] }
  ],
  "changed": [
    { "id": 1, "val": "user@test.com" }
  ],
  "removed": [8, 9]
}
```

### Token Cost

| Page complexity | Elements | Full output tokens | Delta tokens |
|---|---|---|---|
| Simple (login page) | 5-10 | ~80-150 | ~20-40 |
| Medium (search results) | 30-60 | ~400-800 | ~50-150 |
| Complex (dashboard) | 100-200 | ~1,500-2,500 | ~100-300 |

### Compact String Format (optional, for extreme token budgets)

```
[1:input "Email" @560,120 800x48]
[2:input:pw "Password" @560,184 800x48]
[3:button "Sign In" @560,260 800x48]
[4:link "Forgot password?" @560,324]
[5:link "Sign up" @1160,324]
```

5 elements. 5 lines. ~40 tokens.

---

## 4. Agent Action Interface

8 actions. The agent uses element IDs. agentbrowser resolves coordinates internally.

### Action Set

```
click(id)                    // click center of element [id]
type(id, text)               // focus element [id], type text
select(id, value)            // select option in dropdown
scroll(direction, amount?)   // scroll viewport: up/down/left/right
hover(id)                    // hover over element [id]
wait(ms | selector)          // wait for time or element to appear
goto(url)                    // navigate to URL
back()                       // browser back
```

No `evaluate()`, no raw JS injection, no CDP passthrough. The agent operates at the semantic level.

### Action Execution Flow

```
Agent sends:  click(3)

agentbrowser:
  1. Look up element 3 -> bounds [560, 260, 800, 48]
  2. Calculate center   -> (960, 284)
  3. Add human jitter   -> (962, 281)  <- random offset +-5px
  4. Simulate mouse move -> bezier curve from current position
  5. Dispatch mousedown, mouseup, click events
  6. Wait for DOM to settle (MutationObserver)
  7. Re-run layout pipeline
  8. Return delta output
```

### Response Format

```json
{
  "ok": true,
  "action": "click(3)",
  "duration_ms": 45,
  "delta": {
    "added": [
      { "id": 6, "tag": "div", "role": "dialog", "text": "Confirm?", "b": [660, 340, 600, 200] },
      { "id": 7, "tag": "button", "text": "Yes", "b": [860, 480, 80, 36] },
      { "id": 8, "tag": "button", "text": "No", "b": [960, 480, 80, 36] }
    ]
  }
}
```

### Error Handling

Three error types: `element_not_found`, `element_not_visible`, `navigation_error`.

### Anti-Detection Built Into Every Action

- Mouse movement: Bezier curve paths with variable speed
- Click position: Randomized within element bounds
- Typing cadence: Variable inter-key delay (50-150ms)
- Event order: mouseenter -> mouseover -> mousemove -> mousedown -> mouseup -> click

The agent writes clean commands. Stealth is the engine's job.

---

## 5. Auth & Session Layer

Auth is a first-class primitive. The engine manages credentials, sessions, and 2FA without the agent ever seeing raw passwords.

### Credential Store

Credentials stored encrypted on disk (AES-256-GCM), referenced by alias:

```
browser.auth.add("github", {
  username: "user@email.com",
  password: "***",
  totp_secret: "JBSWY3DP..."  // optional
})
```

The agent never sees credentials. It says `login("github")`. The engine handles the rest.

### Login Flow

```
Agent sends:  login("github")

agentbrowser:
  1. Navigate to saved URL for "github"
  2. Detect login form (find input[type=email], input[type=password])
  3. Fill credentials from encrypted store (NOT through the LLM)
  4. Submit form
  5. If 2FA prompt detected:
     a. TOTP? -> generate code from stored secret, enter it
     b. SMS/Push? -> emit "auth_pending" event, wait for callback
  6. Capture session state (cookies, localStorage, tokens)
  7. Return Spatial DOM of authenticated page
```

### Session Persistence

```
~/.agentbrowser/sessions/
  github.session      # encrypted cookies + localStorage + tokens
  stripe.session
```

One file per site. Encrypted. Sessions restore automatically — no re-login unless expired.

```
// First run: logs in, saves session
session.goto("https://github.com", { auth: "github" })

// Every subsequent run: restores session, skips login
session.goto("https://github.com", { auth: "github" })
// Already authenticated.
```

### Session Health

```
On every navigation:
  1. Check for redirect to login page -> session expired
  2. Check for 401/403 response -> session expired
  3. Check cookie expiry timestamps -> proactive re-auth

If expired -> re-run login flow silently -> continue task
```

### 2FA Support

| Method | Handling | Agent involvement |
|---|---|---|
| TOTP | Automatic — generates code from stored secret | None |
| SMS | Emits `auth_pending` event. Accepts code via callback. | Human or external service |
| Push notification | Emits `auth_pending` event. Polls until approved or timeout. | Human taps approve |
| Email code | If IMAP configured, auto-fetches code from inbox | None |
| CAPTCHA | Stealth-first (avoid triggering). Fallback: `captcha_pending` event. | Solving service or human |

---

## 6. Anti-Detection & Stealth

Built into every layer. The agent never thinks about it.

### Proxy Support

Three modes:
- **Static** — one IP per session (for auth flows needing consistent IP)
- **Rotating** — new IP per request or per page
- **Pool** — sticky sessions from a pool, rotate on failure

Geographic consistency enforced automatically — timezone, language headers, and locale match proxy IP geolocation.

### Request Blocking (on by default)

```
BLOCKED (never loaded):         ALLOWED:
Google Analytics                HTML documents
Facebook Pixel                  CSS (parsed for layout)
Ad networks                     JS (executed in V8)
Font files (use metrics only)   API/XHR responses
Images (unless requested)
Video/audio
Source maps, favicons
```

Pages load 2-5x faster. DOM has less noise. Fewer tracking scripts detect the session.

Blocklist is a flat text file. Users can add/remove entries.

### Browser Fingerprint

Hardcoded to match latest stable Chrome. Updated with engine releases:

```
navigator.webdriver        -> undefined
navigator.plugins          -> [Chrome PDF, Chrome PDF Viewer, Native Client]
navigator.languages        -> matches proxy geolocation
window.chrome              -> present, realistic object
WebGL vendor/renderer      -> "Google Inc. (NVIDIA)"
canvas fingerprint         -> deterministic per session, realistic hash
```

Not runtime-patched. Baked into V8 environment at build time.

### TLS Fingerprint

Uses Chromium's BoringSSL compiled directly into the network stack:

```
JA3 hash: matches Chrome stable
JA4 hash: matches Chrome stable
HTTP/2 SETTINGS frame: matches Chrome
Header order: matches Chrome
```

### Behavioral Realism

Built into the action layer:

| Signal | Implementation |
|---|---|
| Mouse movement | Bezier curves, variable speed, micro-jitter |
| Click position | Random offset within element bounds |
| Typing speed | 50-150ms between keys, occasional pauses |
| Scroll | Smooth scroll with deceleration |
| Page load timing | Random delay (200-800ms) before first action |
| Tab focus | `document.hasFocus()` returns true |

### Concurrent Sessions

Each session gets:
- Isolated cookie jar
- Unique (but realistic) canvas/WebGL fingerprint
- Independent proxy connection
- Separate V8 isolate

Sessions share nothing.

---

## 7. SDK Bindings

Thin wrappers around the Rust core via napi-rs (TypeScript) and PyO3 (Python).

### Complete API Surface (15 methods)

| Method | What it does |
|---|---|
| `AgentBrowser.launch(config)` | Start engine |
| `browser.auth.add(name, creds)` | Store credentials |
| `browser.auth.remove(name)` | Remove credentials |
| `browser.session(opts)` | Create isolated session |
| `session.goto(url)` | Navigate, return Spatial DOM |
| `session.click(id)` | Click element, return delta |
| `session.type(id, text)` | Type into element, return delta |
| `session.select(id, value)` | Select option, return delta |
| `session.scroll(dir, amount?)` | Scroll viewport, return delta |
| `session.hover(id)` | Hover element, return delta |
| `session.wait(ms or selector)` | Wait for time or element |
| `session.back()` | Navigate back, return Spatial DOM |
| `session.state()` | Get full current Spatial DOM |
| `session.close()` | End session, save auth state |
| `browser.shutdown()` | Stop engine |

### TypeScript Example

```typescript
import { AgentBrowser } from 'agentbrowser'

const browser = await AgentBrowser.launch({
  proxy: 'residential://user:pass@provider:8080',
  viewport: [1920, 1080]
})

browser.auth.add('github', {
  username: 'user@email.com',
  password: '***',
  totp: 'JBSWY3DP...'
})

const session = await browser.session({ auth: 'github' })
const page = await session.goto('https://github.com')
const delta = await session.click(1)

await session.close()
await browser.shutdown()
```

### Python Example

```python
from agentbrowser import AgentBrowser

browser = AgentBrowser.launch(
    proxy="residential://user:pass@provider:8080",
    viewport=(1920, 1080)
)

browser.auth.add("github",
    username="user@email.com",
    password="***",
    totp="JBSWY3DP..."
)

session = browser.session(auth="github")
page = session.goto("https://github.com")
delta = session.click(1)

session.close()
browser.shutdown()
```

### What's NOT in the SDK

- No callback/event system
- No DOM manipulation API
- No raw JS execution
- No screenshot capture
- No CDP/DevTools exposure
- No plugin architecture

---

## 8. Testing & Benchmarks

### Layout Accuracy (WPT Subset)

| WPT Category | Tests | Target Pass Rate |
|---|---|---|
| CSS Flexbox | ~800 | 95%+ |
| CSS Grid | ~700 | 90%+ |
| CSS Position | ~300 | 90%+ |
| CSS Box Model | ~400 | 90%+ |
| CSS Display | ~200 | 85%+ |

~2,400 tests total out of ~50,000. The rest test visual rendering we don't do.

### Real Website Testing (50 sites)

```
Auth flows:     github.com, google.com, stripe.com, twitter.com, linkedin.com
E-commerce:     amazon.com, shopify stores, ebay.com
Search:         google.com, bing.com, duckduckgo.com
Dashboards:     github.com/settings, vercel.com, netlify.com
Forms:          typeform, google forms, hubspot
SPAs:           gmail.com, notion.com, figma.com
```

Per site, validate:
1. All interactive elements detected
2. Bounding boxes within +-5px of real Chrome's layout
3. Text content matches exactly
4. Login flow completes successfully
5. No bot detection triggered

### Benchmark Targets

```
agentbrowser vs Browser Use vs BrowserBase vs Playwright+Claude

Metric              Target      Browser Use    Screenshot
Tokens per step     <500        3,000-8,000    1,300-2,700
Task accuracy       >90%        35-45%         25-35%
Click accuracy      100%        ~85%           ~70%
Text accuracy       100%        ~95%           ~80-85%
Page load time      <500ms      1-3s           1-3s
Memory per session  <50MB       200-500MB      200-500MB
Cost per 100 tasks  <$0.10      $1-5           $2-8
```

### WebArena Benchmark

Test against the standard academic web agent benchmark with GPT-4o, Claude Sonnet, and open-source models. Hypothesis: better input representation -> better accuracy with cheaper models.

### CI Pipeline

```
On every commit:
  1. WPT subset -> layout accuracy regression
  2. 10 real websites -> element detection + bounds
  3. 3 auth flows -> login success
  4. Token count regression -> alert if output grows

Weekly:
  5. Full 50-site suite
  6. Full benchmark vs competitors
  7. Bot detection check (Cloudflare, DataDome, PerimeterX)
```

---

## Implementation Phases

| Phase | What | Estimate |
|---|---|---|
| 1 | Glue layer: html5ever -> CSS extract -> taffy | 2-3 weeks |
| 2 | Block layout + positioning on top of taffy | 2-3 weeks |
| 3 | Style computation: cascade, inheritance | 3-4 weeks |
| 4 | Absolute/fixed/sticky positioning | 1-2 weeks |
| 5 | Text measurement: approximate line wrapping | 2-3 weeks |
| 6 | Spatial DOM output pipeline + element indexing | 2-3 weeks |
| 7 | V8 integration + DOM mutation handling | 3-4 weeks |
| 8 | Auth/session layer + stealth | 2-3 weeks |
| 9 | TS/Python SDK bindings | 2-3 weeks |
| 10 | Testing, benchmarks, real-site validation | 2-3 weeks |
