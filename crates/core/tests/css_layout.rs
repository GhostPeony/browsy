//! Tests for CSS parsing, style computation, and layout positioning.

use browsy_core::output;

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
    assert!(!dom.els.is_empty());

    let tags: Vec<&str> = dom.els.iter().map(|e| e.tag.as_str()).collect();
    assert!(tags.contains(&"input"));
    assert!(tags.contains(&"button"));
    assert!(tags.contains(&"a"));
    assert!(tags.contains(&"h1"));

    for el in &dom.els {
        assert!(el.b[2] > 0 || el.b[3] > 0, "Element {} ({}) should have non-zero size", el.id, el.tag);
    }

    let email = dom.els.iter().find(|e| e.ph.as_deref() == Some("Email")).unwrap();
    assert_eq!(email.role.as_deref(), Some("textbox"));
    assert_eq!(email.input_type.as_deref(), Some("email"));

    let pw = dom.els.iter().find(|e| e.ph.as_deref() == Some("Password")).unwrap();
    assert_eq!(pw.input_type.as_deref(), Some("password"));

    let buttons: Vec<_> = dom.els.iter().filter(|e| e.role.as_deref() == Some("button")).collect();
    assert!(buttons.len() >= 2);

    let compact = output::to_compact_string(&dom);
    let approx_tokens = compact.len() / 4;
    assert!(approx_tokens < 200);
}

#[test]
fn test_flex_layout_positioning() {
    let html = r#"
    <html>
    <body style="margin: 0;">
        <div style="display: flex; width: 600px; height: 100px;">
            <div style="width: 200px; height: 100px;"><button>A</button></div>
            <div style="width: 200px; height: 100px;"><button>B</button></div>
            <div style="width: 200px; height: 100px;"><button>C</button></div>
        </div>
    </body>
    </html>
    "#;

    let dom = browsy_core::parse(html, 1920.0, 1080.0);
    let buttons: Vec<_> = dom.els.iter().filter(|e| e.tag == "button").collect();
    assert_eq!(buttons.len(), 3);
    assert!(buttons[0].b[0] < buttons[1].b[0]);
    assert!(buttons[1].b[0] < buttons[2].b[0]);
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
    // All buttons are present, but hidden ones are flagged
    let visible: Vec<_> = dom.visible().iter().filter(|e| e.tag == "button").cloned().collect();
    assert_eq!(visible.len(), 1);
    assert_eq!(visible[0].text.as_deref(), Some("Visible"));

    // Hidden buttons are included with hidden: true
    let hidden: Vec<_> = dom.els.iter().filter(|e| e.tag == "button" && e.hidden == Some(true)).collect();
    assert!(hidden.len() >= 2); // "Hidden" and "Also Hidden" (visibility:hidden also flagged)
}

#[test]
fn test_viewport_size() {
    let html = "<html><body><button>Click</button></body></html>";
    assert_eq!(browsy_core::parse(html, 1920.0, 1080.0).vp, [1920.0, 1080.0]);
    assert_eq!(browsy_core::parse(html, 1280.0, 720.0).vp, [1280.0, 720.0]);
}

#[test]
fn test_css_style_tag_selectors() {
    let html = r#"
    <html>
    <head>
        <style>
            .container { display: flex; flex-direction: column; width: 600px; gap: 12px; padding: 20px; }
            .btn { display: flex; height: 40px; width: 200px; }
            .btn-primary { width: 300px; }
            #submit-btn { width: 400px; }
            nav a { display: flex; width: 100px; height: 32px; }
            .hidden { display: none; }
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

    let links: Vec<_> = dom.els.iter().filter(|e| e.tag == "a").collect();
    assert_eq!(links.len(), 2);
    for link in &links {
        assert_eq!(link.b[2], 100);
    }

    assert_eq!(dom.els.iter().find(|e| e.text.as_deref() == Some("Cancel")).unwrap().b[2], 200);
    assert_eq!(dom.els.iter().find(|e| e.text.as_deref() == Some("Save")).unwrap().b[2], 300);
    assert_eq!(dom.els.iter().find(|e| e.text.as_deref() == Some("Submit")).unwrap().b[2], 400);
    let ghost = dom.els.iter().find(|e| e.text.as_deref() == Some("Ghost")).unwrap();
    assert_eq!(ghost.hidden, Some(true));
}

