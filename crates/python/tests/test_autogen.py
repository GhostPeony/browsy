"""Tests for browsy AutoGen integration."""
import pytest
from browsy import Browser

try:
    from browsy.autogen import BrowsyBrowser, get_browsy_functions
    HAS_AUTOGEN = True
except ImportError:
    HAS_AUTOGEN = False

pytestmark = pytest.mark.skipif(not HAS_AUTOGEN, reason="pyautogen not installed")


def test_get_functions_count():
    funcs = get_browsy_functions()
    assert len(funcs) == 6


def test_get_functions_names():
    funcs = get_browsy_functions()
    names = {f["name"] for f in funcs}
    assert "browsy_browse" in names
    assert "browsy_click" in names
    assert "browsy_page_info" in names


def test_browser_agent_creation():
    agent = BrowsyBrowser(name="test_browser", browser=Browser())
    assert agent.name == "test_browser"
