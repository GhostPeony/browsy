use browsy_core::output;
use browsy_core::js;
#[cfg(feature = "fetch")]
use browsy_core::fetch;
#[cfg(feature = "fetch")]
use browsy_core::fetch::Session;

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

    let dom = browsy_core::parse(html, 1920.0, 1080.0);

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

    let dom = browsy_core::parse(html, 1920.0, 1080.0);

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

    let dom = browsy_core::parse(html, 1920.0, 1080.0);

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

    let dom = browsy_core::parse(html, 1920.0, 1080.0);
    assert_eq!(dom.vp, [1920.0, 1080.0]);

    let dom2 = browsy_core::parse(html, 1280.0, 720.0);
    assert_eq!(dom2.vp, [1280.0, 720.0]);
}

#[test]
fn test_css_style_tag_selectors() {
    let html = r#"
    <html>
    <head>
        <style>
            .container {
                display: flex;
                flex-direction: column;
                width: 600px;
                gap: 12px;
                padding: 20px;
            }
            .btn {
                display: flex;
                height: 40px;
                width: 200px;
            }
            .btn-primary {
                width: 300px;
            }
            #submit-btn {
                width: 400px;
            }
            nav a {
                display: flex;
                width: 100px;
                height: 32px;
            }
            .hidden {
                display: none;
            }
        </style>
    </head>
    <body>
        <nav>
            <a href="/">Home</a>
            <a href="/about">About</a>
        </nav>
        <div class="container">
            <button class="btn">Cancel</button>
            <button class="btn btn-primary">Save</button>
            <button class="btn" id="submit-btn">Submit</button>
            <button class="btn hidden">Ghost</button>
        </div>
    </body>
    </html>
    "#;

    let dom = browsy_core::parse(html, 1920.0, 1080.0);

    println!("\n=== CSS Selector Test ===");
    let compact = output::to_compact_string(&dom);
    println!("{}", compact);

    // Nav links should have width 100
    let links: Vec<_> = dom.els.iter().filter(|e| e.tag == "a").collect();
    assert_eq!(links.len(), 2, "Should find 2 nav links");
    for link in &links {
        assert_eq!(link.b[2], 100, "Nav links should be 100px wide from 'nav a' rule");
    }

    // Regular .btn should be 200px wide
    let cancel = dom.els.iter().find(|e| e.text.as_deref() == Some("Cancel"));
    assert!(cancel.is_some(), "Should find Cancel button");
    assert_eq!(cancel.unwrap().b[2], 200, "Cancel should be 200px from .btn");

    // .btn-primary overrides to 300px
    let save = dom.els.iter().find(|e| e.text.as_deref() == Some("Save"));
    assert!(save.is_some(), "Should find Save button");
    assert_eq!(save.unwrap().b[2], 300, "Save should be 300px from .btn-primary");

    // #submit-btn overrides to 400px (highest specificity)
    let submit = dom.els.iter().find(|e| e.text.as_deref() == Some("Submit"));
    assert!(submit.is_some(), "Should find Submit button");
    assert_eq!(submit.unwrap().b[2], 400, "Submit should be 400px from #submit-btn");

    // .hidden should not appear
    let ghost = dom.els.iter().find(|e| e.text.as_deref() == Some("Ghost"));
    assert!(ghost.is_none(), "Ghost button should be hidden via .hidden class");
}

#[test]
fn test_font_size_inheritance() {
    let html = r#"
    <html>
    <head>
        <style>
            .big-text {
                font-size: 24px;
            }
        </style>
    </head>
    <body>
        <div class="big-text">
            <button>Inherited Big</button>
            <p>Big paragraph</p>
            <div>
                <button>Nested Inherited</button>
            </div>
        </div>
        <div>
            <button>Default Size</button>
        </div>
    </body>
    </html>
    "#;

    let dom = browsy_core::parse(html, 1920.0, 1080.0);

    println!("\n=== Font Size Inheritance Test ===");
    for el in &dom.els {
        println!("  {} ({}) text={:?} bounds={:?}", el.id, el.tag, el.text, el.b);
    }

    // Buttons inside .big-text should be taller than the default-size button
    // because they inherit font-size: 24px, which makes text taller
    let big_btn = dom.els.iter().find(|e| e.text.as_deref() == Some("Inherited Big")).unwrap();
    let nested_btn = dom.els.iter().find(|e| e.text.as_deref() == Some("Nested Inherited")).unwrap();
    let default_btn = dom.els.iter().find(|e| e.text.as_deref() == Some("Default Size")).unwrap();

    // big buttons should have larger height than default (24px * 1.2 = ~29 vs 16px * 1.2 = ~19)
    assert!(
        big_btn.b[3] > default_btn.b[3],
        "Inherited big button height ({}) should be greater than default ({})",
        big_btn.b[3], default_btn.b[3]
    );
    assert!(
        nested_btn.b[3] > default_btn.b[3],
        "Nested inherited button height ({}) should be greater than default ({})",
        nested_btn.b[3], default_btn.b[3]
    );
}