#[test]
fn test_font_size_inheritance() {
    let html = r#"
    <html>
    <head><style>.big-text { font-size: 24px; }</style></head>
    <body>
        <div class="big-text">
            <button>Inherited Big</button>
            <div><button>Nested Inherited</button></div>
        </div>
        <div><button>Default Size</button></div>
    </body>
    </html>
    "#;

    let dom = browsy_core::parse(html, 1920.0, 1080.0);
    let big = dom.els.iter().find(|e| e.text.as_deref() == Some("Inherited Big")).unwrap();
    let nested = dom.els.iter().find(|e| e.text.as_deref() == Some("Nested Inherited")).unwrap();
    let default = dom.els.iter().find(|e| e.text.as_deref() == Some("Default Size")).unwrap();

    assert!(big.b[3] > default.b[3]);
    assert!(nested.b[3] > default.b[3]);
}

#[test]
fn test_ecommerce_product_card() {
    let html = r#"
    <html>
    <head>
        <style>
            .card { display: flex; flex-direction: column; width: 300px; padding: 16px; }
            .actions { display: flex; gap: 8px; }
        </style>
    </head>
    <body>
        <div class="card">
            <h2>Wireless Headphones</h2>
            <p>$79.99</p>
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
    assert!(dom.els.iter().any(|e| e.text.as_deref() == Some("Wireless Headphones")));
    assert!(dom.els.iter().any(|e| e.text.as_deref() == Some("$79.99")));
    assert!(dom.els.iter().any(|e| e.text.as_deref() == Some("Add to Cart")));

    let add = dom.els.iter().find(|e| e.text.as_deref() == Some("Add to Cart")).unwrap();
    let buy = dom.els.iter().find(|e| e.text.as_deref() == Some("Buy Now")).unwrap();
    assert!(add.b[0] < buy.b[0]);
}

#[test]
fn test_navigation_with_dropdown() {
    let html = r#"
    <html>
    <head>
        <style>
            nav { display: flex; width: 100%; height: 60px; align-items: center; gap: 20px; padding: 0 24px; }
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
                    <button>Logout</button>
                </div>
            </div>
            <input type="search" placeholder="Search..." />
        </nav>
    </body>
    </html>
    "#;

    let dom = browsy_core::parse(html, 1920.0, 1080.0);
    assert!(dom.els.iter().any(|e| e.text.as_deref() == Some("Account")));
    assert!(dom.els.iter().any(|e| e.ph.as_deref() == Some("Search...")));
    // Dropdown items are present but hidden
    let profile = dom.els.iter().find(|e| e.href.as_deref() == Some("/profile")).unwrap();
    assert_eq!(profile.hidden, Some(true));

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
            <div class="field"><label>Full Name</label><input type="text" placeholder="John Doe" /></div>
            <div class="field"><label>Email</label><input type="email" placeholder="john@example.com" /></div>
            <div class="field"><label>Password</label><input type="password" placeholder="Min 8 characters" /></div>
            <select><option value="">Select Role</option><option value="dev">Developer</option></select>
            <textarea placeholder="Tell us about yourself"></textarea>
            <button>Create Account</button>
        </form>
    </body>
    </html>
    "#;

    let dom = browsy_core::parse(html, 1920.0, 1080.0);
    let tags: Vec<&str> = dom.els.iter().map(|e| e.tag.as_str()).collect();
    assert!(tags.contains(&"h2"));
    assert!(tags.contains(&"label"));
    assert!(tags.contains(&"input"));
    assert!(tags.contains(&"select"));
    assert!(tags.contains(&"textarea"));
    assert!(tags.contains(&"button"));

    let inputs: Vec<_> = dom.els.iter().filter(|e| e.tag == "input").collect();
    assert_eq!(inputs.len(), 3);

    assert_eq!(dom.els.iter().find(|e| e.tag == "select").unwrap().role.as_deref(), Some("combobox"));
    assert_eq!(dom.els.iter().find(|e| e.tag == "textarea").unwrap().role.as_deref(), Some("textbox"));
}

