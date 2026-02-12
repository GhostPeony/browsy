# Framework Integrations & Protocol Support — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Expand browsy from MCP-only into a universal agent browser with REST API, A2A protocol, and framework integrations for LangChain, CrewAI, OpenAI, AutoGen, and Smolagents.

**Architecture:** New `crates/server/` Rust crate (axum) for REST+A2A. Python integrations move from `examples/` into `crates/python/browsy/_integrations/` as optional extras. All Python integrations call PyO3 bindings directly (no subprocess overhead).

**Tech Stack:** Rust (axum, tokio, serde), Python (PyO3/maturin, pytest), frameworks (langchain, crewai, openai, pyautogen, smolagents)

**Design doc:** `docs/plans/2026-02-11-framework-integrations-design.md`

---

## Risk Notes / Caveats

These items can cause issues if not addressed during implementation:

1) **Tooling mismatch (required sub-skill)**  
   The plan requires a sub-skill (`superpowers:executing-plans`) that does not exist in the current tooling list. Implementations will fail or stall unless that requirement is removed or the skill is available.

2) **Python extras volatility**  
   Integrations for langchain/crewai/autogen/smolagents change frequently and break on version mismatches. You should pin minimal compatible versions and validate each extra in CI to avoid brittle installs.

3) **PyO3 + extras testing**  
   Running `maturin develop` with all extras installed is not realistic for CI. Use a test matrix or per-extra jobs; otherwise you risk hidden dependency conflicts.

4) **Server crate footprint**  
   Adding `axum`/`tokio` to the CLI increases binary size and build complexity. Consider feature-gating the server or using a separate binary to keep the core CLI lean.

5) **A2A endpoint scope**  
   The proposed A2A task handling is minimal (URL parsing + basic actions) and may not meet real A2A expectations. Document the scope or delay until a stronger contract is defined.

6) **Session storage limits**  
   In-memory session storage with timeouts is fine for v1 but not durable or horizontally scalable. Document this and add limits to avoid unbounded growth.

7) **Security enforcement**  
   Every REST handler must consistently enforce private-network blocking and URL allow-listing. This is easy to miss across multiple endpoints—centralize checks if possible.

8) **Example deprecation**  
   Deprecating examples without a migration note can break users. Add a short migration section or update the README.

---

## Task 1: Python Package Restructuring

Create the `_integrations` module and proxy imports so `from browsy.langchain import ...` works.

**Files:**
- Create: `crates/python/browsy/_integrations/__init__.py`
- Create: `crates/python/browsy/langchain.py` (proxy)
- Create: `crates/python/browsy/crewai.py` (proxy)
- Create: `crates/python/browsy/openai.py` (proxy)
- Create: `crates/python/browsy/autogen.py` (proxy)
- Create: `crates/python/browsy/smolagents.py` (proxy)
- Modify: `crates/python/pyproject.toml`

**Step 1: Create the _integrations package**

```python
# crates/python/browsy/_integrations/__init__.py
"""Framework integrations for browsy. Import via browsy.<framework> instead."""
```

**Step 2: Create proxy modules for clean imports**

Each top-level proxy module (e.g. `browsy/langchain.py`) re-exports from `browsy._integrations.langchain`. This gives users clean imports like `from browsy.langchain import get_tools`.

```python
# crates/python/browsy/langchain.py
"""browsy LangChain integration. Install: pip install browsy[langchain]"""
from browsy._integrations.langchain import *  # noqa: F401,F403
from browsy._integrations.langchain import get_tools, BrowsyBrowseTool, BrowsyClickTool, BrowsyTypeTextTool, BrowsySearchTool, BrowsyLoginTool, BrowsyPageInfoTool
```

```python
# crates/python/browsy/crewai.py
"""browsy CrewAI integration. Install: pip install browsy[crewai]"""
from browsy._integrations.crewai import *  # noqa: F401,F403
from browsy._integrations.crewai import BrowsyTool
```

```python
# crates/python/browsy/openai.py
"""browsy OpenAI integration. Install: pip install browsy[openai]"""
from browsy._integrations.openai import *  # noqa: F401,F403
from browsy._integrations.openai import get_tool_definitions, handle_tool_call
```

```python
# crates/python/browsy/autogen.py
"""browsy AutoGen integration. Install: pip install browsy[autogen]"""
from browsy._integrations.autogen import *  # noqa: F401,F403
from browsy._integrations.autogen import BrowsyBrowser
```

```python
# crates/python/browsy/smolagents.py
"""browsy Smolagents integration. Install: pip install browsy[smolagents]"""
from browsy._integrations.smolagents import *  # noqa: F401,F403
from browsy._integrations.smolagents import BrowsyTool
```

**Step 3: Update pyproject.toml with extras**

Add `[project.optional-dependencies]` to `crates/python/pyproject.toml`:

```toml
[project.optional-dependencies]
langchain = ["langchain>=0.1"]
crewai = ["crewai>=0.40"]
openai = ["openai>=1.0"]
autogen = ["pyautogen>=0.2"]
smolagents = ["smolagents>=1.0"]
all = ["browsy[langchain]", "browsy[crewai]", "browsy[openai]", "browsy[autogen]", "browsy[smolagents]"]
```

**Step 4: Verify the package builds**

Run: `cd crates/python && maturin develop`
Expected: Build succeeds, `browsy` importable

**Step 5: Commit**

```bash
git add crates/python/browsy/_integrations/__init__.py crates/python/browsy/langchain.py crates/python/browsy/crewai.py crates/python/browsy/openai.py crates/python/browsy/autogen.py crates/python/browsy/smolagents.py crates/python/pyproject.toml
git commit -m "feat: add integration package structure and extras to pyproject.toml"
```

---

## Task 2: Shared Browser Helper

Create a helper module that all integrations use for shared browser instance management and page formatting. Eliminates duplicated `_format_page()` across integrations.

**Files:**
- Create: `crates/python/browsy/_integrations/_shared.py`

**Step 1: Write the shared helper**

