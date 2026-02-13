# browsy

![browsy](https://github.com/user-attachments/assets/7b3984d9-14a0-4457-9949-427a2aff7686)

**Zero-render browser engine for AI agents.** [browsy.dev](https://browsy.dev)

browsy converts web pages into a structured **Spatial DOM** -- a flat list of interactive and text elements with bounding boxes, roles, and states -- without rendering pixels. On top of this, it layers **page intelligence**: automatic page type detection, suggested actions with stable element IDs, CAPTCHA detection, and hidden content exposure.

```
$ browsy fetch https://github.com/login

page_type: Login
suggested_actions:
  Login { username: 19, password: 21, submit: 34 }

[19:input "Username or email address" @top-C]
[21:input "Password" @mid-C]
[34:button "Sign in" @mid-C]

// 203ms. No Chromium. No LLM needed.
```

## Why browsy?

Every AI agent that touches the web today launches a **300MB Chromium instance**, waits 5 seconds for it to render, then asks an LLM "what am I looking at?"

browsy skips all of that:

| | Chromium-based tools | browsy |
|---|---|---|
| **Speed** | 5-30 seconds per page | ~200ms |
| **Dependencies** | 282MB+ Chromium | 6MB binary |
| **Page intelligence** | None (LLM must figure it out) | 12 page types, 13 action recipes |
| **Hidden content** | Not accessible | Exposed with `hidden: true` |
| **CAPTCHA detection** | None | reCAPTCHA, hCaptcha, Turnstile, Cloudflare, image grid |
| **Output** | Raw accessibility tree | Structured Spatial DOM |
| **Deterministic** | No (LLM variance) | Yes (same HTML = same output) |

## When to use browsy

browsy handles **server-rendered HTML** -- the 90% of the web that doesn't need a browser to understand. Login forms, search pages, news sites, government portals, documentation, e-commerce product pages.

For **JS-rendered SPAs** (React, Angular, Vue apps that render client-side), you still need a real browser. browsy is the fast path, not a full browser replacement.

## Key features

- **Page intelligence** -- 12 page types detected automatically, 13 action recipes with element IDs
- **CAPTCHA detection** -- identifies reCAPTCHA, hCaptcha, Cloudflare Turnstile, image grids with sitekey extraction
- **Hidden content exposure** -- dropdowns, modals, accordions included with `hidden: true`
- **Session API** -- navigate, click, type, select, search -- with cookie persistence
- **Built-in web search** -- DuckDuckGo and Google, search and fetch results in one call
- **Smart deduplication** -- 34-42% element reduction on real sites
- **Delta output** -- only changes after first load
- **MCP server** -- use browsy from Claude Code or any MCP client
- **Python bindings** -- PyO3-based, full session API
- **6MB binary** -- zero runtime dependencies
