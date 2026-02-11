//! Tests for Session API, fetch, and browser interaction.

#[cfg(feature = "fetch")]
use browsy_core::fetch;
#[cfg(feature = "fetch")]
use browsy_core::fetch::Session;
#[test]
#[cfg(feature = "fetch")]
fn test_fetch_real_page() {
    let config = fetch::FetchConfig {
        fetch_css: false,
        ..Default::default()
    };

    let result = fetch::fetch("https://example.com", &config);
    assert!(result.is_ok(), "Should fetch example.com: {:?}", result.err());

    let dom = result.unwrap();
    assert!(!dom.els.is_empty());
    assert!(dom.els.iter().any(|e| e.tag == "h1"));
    assert!(dom.els.iter().any(|e| e.tag == "a"));
    assert_eq!(dom.url, "https://example.com");
}

#[test]
#[cfg(feature = "fetch")]
fn test_session_load_html() {
    let mut session = Session::new().unwrap();

    let html = r#"
    <html><body>
        <h1>Hello World</h1>
        <button>Click Me</button>
        <a href="/about">About</a>
    </body></html>
    "#;

    session.load_html(html, "http://localhost/test").unwrap();

    let dom = session.dom().unwrap();
    assert!(!dom.els.is_empty());
    assert_eq!(dom.url, "http://localhost/test");
    assert!(dom.els.iter().any(|e| e.text.as_deref() == Some("Hello World")));
    assert!(dom.els.iter().any(|e| e.text.as_deref() == Some("Click Me")));
    assert!(dom.els.iter().any(|e| e.href.as_deref() == Some("http://localhost/about")));
}

#[test]
#[cfg(feature = "fetch")]
fn test_session_find_helpers() {
    let mut session = Session::new().unwrap();

    let html = r#"
    <html><body>
        <h1>Dashboard</h1>
        <button>Save</button>
        <button>Delete</button>
        <a href="/home">Home</a>
        <input type="text" placeholder="Search" />
    </body></html>
    "#;

    session.load_html(html, "http://localhost").unwrap();

    assert_eq!(session.find_by_text("Save").len(), 1);
    assert_eq!(session.find_by_text("Save")[0].tag, "button");
    assert_eq!(session.find_by_text("Dashboard").len(), 1);
    assert_eq!(session.find_by_role("button").len(), 2);
    assert_eq!(session.find_by_role("link").len(), 1);
    assert_eq!(session.find_by_role("textbox").len(), 1);

    let first_id = session.dom().unwrap().els[0].id;
    assert!(session.element(first_id).is_some());
}

#[test]
#[cfg(feature = "fetch")]
fn test_session_type_text() {
    let mut session = Session::new().unwrap();

    let html = r#"
    <html><body>
        <form>
            <input type="text" name="username" placeholder="Username" />
            <input type="password" name="password" placeholder="Password" />
            <button>Login</button>
        </form>
    </body></html>
    "#;

    session.load_html(html, "http://localhost").unwrap();

    let input_ids: Vec<u32> = session.find_by_role("textbox").iter().map(|e| e.id).collect();
    assert_eq!(input_ids.len(), 2);

    assert!(session.type_text(input_ids[0], "admin").is_ok());
    assert!(session.type_text(input_ids[1], "secret123").is_ok());

    // Typing into non-input should fail
    let button_id = session.find_by_role("button")[0].id;
    assert!(session.type_text(button_id, "text").is_err());
}

#[test]
#[cfg(feature = "fetch")]
fn test_session_select() {
    let mut session = Session::new().unwrap();

    let html = r#"
    <html><body>
        <select name="color">
            <option value="red">Red</option>
            <option value="blue">Blue</option>
        </select>
        <button>Submit</button>
    </body></html>
    "#;

    session.load_html(html, "http://localhost").unwrap();

    let select_id = session.find_by_role("combobox")[0].id;
    assert!(session.select(select_id, "blue").is_ok());

    // Select on non-select should fail
    let button_id = session.find_by_role("button")[0].id;
    assert!(session.select(button_id, "val").is_err());
}