```python
# crates/python/browsy/_integrations/_shared.py
"""Shared utilities for browsy framework integrations."""

from browsy import Browser

# Module-level default browser (lazy-initialized)
_default_browser = None


def get_browser(browser=None):
    """Return the given browser or a shared default instance."""
    global _default_browser
    if browser is not None:
        return browser
    if _default_browser is None:
        _default_browser = Browser()
    return _default_browser


def format_page(page):
    """Format a Page into a compact string with page intelligence."""
    result = f"title: {page.title}\nurl: {page.url}\npage_type: {page.page_type}\n"
    actions = page.suggested_actions()
    if actions:
        result += "suggested_actions:\n"
        for a in actions:
            result += f"  {a}\n"
    result += f"---\n{page.to_compact()}"
    return result


def format_search_results(results):
    """Format search results into a readable string."""
    lines = []
    for i, r in enumerate(results, 1):
        lines.append(f"{i}. {r['title']}")
        lines.append(f"   {r['url']}")
        if r.get("snippet"):
            lines.append(f"   {r['snippet']}")
        lines.append("")
    return "\n".join(lines) if lines else "No results found."
```

**Step 2: Commit**

```bash
git add crates/python/browsy/_integrations/_shared.py
git commit -m "feat: add shared browser helper for integrations"
```

---

## Task 3: LangChain Integration

Move `examples/langchain_tool.py` into the package, refactored to use the shared helper and accept a custom `Browser` instance.

**Files:**
- Create: `crates/python/browsy/_integrations/langchain.py`
- Create: `crates/python/tests/test_langchain.py`

**Step 1: Write the failing test**

```python
# crates/python/tests/test_langchain.py
"""Tests for browsy LangChain integration."""
import pytest
from browsy import Browser

SIMPLE_HTML = '<html><head><title>Test</title></head><body><h1>Hello</h1><a href="/about">About</a></body></html>'
LOGIN_HTML = '<html><head><title>Login</title></head><body><form action="/login" method="post"><input type="text" name="username" /><input type="password" name="password" /><button type="submit">Log In</button></form></body></html>'

try:
    from browsy.langchain import get_tools, BrowsyBrowseTool, BrowsyPageInfoTool
    HAS_LANGCHAIN = True
except ImportError:
    HAS_LANGCHAIN = False

pytestmark = pytest.mark.skipif(not HAS_LANGCHAIN, reason="langchain not installed")


@pytest.fixture
def browser():
    return Browser()


def test_get_tools_returns_six(browser):
    tools = get_tools(browser=browser)
    assert len(tools) == 6


def test_get_tools_default_browser():
    tools = get_tools()
    assert len(tools) == 6


def test_browse_tool_with_load_html(browser):
    browser.load_html(SIMPLE_HTML, "https://example.com")
    tools = get_tools(browser=browser)
    browse_tool = [t for t in tools if t.name == "browsy_browse"][0]
    # Can't call _run with load_html (it needs a URL), so test page_info instead
    info_tool = [t for t in tools if t.name == "browsy_page_info"][0]
    result = info_tool._run()
    assert "Test" in result


def test_tool_names(browser):
    tools = get_tools(browser=browser)
    names = {t.name for t in tools}
    assert names == {
        "browsy_browse", "browsy_click", "browsy_type_text",
        "browsy_search", "browsy_login", "browsy_page_info",
    }
```

**Step 2: Run test to verify it fails**

