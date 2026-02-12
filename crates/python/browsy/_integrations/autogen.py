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