#[test]
#[cfg(feature = "fetch")]
fn test_session_delta() {
    let mut session = Session::new().unwrap();

    assert!(session.delta().is_none());

    session.load_html(
        r#"<html><body><h1>Page 1</h1><button>Next</button></body></html>"#,
        "http://localhost/1",
    ).unwrap();
    assert!(session.delta().is_none());

    session.load_html(
        r#"<html><body><h1>Page 2</h1><button>Back</button></body></html>"#,
        "http://localhost/2",
    ).unwrap();

    let delta = session.delta().unwrap();
    assert!(!delta.changed.is_empty());
    assert!(!delta.removed.is_empty());
    assert!(delta.changed.iter().any(|e| e.text.as_deref() == Some("Page 2")));
}

#[test]
#[cfg(feature = "fetch")]
fn test_session_real_navigation() {
    let mut session = Session::with_config(fetch::SessionConfig {
        fetch_css: false,
        ..Default::default()
    }).unwrap();

    session.goto("https://example.com").unwrap();

    let dom = session.dom().unwrap();
    assert!(!dom.els.is_empty());
    assert!(session.url().unwrap().contains("example.com"));
    assert!(dom.els.iter().any(|e| e.tag == "h1"));
}

#[test]
#[cfg(feature = "fetch")]
fn test_session_js_click_toggle() {
    let mut session = Session::new().unwrap();

    let html = r#"
    <html><body>
        <button onclick="document.getElementById('menu').style.display = 'block'">Open</button>
        <div id="menu" style="display: none;">
            <a href="/profile">Profile</a>
        </div>
    </body></html>
    "#;

    session.load_html(html, "http://localhost").unwrap();

    // Menu hidden initially
    let dom = session.dom().unwrap();
    assert!(dom.els.iter().find(|e| e.href.as_deref() == Some("/profile")).is_none());

    // Click toggle button
    let btn_id = session.find_by_role("button")[0].id;
    session.click(btn_id).unwrap();

    // Menu visible after click
    let dom = session.dom().unwrap();
    assert!(dom.els.iter().any(|e| e.href.as_deref() == Some("/profile")));
}

#[test]
#[cfg(feature = "fetch")]
fn test_search_result_parsing() {
    // Synthetic DuckDuckGo HTML results page
    let html = r#"
    <html><body>
    <div id="links" class="results">
        <div class="result results_links results_links_deep web-result">
            <div class="links_main links_deep result__body">
                <h2 class="result__title">
                    <a class="result__a" href="//duckduckgo.com/l/?uddg=https%3A%2F%2Fwww.rust-lang.org%2F&rut=abc">
                        Rust Programming Language
                    </a>
                </h2>
                <a class="result__snippet" href="//duckduckgo.com/l/?uddg=https%3A%2F%2Fwww.rust-lang.org%2F&rut=abc">
                    A language empowering everyone to build reliable and efficient software.
                </a>
            </div>
        </div>
        <div class="result results_links results_links_deep web-result">
            <div class="links_main links_deep result__body">
                <h2 class="result__title">
                    <a class="result__a" href="//duckduckgo.com/l/?uddg=https%3A%2F%2Fen.wikipedia.org%2Fwiki%2FRust_(programming_language)&rut=def">
                        Rust (programming language) - Wikipedia
                    </a>
                </h2>
                <a class="result__snippet" href="//duckduckgo.com/l/?uddg=https%3A%2F%2Fen.wikipedia.org%2Fwiki%2FRust_(programming_language)&rut=def">
                    Rust is a multi-paradigm, general-purpose programming language.
                </a>
            </div>
        </div>
        <div class="result result--ad">
            <div class="links_main result__body">
                <h2 class="result__title">
                    <a class="result__a" href="//duckduckgo.com/l/?uddg=https%3A%2F%2Fads.example.com&ad_provider=bingv7aa">
                        Sponsored Ad
                    </a>
                </h2>
            </div>
        </div>
    </div>
    </body></html>
    "#;

    let mut session = Session::new().unwrap();
    session.load_html(html, "https://html.duckduckgo.com/html/?q=rust").unwrap();

    // Parse as if it were DDG results
    let dom_tree = browsy_core::dom::parse_html(html);
    let results = fetch::extract_search_results_from(&dom_tree);

    // Should find 2 organic results, skip the ad
    assert_eq!(results.len(), 2);

    assert_eq!(results[0].title, "Rust Programming Language");
    assert_eq!(results[0].url, "https://www.rust-lang.org/");
    assert!(results[0].snippet.contains("reliable and efficient"));

    assert!(results[1].title.contains("Wikipedia"));
    assert!(results[1].url.contains("wikipedia.org"));
}

