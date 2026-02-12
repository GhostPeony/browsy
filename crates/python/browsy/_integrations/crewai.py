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