#[test]
fn test_ecommerce_product_card() {
    let html = r#"
    <html>
    <head>
        <style>
            .card {
                display: flex;
                flex-direction: column;
                width: 300px;
                padding: 16px;
            }
            .price {
                font-size: 24px;
            }
            .actions {
                display: flex;
                gap: 8px;
            }
        </style>
    </head>
    <body>
        <div class="card">
            <h2>Wireless Headphones</h2>
            <p class="price">$79.99</p>
            <p>Noise-cancelling, 30hr battery</p>
            <div class="actions">
                <button>Add to Cart</button>
                <button>Buy Now</button>
            </div>
            <a href="/products/headphones">View Details</a>
        </div>
    </body>
    </html>
    "#;

    let dom = browsy_core::parse(html, 1920.0, 1080.0);
    let compact = output::to_compact_string(&dom);

    println!("\n=== E-commerce Card ===");
    println!("{}", compact);

    // Should find heading, price text, buttons, and link
    assert!(dom.els.iter().any(|e| e.text.as_deref() == Some("Wireless Headphones")));
    assert!(dom.els.iter().any(|e| e.text.as_deref() == Some("$79.99")));
    assert!(dom.els.iter().any(|e| e.text.as_deref() == Some("Add to Cart")));
    assert!(dom.els.iter().any(|e| e.text.as_deref() == Some("Buy Now")));
    assert!(dom.els.iter().any(|e| e.href.as_deref() == Some("/products/headphones")));

    // Buttons should be side-by-side (flex row)
    let add = dom.els.iter().find(|e| e.text.as_deref() == Some("Add to Cart")).unwrap();
    let buy = dom.els.iter().find(|e| e.text.as_deref() == Some("Buy Now")).unwrap();
    assert!(add.b[0] < buy.b[0], "Add to Cart should be left of Buy Now");

    // Compact output should be minimal
    let approx_tokens = compact.len() / 4;
    println!("Approx tokens: {}", approx_tokens);
    assert!(approx_tokens < 150, "Should be under 150 tokens, got {}", approx_tokens);
}

#[test]
fn test_navigation_with_dropdown() {
    let html = r#"
    <html>
    <head>
        <style>
            nav { display: flex; width: 100%; height: 60px; align-items: center; gap: 20px; padding: 0 24px; }
            .dropdown { display: flex; flex-direction: column; }
            .dropdown-menu { display: none; }
        </style>
    </head>
    <body>
        <nav>
            <a href="/">Home</a>
            <a href="/products">Products</a>
            <div class="dropdown">
                <button>Account</button>
                <div class="dropdown-menu">
                    <a href="/profile">Profile</a>
                    <a href="/settings">Settings</a>
                    <button>Logout</button>
                </div>
            </div>
            <input type="search" placeholder="Search..." />
        </nav>
    </body>
    </html>
    "#;

    let dom = browsy_core::parse(html, 1920.0, 1080.0);
    let compact = output::to_compact_string(&dom);

    println!("\n=== Navigation Test ===");
    println!("{}", compact);

    // Visible elements: Home, Products, Account button, Search input
    assert!(dom.els.iter().any(|e| e.href.as_deref() == Some("/")));
    assert!(dom.els.iter().any(|e| e.href.as_deref() == Some("/products")));
    assert!(dom.els.iter().any(|e| e.text.as_deref() == Some("Account")));
    assert!(dom.els.iter().any(|e| e.ph.as_deref() == Some("Search...")));

    // Dropdown-menu items should NOT appear (display: none)
    assert!(
        dom.els.iter().find(|e| e.href.as_deref() == Some("/profile")).is_none(),
        "Profile link should be hidden in closed dropdown"
    );
    assert!(
        dom.els.iter().find(|e| e.text.as_deref() == Some("Logout")).is_none(),
        "Logout button should be hidden in closed dropdown"
    );

    // Search input should have role searchbox
    let search = dom.els.iter().find(|e| e.ph.as_deref() == Some("Search...")).unwrap();
    assert_eq!(search.role.as_deref(), Some("searchbox"));
}

