# Python Bindings

browsy provides Python bindings via PyO3. The API closely mirrors the Rust `Session` API.

## Installation

```bash
pip install browsy
```

The package ships a compiled native extension (`_core.pyd` / `_core.so`). No Rust toolchain required for installation from wheels.

## Module contents

```python
from browsy import Browser, Page, Element
```

| Class | Description |
|-------|-------------|
| `Browser` | A browsing session with cookie persistence and form state |
| `Page` | A parsed page (the Spatial DOM) |
| `Element` | A single element in the Spatial DOM |

## Basic usage: parsing HTML

The `Browser` class can parse local HTML without network access:

```python
from browsy import Browser

browser = Browser(viewport_width=1920, viewport_height=1080)
page = browser.load_html('<h1>Hello</h1><a href="/about">About</a>', 'https://example.com')

print(page.title)       # ""
print(len(page))        # 2
for el in page.elements:
    print(el.id, el.tag, el.text)
# 1 h1 Hello
# 2 a About
```

## Browsing: navigating URLs

```python
from browsy import Browser

browser = Browser()
page = browser.goto("https://example.com")

print(page.title)       # "Example Domain"
print(page.url)         # "https://example.com"
print(page.page_type()) # "Other"
```

## Page properties and methods

```python
page.title              # str: page title
page.url                # str: current URL
page.elements           # list[Element]: all elements
page.visible()          # list[Element]: non-hidden elements only
page.above_fold()       # list[Element]: elements with top edge within viewport
page.get(id)            # Element or None: lookup by ID
page.page_type()        # str: "Login", "Search", "Article", "List", etc.
page.suggested_actions() # list[dict]: detected action recipes
page.alerts()           # list[Element]: elements with alert_type set
page.tables()           # list[dict]: extracted table data (headers + rows)
page.pagination()       # dict or None: next/prev/pages links
page.to_json()          # str: full JSON serialization
page.to_compact()       # str: compact text format
len(page)               # int: element count
```

## Element properties

```python
el.id                   # int: unique element ID
el.tag                  # str: HTML tag name
el.role                 # str or None: ARIA role (implicit or explicit)
el.text                 # str or None: visible text content
el.href                 # str or None: link target (resolved to absolute URL)
el.placeholder          # str or None: placeholder text
el.value                # str or None: current value
el.input_type           # str or None: input type attribute
el.name                 # str or None: HTML name attribute
el.label                # str or None: associated label text
el.alert_type           # str or None: "alert", "error", "success", "warning"
el.disabled             # bool or None
el.checked              # bool or None
el.expanded             # bool or None
el.selected             # bool or None
el.required             # bool or None
el.hidden               # bool or None: True if element is hidden
el.bounds               # tuple[int, int, int, int]: (x, y, width, height)
```

## Form interaction

```python
browser = Browser()
page = browser.goto("https://example.com/login")

# Type into fields by element ID
browser.type_text(5, "user@example.com")
browser.type_text(8, "secretpassword")

# Check a "remember me" checkbox
browser.check(10)

# Select a dropdown option
browser.select(12, "en-US")

# Read the updated DOM with form state overlaid
page = browser.dom()

# Submit by clicking the submit button
page = browser.click(15)
```

## Compound actions

For detected form patterns, compound actions handle the full workflow:

```python
# Login (requires Login suggested action on current page)
page = browser.login("user@example.com", "password123")

# Enter verification code (requires EnterCode suggested action)
page = browser.enter_code("123456")
```

## Search

```python
# Search the web (DuckDuckGo by default)
results = browser.search("python web scraping")
for r in results:
    print(r["title"], r["url"], r["snippet"])
```

## Finding elements

```python
# Find by text content (exact substring match)
elements = browser.find_by_text("Sign In")

# Find by text content (case-insensitive substring)
elements = browser.find_by_text_fuzzy("sign in")

# Find by ARIA role
buttons = browser.find_by_role("button")
headings = browser.find_by_role("heading")
links = browser.find_by_role("link")

# Find input by semantic purpose
password_input = browser.find_input_by_purpose("password")
email_input = browser.find_input_by_purpose("email")
search_input = browser.find_input_by_purpose("search")
# Supported purposes: "password", "email", "username", "code", "search", "phone"

# Find verification codes on the page
code = browser.find_verification_code()  # str or None
```

## Navigation

```python
# Navigate to a URL
page = browser.goto("https://example.com")

# Click a link (navigates to its href)
page = browser.click(3)

# Go back
page = browser.back()
```

## Suggested actions

```python
page = browser.goto("https://example.com/login")

for action in page.suggested_actions():
    print(action)
    # {"action": "Login", "username_id": 5, "password_id": 8, "submit_id": 12}
```

Each action is a dictionary with an `"action"` key identifying the type and additional fields with element IDs. See the [Action Recipes Reference](ref-action-recipes.md) for all variants.

## Viewport configuration

```python
# Mobile viewport
browser = Browser(viewport_width=375, viewport_height=812)

# Desktop viewport (default)
browser = Browser(viewport_width=1920, viewport_height=1080)
```

The viewport dimensions affect CSS media query evaluation and layout computation, which in turn affects element positions and visibility.