#[test]
fn test_table_layout() {
    let html = r#"
    <html><body>
        <table>
            <thead><tr><th>Name</th><th>Price</th><th>Action</th></tr></thead>
            <tbody>
                <tr><td>Widget A</td><td>$9.99</td><td><button>Buy</button></td></tr>
                <tr><td>Widget B</td><td>$14.99</td><td><a href="/buy/b">Purchase</a></td></tr>
            </tbody>
        </table>
    </body></html>
    "#;

    let dom = browsy_core::parse(html, 1920.0, 1080.0);
    let ths: Vec<_> = dom.els.iter().filter(|e| e.tag == "th").collect();
    assert_eq!(ths.len(), 3);
    let tds: Vec<_> = dom.els.iter().filter(|e| e.tag == "td").collect();
    assert_eq!(tds.len(), 4); // interactive wrappers deduped

    assert_eq!(ths[0].b[1], ths[1].b[1]);
    assert!(ths[0].b[0] < ths[1].b[0]);
}

#[test]
fn test_percentage_widths() {
    let html = r#"
    <html>
    <head><style>.container { width: 100%; display: flex; } .sidebar { width: 25%; } .main { width: 75%; }</style></head>
    <body style="margin: 0;">
        <div class="container">
            <div class="sidebar"><button>Menu</button></div>
            <div class="main"><h1>Content</h1><button>Action</button></div>
        </div>
    </body>
    </html>
    "#;

    let dom = browsy_core::parse(html, 1920.0, 1080.0);
    let menu = dom.els.iter().find(|e| e.text.as_deref() == Some("Menu")).unwrap();
    let action = dom.els.iter().find(|e| e.text.as_deref() == Some("Action")).unwrap();
    assert!(menu.b[0] < 480);
    assert!(action.b[0] >= 480);
}

#[test]
fn test_css_grid_layout() {
    let html = r#"
    <html>
    <head><style>.grid { display: grid; grid-template-columns: 1fr 1fr 1fr; gap: 10px; width: 600px; }</style></head>
    <body style="margin: 0;">
        <div class="grid">
            <button>A</button><button>B</button><button>C</button>
            <button>D</button><button>E</button><button>F</button>
        </div>
    </body>
    </html>
    "#;

    let dom = browsy_core::parse(html, 1920.0, 1080.0);
    let buttons: Vec<_> = dom.els.iter().filter(|e| e.tag == "button").collect();
    assert_eq!(buttons.len(), 6);

    assert_eq!(buttons[0].b[1], buttons[1].b[1]);
    assert!(buttons[0].b[0] < buttons[1].b[0]);
    assert!(buttons[3].b[1] > buttons[0].b[1]);
}

#[test]
fn test_flex_shorthand() {
    let html = r#"
    <html>
    <head><style>.container { display: flex; width: 600px; } .sidebar { flex: 0 0 200px; } .main { flex: 1; }</style></head>
    <body style="margin: 0;">
        <div class="container">
            <div class="sidebar"><button>Nav</button></div>
            <div class="main"><button>Content</button></div>
        </div>
    </body>
    </html>
    "#;

    let dom = browsy_core::parse(html, 1920.0, 1080.0);
    assert_eq!(dom.els.iter().find(|e| e.text.as_deref() == Some("Nav")).unwrap().b[2], 200);
    assert_eq!(dom.els.iter().find(|e| e.text.as_deref() == Some("Content")).unwrap().b[2], 400);
}

#[test]
fn test_calc_pure_px() {
    let html = r#"
    <html>
    <body style="margin: 0;">
        <div style="width: calc(200px + 100px); height: 50px;"><button>A</button></div>
    </body>
    </html>
    "#;

    let dom = browsy_core::parse(html, 1920.0, 1080.0);
    let btn = dom.els.iter().find(|e| e.text.as_deref() == Some("A")).unwrap();
    assert_eq!(btn.b[2], 300);
}