#[test]
fn test_form_with_labels_and_validation() {
    let html = r#"
    <html>
    <head>
        <style>
            form { display: flex; flex-direction: column; width: 500px; gap: 12px; padding: 24px; }
            .field { display: flex; flex-direction: column; gap: 4px; }
        </style>
    </head>
    <body>
        <form>
            <h2>Create Account</h2>
            <div class="field">
                <label>Full Name</label>
                <input type="text" placeholder="John Doe" />
            </div>
            <div class="field">
                <label>Email</label>
                <input type="email" placeholder="john@example.com" />
            </div>
            <div class="field">
                <label>Password</label>
                <input type="password" placeholder="Min 8 characters" />
            </div>
            <select>
                <option value="">Select Role</option>
                <option value="dev">Developer</option>
                <option value="pm">Project Manager</option>
            </select>
            <textarea placeholder="Tell us about yourself"></textarea>
            <button>Create Account</button>
        </form>
    </body>
    </html>
    "#;

    let dom = browsy_core::parse(html, 1920.0, 1080.0);
    let compact = output::to_compact_string(&dom);

    println!("\n=== Form Test ===");
    println!("{}", compact);

    // Check all form elements present
    let tags: Vec<&str> = dom.els.iter().map(|e| e.tag.as_str()).collect();
    assert!(tags.contains(&"h2"), "Should find heading");
    assert!(tags.contains(&"label"), "Should find labels");
    assert!(tags.contains(&"input"), "Should find inputs");
    assert!(tags.contains(&"select"), "Should find select");
    assert!(tags.contains(&"textarea"), "Should find textarea");
    assert!(tags.contains(&"button"), "Should find submit button");

    // Check input types
    let inputs: Vec<_> = dom.els.iter().filter(|e| e.tag == "input").collect();
    assert_eq!(inputs.len(), 3, "Should find 3 inputs");

    let email_input = dom.els.iter().find(|e| e.input_type.as_deref() == Some("email")).unwrap();
    assert_eq!(email_input.role.as_deref(), Some("textbox"));

    let pw_input = dom.els.iter().find(|e| e.input_type.as_deref() == Some("password")).unwrap();
    assert_eq!(pw_input.ph.as_deref(), Some("Min 8 characters"));

    // Select should have combobox role
    let select = dom.els.iter().find(|e| e.tag == "select").unwrap();
    assert_eq!(select.role.as_deref(), Some("combobox"));

    // Textarea should have textbox role
    let textarea = dom.els.iter().find(|e| e.tag == "textarea").unwrap();
    assert_eq!(textarea.role.as_deref(), Some("textbox"));

    // Vertical layout: each field should be below the previous
    let labels: Vec<_> = dom.els.iter().filter(|e| e.tag == "label").collect();
    assert_eq!(labels.len(), 3, "Should find 3 labels");
    for i in 1..labels.len() {
        assert!(
            labels[i].b[1] > labels[i - 1].b[1],
            "Label {} should be below label {}",
            i, i - 1
        );
    }
}

#[test]
fn test_table_layout() {
    let html = r#"
    <html>
    <body>
        <table>
            <thead>
                <tr>
                    <th>Name</th>
                    <th>Price</th>
                    <th>Action</th>
                </tr>
            </thead>
            <tbody>
                <tr>
                    <td>Widget A</td>
                    <td>$9.99</td>
                    <td><button>Buy</button></td>
                </tr>
                <tr>
                    <td>Widget B</td>
                    <td>$14.99</td>
                    <td><a href="/buy/b">Purchase</a></td>
                </tr>
            </tbody>
        </table>
    </body>
    </html>
    "#;

    let dom = browsy_core::parse(html, 1920.0, 1080.0);
    let compact = output::to_compact_string(&dom);

    println!("\n=== Table Test ===");
    println!("{}", compact);

    // Should find table headers, cells, buttons, and links
    let ths: Vec<_> = dom.els.iter().filter(|e| e.tag == "th").collect();
    assert_eq!(ths.len(), 3, "Should find 3 table headers");

    let tds: Vec<_> = dom.els.iter().filter(|e| e.tag == "td").collect();
    // 4 text-only cells; 2 cells with interactive children are deduped
    assert_eq!(tds.len(), 4, "Should find 4 text-only table cells (interactive wrappers deduped)");

    assert!(dom.els.iter().any(|e| e.text.as_deref() == Some("Buy")));
    assert!(dom.els.iter().any(|e| e.href.as_deref() == Some("/buy/b")));

    // Table headers should be in the same row (same y, different x)
    assert_eq!(ths[0].b[1], ths[1].b[1], "Headers should share same y");
    assert!(ths[0].b[0] < ths[1].b[0], "Name should be left of Price");
    assert!(ths[1].b[0] < ths[2].b[0], "Price should be left of Action");
}