#[test]
#[cfg(feature = "fetch")]
fn test_search_live() {
    let mut session = Session::with_config(fetch::SessionConfig {
        fetch_css: false,
        ..Default::default()
    }).unwrap();

    let results = session.search("rust programming language");
    match results {
        Ok(results) => {
            assert!(!results.is_empty(), "Should get at least 1 result");
            // First result should have a title and URL
            assert!(!results[0].title.is_empty());
            assert!(results[0].url.starts_with("http"));
        }
        Err(e) => {
            // DDG may CAPTCHA us in CI — don't fail the test
            eprintln!("Search failed (may be CAPTCHA): {}", e);
        }
    }
}

#[test]
#[cfg(feature = "fetch")]
fn test_google_result_parsing() {
    // Synthetic Google-style HTML results page
    let html = r#"
    <html><body>
    <div id="rso">
        <div class="MjjYud">
            <div class="tF2Cxc">
                <div class="yuRUbf">
                    <a href="/url?q=https://www.rust-lang.org/&sa=U">
                        <h3 class="LC20lb">Rust Programming Language</h3>
                    </a>
                </div>
                <div class="VwiC3b yDYNvb">
                    <span>A language empowering everyone to build reliable and efficient software.</span>
                </div>
            </div>
        </div>
        <div class="MjjYud">
            <div class="tF2Cxc">
                <div class="yuRUbf">
                    <a href="https://en.wikipedia.org/wiki/Rust_(programming_language)">
                        <h3 class="LC20lb">Rust (programming language) - Wikipedia</h3>
                    </a>
                </div>
                <div class="VwiC3b yDYNvb">
                    <span>Rust is a multi-paradigm, general-purpose programming language that emphasizes performance and safety.</span>
                </div>
            </div>
        </div>
        <div class="MjjYud">
            <div>
                <a href="https://www.google.com/aclk?sponsored=true">
                    <h3>Sponsored Ad</h3>
                </a>
            </div>
        </div>
    </div>
    </body></html>
    "#;

    let dom_tree = browsy_core::dom::parse_html(html);
    let results = fetch::extract_google_results_from(&dom_tree);

    // Should find 2 organic results, skip the ad
    assert_eq!(results.len(), 2);

    assert_eq!(results[0].title, "Rust Programming Language");
    assert_eq!(results[0].url, "https://www.rust-lang.org/");
    assert!(results[0].snippet.contains("reliable and efficient"));

    assert!(results[1].title.contains("Wikipedia"));
    assert!(results[1].url.contains("wikipedia.org"));
    assert!(results[1].snippet.contains("multi-paradigm"));
}

#[test]
#[cfg(feature = "fetch")]
fn test_google_search_live() {
    let mut session = Session::with_config(fetch::SessionConfig {
        fetch_css: false,
        ..Default::default()
    }).unwrap();

    let results = session.search_with("rust programming language", fetch::SearchEngine::Google);
    match results {
        Ok(results) => {
            // Google may return results or may block us
            if !results.is_empty() {
                assert!(!results[0].title.is_empty());
                assert!(results[0].url.starts_with("http"));
                eprintln!("Google returned {} results", results.len());
                for (i, r) in results.iter().take(3).enumerate() {
                    eprintln!("  {}: {} -> {}", i, r.title, r.url);
                }
            } else {
                eprintln!("Google returned 0 results (may be blocked/CAPTCHA)");
            }
        }
        Err(e) => {
            eprintln!("Google search failed (expected in CI): {}", e);
        }
    }
}

