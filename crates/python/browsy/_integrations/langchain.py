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