#[test]
fn test_percentage_widths() {
    let html = r#"
    <html>
    <head>
        <style>
            .container { width: 100%; display: flex; }
            .sidebar { width: 25%; }
            .main { width: 75%; }
        </style>
    </head>
    <body style="margin: 0;">
        <div class="container">
            <div class="sidebar">
                <button>Menu</button>
            </div>
            <div class="main">
                <h1>Content</h1>
                <button>Action</button>
            </div>
        </div>
    </body>
    </html>
    "#;

    let dom = browsy_core::parse(html, 1920.0, 1080.0);
    let compact = output::to_compact_string(&dom);

    println!("\n=== Percentage Width Test ===");
    println!("{}", compact);

    let menu = dom.els.iter().find(|e| e.text.as_deref() == Some("Menu")).unwrap();
    let action = dom.els.iter().find(|e| e.text.as_deref() == Some("Action")).unwrap();

    // Menu button should be in the left 25% (x < 480 on 1920px viewport)
    assert!(
        menu.b[0] < 480,
        "Menu should be in left 25%, got x={}",
        menu.b[0]
    );

    // Action button should be in the right 75% (x >= 480)
    assert!(
        action.b[0] >= 480,
        "Action should be in right 75%, got x={}",
        action.b[0]
    );
}

#[test]
fn test_delta_output() {
    // Page 1: Login page
    let page1 = r#"
    <html><body>
        <h1>Login</h1>
        <input type="email" placeholder="Email" />
        <input type="password" placeholder="Password" />
        <button>Sign In</button>
        <a href="/forgot">Forgot password?</a>
    </body></html>
    "#;

    // Page 2: Dashboard (after login) — different content
    let page2 = r#"
    <html><body>
        <h1>Dashboard</h1>
        <p>Welcome back, User!</p>
        <button>Logout</button>
        <a href="/settings">Settings</a>
    </body></html>
    "#;

    let dom1 = browsy_core::parse(page1, 1920.0, 1080.0);
    let dom2 = browsy_core::parse(page2, 1920.0, 1080.0);

    let delta = output::diff(&dom1, &dom2);

    println!("\n=== Delta Output ===");
    println!("Changed: {} elements", delta.changed.len());
    println!("Removed: {} elements", delta.removed.len());
    println!("{}", output::delta_to_compact_string(&delta));

    // All of page2's elements should show as changed (since content differs)
    assert!(!delta.changed.is_empty(), "Should have changed elements");
    assert!(!delta.removed.is_empty(), "Should have removed elements");

    // Dashboard heading should be in changed
    assert!(
        delta.changed.iter().any(|e| e.text.as_deref() == Some("Dashboard")),
        "Dashboard heading should be in changed"
    );

    // Login heading should be in removed
    let login_h1 = dom1.els.iter().find(|e| e.text.as_deref() == Some("Login")).unwrap();
    assert!(
        delta.removed.contains(&login_h1.id),
        "Login heading should be in removed"
    );

    // Delta should be smaller than full page
    let full_tokens = output::to_compact_string(&dom2).len() / 4;
    let delta_tokens = output::delta_to_compact_string(&delta).len() / 4;
    println!("Full page: ~{} tokens, Delta: ~{} tokens", full_tokens, delta_tokens);

    // Now test minimal change — same page, button text changes
    let page3 = r#"
    <html><body>
        <h1>Dashboard</h1>
        <p>Welcome back, User!</p>
        <button>Sign Out</button>
        <a href="/settings">Settings</a>
    </body></html>
    "#;

    let dom3 = browsy_core::parse(page3, 1920.0, 1080.0);
    let delta2 = output::diff(&dom2, &dom3);

    println!("\n=== Minimal Delta ===");
    println!("Changed: {} elements", delta2.changed.len());
    println!("Removed: {} elements", delta2.removed.len());
    println!("{}", output::delta_to_compact_string(&delta2));

    // Only the button should differ
    assert_eq!(delta2.changed.len(), 1, "Only 1 element should change");
    assert_eq!(delta2.removed.len(), 1, "Only 1 element should be removed");
    assert_eq!(
        delta2.changed[0].text.as_deref(),
        Some("Sign Out"),
        "Changed element should be the new button"
    );
}

