import json
import pytest
from browsy import Browser


@pytest.fixture
def browser():
    return Browser()


SIMPLE_HTML = """
<html>
<head><title>Test Page</title></head>
<body>
    <h1>Welcome</h1>
    <p>Hello World</p>
    <a href="/about">About</a>
</body>
</html>
"""

FORM_HTML = """
<html>
<head><title>Form Page</title></head>
<body>
    <form action="/submit" method="post">
        <label for="email">Email</label>
        <input type="email" id="email" name="email" placeholder="you@example.com" required />
        <label for="pass">Password</label>
        <input type="password" id="pass" name="password" />
        <input type="checkbox" id="remember" name="remember" />
        <label for="remember">Remember me</label>
        <button type="submit">Sign In</button>
    </form>
</body>
</html>
"""

LOGIN_HTML = """
<html>
<head><title>Login</title></head>
<body>
    <form action="/login" method="post">
        <label for="user">Username</label>
        <input type="text" id="user" name="username" />
        <label for="pass">Password</label>
        <input type="password" id="pass" name="password" />
        <button type="submit">Log In</button>
    </form>
</body>
</html>
"""

TABLE_HTML = """
<html>
<head><title>Table</title></head>
<body>
    <table>
        <thead><tr><th>Name</th><th>Age</th></tr></thead>
        <tbody>
            <tr><td>Alice</td><td>30</td></tr>
            <tr><td>Bob</td><td>25</td></tr>
        </tbody>
    </table>
</body>
</html>
"""

VERIFY_HTML = """
<html>
<head><title>Verify</title></head>
<body>
    <p>Your verification code is 847291. Enter it below.</p>
    <form action="/verify" method="post">
        <label for="code">Code</label>
        <input type="text" id="code" name="code" placeholder="Enter code" />
        <button type="submit">Verify</button>
    </form>
</body>
</html>
"""

HIDDEN_HTML = """
<html>
<head><title>Hidden Test</title></head>
<body>
    <p>Visible text</p>
    <div style="display:none">
        <a href="/secret">Hidden link</a>
    </div>
</body>
</html>
"""


def test_load_html(browser):
    page = browser.load_html(SIMPLE_HTML, "https://example.com")
    assert page.title == "Test Page"
    assert page.url == "https://example.com"
    assert len(page) > 0


def test_page_elements(browser):
    page = browser.load_html(SIMPLE_HTML, "https://example.com")
    elements = page.elements
    tags = [e.tag for e in elements]
    assert "a" in tags
    texts = [e.text for e in elements if e.text]
    assert any("Welcome" in t for t in texts)


def test_element_properties(browser):
    page = browser.load_html(FORM_HTML, "https://example.com/form")
    elements = page.elements
    email_input = [e for e in elements if e.input_type == "email"]
    assert len(email_input) > 0
    el = email_input[0]
    assert el.name == "email"
    assert el.placeholder == "you@example.com"
    assert el.required == True
    bounds = el.bounds
    assert len(bounds) == 4


def test_type_text(browser):
    browser.load_html(FORM_HTML, "https://example.com/form")
    page = browser.dom()
    email_input = [e for e in page.elements if e.input_type == "email"][0]
    browser.type_text(email_input.id, "test@test.com")
    page = browser.dom()
    compact = page.to_compact()
    assert "test@test.com" in compact


def test_check_uncheck(browser):
    browser.load_html(FORM_HTML, "https://example.com/form")
    page = browser.dom()
    checkbox = [e for e in page.elements if e.input_type == "checkbox"][0]

    browser.check(checkbox.id)
    page = browser.dom()
    cb = page.get(checkbox.id)
    assert cb.checked == True

    browser.uncheck(checkbox.id)
    page = browser.dom()
    cb = page.get(checkbox.id)
    assert cb.checked == False


def test_find_by_text(browser):
    browser.load_html(SIMPLE_HTML, "https://example.com")
    results = browser.find_by_text("About")
    assert len(results) > 0
    assert any("About" in (e.text or "") for e in results)


def test_find_by_role(browser):
    browser.load_html(SIMPLE_HTML, "https://example.com")
    results = browser.find_by_role("link")
    assert len(results) > 0


def test_find_by_text_fuzzy(browser):
    browser.load_html(SIMPLE_HTML, "https://example.com")
    # Fuzzy is case-insensitive substring
    results = browser.find_by_text_fuzzy("about")
    assert len(results) > 0
    assert any("About" in (e.text or "") for e in results)


def test_find_input_by_purpose(browser):
    browser.load_html(FORM_HTML, "https://example.com/form")
    email = browser.find_input_by_purpose("email")
    assert email is not None
    assert email.input_type == "email"

    password = browser.find_input_by_purpose("password")
    assert password is not None
    assert password.input_type == "password"

    # Unknown purpose returns None
    result = browser.find_input_by_purpose("nonexistent")
    assert result is None


def test_find_verification_code(browser):
    browser.load_html(VERIFY_HTML, "https://example.com/verify")
    code = browser.find_verification_code()
    assert code is not None
    assert code == "847291"


def test_suggested_actions(browser):
    page = browser.load_html(LOGIN_HTML, "https://example.com/login")
    actions = page.suggested_actions()
    assert len(actions) > 0
    login_action = actions[0]
    assert isinstance(login_action, dict)
    assert login_action["action"] == "Login"
    assert "username_id" in login_action
    assert "password_id" in login_action
    assert "submit_id" in login_action


def test_to_compact(browser):
    page = browser.load_html(SIMPLE_HTML, "https://example.com")
    compact = page.to_compact()
    assert isinstance(compact, str)
    assert "About" in compact


def test_to_json(browser):
    page = browser.load_html(SIMPLE_HTML, "https://example.com")
    json_str = page.to_json()
    data = json.loads(json_str)
    assert data["title"] == "Test Page"
    assert "els" in data


def test_tables(browser):
    page = browser.load_html(TABLE_HTML, "https://example.com/table")
    tables = page.tables()
    assert len(tables) > 0
    table = tables[0]
    assert "headers" in table
    assert "rows" in table
    assert "Name" in table["headers"]
    assert "Age" in table["headers"]
    assert any("Alice" in row for row in table["rows"])


def test_visible(browser):
    page = browser.load_html(HIDDEN_HTML, "https://example.com")
    all_els = page.elements
    visible_els = page.visible()
    # Hidden elements should be in all but not visible
    assert len(all_els) >= len(visible_els)
    hidden_texts = [e.text for e in all_els if e.hidden and e.text]
    visible_texts = [e.text for e in visible_els if e.text]
    # "Hidden link" should be in all elements but not in visible
    if hidden_texts:
        for ht in hidden_texts:
            assert ht not in visible_texts


def test_repr(browser):
    page = browser.load_html(SIMPLE_HTML, "https://example.com")
    page_repr = repr(page)
    assert "Page" in page_repr
    assert "Test Page" in page_repr

    elements = page.elements
    if elements:
        el_repr = repr(elements[0])
        assert "Element" in el_repr
