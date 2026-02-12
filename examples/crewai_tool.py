# DEPRECATED: This example has moved into the browsy package.
# Use: from browsy.crewai import BrowsyTool
# Install: pip install browsy[crewai]
# See: crates/python/browsy/_integrations/crewai.py

"""
browsy CrewAI Tool -- drop-in web browsing tool for CrewAI agents.

Usage:
    pip install browsy crewai crewai-tools

    from crewai_tool import BrowsyTool
    agent = Agent(tools=[BrowsyTool()])
"""

from typing import Optional
from crewai.tools import BaseTool
from browsy import Browser


class BrowsyTool(BaseTool):
    """Browse websites using browsy's zero-render engine.

    Provides page intelligence (type detection, action recipes), hidden content
    exposure, and form interaction. 26x faster than Chromium-based tools.
    """

    name: str = "browsy_web_browser"
    description: str = (
        "Browse websites without launching a browser. Navigates to URLs, detects "
        "page types (Login, Search, Form, Article, List, Captcha), provides action "
        "recipes with element IDs, and exposes hidden content. Supports clicking, "
        "typing, form submission, and web search.\n\n"
        "Commands:\n"
        "  browse <url> -- Navigate to a URL\n"
        "  click <id> -- Click an element\n"
        "  type <id> <text> -- Type text into an input\n"
        "  search <query> -- Search the web\n"
        "  login <username> <password> -- Log in using detected form\n"
        "  info -- Get page type and suggested actions\n"
        "  back -- Go back to previous page\n"
    )

    _browser: Optional[Browser] = None

    def model_post_init(self, __context) -> None:
        self._browser = Browser()

    def _run(self, command: str) -> str:
        parts = command.strip().split(None, 2)
        if not parts:
            return "Error: empty command. Use 'browse <url>', 'click <id>', etc."

        action = parts[0].lower()

        if action == "browse" and len(parts) >= 2:
            url = parts[1]
            page = self._browser.goto(url)
            return self._format_page(page)

        elif action == "click" and len(parts) >= 2:
            try:
                element_id = int(parts[1])
            except ValueError:
                return f"Error: invalid element ID '{parts[1]}'"
            page = self._browser.click(element_id)
            return self._format_page(page)

        elif action == "type" and len(parts) >= 3:
            try:
                element_id = int(parts[1])
            except ValueError:
                return f"Error: invalid element ID '{parts[1]}'"
            text = parts[2]
            self._browser.type_text(element_id, text)
            return f"Typed '{text}' into element {element_id}"

        elif action == "search" and len(parts) >= 2:
            query = " ".join(parts[1:])
            results = self._browser.search(query)
            lines = []
            for i, r in enumerate(results, 1):
                lines.append(f"{i}. {r['title']}")
                lines.append(f"   {r['url']}")
                if r.get("snippet"):
                    lines.append(f"   {r['snippet']}")
                lines.append("")
            return "\n".join(lines) if lines else "No results found."

        elif action == "login" and len(parts) >= 3:
            username = parts[1]
            password = parts[2]
            page = self._browser.login(username, password)
            return self._format_page(page)

        elif action == "info":
            page = self._browser.dom()
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
            page = self._browser.back()
            return self._format_page(page)

        else:
            return (
                f"Unknown command '{action}'. Available: "
                "browse, click, type, search, login, info, back"
            )

    def _format_page(self, page) -> str:
        result = f"title: {page.title}\nurl: {page.url}\n"
        result += f"page_type: {page.page_type}\n"
        actions = page.suggested_actions()
        if actions:
            result += "suggested_actions:\n"
            for a in actions:
                result += f"  {a}\n"
        result += f"---\n{page.to_compact()}"
        return result