#[test]
#[cfg(feature = "fetch")]
fn test_fetch_real_page() {
    let config = fetch::FetchConfig {
        fetch_css: false, // Skip external CSS for speed
        ..Default::default()
    };

    let result = fetch::fetch("https://example.com", &config);
    assert!(result.is_ok(), "Should fetch example.com: {:?}", result.err());

    let dom = result.unwrap();

    println!("\n=== Real Page: example.com ===");
    let compact = output::to_compact_string(&dom);
    println!("{}", compact);
    println!("URL: {}", dom.url);
    println!("Elements: {}", dom.els.len());

    // example.com has a heading and a link
    assert!(!dom.els.is_empty(), "Should find elements on example.com");
    assert!(
        dom.els.iter().any(|e| e.tag == "h1"),
        "Should find h1 heading"
    );
    assert!(
        dom.els.iter().any(|e| e.tag == "a"),
        "Should find a link"
    );
    assert_eq!(dom.url, "https://example.com");

    let approx_tokens = compact.len() / 4;
    println!("Approx tokens: {}", approx_tokens);
}

#[test]
fn test_css_grid_layout() {
    let html = r#"
    <html>
    <head>
        <style>
            .grid {
                display: grid;
                grid-template-columns: 1fr 1fr 1fr;
                gap: 10px;
                width: 600px;
            }
        </style>
    </head>
    <body style="margin: 0;">
        <div class="grid">
            <button>A</button>
            <button>B</button>
            <button>C</button>
            <button>D</button>
            <button>E</button>
            <button>F</button>
        </div>
    </body>
    </html>
    "#;

    let dom = browsy_core::parse(html, 1920.0, 1080.0);
    let compact = output::to_compact_string(&dom);

    println!("\n=== CSS Grid Layout ===");
    println!("{}", compact);

    let buttons: Vec<_> = dom.els.iter().filter(|e| e.tag == "button").collect();
    assert_eq!(buttons.len(), 6, "Should find 6 buttons");

    // First row: A, B, C should be at same y but different x
    assert_eq!(buttons[0].b[1], buttons[1].b[1], "A and B should be on same row");
    assert_eq!(buttons[1].b[1], buttons[2].b[1], "B and C should be on same row");
    assert!(buttons[0].b[0] < buttons[1].b[0], "A should be left of B");
    assert!(buttons[1].b[0] < buttons[2].b[0], "B should be left of C");

    // Second row: D, E, F should be below first row
    assert!(buttons[3].b[1] > buttons[0].b[1], "D should be below A");
}

#[test]
fn test_flex_shorthand() {
    let html = r#"
    <html>
    <head>
        <style>
            .container { display: flex; width: 600px; }
            .sidebar { flex: 0 0 200px; }
            .main { flex: 1; }
        </style>
    </head>
    <body style="margin: 0;">
        <div class="container">
            <div class="sidebar"><button>Nav</button></div>
            <div class="main"><button>Content</button></div>
        </div>
    </body>
    </html>
    "#;

    let dom = browsy_core::parse(html, 1920.0, 1080.0);
    let compact = output::to_compact_string(&dom);

    println!("\n=== Flex Shorthand ===");
    println!("{}", compact);

    let nav = dom.els.iter().find(|e| e.text.as_deref() == Some("Nav")).unwrap();
    let content = dom.els.iter().find(|e| e.text.as_deref() == Some("Content")).unwrap();

    // Sidebar should be 200px wide (flex: 0 0 200px)
    assert_eq!(nav.b[2], 200, "Sidebar should be 200px wide, got {}", nav.b[2]);
    // Content should fill remaining space (400px)
    assert_eq!(content.b[2], 400, "Main should be 400px wide, got {}", content.b[2]);
}

