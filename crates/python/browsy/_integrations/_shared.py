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
