"""browsy OpenAI function calling integration.

Install: pip install browsy-ai[openai]

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
