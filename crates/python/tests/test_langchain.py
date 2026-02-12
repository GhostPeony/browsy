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
