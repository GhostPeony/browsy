"""Tests for browsy Smolagents integration."""
import pytest
from browsy import Browser

try:
    from browsy.smolagents import BrowsyTool
    HAS_SMOLAGENTS = True
except ImportError:
    HAS_SMOLAGENTS = False

pytestmark = pytest.mark.skipif(not HAS_SMOLAGENTS, reason="smolagents not installed")

SIMPLE_HTML = '<html><head><title>Test</title></head><body><h1>Hello</h1></body></html>'


def test_tool_name():
    tool = BrowsyTool(browser=Browser())
    assert tool.name == "web_browser"


def test_tool_has_inputs():
    tool = BrowsyTool(browser=Browser())
    assert "action" in tool.inputs


def test_info_no_page():
    tool = BrowsyTool(browser=Browser())
    result = tool.forward("info")
    assert "No page loaded" in result


def test_unknown_command():
    tool = BrowsyTool(browser=Browser())
    result = tool.forward("foobar")
    assert "Unknown command" in result
