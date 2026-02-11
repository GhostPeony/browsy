use agentbrowser_core::output;

#[test]
fn test_login_page_spatial_dom() {
    let html = r#"
    <html>
    <body>
        <nav style="display: flex; height: 64px; width: 100%; padding: 0 24px; align-items: center; justify-content: space-between;">
            <a href="/">Home</a>
            <a href="/about">About</a>
            <button>Sign In</button>
        </nav>
        <main style="display: flex; flex-direction: column; align-items: center; padding-top: 100px;">
            <h1>Welcome</h1>
            <form style="display: flex; flex-direction: column; gap: 16px; width: 400px;">
                <input type="email" placeholder="Email" />
                <input type="password" placeholder="Password" />
                <button>Log In</button>
                <a href="/forgot">Forgot password?</a>
            </form>
        </main>
    </body>
    </html>
    "#;

    let dom = agentbrowser_core::parse(html, 1920.0, 1080.0);

    // Should have elements
    assert!(!dom.els.is_empty(), "Should find interactive/text elements");

    // Print for inspection
    println!("\n=== Spatial DOM (JSON) ===");
    println!("{}", serde_json::to_string_pretty(&dom).unwrap());

    println!("\n=== Compact Format ===");
    println!("{}", output::to_compact_string(&dom));

    // Verify key elements exist
    let tags: Vec<&str> = dom.els.iter().map(|e| e.tag.as_str()).collect();
    assert!(tags.contains(&"input"), "Should find input elements");
    assert!(tags.contains(&"button"), "Should find button elements");
    assert!(tags.contains(&"a"), "Should find link elements");
    assert!(tags.contains(&"h1"), "Should find heading");

    // Verify all elements have non-zero bounds
    for el in &dom.els {
        assert!(
            el.b[2] > 0 || el.b[3] > 0,
            "Element {} ({}) should have non-zero size: {:?}",
            el.id, el.tag, el.b
        );
    }

    // Verify element properties
    let email_input = dom.els.iter().find(|e| e.ph.as_deref() == Some("Email"));
    assert!(email_input.is_some(), "Should find email input");
    let email = email_input.unwrap();
    assert_eq!(email.role.as_deref(), Some("textbox"));
    assert_eq!(email.input_type.as_deref(), Some("email"));

    let password_input = dom.els.iter().find(|e| e.ph.as_deref() == Some("Password"));
    assert!(password_input.is_some(), "Should find password input");
    let pw = password_input.unwrap();
    assert_eq!(pw.input_type.as_deref(), Some("password"));

    // Find buttons by role
    let buttons: Vec<_> = dom.els.iter().filter(|e| e.role.as_deref() == Some("button")).collect();
    assert!(buttons.len() >= 2, "Should find at least 2 buttons, got {}", buttons.len());

    let forgot_link = dom.els.iter().find(|e| e.href.as_deref() == Some("/forgot"));
    assert!(forgot_link.is_some(), "Should find forgot password link");
    assert_eq!(forgot_link.unwrap().role.as_deref(), Some("link"));

    // Token count check — compact format should be very small
    let compact = output::to_compact_string(&dom);
    let approx_tokens = compact.len() / 4; // rough estimate: ~4 chars per token
    println!("\nApprox tokens (compact): {}", approx_tokens);
    assert!(approx_tokens < 200, "Compact output should be under 200 tokens");
}

#[test]
fn test_flex_layout_positioning() {
    let html = r#"
    <html>
    <body style="margin: 0;">
        <div style="display: flex; width: 600px; height: 100px;">
            <div style="width: 200px; height: 100px;">
                <button>A</button>
            </div>
            <div style="width: 200px; height: 100px;">
                <button>B</button>
            </div>
            <div style="width: 200px; height: 100px;">
                <button>C</button>
            </div>
        </div>
    </body>
    </html>
    "#;

    let dom = agentbrowser_core::parse(html, 1920.0, 1080.0);

    let buttons: Vec<_> = dom.els.iter().filter(|e| e.tag == "button").collect();
    assert_eq!(buttons.len(), 3, "Should find 3 buttons");

    // Button A should be left of B, B left of C
    assert!(
        buttons[0].b[0] < buttons[1].b[0],
        "Button A ({}) should be left of Button B ({})",
        buttons[0].b[0], buttons[1].b[0]
    );
    assert!(
        buttons[1].b[0] < buttons[2].b[0],
        "Button B ({}) should be left of Button C ({})",
        buttons[1].b[0], buttons[2].b[0]
    );

    println!("\n=== Flex Layout Positions ===");
    for btn in &buttons {
        println!(
            "Button '{}' at x={}, y={}, w={}, h={}",
            btn.text.as_deref().unwrap_or(""),
            btn.b[0], btn.b[1], btn.b[2], btn.b[3]
        );
    }
}

#[test]
fn test_display_none_hidden() {
    let html = r#"
    <html>
    <body>
        <button>Visible</button>
        <button style="display: none;">Hidden</button>
        <button style="visibility: hidden;">Invisible</button>
        <div hidden><button>Also Hidden</button></div>
    </body>
    </html>
    "#;

    let dom = agentbrowser_core::parse(html, 1920.0, 1080.0);

    // Debug: print what we got
    println!("\n=== Display None Test ===");
    for el in &dom.els {
        println!("  {} ({}) text={:?} bounds={:?}", el.id, el.tag, el.text, el.b);
    }

    let buttons: Vec<_> = dom.els.iter()
        .filter(|e| e.tag == "button")
        .collect();

    // Should have exactly 1 visible button
    assert_eq!(buttons.len(), 1, "Should find exactly 1 visible button, found {}: {:?}",
        buttons.len(),
        buttons.iter().map(|b| b.text.as_deref().unwrap_or("")).collect::<Vec<_>>()
    );
}

#[test]
fn test_viewport_size() {
    let html = "<html><body><button>Click</button></body></html>";

    let dom = agentbrowser_core::parse(html, 1920.0, 1080.0);
    assert_eq!(dom.vp, [1920.0, 1080.0]);

    let dom2 = agentbrowser_core::parse(html, 1280.0, 720.0);
    assert_eq!(dom2.vp, [1280.0, 720.0]);
}
