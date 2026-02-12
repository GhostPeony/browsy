"""Tests for browsy CrewAI integration."""
import pytest
from browsy import Browser

try:
    from browsy.crewai import BrowsyTool
    HAS_CREWAI = True
except ImportError:
    HAS_CREWAI = False

pytestmark = pytest.mark.skipif(not HAS_CREWAI, reason="crewai not installed")

SIMPLE_HTML = '<html><head><title>Test</title></head><body><h1>Hello</h1><a href="/about">About</a></body></html>'


@pytest.fixture
def tool():
    return BrowsyTool(browser=Browser())


def test_tool_name(tool):
    assert tool.name == "browsy_web_browser"


def test_info_no_page(tool):
    result = tool._run("info")
    assert "No page loaded" in result


def test_unknown_command(tool):
    result = tool._run("foobar")
    assert "Unknown command" in result


def test_invalid_click_id(tool):
    tool._browser.load_html(SIMPLE_HTML, "https://example.com")
    result = tool._run("click notanumber")
    assert "invalid element ID" in result