#[test]
fn test_calc_percent_minus_px() {
    let html = r#"
    <html>
    <head><style>
        .container { width: 1000px; display: flex; }
        .main { width: calc(100% - 200px); }
    </style></head>
    <body style="margin: 0;">
        <div class="container">
            <div class="main"><button>Content</button></div>
        </div>
    </body>
    </html>
    "#;

    let dom = browsy_core::parse(html, 1920.0, 1080.0);
    let btn = dom.els.iter().find(|e| e.text.as_deref() == Some("Content")).unwrap();
    // 100% of 1000px - 200px = 800px
    assert_eq!(btn.b[2], 800);
}

#[test]
fn test_calc_multiplication() {
    let html = r#"
    <html>
    <body style="margin: 0;">
        <div style="width: calc(3 * 100px); height: 50px;"><button>B</button></div>
    </body>
    </html>
    "#;

    let dom = browsy_core::parse(html, 1920.0, 1080.0);
    let btn = dom.els.iter().find(|e| e.text.as_deref() == Some("B")).unwrap();
    assert_eq!(btn.b[2], 300);
}

#[test]
fn test_calc_division() {
    let html = r#"
    <html>
    <head><style>
        .third { width: calc(100% / 3); }
        .container { width: 900px; display: flex; }
    </style></head>
    <body style="margin: 0;">
        <div class="container">
            <div class="third"><button>C</button></div>
        </div>
    </body>
    </html>
    "#;

    let dom = browsy_core::parse(html, 1920.0, 1080.0);
    let btn = dom.els.iter().find(|e| e.text.as_deref() == Some("C")).unwrap();
    assert_eq!(btn.b[2], 300);
}

#[test]
fn test_calc_em_units() {
    let html = r#"
    <html>
    <head><style>.big { font-size: 20px; }</style></head>
    <body style="margin: 0;">
        <div class="big" style="width: calc(10em + 50px); height: 50px;"><button>D</button></div>
    </body>
    </html>
    "#;

    let dom = browsy_core::parse(html, 1920.0, 1080.0);
    let btn = dom.els.iter().find(|e| e.text.as_deref() == Some("D")).unwrap();
    // 10 * 20px + 50px = 250px
    assert_eq!(btn.b[2], 250);
}

#[test]
fn test_media_query_max_width() {
    let html = r#"
    <html>
    <head><style>
        .box { width: 500px; }
        @media (max-width: 768px) {
            .box { width: 300px; }
        }
    </style></head>
    <body style="margin: 0;">
        <div class="box"><button>Mobile</button></div>
    </body>
    </html>
    "#;

    // Desktop viewport (1920px): max-width: 768px does NOT match
    let dom = browsy_core::parse(html, 1920.0, 1080.0);
    let btn = dom.els.iter().find(|e| e.text.as_deref() == Some("Mobile")).unwrap();
    assert_eq!(btn.b[2], 500);

    // Mobile viewport (375px): max-width: 768px DOES match
    let dom = browsy_core::parse(html, 375.0, 812.0);
    let btn = dom.els.iter().find(|e| e.text.as_deref() == Some("Mobile")).unwrap();
    assert_eq!(btn.b[2], 300);
}

#[test]
fn test_media_query_min_width() {
    let html = r#"
    <html>
    <head><style>
        .box { width: 200px; }
        @media (min-width: 1024px) {
            .box { width: 600px; }
        }
    </style></head>
    <body style="margin: 0;">
        <div class="box"><button>Wide</button></div>
    </body>
    </html>
    "#;

    // Desktop: min-width: 1024 matches
    let dom = browsy_core::parse(html, 1920.0, 1080.0);
    let btn = dom.els.iter().find(|e| e.text.as_deref() == Some("Wide")).unwrap();
    assert_eq!(btn.b[2], 600);

    // Tablet: min-width: 1024 does NOT match
    let dom = browsy_core::parse(html, 768.0, 1024.0);
    let btn = dom.els.iter().find(|e| e.text.as_deref() == Some("Wide")).unwrap();
    assert_eq!(btn.b[2], 200);
}

