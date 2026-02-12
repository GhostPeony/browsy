# DEPRECATED: This example has moved into the browsy package.
# Use: from browsy.openai import get_tool_definitions, handle_tool_call
# Install: pip install browsy-ai[openai]
# See: crates/python/browsy/_integrations/openai.py

"""
browsy OpenAI Function Calling -- tool definitions for any OpenAI-compatible agent.

Usage:
    pip install browsy-ai openai

    tools = get_browsy_tool_definitions()
    response = client.chat.completions.create(model="gpt-4", tools=tools, ...)
    # Handle tool calls with handle_tool_call()
"""

import json
from browsy import Browser


_browser = Browser()


def get_browsy_tool_definitions():
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
                        "url": {
                            "type": "string",
                            "description": "URL to navigate to",
                        }
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
                        "element_id": {
                            "type": "integer",
                            "description": "Element ID to click",
                        }
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
                        "element_id": {
                            "type": "integer",
                            "description": "Element ID of the text input",
                        },
                        "text": {
                            "type": "string",
                            "description": "Text to type into the input",
                        },
                    },
                    "required": ["element_id", "text"],
                },
            },
        },
        {
            "type": "function",
            "function": {
                "name": "browsy_search",
                "description": "Search the web and return structured results with title, URL, and snippet.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "Search query",
                        }
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
                        "username": {
                            "type": "string",
                            "description": "Username or email",
                        },
                        "password": {
                            "type": "string",
                            "description": "Password",
                        },
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
                "parameters": {
                    "type": "object",
                    "properties": {},
                },
            },
        },
    ]


def handle_tool_call(function_name: str, arguments: dict) -> str:
    """Handle a tool call from the OpenAI API and return the result string."""

    if function_name == "browsy_browse":
        page = _browser.goto(arguments["url"])
        return _format_page(page)

    elif function_name == "browsy_click":
        page = _browser.click(arguments["element_id"])
        return _format_page(page)

    elif function_name == "browsy_type_text":
        _browser.type_text(arguments["element_id"], arguments["text"])
        return f"Typed '{arguments['text']}' into element {arguments['element_id']}"

    elif function_name == "browsy_search":
        results = _browser.search(arguments["query"])
        lines = []
        for i, r in enumerate(results, 1):
            lines.append(f"{i}. {r['title']}")
            lines.append(f"   {r['url']}")
            if r.get("snippet"):
                lines.append(f"   {r['snippet']}")
            lines.append("")
        return "\n".join(lines) if lines else "No results found."

    elif function_name == "browsy_login":
        page = _browser.login(arguments["username"], arguments["password"])
        return _format_page(page)

    elif function_name == "browsy_page_info":
        page = _browser.dom()
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


def _format_page(page) -> str:
    result = f"title: {page.title}\nurl: {page.url}\n"
    result += f"page_type: {page.page_type}\n"
    actions = page.suggested_actions()
    if actions:
        result += "suggested_actions:\n"
        for a in actions:
            result += f"  {a}\n"
    result += f"---\n{page.to_compact()}"
    return result