#[test]
#[cfg(feature = "fetch")]
fn test_session_form_state_overlay() {
    let mut session = Session::new().unwrap();

    let html = r#"
    <html><body>
        <form>
            <input type="text" name="username" placeholder="Username" />
            <input type="email" name="email" placeholder="Email" />
            <button>Submit</button>
        </form>
    </body></html>
    "#;

    session.load_html(html, "http://localhost").unwrap();

    // Before typing, val should be None
    let dom = session.dom().unwrap();
    let username = dom.els.iter().find(|e| e.name.as_deref() == Some("username")).unwrap();
    assert!(username.val.is_none());

    // Type into fields
    let username_id = session.dom_ref().unwrap().els.iter()
        .find(|e| e.name.as_deref() == Some("username")).unwrap().id;
    let email_id = session.dom_ref().unwrap().els.iter()
        .find(|e| e.name.as_deref() == Some("email")).unwrap().id;

    session.type_text(username_id, "admin").unwrap();
    session.type_text(email_id, "admin@example.com").unwrap();

    // After typing, dom() should overlay the values
    let dom = session.dom().unwrap();
    let username = dom.els.iter().find(|e| e.name.as_deref() == Some("username")).unwrap();
    assert_eq!(username.val.as_deref(), Some("admin"));

    let email = dom.els.iter().find(|e| e.name.as_deref() == Some("email")).unwrap();
    assert_eq!(email.val.as_deref(), Some("admin@example.com"));
}

#[test]
#[cfg(feature = "fetch")]
fn test_session_checkbox_toggle() {
    let mut session = Session::new().unwrap();

    let html = r#"
    <html><body>
        <form>
            <input type="checkbox" name="agree" id="cb1" />
            <input type="checkbox" name="newsletter" id="cb2" checked />
            <input type="radio" name="plan" value="free" id="r1" checked />
            <input type="radio" name="plan" value="pro" id="r2" />
            <button>Submit</button>
        </form>
    </body></html>
    "#;

    session.load_html(html, "http://localhost").unwrap();

    let cb1_id = session.dom_ref().unwrap().els.iter()
        .find(|e| e.name.as_deref() == Some("agree")).unwrap().id;
    let cb2_id = session.dom_ref().unwrap().els.iter()
        .find(|e| e.name.as_deref() == Some("newsletter")).unwrap().id;

    // Check the unchecked checkbox
    session.check(cb1_id).unwrap();
    let dom = session.dom().unwrap();
    let cb1 = dom.els.iter().find(|e| e.name.as_deref() == Some("agree")).unwrap();
    assert_eq!(cb1.checked, Some(true));

    // Uncheck the checked checkbox
    session.uncheck(cb2_id).unwrap();
    let dom = session.dom().unwrap();
    let cb2 = dom.els.iter().find(|e| e.name.as_deref() == Some("newsletter")).unwrap();
    assert_eq!(cb2.checked, Some(false));

    // Toggle should flip state
    session.toggle(cb1_id).unwrap(); // was checked -> unchecked
    let dom = session.dom().unwrap();
    let cb1 = dom.els.iter().find(|e| e.name.as_deref() == Some("agree")).unwrap();
    assert_eq!(cb1.checked, Some(false));

    // Non-checkbox should fail
    let btn_id = session.dom_ref().unwrap().els.iter()
        .find(|e| e.tag == "button").unwrap().id;
    assert!(session.check(btn_id).is_err());
}

#[test]
#[cfg(feature = "fetch")]
fn test_session_name_field() {
    let mut session = Session::new().unwrap();

    let html = r#"
    <html><body>
        <form>
            <input type="text" name="search" placeholder="Search..." />
            <select name="category">
                <option value="all">All</option>
            </select>
        </form>
    </body></html>
    "#;

    session.load_html(html, "http://localhost").unwrap();

    let dom = session.dom().unwrap();
    let search = dom.els.iter().find(|e| e.ph.as_deref() == Some("Search...")).unwrap();
    assert_eq!(search.name.as_deref(), Some("search"));

    let select = dom.els.iter().find(|e| e.tag == "select").unwrap();
    assert_eq!(select.name.as_deref(), Some("category"));
}

#[test]
#[cfg(feature = "fetch")]
fn test_session_login_compound() {
    let mut session = Session::new().unwrap();

    let html = r#"
    <html><body>
        <h1>Login</h1>
        <form action="/login" method="post">
            <input type="email" name="email" placeholder="Email" />
            <input type="password" name="password" placeholder="Password" />
            <button>Sign In</button>
        </form>
    </body></html>
    "#;

    session.load_html(html, "http://localhost").unwrap();

    // Verify login action was detected
    let dom = session.dom_ref().unwrap();
    let has_login = dom.suggested_actions.iter().any(|a| {
        matches!(a, browsy_core::output::SuggestedAction::Login { .. })
    });
    assert!(has_login, "Should detect login action");

    // Test login compound action — type_text works, click will fail since no server
    // but the action extraction should succeed
    let (uid, pid, _sid) = {
        let dom = session.dom_ref().unwrap();
        dom.suggested_actions.iter().find_map(|a| match a {
            browsy_core::output::SuggestedAction::Login { username_id, password_id, submit_id, .. } => {
                Some((*username_id, *password_id, *submit_id))
            }
            _ => None,
        }).unwrap()
    };

    // Verify type_text works for the detected IDs
    assert!(session.type_text(uid, "test@example.com").is_ok());
    assert!(session.type_text(pid, "password123").is_ok());

    // Verify the values were stored
    let dom = session.dom().unwrap();
    let email = dom.get(uid).unwrap();
    assert_eq!(email.val.as_deref(), Some("test@example.com"));
}