#[test]
fn test_media_query_range() {
    let html = r#"
    <html>
    <head><style>
        .nav { display: none; }
        @media (min-width: 768px) and (max-width: 1024px) {
            .nav { display: flex; width: 768px; }
        }
    </style></head>
    <body style="margin: 0;">
        <div class="nav"><button>Tablet Nav</button></div>
    </body>
    </html>
    "#;

    // Mobile: out of range, nav hidden but present
    let dom = browsy_core::parse(html, 375.0, 812.0);
    let tab_nav = dom.els.iter().find(|e| e.text.as_deref() == Some("Tablet Nav")).unwrap();
    assert_eq!(tab_nav.hidden, Some(true));

    // Tablet: in range, nav visible
    let dom = browsy_core::parse(html, 800.0, 1024.0);
    let tab_nav = dom.els.iter().find(|e| e.text.as_deref() == Some("Tablet Nav")).unwrap();
    assert!(tab_nav.hidden.is_none());

    // Desktop: out of range, nav hidden but present
    let dom = browsy_core::parse(html, 1920.0, 1080.0);
    let tab_nav = dom.els.iter().find(|e| e.text.as_deref() == Some("Tablet Nav")).unwrap();
    assert_eq!(tab_nav.hidden, Some(true));
}

#[test]
fn test_media_query_screen_and_print() {
    let html = r#"
    <html>
    <head><style>
        .screen-only { width: 400px; }
        @media print { .screen-only { display: none; } }
        @media screen { .visible { width: 200px; } }
    </style></head>
    <body style="margin: 0;">
        <div class="screen-only"><button>Screen</button></div>
        <div class="visible"><button>Both</button></div>
    </body>
    </html>
    "#;

    let dom = browsy_core::parse(html, 1920.0, 1080.0);
    // screen-only should be visible (print media doesn't match)
    assert!(dom.els.iter().any(|e| e.text.as_deref() == Some("Screen")));
    // @media screen rules should apply
    let both = dom.els.iter().find(|e| e.text.as_deref() == Some("Both")).unwrap();
    assert_eq!(both.b[2], 200);
}

#[test]
fn test_attr_selector_exact() {
    let html = r#"
    <html><head><style>
        [type="email"] { width: 300px; }
    </style></head>
    <body style="margin: 0;">
        <input type="email" />
        <input type="text" />
    </body></html>
    "#;
    let dom = browsy_core::parse(html, 1920.0, 1080.0);
    let inputs: Vec<_> = dom.els.iter().filter(|e| e.tag == "input").collect();
    assert_eq!(inputs[0].b[2], 300); // email matches
    assert_ne!(inputs[1].b[2], 300); // text does not match
}

#[test]
fn test_attr_selector_prefix() {
    let html = r#"
    <html><head><style>
        [href^="/docs"] { width: 200px; }
    </style></head>
    <body style="margin: 0;">
        <a href="/docs/intro">Docs</a>
        <a href="/about">About</a>
    </body></html>
    "#;
    let dom = browsy_core::parse(html, 1920.0, 1080.0);
    let docs = dom.els.iter().find(|e| e.text.as_deref() == Some("Docs")).unwrap();
    let about = dom.els.iter().find(|e| e.text.as_deref() == Some("About")).unwrap();
    assert_eq!(docs.b[2], 200);
    assert_ne!(about.b[2], 200);
}

#[test]
fn test_attr_selector_contains() {
    let html = r#"
    <html><head><style>
        [class*="btn"] { width: 150px; }
    </style></head>
    <body style="margin: 0;">
        <button class="btn-primary">Primary</button>
        <button class="link">Link</button>
    </body></html>
    "#;
    let dom = browsy_core::parse(html, 1920.0, 1080.0);
    let primary = dom.els.iter().find(|e| e.text.as_deref() == Some("Primary")).unwrap();
    let link = dom.els.iter().find(|e| e.text.as_deref() == Some("Link")).unwrap();
    assert_eq!(primary.b[2], 150);
    assert_ne!(link.b[2], 150);
}

