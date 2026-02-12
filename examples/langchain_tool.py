# DEPRECATED: This example has moved into the browsy package.
# Use: from browsy.langchain import get_tools
# Install: pip install browsy-ai[langchain]
# See: crates/python/browsy/_integrations/langchain.py

"""
browsy LangChain Tool -- drop-in web browsing tool for LangChain agents.

Usage:
    pip install browsy-ai langchain

    from langchain_tool import BrowsyBrowseTool, BrowsySearchTool
    tools = [BrowsyBrowseTool(), BrowsySearchTool()]
    agent = create_react_agent(llm, tools)
"""

from typing import Optional, Type
from pydantic import BaseModel, Field
from langchain.tools import BaseTool
from browsy import Browser


# Shared browser instance across tools
_browser = Browser()


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
        "element IDs, and all interactive elements. Use this to browse websites. "
        "Much faster than browser-based tools (200ms vs 5s)."
    )
    args_schema: Type[BaseModel] = BrowseInput

    def _run(self, url: str) -> str:
        page = _browser.goto(url)
        result = f"title: {page.title}\nurl: {page.url}\npage_type: {page.page_type}\n"

        actions = page.suggested_actions()
        if actions:
            result += "suggested_actions:\n"
            for action in actions:
                result += f"  {action}\n"

        result += f"---\n{page.to_compact()}"
        return result


class BrowsyClickTool(BaseTool):
    """Click an element by its ID. Links navigate, buttons submit forms."""

    name: str = "browsy_click"
    description: str = (
        "Click an element by its ID. Links navigate to new pages, buttons "
        "submit forms. Returns the resulting page content."
    )
    args_schema: Type[BaseModel] = ClickInput

    def _run(self, element_id: int) -> str:
        page = _browser.click(element_id)
        result = f"title: {page.title}\nurl: {page.url}\npage_type: {page.page_type}\n"
        result += f"---\n{page.to_compact()}"
        return result


class BrowsyTypeTextTool(BaseTool):
    """Type text into an input field or textarea."""

    name: str = "browsy_type_text"
    description: str = (
        "Type text into an input field or textarea by element ID. "
        "Use browsy_browse first to find the element ID."
    )
    args_schema: Type[BaseModel] = TypeTextInput

    def _run(self, element_id: int, text: str) -> str:
        _browser.type_text(element_id, text)
        return f"Typed '{text}' into element {element_id}"


class BrowsySearchTool(BaseTool):
    """Search the web using DuckDuckGo and return structured results."""

    name: str = "browsy_search"
    description: str = (
        "Search the web and return structured results with title, URL, and "
        "snippet. Uses DuckDuckGo. No API key needed."
    )
    args_schema: Type[BaseModel] = SearchInput

    def _run(self, query: str) -> str:
        results = _browser.search(query)
        lines = []
        for i, r in enumerate(results, 1):
            lines.append(f"{i}. {r['title']}")
            lines.append(f"   {r['url']}")
            if r.get("snippet"):
                lines.append(f"   {r['snippet']}")
            lines.append("")
        return "\n".join(lines) if lines else "No results found."


class BrowsyLoginTool(BaseTool):
    """Log in to a website using detected login form fields."""

    name: str = "browsy_login"
    description: str = (
        "Log in to the current page using detected login form fields. "
        "Requires a page with a login form loaded (use browsy_browse first). "
        "Automatically finds username, password, and submit fields."
    )
    args_schema: Type[BaseModel] = LoginInput

    def _run(self, username: str, password: str) -> str:
        page = _browser.login(username, password)
        result = f"title: {page.title}\nurl: {page.url}\npage_type: {page.page_type}\n"
        result += f"---\n{page.to_compact()}"
        return result


class BrowsyPageInfoTool(BaseTool):
    """Get page metadata: type, actions, alerts, pagination."""

    name: str = "browsy_page_info"
    description: str = (
        "Get metadata about the current page: page type, suggested actions "
        "(login/search/consent), alerts, and pagination info. Use after "
        "browsy_browse to understand what kind of page you're on."
    )

    def _run(self) -> str:
        page = _browser.dom()
        if page is None:
            return "No page loaded. Use browsy_browse first."

        result = f"title: {page.title}\n"
        result += f"url: {page.url}\n"
        result += f"page_type: {page.page_type}\n"

        actions = page.suggested_actions()
        if actions:
            result += "suggested_actions:\n"
            for action in actions:
                result += f"  {action}\n"

        return result


def get_browsy_tools():
    """Return all browsy tools for use with a LangChain agent."""
    return [
        BrowsyBrowseTool(),
        BrowsyClickTool(),
        BrowsyTypeTextTool(),
        BrowsySearchTool(),
        BrowsyLoginTool(),
        BrowsyPageInfoTool(),
    ]