#[test]
#[cfg(feature = "fetch")]
fn test_session_enter_code_compound() {
    let mut session = Session::new().unwrap();

    let html = r#"
    <html><head><title>Verify Your Identity</title></head>
    <body>
        <h1>Enter verification code</h1>
        <form action="/verify" method="post">
            <input type="text" name="code" placeholder="Enter code" />
            <button>Verify</button>
        </form>
    </body></html>
    "#;

    session.load_html(html, "http://localhost").unwrap();

    let dom = session.dom_ref().unwrap();
    let has_code = dom.suggested_actions.iter().any(|a| {
        matches!(a, browsy_core::output::SuggestedAction::EnterCode { .. })
    });
    assert!(has_code, "Should detect enter code action");
}

#[test]
#[cfg(feature = "fetch")]
fn test_session_find_verification_code() {
    let mut session = Session::new().unwrap();

    let html = r#"
    <html><body>
        <p>From: noreply@example.com</p>
        <p>Your verification code is 482901. This code expires in 10 minutes.</p>
    </body></html>
    "#;

    session.load_html(html, "http://localhost").unwrap();

    let code = session.find_verification_code();
    assert_eq!(code.as_deref(), Some("482901"));
}

#[test]
#[cfg(feature = "fetch")]
fn test_find_by_text_fuzzy() {
    let mut session = Session::new().unwrap();

    let html = r#"
    <html><body>
        <h1>Welcome to the Dashboard</h1>
        <button>Save Changes</button>
        <button>Delete Account</button>
        <a href="/settings">Account Settings</a>
    </body></html>
    "#;

    session.load_html(html, "http://localhost").unwrap();

    // Case-insensitive fuzzy match
    let results = session.find_by_text_fuzzy("save");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].text.as_deref(), Some("Save Changes"));

    let results = session.find_by_text_fuzzy("DASHBOARD");
    assert_eq!(results.len(), 1);

    let results = session.find_by_text_fuzzy("account");
    assert_eq!(results.len(), 2); // "Delete Account" and "Account Settings"
}

#[test]
#[cfg(feature = "fetch")]
fn test_find_input_by_purpose() {
    let mut session = Session::new().unwrap();

    let html = r#"
    <html><body>
        <form>
            <input type="email" name="email" placeholder="Email" />
            <input type="password" name="password" placeholder="Password" />
            <input type="text" name="username" placeholder="Username" />
            <input type="search" name="q" placeholder="Search" />
            <input type="tel" name="phone" placeholder="Phone" />
        </form>
    </body></html>
    "#;

    session.load_html(html, "http://localhost").unwrap();

    let email = session.find_input_by_purpose(fetch::InputPurpose::Email);
    assert!(email.is_some());
    assert_eq!(email.unwrap().input_type.as_deref(), Some("email"));

    let password = session.find_input_by_purpose(fetch::InputPurpose::Password);
    assert!(password.is_some());
    assert_eq!(password.unwrap().input_type.as_deref(), Some("password"));

    let username = session.find_input_by_purpose(fetch::InputPurpose::Username);
    assert!(username.is_some());
    assert_eq!(username.unwrap().name.as_deref(), Some("username"));

    let search = session.find_input_by_purpose(fetch::InputPurpose::Search);
    assert!(search.is_some());
    assert_eq!(search.unwrap().input_type.as_deref(), Some("search"));

    let phone = session.find_input_by_purpose(fetch::InputPurpose::Phone);
    assert!(phone.is_some());
    assert_eq!(phone.unwrap().input_type.as_deref(), Some("tel"));
}