Run: `cd crates/python && python -m pytest tests/test_langchain.py -v`
Expected: ImportError or FAIL (module doesn't exist yet)

**Step 3: Write the LangChain integration**

```python
# crates/python/browsy/_integrations/langchain.py
"""browsy LangChain integration — drop-in web browsing tools for LangChain agents.

Install: pip install browsy[langchain]

Usage:
    from browsy.langchain import get_tools
    tools = get_tools()
    agent = create_react_agent(llm, tools)
"""

try:
    from pydantic import BaseModel, Field
    from langchain.tools import BaseTool
except ImportError:
    raise ImportError(
        "LangChain is required for this integration. "
        "Install it with: pip install browsy[langchain]"
    )

from typing import Optional, Type
from browsy._integrations._shared import get_browser, format_page, format_search_results


class BrowseInput(BaseModel):
    url: str = Field(description="URL to navigate to")


class ClickInput(BaseModel):
    element_id: int = Field(description="Element ID to click")


class TypeTextInput(BaseModel):
    element_id: int = Field(description="Element ID of the text input")
    text: str = Field(description="Text to type into the input")


class SearchInput(BaseModel):
    query: str = Field(description="Search query")


class LoginInput(BaseModel):
    username: str = Field(description="Username or email")
    password: str = Field(description="Password")


class BrowsyBrowseTool(BaseTool):
    """Navigate to a URL and return page content with page intelligence."""

    name: str = "browsy_browse"
    description: str = (
        "Navigate to a URL and return the page content. Returns page type "
        "(Login, Search, Form, Article, List, etc.), suggested actions with "
        "element IDs, and all interactive elements."
    )
    args_schema: Type[BaseModel] = BrowseInput
    _browser: object = None

    def __init__(self, browser=None, **kwargs):
        super().__init__(**kwargs)
        self._browser = browser

    def _run(self, url: str) -> str:
        browser = get_browser(self._browser)
        page = browser.goto(url)
        return format_page(page)


class BrowsyClickTool(BaseTool):
    """Click an element by its ID."""

    name: str = "browsy_click"
    description: str = (
        "Click an element by its ID. Links navigate to new pages, buttons "
        "submit forms. Returns the resulting page content."
    )
    args_schema: Type[BaseModel] = ClickInput
    _browser: object = None

    def __init__(self, browser=None, **kwargs):
        super().__init__(**kwargs)
        self._browser = browser

    def _run(self, element_id: int) -> str:
        browser = get_browser(self._browser)
        page = browser.click(element_id)
        return format_page(page)


class BrowsyTypeTextTool(BaseTool):
    """Type text into an input field or textarea."""

    name: str = "browsy_type_text"
    description: str = (
        "Type text into an input field or textarea by element ID. "
        "Use browsy_browse first to find the element ID."
    )
    args_schema: Type[BaseModel] = TypeTextInput
    _browser: object = None

    def __init__(self, browser=None, **kwargs):
        super().__init__(**kwargs)
        self._browser = browser

    def _run(self, element_id: int, text: str) -> str:
        browser = get_browser(self._browser)
        browser.type_text(element_id, text)
        return f"Typed '{text}' into element {element_id}"


class BrowsySearchTool(BaseTool):
    """Search the web using DuckDuckGo."""

    name: str = "browsy_search"
    description: str = (
        "Search the web and return structured results with title, URL, and "
        "snippet. Uses DuckDuckGo. No API key needed."
    )
    args_schema: Type[BaseModel] = SearchInput
    _browser: object = None

    def __init__(self, browser=None, **kwargs):
        super().__init__(**kwargs)
        self._browser = browser

    def _run(self, query: str) -> str:
        browser = get_browser(self._browser)
        results = browser.search(query)
        return format_search_results(results)


class BrowsyLoginTool(BaseTool):
    """Log in using detected login form fields."""

    name: str = "browsy_login"
    description: str = (
        "Log in to the current page using detected login form fields. "
        "Requires a page with a login form loaded (use browsy_browse first)."
    )
    args_schema: Type[BaseModel] = LoginInput
    _browser: object = None

    def __init__(self, browser=None, **kwargs):
        super().__init__(**kwargs)
        self._browser = browser

    def _run(self, username: str, password: str) -> str:
        browser = get_browser(self._browser)
        page = browser.login(username, password)
        return format_page(page)


class BrowsyPageInfoTool(BaseTool):
    """Get page metadata: type, actions, alerts, pagination."""

    name: str = "browsy_page_info"
    description: str = (
        "Get metadata about the current page: page type, suggested actions "
        "(login/search/consent), alerts, and pagination info."
    )
    _browser: object = None

    def __init__(self, browser=None, **kwargs):
        super().__init__(**kwargs)
        self._browser = browser

    def _run(self) -> str:
        browser = get_browser(self._browser)
        page = browser.dom()
        if page is None:
            return "No page loaded. Use browsy_browse first."
        result = f"title: {page.title}\nurl: {page.url}\npage_type: {page.page_type}\n"
        actions = page.suggested_actions()
        if actions:
            result += "suggested_actions:\n"
            for action in actions:
                result += f"  {action}\n"
        return result


def get_tools(browser=None):
    """Return all browsy tools for use with a LangChain agent.

    Args:
        browser: Optional Browser instance. If None, a shared default is used.
    """
    return [
        BrowsyBrowseTool(browser=browser),
        BrowsyClickTool(browser=browser),
        BrowsyTypeTextTool(browser=browser),
        BrowsySearchTool(browser=browser),
        BrowsyLoginTool(browser=browser),
        BrowsyPageInfoTool(browser=browser),
    ]
```

**Step 4: Run test to verify it passes**

Run: `cd crates/python && python -m pytest tests/test_langchain.py -v`
Expected: PASS (or SKIP if langchain not installed)

**Step 5: Commit**

```bash
git add crates/python/browsy/_integrations/langchain.py crates/python/tests/test_langchain.py
git commit -m "feat: add LangChain integration module with tests"
```

---

## Task 4: CrewAI Integration

Move `examples/crewai_tool.py` into the package, refactored with shared helper and injectable browser.

**Files:**
- Create: `crates/python/browsy/_integrations/crewai.py`
- Create: `crates/python/tests/test_crewai.py`

**Step 1: Write the failing test**

```python
# crates/python/tests/test_crewai.py
"""Tests for browsy CrewAI integration."""
import pytest
from browsy import Browser

try:
    from browsy.crewai import BrowsyTool
    HAS_CREWAI = True
except ImportError:
    HAS_CREWAI = False

pytestmark = pytest.mark.skipif(not HAS_CREWAI, reason="crewai not installed")

SIMPLE_HTML = '<html><head><title>Test</title></head><body><h1>Hello</h1><a href="/about">About</a></body></html>'


@pytest.fixture
def tool():
    return BrowsyTool(browser=Browser())


def test_tool_name(tool):
    assert tool.name == "browsy_web_browser"


def test_info_no_page(tool):
    result = tool._run("info")
    assert "No page loaded" in result


def test_unknown_command(tool):
    result = tool._run("foobar")
    assert "Unknown command" in result


def test_invalid_click_id(tool):
    tool._browser.load_html(SIMPLE_HTML, "https://example.com")
    result = tool._run("click notanumber")
    assert "invalid element ID" in result
```

**Step 2: Run test to verify it fails**

Run: `cd crates/python && python -m pytest tests/test_crewai.py -v`
Expected: FAIL

**Step 3: Write the CrewAI integration**

```python
# crates/python/browsy/_integrations/crewai.py
"""browsy CrewAI integration — single-tool web browser for CrewAI agents.

Install: pip install browsy[crewai]

Usage:
    from browsy.crewai import BrowsyTool
    agent = Agent(tools=[BrowsyTool()])
"""

try:
    from crewai.tools import BaseTool
except ImportError:
    raise ImportError(
        "CrewAI is required for this integration. "
        "Install it with: pip install browsy[crewai]"
    )

from typing import Optional
from browsy._integrations._shared import get_browser, format_page, format_search_results


class BrowsyTool(BaseTool):
    """Browse websites using browsy's zero-render engine.

    Commands: browse <url>, click <id>, type <id> <text>,
    search <query>, login <user> <pass>, info, back
    """

    name: str = "browsy_web_browser"
    description: str = (
        "Browse websites without launching a browser. Navigates to URLs, detects "
        "page types (Login, Search, Form, Article, List, Captcha), provides action "
        "recipes with element IDs, and exposes hidden content.\n\n"
        "Commands:\n"
        "  browse <url> -- Navigate to a URL\n"
        "  click <id> -- Click an element\n"
        "  type <id> <text> -- Type text into an input\n"
        "  search <query> -- Search the web\n"
        "  login <username> <password> -- Log in using detected form\n"
        "  info -- Get page type and suggested actions\n"
        "  back -- Go back to previous page\n"
    )

    _browser: Optional[object] = None

    def __init__(self, browser=None, **kwargs):
        super().__init__(**kwargs)
        self._browser = browser

    def _run(self, command: str) -> str:
        browser = get_browser(self._browser)
        parts = command.strip().split(None, 2)
        if not parts:
            return "Error: empty command. Use 'browse <url>', 'click <id>', etc."

        action = parts[0].lower()

        if action == "browse" and len(parts) >= 2:
            page = browser.goto(parts[1])
            return format_page(page)

        elif action == "click" and len(parts) >= 2:
            try:
                element_id = int(parts[1])
            except ValueError:
                return f"Error: invalid element ID '{parts[1]}'"
            page = browser.click(element_id)
            return format_page(page)

        elif action == "type" and len(parts) >= 3:
            try:
                element_id = int(parts[1])
            except ValueError:
                return f"Error: invalid element ID '{parts[1]}'"
            browser.type_text(element_id, parts[2])
            return f"Typed '{parts[2]}' into element {element_id}"

        elif action == "search" and len(parts) >= 2:
            query = " ".join(parts[1:])
            results = browser.search(query)
            return format_search_results(results)

        elif action == "login" and len(parts) >= 3:
            page = browser.login(parts[1], parts[2])
            return format_page(page)

        elif action == "info":
            page = browser.dom()
            if page is None:
                return "No page loaded. Use 'browse <url>' first."
            result = f"page_type: {page.page_type}\n"
            actions = page.suggested_actions()
            if actions:
                result += "suggested_actions:\n"
                for a in actions:
                    result += f"  {a}\n"
            return result

        elif action == "back":
            page = browser.back()
            return format_page(page)

        else:
            return (
                f"Unknown command '{action}'. Available: "
                "browse, click, type, search, login, info, back"
            )
```

**Step 4: Run test to verify it passes**

Run: `cd crates/python && python -m pytest tests/test_crewai.py -v`
Expected: PASS (or SKIP)

**Step 5: Commit**

```bash
git add crates/python/browsy/_integrations/crewai.py crates/python/tests/test_crewai.py
git commit -m "feat: add CrewAI integration module with tests"
```

---

## Task 5: OpenAI Integration

Move `examples/openai_functions.py` into the package with injectable browser.

**Files:**
- Create: `crates/python/browsy/_integrations/openai.py`
- Create: `crates/python/tests/test_openai.py`

**Step 1: Write the failing test**

```python
# crates/python/tests/test_openai.py
"""Tests for browsy OpenAI integration."""
import pytest
from browsy import Browser

try:
    from browsy.openai import get_tool_definitions, handle_tool_call
    HAS_OPENAI = True
except ImportError:
    HAS_OPENAI = False

pytestmark = pytest.mark.skipif(not HAS_OPENAI, reason="openai not installed")


def test_tool_definitions_count():
    defs = get_tool_definitions()
    assert len(defs) == 6


def test_tool_definitions_shape():
    defs = get_tool_definitions()
    for d in defs:
        assert d["type"] == "function"
        assert "name" in d["function"]
        assert "description" in d["function"]
        assert "parameters" in d["function"]


def test_tool_definition_names():
    defs = get_tool_definitions()
    names = {d["function"]["name"] for d in defs}
    assert names == {
        "browsy_browse", "browsy_click", "browsy_type_text",
        "browsy_search", "browsy_login", "browsy_page_info",
    }


def test_handle_unknown_function():
    result = handle_tool_call("nonexistent", {})
    assert "Unknown function" in result


def test_handle_page_info_no_page():
    result = handle_tool_call("browsy_page_info", {}, browser=Browser())
    assert "No page loaded" in result
```

**Step 2: Run test to verify it fails**

Run: `cd crates/python && python -m pytest tests/test_openai.py -v`
Expected: FAIL

**Step 3: Write the OpenAI integration**

```python
# crates/python/browsy/_integrations/openai.py
"""browsy OpenAI function calling integration.

Install: pip install browsy[openai]

Usage:
    from browsy.openai import get_tool_definitions, handle_tool_call
    tools = get_tool_definitions()
    response = client.chat.completions.create(model="gpt-4", tools=tools, ...)
    result = handle_tool_call(function_name, arguments)
"""

from browsy._integrations._shared import get_browser, format_page, format_search_results


def get_tool_definitions():
    """Return OpenAI-compatible tool definitions for browsy."""
    return [
        {
            "type": "function",
            "function": {
                "name": "browsy_browse",
                "description": (
                    "Navigate to a URL and return page content with page intelligence. "
                    "Returns page type (Login, Search, Form, etc.), suggested actions "
                    "with element IDs, and all interactive elements."
                ),
                "parameters": {
                    "type": "object",
                    "properties": {
                        "url": {"type": "string", "description": "URL to navigate to"}
                    },
                    "required": ["url"],
                },
            },
        },
        {
            "type": "function",
            "function": {
                "name": "browsy_click",
                "description": "Click an element by its ID. Links navigate, buttons submit forms.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "element_id": {"type": "integer", "description": "Element ID to click"}
                    },
                    "required": ["element_id"],
                },
            },
        },
        {
            "type": "function",
            "function": {
                "name": "browsy_type_text",
                "description": "Type text into an input field or textarea by element ID.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "element_id": {"type": "integer", "description": "Element ID of the text input"},
                        "text": {"type": "string", "description": "Text to type"},
                    },
                    "required": ["element_id", "text"],
                },
            },
        },
        {
            "type": "function",
            "function": {
                "name": "browsy_search",
                "description": "Search the web and return results with title, URL, and snippet.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "query": {"type": "string", "description": "Search query"}
                    },
                    "required": ["query"],
                },
            },
        },
        {
            "type": "function",
            "function": {
                "name": "browsy_login",
                "description": "Log in using detected login form fields on the current page.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "username": {"type": "string", "description": "Username or email"},
                        "password": {"type": "string", "description": "Password"},
                    },
                    "required": ["username", "password"],
                },
            },
        },
        {
            "type": "function",
            "function": {
                "name": "browsy_page_info",
                "description": "Get current page metadata: type, suggested actions, alerts.",
                "parameters": {"type": "object", "properties": {}},
            },
        },
    ]


def handle_tool_call(function_name, arguments, browser=None):
    """Handle a tool call from the OpenAI API and return the result string."""
    b = get_browser(browser)

    if function_name == "browsy_browse":
        page = b.goto(arguments["url"])
        return format_page(page)

    elif function_name == "browsy_click":
        page = b.click(arguments["element_id"])
        return format_page(page)

    elif function_name == "browsy_type_text":
        b.type_text(arguments["element_id"], arguments["text"])
        return f"Typed '{arguments['text']}' into element {arguments['element_id']}"

    elif function_name == "browsy_search":
        results = b.search(arguments["query"])
        return format_search_results(results)

    elif function_name == "browsy_login":
        page = b.login(arguments["username"], arguments["password"])
        return format_page(page)

    elif function_name == "browsy_page_info":
        page = b.dom()
        if page is None:
            return "No page loaded."
        result = f"page_type: {page.page_type}\n"
        actions = page.suggested_actions()
        if actions:
            for a in actions:
                result += f"  {a}\n"
        return result

    else:
        return f"Unknown function: {function_name}"
```

**Step 4: Run test to verify it passes**

Run: `cd crates/python && python -m pytest tests/test_openai.py -v`
Expected: PASS

Note: The OpenAI integration doesn't actually import `openai` — it only provides schemas and a handler. The `openai` extra exists so users know they need the openai SDK for the outer agent loop. The integration tests don't need `openai` installed.

**Step 5: Commit**

```bash
git add crates/python/browsy/_integrations/openai.py crates/python/tests/test_openai.py
git commit -m "feat: add OpenAI function calling integration with tests"
```

---

## Task 6: AutoGen Integration

New integration. A `ConversableAgent` subclass that registers browsing functions for AutoGen group chats.

**Files:**
- Create: `crates/python/browsy/_integrations/autogen.py`
- Create: `crates/python/tests/test_autogen.py`

**Step 1: Write the failing test**

```python
# crates/python/tests/test_autogen.py
"""Tests for browsy AutoGen integration."""
import pytest
from browsy import Browser

try:
    from browsy.autogen import BrowsyBrowser, get_browsy_functions
    HAS_AUTOGEN = True
except ImportError:
    HAS_AUTOGEN = False

pytestmark = pytest.mark.skipif(not HAS_AUTOGEN, reason="pyautogen not installed")


def test_get_functions_count():
    funcs = get_browsy_functions()
    assert len(funcs) == 6


def test_get_functions_names():
    funcs = get_browsy_functions()
    names = {f["name"] for f in funcs}
    assert "browsy_browse" in names
    assert "browsy_click" in names
    assert "browsy_page_info" in names


def test_browser_agent_creation():
    agent = BrowsyBrowser(name="test_browser", browser=Browser())
    assert agent.name == "test_browser"
```

**Step 2: Run test to verify it fails**

Run: `cd crates/python && python -m pytest tests/test_autogen.py -v`
Expected: FAIL

**Step 3: Write the AutoGen integration**

```python
# crates/python/browsy/_integrations/autogen.py
"""browsy AutoGen integration — browsing agent for AutoGen group chats.

Install: pip install browsy[autogen]

Usage:
    from browsy.autogen import BrowsyBrowser
    browser_agent = BrowsyBrowser(name="browser")
    # Add to AutoGen group chat
"""

try:
    from autogen import ConversableAgent
except ImportError:
    raise ImportError(
        "AutoGen is required for this integration. "
        "Install it with: pip install browsy[autogen]"
    )

from browsy._integrations._shared import get_browser, format_page, format_search_results


def get_browsy_functions(browser=None):
    """Return AutoGen-compatible function definitions for browsy."""
    b = get_browser(browser)

    def browsy_browse(url: str) -> str:
        """Navigate to a URL and return page content."""
        page = b.goto(url)
        return format_page(page)

    def browsy_click(element_id: int) -> str:
        """Click an element by its ID."""
        page = b.click(element_id)
        return format_page(page)

    def browsy_type_text(element_id: int, text: str) -> str:
        """Type text into an input field by element ID."""
        b.type_text(element_id, text)
        return f"Typed '{text}' into element {element_id}"

    def browsy_search(query: str) -> str:
        """Search the web and return results."""
        results = b.search(query)
        return format_search_results(results)

    def browsy_login(username: str, password: str) -> str:
        """Log in using detected login form fields."""
        page = b.login(username, password)
        return format_page(page)

    def browsy_page_info() -> str:
        """Get current page metadata."""
        page = b.dom()
        if page is None:
            return "No page loaded."
        result = f"page_type: {page.page_type}\n"
        actions = page.suggested_actions()
        if actions:
            for a in actions:
                result += f"  {a}\n"
        return result

    return [
        {"name": "browsy_browse", "func": browsy_browse, "description": "Navigate to a URL and return page content with page intelligence."},
        {"name": "browsy_click", "func": browsy_click, "description": "Click an element by its ID."},
        {"name": "browsy_type_text", "func": browsy_type_text, "description": "Type text into an input field."},
        {"name": "browsy_search", "func": browsy_search, "description": "Search the web."},
        {"name": "browsy_login", "func": browsy_login, "description": "Log in using detected form fields."},
        {"name": "browsy_page_info", "func": browsy_page_info, "description": "Get page metadata."},
    ]


class BrowsyBrowser(ConversableAgent):
    """AutoGen agent that can browse the web using browsy.

    Add to group chats so other agents can request web browsing.
    """

    def __init__(self, name="browsy_browser", browser=None, **kwargs):
        system_message = kwargs.pop("system_message", (
            "You are a web browsing agent. Use your browsing tools to navigate "
            "websites, fill forms, and extract content as requested."
        ))
        super().__init__(name=name, system_message=system_message, **kwargs)
        funcs = get_browsy_functions(browser=browser)
        for f in funcs:
            self.register_for_llm(name=f["name"], description=f["description"])(f["func"])
```

**Step 4: Run test to verify it passes**

Run: `cd crates/python && python -m pytest tests/test_autogen.py -v`
Expected: PASS (or SKIP)

**Step 5: Commit**

```bash
git add crates/python/browsy/_integrations/autogen.py crates/python/tests/test_autogen.py
git commit -m "feat: add AutoGen integration with tests"
```

---

## Task 7: Smolagents Integration

New integration. HuggingFace's lightweight agent framework.

**Files:**
- Create: `crates/python/browsy/_integrations/smolagents.py`
- Create: `crates/python/tests/test_smolagents.py`

**Step 1: Write the failing test**

```python
# crates/python/tests/test_smolagents.py
"""Tests for browsy Smolagents integration."""
import pytest
from browsy import Browser

try:
    from browsy.smolagents import BrowsyTool
    HAS_SMOLAGENTS = True
except ImportError:
    HAS_SMOLAGENTS = False

pytestmark = pytest.mark.skipif(not HAS_SMOLAGENTS, reason="smolagents not installed")

SIMPLE_HTML = '<html><head><title>Test</title></head><body><h1>Hello</h1></body></html>'


def test_tool_name():
    tool = BrowsyTool(browser=Browser())
    assert tool.name == "web_browser"


def test_tool_has_inputs():
    tool = BrowsyTool(browser=Browser())
    assert "action" in tool.inputs


def test_info_no_page():
    tool = BrowsyTool(browser=Browser())
    result = tool.forward("info")
    assert "No page loaded" in result


def test_unknown_command():
    tool = BrowsyTool(browser=Browser())
    result = tool.forward("foobar")
    assert "Unknown command" in result
```

**Step 2: Run test to verify it fails**

Run: `cd crates/python && python -m pytest tests/test_smolagents.py -v`
Expected: FAIL

**Step 3: Write the Smolagents integration**

```python
# crates/python/browsy/_integrations/smolagents.py
"""browsy Smolagents integration — web browser tool for HuggingFace agents.

Install: pip install browsy[smolagents]

Usage:
    from browsy.smolagents import BrowsyTool
    agent = CodeAgent(tools=[BrowsyTool()])
"""

try:
    from smolagents import Tool
except ImportError:
    raise ImportError(
        "Smolagents is required for this integration. "
        "Install it with: pip install browsy[smolagents]"
    )

from browsy._integrations._shared import get_browser, format_page, format_search_results


class BrowsyTool(Tool):
    """Browse websites using browsy's zero-render engine."""

    name = "web_browser"
    description = (
        "Browse websites without launching a browser. Supports navigation, "
        "clicking, typing, search, and login.\n\n"
        "Commands:\n"
        "  browse <url> -- Navigate to a URL\n"
        "  click <id> -- Click an element\n"
        "  type <id> <text> -- Type text into an input\n"
        "  search <query> -- Search the web\n"
        "  login <username> <password> -- Log in\n"
        "  info -- Get page type and suggested actions\n"
        "  back -- Go back\n"
    )
    inputs = {
        "action": {
            "type": "string",
            "description": "Command to execute (e.g. 'browse https://example.com')",
        }
    }
    output_type = "string"

    def __init__(self, browser=None, **kwargs):
        super().__init__(**kwargs)
        self._browser = browser

    def forward(self, action: str) -> str:
        browser = get_browser(self._browser)
        parts = action.strip().split(None, 2)
        if not parts:
            return "Error: empty command."

        cmd = parts[0].lower()

        if cmd == "browse" and len(parts) >= 2:
            page = browser.goto(parts[1])
            return format_page(page)

        elif cmd == "click" and len(parts) >= 2:
            try:
                eid = int(parts[1])
            except ValueError:
                return f"Error: invalid element ID '{parts[1]}'"
            page = browser.click(eid)
            return format_page(page)

        elif cmd == "type" and len(parts) >= 3:
            try:
                eid = int(parts[1])
            except ValueError:
                return f"Error: invalid element ID '{parts[1]}'"
            browser.type_text(eid, parts[2])
            return f"Typed '{parts[2]}' into element {eid}"

        elif cmd == "search" and len(parts) >= 2:
            query = " ".join(parts[1:])
            results = browser.search(query)
            return format_search_results(results)

        elif cmd == "login" and len(parts) >= 3:
            page = browser.login(parts[1], parts[2])
            return format_page(page)

        elif cmd == "info":
            page = browser.dom()
            if page is None:
                return "No page loaded. Use 'browse <url>' first."
            result = f"page_type: {page.page_type}\n"
            actions = page.suggested_actions()
            if actions:
                result += "suggested_actions:\n"
                for a in actions:
                    result += f"  {a}\n"
            return result

        elif cmd == "back":
            page = browser.back()
            return format_page(page)

        else:
            return f"Unknown command '{cmd}'. Available: browse, click, type, search, login, info, back"
```

**Step 4: Run test to verify it passes**

Run: `cd crates/python && python -m pytest tests/test_smolagents.py -v`
Expected: PASS (or SKIP)

**Step 5: Commit**

```bash
git add crates/python/browsy/_integrations/smolagents.py crates/python/tests/test_smolagents.py
git commit -m "feat: add Smolagents integration with tests"
```

---

## Task 8: HTTP Server Crate Setup

Create `crates/server/` with axum, add to workspace, and implement basic health check.

**Files:**
- Create: `crates/server/Cargo.toml`
- Create: `crates/server/src/lib.rs`
- Create: `crates/server/src/main.rs`
- Modify: `Cargo.toml` (workspace)
- Modify: `crates/cli/src/main.rs` (add `serve` subcommand)

**Step 1: Create Cargo.toml for the server crate**

```toml
# crates/server/Cargo.toml
[package]
name = "browsy-server"
version = "0.1.0"
edition = "2021"
license = "MIT"
description = "REST API + A2A server for browsy"
repository = "https://github.com/GhostPeony/browsy"

[dependencies]
browsy-core = { path = "../core" }
axum = "0.8"
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
uuid = { version = "1", features = ["v4"] }
tower-http = { version = "0.6", features = ["cors"] }
```

**Step 2: Add to workspace**

In root `Cargo.toml`, change:
```toml
members = ["crates/core", "crates/cli", "crates/python", "crates/mcp"]
```
to:
```toml
members = ["crates/core", "crates/cli", "crates/python", "crates/mcp", "crates/server"]
```

**Step 3: Write the server library with session management and health check**

```rust
// crates/server/src/lib.rs
//! REST API + A2A server for browsy.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::Json,
    routing::{get, post},
    Router,
};
use browsy_core::fetch::{Session, SessionConfig};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A session entry with last-access tracking.
struct SessionEntry {
    session: Session,
    last_access: Instant,
}

/// Shared server state.
pub struct AppState {
    sessions: Mutex<HashMap<String, SessionEntry>>,
    config: ServerConfig,
}

/// Server configuration.
pub struct ServerConfig {
    pub port: u16,
    pub session_timeout: Duration,
    pub allow_private_network: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port: 3847,
            session_timeout: Duration::from_secs(30 * 60),
            allow_private_network: false,
        }
    }
}

impl AppState {
    pub fn new(config: ServerConfig) -> Self {
        Self {
            sessions: Mutex::new(HashMap::new()),
            config,
        }
    }

    /// Get or create a session from the X-Browsy-Session header.
    /// Returns (session_token, was_new).
    fn get_or_create_session(&self, headers: &HeaderMap) -> Result<String, StatusCode> {
        let token = headers
            .get("X-Browsy-Session")
            .and_then(|v| v.to_str().ok())
            .map(String::from);

        let mut sessions = self.sessions.lock().unwrap();

        // Purge expired sessions
        let timeout = self.config.session_timeout;
        sessions.retain(|_, entry| entry.last_access.elapsed() < timeout);

        if let Some(ref t) = token {
            if sessions.contains_key(t) {
                sessions.get_mut(t).unwrap().last_access = Instant::now();
                return Ok(t.clone());
            }
        }

        // Create new session
        let mut session_config = SessionConfig::default();
        session_config.allow_private_network = self.config.allow_private_network;
        let session = Session::with_config(session_config)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let new_token = Uuid::new_v4().to_string();
        sessions.insert(new_token.clone(), SessionEntry {
            session,
            last_access: Instant::now(),
        });
        Ok(new_token)
    }
}

/// Build the axum router.
pub fn build_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(health))
        .with_state(state)
}

async fn health() -> &'static str {
    "ok"
}
```

**Step 4: Write main.rs**

```rust
// crates/server/src/main.rs
use std::sync::Arc;
use browsy_server::{AppState, ServerConfig, build_router};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ServerConfig::default();
    let port = config.port;
    let state = Arc::new(AppState::new(config));
    let app = build_router(state);

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(async move {
            let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await?;
            eprintln!("browsy server listening on http://localhost:{port}");
            axum::serve(listener, app).await?;
            Ok::<(), Box<dyn std::error::Error>>(())
        })?;

    Ok(())
}
```

**Step 5: Verify it compiles**

Run: `cargo build -p browsy-server`
Expected: Builds successfully

**Step 6: Commit**

```bash
git add crates/server/Cargo.toml crates/server/src/lib.rs crates/server/src/main.rs Cargo.toml
git commit -m "feat: add browsy-server crate with health endpoint and session management"
```

---

## Task 9: REST API Endpoints

Implement all 14 REST endpoints, reusing the same logic patterns from the MCP server.

**Files:**
- Modify: `crates/server/src/lib.rs`

**Step 1: Write the failing test**

Create `crates/server/tests/api.rs`:

```rust
//! REST API integration tests.

use axum::http::StatusCode;
use axum_test::TestServer;
use browsy_server::{AppState, ServerConfig, build_router};
use serde_json::json;
use std::sync::Arc;

fn test_app() -> TestServer {
    let state = Arc::new(AppState::new(ServerConfig::default()));
    let app = build_router(state);
    TestServer::new(app).unwrap()
}

#[tokio::test]
async fn test_health() {
    let server = test_app();
    let resp = server.get("/health").await;
    resp.assert_status_ok();
    resp.assert_text("ok");
}

#[tokio::test]
async fn test_browse_missing_url() {
    let server = test_app();
    let resp = server.post("/api/browse").json(&json!({})).await;
    resp.assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_page_no_session() {
    let server = test_app();
    let resp = server.get("/api/page").await;
    // Should return error since no page is loaded
    resp.assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_session_header_returned() {
    let server = test_app();
    let resp = server.get("/api/page").await;
    // Even on error, session header should be present
    assert!(resp.header("X-Browsy-Session").len() > 0);
}
```

Add `axum-test` as a dev-dependency in `crates/server/Cargo.toml`:

```toml
[dev-dependencies]
axum-test = "16"
tokio = { version = "1", features = ["full", "test-util"] }
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p browsy-server`
Expected: FAIL (routes don't exist yet)

**Step 3: Implement all REST endpoints**

Add to `crates/server/src/lib.rs` — the full set of endpoints. Key patterns copied from `crates/mcp/src/lib.rs`:

- Request structs reuse the same field names as MCP params
- `format_page()`, `apply_scope()`, `captcha_warning()` copied from MCP lib (or extracted to a shared module)
- Session lookup via `X-Browsy-Session` header on every request
- Response includes `X-Browsy-Session` header

Each handler follows the pattern:
1. Extract session token from header (or create new)
2. Lock the session
3. Call the corresponding `Session` method
4. Apply scope/format as needed
5. Return JSON response with session header

The exact endpoint implementations mirror the MCP tool handlers in `crates/mcp/src/lib.rs:169-364` but return axum responses instead of MCP `CallToolResult`.

**Step 4: Run tests to verify they pass**

Run: `cargo test -p browsy-server`
Expected: PASS

**Step 5: Commit**

```bash
git add crates/server/src/lib.rs crates/server/Cargo.toml crates/server/tests/api.rs
git commit -m "feat: implement all 14 REST API endpoints with session management"
```

---

## Task 10: A2A Agent Card & Task Endpoints

Add A2A protocol support: agent card discovery and task execution.

**Files:**
- Modify: `crates/server/src/lib.rs`
- Create: `crates/server/tests/a2a.rs`

**Step 1: Write the failing test**

```rust
// crates/server/tests/a2a.rs
//! A2A protocol conformance tests.

use axum_test::TestServer;
use browsy_server::{AppState, ServerConfig, build_router};
use serde_json::Value;
use std::sync::Arc;

fn test_app() -> TestServer {
    let state = Arc::new(AppState::new(ServerConfig::default()));
    let app = build_router(state);
    TestServer::new(app).unwrap()
}

#[tokio::test]
async fn test_agent_card_discovery() {
    let server = test_app();
    let resp = server.get("/.well-known/agent.json").await;
    resp.assert_status_ok();
    let card: Value = resp.json();
    assert_eq!(card["name"], "browsy");
    assert!(card["skills"].as_array().unwrap().len() > 0);
    assert!(card["capabilities"]["streaming"].as_bool().unwrap());
}

#[tokio::test]
async fn test_agent_card_has_required_fields() {
    let server = test_app();
    let resp = server.get("/.well-known/agent.json").await;
    let card: Value = resp.json();
    assert!(card["name"].is_string());
    assert!(card["description"].is_string());
    assert!(card["url"].is_string());
    assert!(card["version"].is_string());
    assert!(card["skills"].is_array());
    assert!(card["defaultInputModes"].is_array());
    assert!(card["defaultOutputModes"].is_array());
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p browsy-server --test a2a`
Expected: FAIL

**Step 3: Implement agent card endpoint and task execution**

Add to `crates/server/src/lib.rs`:

- `GET /.well-known/agent.json` — returns the agent card JSON (static, defined in code)
- `POST /a2a/tasks` — accepts a task with a goal string, executes browsing actions, returns result

The agent card is a static JSON value matching the design doc spec. The task endpoint:
1. Parses the goal string for a URL (regex: `https?://\S+`)
2. Browses to the URL
3. Uses `page_type` and `suggested_actions` to determine next steps
4. Returns the final page content as the task result

**Step 4: Run tests to verify they pass**

Run: `cargo test -p browsy-server --test a2a`
Expected: PASS

**Step 5: Commit**

```bash
git add crates/server/src/lib.rs crates/server/tests/a2a.rs
git commit -m "feat: add A2A agent card discovery and task execution"
```

---

## Task 11: Add `serve` Subcommand to CLI

Wire `browsy serve` into the existing CLI binary so the server is launchable from the main browsy command.

**Files:**
- Modify: `crates/cli/Cargo.toml` (add browsy-server dependency)
- Modify: `crates/cli/src/main.rs` (add Serve variant)

**Step 1: Add dependency**

In `crates/cli/Cargo.toml`, add:
```toml
browsy-server = { path = "../server" }
```

**Step 2: Add Serve subcommand**

In `crates/cli/src/main.rs`, add to the `Commands` enum:

```rust
/// Start the REST API + A2A server
Serve {
    /// Port to listen on (default: 3847)
    #[arg(long, default_value = "3847")]
    port: u16,

    /// Allow fetching private/LAN addresses
    #[arg(long)]
    allow_private_network: bool,
},
```

And add the match arm in `main()`:

```rust
Commands::Serve { port, allow_private_network } => {
    let config = browsy_server::ServerConfig {
        port,
        allow_private_network,
        ..Default::default()
    };
    let state = std::sync::Arc::new(browsy_server::AppState::new(config));
    let app = browsy_server::build_router(state);
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to build tokio runtime")
        .block_on(async move {
            let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}"))
                .await
                .expect("Failed to bind");
            eprintln!("browsy server listening on http://localhost:{port}");
            axum::serve(listener, app).await.expect("Server error");
        });
}
```

**Step 3: Add tokio and axum dependencies to CLI**

In `crates/cli/Cargo.toml`:
```toml
tokio = { version = "1", features = ["full"] }
axum = "0.8"
```

**Step 4: Verify it compiles**

Run: `cargo build -p browsy-cli`
Expected: Builds

**Step 5: Commit**

```bash
git add crates/cli/Cargo.toml crates/cli/src/main.rs
git commit -m "feat: add 'browsy serve' subcommand for REST API + A2A server"
```

---

## Task 12: Run Full Test Suite

Verify nothing is broken across all crates.

**Step 1: Run core tests**

Run: `cargo test -p browsy-core`
Expected: All existing tests pass

**Step 2: Run MCP tests**

Run: `cargo test -p browsy-mcp`
Expected: Pass (untouched)

**Step 3: Run server tests**

Run: `cargo test -p browsy-server`
Expected: All new tests pass

**Step 4: Run Python tests**

Run: `cd crates/python && maturin develop && python -m pytest tests/ -v`
Expected: Core tests pass, integration tests pass or skip

**Step 5: Run full build**

Run: `cargo build`
Expected: All crates compile

**Step 6: Commit any fixes if needed**

---

## Task 13: Clean Up Examples Directory

The original example files are now superseded by the package integrations. Add a note pointing to the new location.

**Files:**
- Modify: `examples/langchain_tool.py`
- Modify: `examples/crewai_tool.py`
- Modify: `examples/openai_functions.py`

**Step 1: Add deprecation notices**

Add to the top of each example file:

```python
# DEPRECATED: This example has moved into the browsy package.
# Use: from browsy.langchain import get_tools
# Install: pip install browsy[langchain]
# See: crates/python/browsy/_integrations/langchain.py
```

(Similar for crewai and openai)

**Step 2: Commit**

```bash
git add examples/
git commit -m "chore: add deprecation notices to old example integrations"
```
