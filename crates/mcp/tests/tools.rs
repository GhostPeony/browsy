use std::sync::{Arc, Mutex};

use browsy_core::fetch::{Session, SessionConfig};
use browsy_mcp::BrowsyServer;
use rmcp::handler::server::wrapper::Parameters;
use browsy_mcp::*;

fn make_config() -> SessionConfig {
    SessionConfig {
        fetch_css: false,
        ..SessionConfig::default()
    }
}

fn make_server() -> BrowsyServer {
    let session = Session::with_config(make_config()).unwrap();
    BrowsyServer::with_session(Arc::new(Mutex::new(session)))
}

fn make_server_with_html(html: &str, url: &str) -> BrowsyServer {
    let mut session = Session::with_config(make_config()).unwrap();
    session.load_html(html, url).unwrap();
    BrowsyServer::with_session(Arc::new(Mutex::new(session)))
}

fn extract_text(result: rmcp::model::CallToolResult) -> String {
    result
        .content
        .first()
        .and_then(|c| c.raw.as_text())
        .map(|t| t.text.clone())
        .unwrap_or_default()
}

/// Run an async closure on a fresh tokio runtime.
/// The server (containing the Session) must be moved in and returned
/// so it's dropped outside async context.
fn run_async<F, T>(f: F) -> T
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    // Run on a separate thread to avoid nested runtime issues
    std::thread::spawn(f).join().unwrap()
}

#[test]
fn test_get_page() {
    let html = r#"<html><head><title>Test Page</title></head><body><p>Hello World</p></body></html>"#;
    let server = make_server_with_html(html, "https://example.com");

    let (text, _server) = run_async(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(async {
            server
                .get_page(Parameters(GetPageParams { format: None, scope: None }))
                .await
                .unwrap()
        });
        let text = extract_text(result);
        // Return server so it drops outside async context
        drop(rt);
        (text, server)
    });

    assert!(text.contains("Test Page"), "should contain title");
    assert!(text.contains("https://example.com"), "should contain URL");
    assert!(text.contains("Hello World"), "should contain page text");
}

#[test]
fn test_page_info_suggested_actions() {
    let html = r#"
    <html><head><title>Login</title></head>
    <body>
        <form action="/login" method="post">
            <label for="user">Username</label>
            <input type="text" id="user" name="username" />
            <label for="pass">Password</label>
            <input type="password" id="pass" name="password" />
            <button type="submit">Sign In</button>
        </form>
    </body></html>"#;
    let server = make_server_with_html(html, "https://example.com/login");

    let (info, _server) = run_async(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(async {
            server.page_info().await.unwrap()
        });
        let text = extract_text(result);
        let info: serde_json::Value = serde_json::from_str(&text).unwrap();
        drop(rt);
        (info, server)
    });

    assert!(info.get("suggested_actions").is_some(), "should have suggested_actions field");
    let actions = info["suggested_actions"].as_array().unwrap();
    assert!(!actions.is_empty(), "should detect login action");
    assert_eq!(actions[0]["action"], "Login");
}

#[test]
fn test_type_text_and_get_page() {
    let html = r#"
    <html><head><title>Form</title></head>
    <body>
        <input type="text" id="name" name="name" placeholder="Your name" />
    </body></html>"#;
    let server = make_server_with_html(html, "https://example.com/form");

    let (typed_text, page_text, _server) = run_async(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let (typed_text, page_text) = rt.block_on(async {
            // Find the input element ID
            let result = server
                .get_page(Parameters(GetPageParams { format: Some("json".to_string()), scope: None }))
                .await
                .unwrap();
            let text = extract_text(result);
            let dom: serde_json::Value = serde_json::from_str(&text).unwrap();
            let input_id = dom["els"]
                .as_array()
                .unwrap()
                .iter()
                .find(|e| e["tag"] == "input")
                .unwrap()["id"]
                .as_u64()
                .unwrap() as u32;

            // Type text
            let result = server
                .type_text(Parameters(TypeTextParams {
                    id: input_id,
                    text: "Alice".to_string(),
                }))
                .await
                .unwrap();
            let typed_text = extract_text(result);

            // Verify value in get_page
            let result = server
                .get_page(Parameters(GetPageParams { format: Some("json".to_string()), scope: None }))
                .await
                .unwrap();
            let page_text = extract_text(result);

            (typed_text, page_text)
        });
        drop(rt);
        (typed_text, page_text, server)
    });

    assert!(typed_text.contains("Alice"), "should confirm typed text");
    assert!(page_text.contains("Alice"), "typed value should appear in page");
}

#[test]
fn test_find_tool() {
    let html = r#"
    <html><head><title>Find Test</title></head>
    <body>
        <a href="/about">About Us</a>
        <a href="/contact">Contact</a>
        <button>Submit</button>
    </body></html>"#;
    let server = make_server_with_html(html, "https://example.com");

    let (elements, _server) = run_async(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(async {
            server
                .find(Parameters(FindParams {
                    text: Some("About".to_string()),
                    role: None,
                }))
                .await
                .unwrap()
        });
        let text = extract_text(result);
        let elements: Vec<serde_json::Value> = serde_json::from_str(&text).unwrap();
        drop(rt);
        (elements, server)
    });

    assert!(!elements.is_empty(), "should find elements with 'About' text");
    assert!(
        elements.iter().any(|e| e["text"].as_str().unwrap_or("").contains("About")),
        "found element should contain 'About' text"
    );
}

#[test]
fn test_login_tool() {
    let html = r#"
    <html><head><title>Login</title></head>
    <body>
        <form action="/login" method="post">
            <label for="user">Username</label>
            <input type="text" id="user" name="username" />
            <label for="pass">Password</label>
            <input type="password" id="pass" name="password" />
            <button type="submit">Sign In</button>
        </form>
    </body></html>"#;
    let server = make_server_with_html(html, "https://example.com/login");

    let (_result, _server) = run_async(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(async {
            server
                .login(Parameters(LoginParams {
                    username: "admin".to_string(),
                    password: "secret".to_string(),
                }))
                .await
        });
        drop(rt);
        // login() types values and clicks submit.
        // With load_html, the submit click may fail with network error — that's expected.
        (result, server)
    });
    // We just verify it doesn't panic — the result may be Ok or Err depending on
    // whether the form action URL is reachable.
}

#[test]
fn test_enter_code_tool() {
    let html = r#"
    <html><head><title>Verify</title></head>
    <body>
        <form action="/verify" method="post">
            <label for="code">Verification Code</label>
            <input type="text" id="code" name="code" placeholder="Enter code" />
            <button type="submit">Verify</button>
        </form>
    </body></html>"#;
    let server = make_server_with_html(html, "https://example.com/verify");

    let (_result, _server) = run_async(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(async {
            server
                .enter_code(Parameters(EnterCodeParams {
                    code: "123456".to_string(),
                }))
                .await
        });
        drop(rt);
        (result, server)
    });
}

#[test]
fn test_error_on_no_page() {
    let server = make_server();

    let (result, _server) = run_async(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(async {
            server
                .get_page(Parameters(GetPageParams { format: None, scope: None }))
                .await
        });
        drop(rt);
        (result, server)
    });

    assert!(result.is_err(), "get_page with no page loaded should error");
    let err = result.unwrap_err();
    assert!(
        err.message.contains("No page loaded"),
        "error message should say no page loaded"
    );
}
