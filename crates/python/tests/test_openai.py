"""Tests for browsy OpenAI integration."""
import pytest
from browsy import Browser

try:
    from browsy.openai import get_tool_definitions, handle_tool_call
    HAS_OPENAI = True
except ImportError:
    HAS_OPENAI = False

pytestmark = pytest.mark.skipif(not HAS_OPENAI, reason="openai not installed")


def test_tool_definitions_count():
    defs = get_tool_definitions()
    assert len(defs) == 6


def test_tool_definitions_shape():
    defs = get_tool_definitions()
    for d in defs:
        assert d["type"] == "function"
        assert "name" in d["function"]
        assert "description" in d["function"]
        assert "parameters" in d["function"]


def test_tool_definition_names():
    defs = get_tool_definitions()
    names = {d["function"]["name"] for d in defs}
    assert names == {
        "browsy_browse", "browsy_click", "browsy_type_text",
        "browsy_search", "browsy_login", "browsy_page_info",
    }


def test_handle_unknown_function():
    result = handle_tool_call("nonexistent", {})
    assert "Unknown function" in result


def test_handle_page_info_no_page():
    result = handle_tool_call("browsy_page_info", {}, browser=Browser())
    assert "No page loaded" in result
