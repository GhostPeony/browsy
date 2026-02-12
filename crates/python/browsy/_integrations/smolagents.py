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