#[test]
fn test_attr_selector_suffix() {
    let html = r#"
    <html><head><style>
        [href$=".pdf"] { width: 250px; }
    </style></head>
    <body style="margin: 0;">
        <a href="/file.pdf">PDF</a>
        <a href="/file.html">HTML</a>
    </body></html>
    "#;
    let dom = browsy_core::parse(html, 1920.0, 1080.0);
    let pdf = dom.els.iter().find(|e| e.text.as_deref() == Some("PDF")).unwrap();
    let html_link = dom.els.iter().find(|e| e.text.as_deref() == Some("HTML")).unwrap();
    assert_eq!(pdf.b[2], 250);
    assert_ne!(html_link.b[2], 250);
}

#[test]
fn test_attr_selector_exists() {
    let html = r#"
    <html><head><style>
        [disabled] { width: 100px; }
    </style></head>
    <body style="margin: 0;">
        <input disabled />
        <input />
    </body></html>
    "#;
    let dom = browsy_core::parse(html, 1920.0, 1080.0);
    let inputs: Vec<_> = dom.els.iter().filter(|e| e.tag == "input").collect();
    assert_eq!(inputs[0].b[2], 100); // disabled matches
    assert_ne!(inputs[1].b[2], 100); // no disabled
}

#[test]
fn test_css_variables_basic() {
    let html = r#"
    <html><head><style>
        :root {
            --main-width: 400px;
            --padding: 20px;
        }
        .box { width: var(--main-width); padding: var(--padding); }
    </style></head>
    <body style="margin: 0;">
        <div class="box"><button>Click</button></div>
    </body></html>
    "#;
    let dom = browsy_core::parse(html, 1920.0, 1080.0);
    let button = dom.els.iter().find(|e| e.tag == "button").unwrap();
    // Button should be inside a 400px-wide box with 20px padding
    // So button x should be at least 20 (padding-left)
    assert!(button.b[0] >= 20, "button should be offset by padding, got x={}", button.b[0]);
}

#[test]
fn test_css_variables_fallback() {
    let html = r#"
    <html><head><style>
        .box { width: var(--undefined, 300px); }
    </style></head>
    <body style="margin: 0;">
        <div class="box"><button>Click</button></div>
    </body></html>
    "#;
    let dom = browsy_core::parse(html, 1920.0, 1080.0);
    let button = dom.els.iter().find(|e| e.tag == "button").unwrap();
    // Box should use fallback value of 300px
    assert!(button.b[2] <= 300, "button should fit in 300px box, got w={}", button.b[2]);
}

#[test]
fn test_css_variables_inheritance() {
    let html = r#"
    <html><head><style>
        .parent { --item-width: 200px; }
        .child { width: var(--item-width); }
    </style></head>
    <body style="margin: 0;">
        <div class="parent">
            <div class="child"><button>Inside</button></div>
        </div>
    </body></html>
    "#;
    let dom = browsy_core::parse(html, 1920.0, 1080.0);
    let button = dom.els.iter().find(|e| e.tag == "button").unwrap();
    // Child should inherit --item-width from parent and be 200px wide
    assert!(button.b[2] <= 200, "button should fit in 200px child, got w={}", button.b[2]);
}

#[test]
fn test_css_variables_chain() {
    let html = r#"
    <html><head><style>
        :root {
            --base-size: 150px;
            --element-width: var(--base-size);
        }
        .item { width: var(--element-width); }
    </style></head>
    <body style="margin: 0;">
        <div class="item"><button>Chained</button></div>
    </body></html>
    "#;
    let dom = browsy_core::parse(html, 1920.0, 1080.0);
    let button = dom.els.iter().find(|e| e.tag == "button").unwrap();
    // Should resolve var chain: --element-width -> --base-size -> 150px
    assert!(button.b[2] <= 150, "button should fit in 150px item, got w={}", button.b[2]);
}