#[test]
fn test_aria_attributes() {
    let html = r#"
    <html><body>
        <button aria-label="Close dialog">X</button>
        <input type="checkbox" checked aria-checked="true" />
        <button disabled>Disabled Button</button>
        <details>
            <summary aria-expanded="false">Show More</summary>
        </details>
        <input type="email" required placeholder="Email" />
        <div aria-hidden="true"><button>Hidden by ARIA</button></div>
        <button aria-label="Search">🔍</button>
    </body></html>
    "#;

    let dom = browsy_core::parse(html, 1920.0, 1080.0);
    let compact = output::to_compact_string(&dom);

    println!("\n=== ARIA Attributes ===");
    println!("{}", compact);
    println!("{}", serde_json::to_string_pretty(&dom).unwrap());

    // Button with aria-label should have text "X" (visible text takes priority)
    let close_btn = dom.els.iter().find(|e| e.text.as_deref() == Some("X")).unwrap();
    assert_eq!(close_btn.role.as_deref(), Some("button"));

    // Checkbox should be checked
    let checkbox = dom.els.iter().find(|e| e.tag == "input" && e.role.as_deref() == Some("checkbox"));
    assert!(checkbox.is_some(), "Should find checkbox");
    assert_eq!(checkbox.unwrap().checked, Some(true));

    // Disabled button
    let disabled = dom.els.iter().find(|e| e.text.as_deref() == Some("Disabled Button")).unwrap();
    assert_eq!(disabled.disabled, Some(true));

    // Required input
    let email = dom.els.iter().find(|e| e.ph.as_deref() == Some("Email")).unwrap();
    assert_eq!(email.required, Some(true));

    // aria-hidden element should not appear
    assert!(
        dom.els.iter().find(|e| e.text.as_deref() == Some("Hidden by ARIA")).is_none(),
        "aria-hidden elements should not appear"
    );

    // Button with aria-label and no visible text uses aria-label
    // (the emoji button has visible text "🔍" so it uses that, but let's check aria-label fallback)
}

#[test]
fn test_landmark_roles() {
    let html = r#"
    <html><body>
        <header><a href="/">Logo</a></header>
        <nav><a href="/about">About</a></nav>
        <main><h1>Content</h1></main>
        <aside><p>Sidebar info</p></aside>
        <footer><p>Copyright 2026</p></footer>
    </body></html>
    "#;

    let dom = browsy_core::parse(html, 1920.0, 1080.0);

    println!("\n=== Landmarks ===");
    println!("{}", serde_json::to_string_pretty(&dom).unwrap());

    // Check heading role
    let h1 = dom.els.iter().find(|e| e.tag == "h1").unwrap();
    assert_eq!(h1.role.as_deref(), Some("heading"));

    // Link roles
    let links: Vec<_> = dom.els.iter().filter(|e| e.tag == "a").collect();
    assert!(links.len() >= 2);
    for link in &links {
        assert_eq!(link.role.as_deref(), Some("link"));
    }
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

    // Verify elements
    assert!(dom.els.iter().any(|e| e.text.as_deref() == Some("Hello World")));
    assert!(dom.els.iter().any(|e| e.text.as_deref() == Some("Click Me")));
    assert!(dom.els.iter().any(|e| e.href.as_deref() == Some("/about")));
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

    // find_by_text
    let save_els = session.find_by_text("Save");
    assert_eq!(save_els.len(), 1);
    assert_eq!(save_els[0].tag, "button");

    // find_by_text partial match
    let dashboard = session.find_by_text("Dashboard");
    assert_eq!(dashboard.len(), 1);

    // find_by_role
    let buttons = session.find_by_role("button");
    assert_eq!(buttons.len(), 2, "Should find 2 buttons");

    let links = session.find_by_role("link");
    assert_eq!(links.len(), 1, "Should find 1 link");

    let textboxes = session.find_by_role("textbox");
    assert_eq!(textboxes.len(), 1, "Should find 1 textbox");

    // element() by ID
    let first_id = session.dom().unwrap().els[0].id;
    let el = session.element(first_id);
    assert!(el.is_some());
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

    let inputs = session.find_by_role("textbox");
    assert_eq!(inputs.len(), 2);
    let username_id = inputs[0].id;
    let password_id = inputs[1].id;

    // Type into username field
    let result = session.type_text(username_id, "admin");
    assert!(result.is_ok());

    // Type into password (textbox role)
    let result = session.type_text(password_id, "secret123");
    assert!(result.is_ok());

    // Typing into non-input should fail
    let button_id = session.find_by_role("button")[0].id;
    let result = session.type_text(button_id, "text");
    assert!(result.is_err());
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

    let selects = session.find_by_role("combobox");
    assert_eq!(selects.len(), 1);
    let select_id = selects[0].id;

    // Select an option
    let result = session.select(select_id, "blue");
    assert!(result.is_ok());

    // Select on non-select should fail
    let button_id = session.find_by_role("button")[0].id;
    let result = session.select(button_id, "val");
    assert!(result.is_err());
}

#[test]
#[cfg(feature = "fetch")]
fn test_session_delta() {
    let mut session = Session::new().unwrap();

    // No delta before loading
    assert!(session.delta().is_none());

    // Load first page
    let page1 = r#"<html><body><h1>Page 1</h1><button>Next</button></body></html>"#;
    session.load_html(page1, "http://localhost/1").unwrap();

    // Still no delta (no previous page)
    assert!(session.delta().is_none());

    // Load second page
    let page2 = r#"<html><body><h1>Page 2</h1><button>Back</button></body></html>"#;
    session.load_html(page2, "http://localhost/2").unwrap();

    // Now we should have a delta
    let delta = session.delta().unwrap();
    assert!(!delta.changed.is_empty(), "Should have changed elements");
    assert!(!delta.removed.is_empty(), "Should have removed elements");

    // New heading should be in changed
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

    // Should have a heading
    assert!(dom.els.iter().any(|e| e.tag == "h1"));
}

#[test]
fn test_js_detect_onclick_toggle() {
    let html = r#"
    <html><body>
        <button onclick="document.getElementById('menu').style.display = 'block'">Open Menu</button>
        <div id="menu" style="display: none;">
            <a href="/profile">Profile</a>
            <a href="/settings">Settings</a>
        </div>
    </body></html>
    "#;

    let dom_tree = browsy_core::dom::parse_html(html);
    let behaviors = js::detect_behaviors(&dom_tree);

    assert!(!behaviors.is_empty(), "Should detect onclick behavior");
    let first = &behaviors[0];
    match &first.action {
        js::JsAction::ToggleVisibility { target } => {
            assert_eq!(target, "#menu");
        }
        other => panic!("Expected ToggleVisibility, got {:?}", other),
    }
}

#[test]
fn test_js_apply_toggle_visibility() {
    let html = r#"
    <html><body>
        <button onclick="toggle('dropdown')">Toggle</button>
        <div id="dropdown" style="display: none;">
            <a href="/a">Option A</a>
            <a href="/b">Option B</a>
        </div>
    </body></html>
    "#;

    // Before toggle: dropdown hidden
    let dom = browsy_core::parse(html, 1920.0, 1080.0);
    assert!(
        dom.els.iter().find(|e| e.href.as_deref() == Some("/a")).is_none(),
        "Options should be hidden before toggle"
    );

    // Apply toggle
    let dom_tree = browsy_core::dom::parse_html(html);
    let action = js::JsAction::ToggleVisibility {
        target: "#dropdown".to_string(),
    };
    let modified = js::apply_action(&dom_tree, &action);

    // Re-parse the modified DOM
    let styled = browsy_core::css::compute_styles(&modified);
    let laid_out = browsy_core::layout::compute_layout(&styled, 1920.0, 1080.0);
    let dom2 = browsy_core::output::generate_spatial_dom(&laid_out, 1920.0, 1080.0);

    // After toggle: dropdown visible
    assert!(
        dom2.els.iter().any(|e| e.href.as_deref() == Some("/a")),
        "Option A should be visible after toggle"
    );
    assert!(
        dom2.els.iter().any(|e| e.href.as_deref() == Some("/b")),
        "Option B should be visible after toggle"
    );
}

#[test]
fn test_js_class_toggle() {
    let html = r#"
    <html>
    <head><style>.hidden { display: none; }</style></head>
    <body>
        <button onclick="document.getElementById('panel').classList.toggle('hidden')">Toggle Panel</button>
        <div id="panel" class="hidden">
            <p>Panel content</p>
        </div>
    </body></html>
    "#;

    let dom_tree = browsy_core::dom::parse_html(html);
    let behaviors = js::detect_behaviors(&dom_tree);

    assert!(!behaviors.is_empty(), "Should detect class toggle");
    match &behaviors[0].action {
        js::JsAction::ToggleClass { target, class } => {
            assert_eq!(target, "#panel");
            assert_eq!(class, "hidden");
        }
        other => panic!("Expected ToggleClass, got {:?}", other),
    }

    // Apply the class toggle
    let action = &behaviors[0].action;
    let modified = js::apply_action(&dom_tree, action);

    // Panel should no longer have 'hidden' class
    fn find_panel(node: &browsy_core::dom::DomNode) -> Option<String> {
        if node.get_attr("id") == Some("panel") {
            return node.get_attr("class").map(|s| s.to_string());
        }
        for child in &node.children {
            if let Some(class) = find_panel(child) {
                return Some(class);
            }
        }
        None
    }

    let panel_class = find_panel(&modified).unwrap_or_default();
    assert!(
        !panel_class.contains("hidden"),
        "Panel should no longer have 'hidden' class, got '{}'",
        panel_class
    );
}

#[test]
fn test_js_data_toggle_bootstrap() {
    let html = r##"
    <html><body>
        <button data-toggle="collapse" data-target="#navbar">Menu</button>
        <div id="navbar" style="display: none;">
            <a href="/home">Home</a>
        </div>
    </body></html>
    "##;

    let dom_tree = browsy_core::dom::parse_html(html);
    let behaviors = js::detect_behaviors(&dom_tree);

    assert!(!behaviors.is_empty(), "Should detect Bootstrap data-toggle");
    match &behaviors[0].action {
        js::JsAction::ToggleVisibility { target } => {
            assert_eq!(target, "#navbar");
        }
        other => panic!("Expected ToggleVisibility, got {:?}", other),
    }
}

#[test]
fn test_js_aria_controls() {
    let html = r#"
    <html><body>
        <button aria-expanded="false" aria-controls="details-panel">Show Details</button>
        <div id="details-panel" style="display: none;">
            <p>Detailed information here</p>
        </div>
    </body></html>
    "#;

    let dom_tree = browsy_core::dom::parse_html(html);
    let behaviors = js::detect_behaviors(&dom_tree);

    assert!(!behaviors.is_empty(), "Should detect aria-controls toggle");
    match &behaviors[0].action {
        js::JsAction::ToggleVisibility { target } => {
            assert_eq!(target, "#details-panel");
        }
        other => panic!("Expected ToggleVisibility, got {:?}", other),
    }
}

#[test]
fn test_js_tab_detection() {
    let html = r#"
    <html><body>
        <div role="tablist">
            <button role="tab" aria-controls="tab1-panel" aria-selected="true">Tab 1</button>
            <button role="tab" aria-controls="tab2-panel" aria-selected="false">Tab 2</button>
        </div>
        <div id="tab1-panel" role="tabpanel">
            <p>Content for tab 1</p>
        </div>
        <div id="tab2-panel" role="tabpanel" style="display: none;">
            <p>Content for tab 2</p>
        </div>
    </body></html>
    "#;

    let dom_tree = browsy_core::dom::parse_html(html);

    // Detect tab groups
    let tab_groups = js::detect_tab_groups(&dom_tree);
    assert_eq!(tab_groups.len(), 1, "Should find 1 tab group");
    assert_eq!(tab_groups[0].tabs.len(), 2, "Should find 2 tabs");
    assert_eq!(tab_groups[0].tabs[0].label, "Tab 1");
    assert!(tab_groups[0].tabs[0].selected);
    assert!(!tab_groups[0].tabs[1].selected);

    // Detect tab switch behaviors
    let behaviors = js::detect_behaviors(&dom_tree);
    let tab_behaviors: Vec<_> = behaviors.iter()
        .filter(|b| matches!(&b.action, js::JsAction::TabSwitch { .. }))
        .collect();
    assert_eq!(tab_behaviors.len(), 2, "Should detect 2 tab switch behaviors");
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

    // Menu should be hidden initially
    let dom = session.dom().unwrap();
    assert!(
        dom.els.iter().find(|e| e.href.as_deref() == Some("/profile")).is_none(),
        "Profile link should be hidden initially"
    );

    // Click the toggle button
    let btn = session.find_by_role("button");
    assert!(!btn.is_empty(), "Should find toggle button");
    let btn_id = btn[0].id;
    session.click(btn_id).unwrap();

    // Menu should now be visible
    let dom = session.dom().unwrap();
    assert!(
        dom.els.iter().any(|e| e.href.as_deref() == Some("/profile")),
        "Profile link should be visible after toggle"
    );
}
