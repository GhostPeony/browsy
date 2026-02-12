//! Tests for output generation, delta diffing, ARIA, URL resolution, and labels.

use browsy_core::output;

#[test]
fn test_delta_output() {
    let page1 = r#"
    <html><body>
        <h1>Login</h1>
        <input type="email" placeholder="Email" />
        <input type="password" placeholder="Password" />
        <button>Sign In</button>
        <a href="/forgot">Forgot password?</a>
    </body></html>
    "#;

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

    assert!(!delta.changed.is_empty());
    assert!(!delta.removed.is_empty());
    assert!(delta.changed.iter().any(|e| e.text.as_deref() == Some("Dashboard")));

    let login_h1 = dom1.els.iter().find(|e| e.text.as_deref() == Some("Login")).unwrap();
    assert!(delta.removed.contains(&login_h1.id));

    // Minimal change: only button text differs
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

    assert_eq!(delta2.changed.len(), 1);
    assert_eq!(delta2.removed.len(), 1);
    assert_eq!(delta2.changed[0].text.as_deref(), Some("Sign Out"));
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

    let close_btn = dom.els.iter().find(|e| e.text.as_deref() == Some("X")).unwrap();
    assert_eq!(close_btn.role.as_deref(), Some("button"));

    let checkbox = dom.els.iter().find(|e| e.tag == "input" && e.role.as_deref() == Some("checkbox"));
    assert!(checkbox.is_some());
    assert_eq!(checkbox.unwrap().checked, Some(true));

    let disabled = dom.els.iter().find(|e| e.text.as_deref() == Some("Disabled Button")).unwrap();
    assert_eq!(disabled.disabled, Some(true));

    let email = dom.els.iter().find(|e| e.ph.as_deref() == Some("Email")).unwrap();
    assert_eq!(email.required, Some(true));

    let aria_hidden = dom.els.iter().find(|e| e.text.as_deref() == Some("Hidden by ARIA")).unwrap();
    assert_eq!(aria_hidden.hidden, Some(true));
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

    // Landmark elements are emitted as structural markers
    let header = dom.els.iter().find(|e| e.tag == "header").unwrap();
    assert_eq!(header.role.as_deref(), Some("banner"));
    let nav = dom.els.iter().find(|e| e.tag == "nav").unwrap();
    assert_eq!(nav.role.as_deref(), Some("navigation"));
    let main_el = dom.els.iter().find(|e| e.tag == "main").unwrap();
    assert_eq!(main_el.role.as_deref(), Some("main"));
    let aside = dom.els.iter().find(|e| e.tag == "aside").unwrap();
    assert_eq!(aside.role.as_deref(), Some("complementary"));
    let footer = dom.els.iter().find(|e| e.tag == "footer").unwrap();
    assert_eq!(footer.role.as_deref(), Some("contentinfo"));

    // Children are still emitted normally
    let h1 = dom.els.iter().find(|e| e.tag == "h1").unwrap();
    assert_eq!(h1.role.as_deref(), Some("heading"));

    let links: Vec<_> = dom.els.iter().filter(|e| e.tag == "a").collect();
    assert!(links.len() >= 2);
    for link in &links {
        assert_eq!(link.role.as_deref(), Some("link"));
    }
}

#[test]
fn test_landmark_no_text_blobs() {
    // Landmarks should NOT collect all descendant text — that would be
    // a massive duplication of content already in child elements.
    let html = r#"
    <html><body>
        <nav>
            <a href="/home">Home</a>
            <a href="/about">About</a>
            <a href="/contact">Contact</a>
            <a href="/blog">Blog</a>
        </nav>
        <main>
            <h1>Welcome</h1>
            <p>This is a long paragraph with lots of text that should not be duplicated.</p>
        </main>
    </body></html>
    "#;

    let dom = browsy_core::parse(html, 1920.0, 1080.0);

    // Nav should be emitted but without "Home About Contact Blog" text blob
    let nav = dom.els.iter().find(|e| e.tag == "nav").unwrap();
    assert_eq!(nav.role.as_deref(), Some("navigation"));
    assert!(nav.text.is_none(), "nav should not have recursive text, got: {:?}", nav.text);

    // Main should be emitted without "Welcome This is a long paragraph..." text blob
    let main_el = dom.els.iter().find(|e| e.tag == "main").unwrap();
    assert_eq!(main_el.role.as_deref(), Some("main"));
    assert!(main_el.text.is_none(), "main should not have recursive text, got: {:?}", main_el.text);

    // Children carry their own text
    let links: Vec<_> = dom.els.iter().filter(|e| e.tag == "a").collect();
    assert_eq!(links.len(), 4);
    assert_eq!(links[0].text.as_deref(), Some("Home"));
    assert_eq!(links[1].text.as_deref(), Some("About"));

    let h1 = dom.els.iter().find(|e| e.tag == "h1").unwrap();
    assert_eq!(h1.text.as_deref(), Some("Welcome"));
}

#[test]
fn test_landmark_role_attr_no_text_blobs() {
    // Elements with explicit landmark role attributes should also not collect text blobs
    let html = r#"
    <html><body>
        <div role="navigation">
            <a href="/a">Link A</a>
            <a href="/b">Link B</a>
        </div>
        <div role="main">
            <h1>Title</h1>
            <p>Content here</p>
        </div>
    </body></html>
    "#;

    let dom = browsy_core::parse(html, 1920.0, 1080.0);

    let nav = dom.els.iter().find(|e| e.role.as_deref() == Some("navigation")).unwrap();
    assert!(nav.text.is_none(), "div[role=navigation] should not have text blob, got: {:?}", nav.text);

    let main_el = dom.els.iter().find(|e| e.role.as_deref() == Some("main")).unwrap();
    assert!(main_el.text.is_none(), "div[role=main] should not have text blob, got: {:?}", main_el.text);

    // Children still have their text
    let links: Vec<_> = dom.els.iter().filter(|e| e.tag == "a").collect();
    assert_eq!(links.len(), 2);
    assert_eq!(links[0].text.as_deref(), Some("Link A"));
}

#[test]
fn test_textless_link_fallbacks() {
    let html = r#"
    <html><body>
        <!-- Link with only an image -->
        <a href="/home"><img src="logo.png" alt="Home Page" /></a>

        <!-- Link with aria-label -->
        <a href="/search" aria-label="Search"></a>

        <!-- Link with title attribute -->
        <a href="/menu" title="Open Menu"></a>

        <!-- Link with SVG icon containing title -->
        <a href="/settings"><svg><title>Settings</title><path d="M0 0"/></svg></a>

        <!-- Link with nested img in span -->
        <a href="/profile"><span><img src="avatar.png" alt="User Profile" /></span></a>

        <!-- Button with img alt -->
        <button><img src="close.png" alt="Close" /></button>

        <!-- Normal link (should still work) -->
        <a href="/about">About Us</a>
    </body></html>
    "#;

    let dom = browsy_core::parse(html, 1920.0, 1080.0);
    let links: Vec<_> = dom.els.iter().filter(|e| e.tag == "a").collect();

    // img alt fallback
    assert_eq!(links[0].text.as_deref(), Some("Home Page"));

    // aria-label fallback
    assert_eq!(links[1].text.as_deref(), Some("Search"));

    // title attribute fallback
    assert_eq!(links[2].text.as_deref(), Some("Open Menu"));

    // SVG title → aria-label fallback
    assert_eq!(links[3].text.as_deref(), Some("Settings"));

    // Nested img alt fallback
    assert_eq!(links[4].text.as_deref(), Some("User Profile"));

    // Normal text link
    assert_eq!(links[5].text.as_deref(), Some("About Us"));

    // Button with img alt
    let button = dom.els.iter().find(|e| e.tag == "button").unwrap();
    assert_eq!(button.text.as_deref(), Some("Close"));
}

#[test]
fn test_url_resolution() {
    let html = r##"
    <html><body>
        <a href="/about">About</a>
        <a href="page.html">Page</a>
        <a href="../up">Up</a>
        <a href="https://example.com">External</a>
        <a href="mailto:test@example.com">Email</a>
        <a href="#section">Anchor</a>
        <a href="javascript:void(0)">JS</a>
    </body></html>
    "##;

    let mut dom = browsy_core::parse(html, 1920.0, 1080.0);
    output::resolve_urls(&mut dom, "https://mysite.com/blog/post");

    let hrefs: Vec<Option<&str>> = dom.els.iter()
        .filter(|e| e.role.as_deref() == Some("link"))
        .map(|e| e.href.as_deref())
        .collect();

    assert!(hrefs.contains(&Some("https://mysite.com/about")));
    assert!(hrefs.contains(&Some("https://mysite.com/blog/page.html")));
    assert!(hrefs.contains(&Some("https://mysite.com/up")));
    assert!(hrefs.contains(&Some("https://example.com")));
    assert!(hrefs.contains(&Some("mailto:test@example.com")));
    assert!(hrefs.contains(&Some("#section")));
    assert!(hrefs.contains(&Some("javascript:void(0)")));
}

#[test]
fn test_label_input_association() {
    let html = r#"
    <html><body>
        <form>
            <label for="email">Email Address</label>
            <input type="email" id="email" placeholder="you@example.com">

            <label for="pass">Password</label>
            <input type="password" id="pass">

            <input type="text" id="nolabel" placeholder="No label">
        </form>
    </body></html>
    "#;

    let dom = browsy_core::parse(html, 1920.0, 1080.0);

    let email_input = dom.els.iter().find(|e| e.ph.as_deref() == Some("you@example.com")).unwrap();
    assert_eq!(email_input.label.as_deref(), Some("Email Address"));

    let pass_input = dom.els.iter().find(|e| e.input_type.as_deref() == Some("password")).unwrap();
    assert_eq!(pass_input.label.as_deref(), Some("Password"));

    let nolabel_input = dom.els.iter().find(|e| e.ph.as_deref() == Some("No label")).unwrap();
    assert_eq!(nolabel_input.label, None);
}

#[test]
fn test_above_fold_filtering() {
    let html = r#"
    <html>
    <body style="margin: 0;">
        <div style="height: 500px;"><button>Top Button</button></div>
        <div style="height: 500px;"><button>Middle Button</button></div>
        <div style="height: 500px; margin-top: 1000px;"><button>Far Below</button></div>
    </body>
    </html>
    "#;

    // Small viewport: 800px tall
    let dom = browsy_core::parse(html, 1920.0, 800.0);

    let above = dom.above_fold();
    let below = dom.below_fold();

    // Top button (y < 800) should be above fold
    assert!(above.iter().any(|e| e.text.as_deref() == Some("Top Button")));

    // Far Below button should be below fold
    assert!(below.iter().any(|e| e.text.as_deref() == Some("Far Below")));

    // filter_above_fold returns a new SpatialDom
    let filtered = dom.filter_above_fold();
    assert!(filtered.els.iter().any(|e| e.text.as_deref() == Some("Top Button")));
    assert!(filtered.els.len() < dom.els.len() || dom.els.iter().all(|e| e.b[1] < 800));
}

#[test]
fn test_above_fold_compact_output() {
    let html = r#"
    <html>
    <body style="margin: 0;">
        <button>Visible</button>
        <div style="margin-top: 2000px;"><button>Hidden Below</button></div>
    </body>
    </html>
    "#;

    let dom = browsy_core::parse(html, 1920.0, 1080.0);
    let filtered = dom.filter_above_fold();
    let compact = output::to_compact_string(&filtered);

    assert!(compact.contains("Visible"));
    assert!(!compact.contains("Hidden Below"));
}

#[test]
fn test_hidden_dropdown_menu() {
    // Real-world pattern: nav dropdown where items are in DOM but hidden
    let html = r#"
    <html><head><style>
        .dropdown-menu { display: none; }
    </style></head>
    <body>
        <nav>
            <a href="/">Home</a>
            <button>Products</button>
            <div class="dropdown-menu">
                <a href="/widgets">Widgets</a>
                <a href="/gadgets">Gadgets</a>
                <a href="/tools">Tools</a>
            </div>
        </nav>
    </body></html>
    "#;

    let dom = browsy_core::parse(html, 1920.0, 1080.0);

    // Agent can see ALL dropdown items even though they're hidden
    let widgets = dom.els.iter().find(|e| e.text.as_deref() == Some("Widgets")).unwrap();
    assert_eq!(widgets.hidden, Some(true));
    assert_eq!(widgets.href.as_deref(), Some("/widgets"));

    let gadgets = dom.els.iter().find(|e| e.text.as_deref() == Some("Gadgets")).unwrap();
    assert_eq!(gadgets.hidden, Some(true));

    // visible() filters them out
    let visible = dom.visible();
    assert!(visible.iter().any(|e| e.text.as_deref() == Some("Home")));
    assert!(!visible.iter().any(|e| e.text.as_deref() == Some("Widgets")));
}

#[test]
fn test_hidden_accordion_panels() {
    // Real-world pattern: FAQ accordion with hidden answer panels
    let html = r#"
    <html><head><style>
        .panel { display: none; }
        .panel.active { display: block; }
    </style></head>
    <body>
        <div class="accordion">
            <button>What is browsy?</button>
            <div class="panel active">
                <p>A zero-render browser engine for AI agents.</p>
            </div>
            <button>How fast is it?</button>
            <div class="panel">
                <p>Under 100ms per page.</p>
            </div>
            <button>Does it run JS?</button>
            <div class="panel">
                <p>No, but it exposes hidden content so JS is unnecessary.</p>
            </div>
        </div>
    </body></html>
    "#;

    let dom = browsy_core::parse(html, 1920.0, 1080.0);

    // First panel (active) is visible
    let answer1 = dom.els.iter().find(|e| e.text.as_deref() == Some("A zero-render browser engine for AI agents.")).unwrap();
    assert!(answer1.hidden.is_none() || answer1.hidden == Some(false));

    // Second panel (no .active) is hidden but present
    let answer2 = dom.els.iter().find(|e| e.text.as_deref() == Some("Under 100ms per page.")).unwrap();
    assert_eq!(answer2.hidden, Some(true));

    // Third panel also hidden but present
    let answer3 = dom.els.iter().find(|e| e.text.as_deref().map(|t| t.contains("exposes hidden")).unwrap_or(false)).unwrap();
    assert_eq!(answer3.hidden, Some(true));
}

#[test]
fn test_hidden_modal_dialog() {
    // Real-world pattern: modal dialog hidden until triggered
    let html = r#"
    <html><head><style>
        .modal { display: none; }
    </style></head>
    <body>
        <button>Delete Account</button>
        <div class="modal" role="dialog">
            <h2>Are you sure?</h2>
            <p>This action cannot be undone.</p>
            <button>Confirm Delete</button>
            <button>Cancel</button>
        </div>
    </body></html>
    "#;

    let dom = browsy_core::parse(html, 1920.0, 1080.0);

    // Agent can see the modal content and know what confirming does
    let confirm = dom.els.iter().find(|e| e.text.as_deref() == Some("Confirm Delete")).unwrap();
    assert_eq!(confirm.hidden, Some(true));

    let warning = dom.els.iter().find(|e| e.text.as_deref() == Some("This action cannot be undone.")).unwrap();
    assert_eq!(warning.hidden, Some(true));

    // The dialog role is exposed
    let dialog = dom.els.iter().find(|e| e.role.as_deref() == Some("dialog")).unwrap();
    assert_eq!(dialog.hidden, Some(true));
}

#[test]
fn test_hidden_tab_panels() {
    // Real-world pattern: tab interface with sibling panels
    let html = r#"
    <html><head><style>
        .tab-panel { display: none; }
        .tab-panel.active { display: block; }
    </style></head>
    <body>
        <div role="tablist">
            <button role="tab" aria-selected="true" aria-controls="overview">Overview</button>
            <button role="tab" aria-selected="false" aria-controls="specs">Specs</button>
            <button role="tab" aria-selected="false" aria-controls="reviews">Reviews</button>
        </div>
        <div id="overview" class="tab-panel active" role="tabpanel">
            <p>Product overview goes here.</p>
        </div>
        <div id="specs" class="tab-panel" role="tabpanel">
            <p>Weight: 2.5 lbs</p>
            <p>Dimensions: 12 x 8 x 1 inches</p>
        </div>
        <div id="reviews" class="tab-panel" role="tabpanel">
            <p>Great product! 5 stars.</p>
        </div>
    </body></html>
    "#;

    let dom = browsy_core::parse(html, 1920.0, 1080.0);

    // Active tab panel is visible
    let overview = dom.els.iter().find(|e| e.text.as_deref() == Some("Product overview goes here.")).unwrap();
    assert!(overview.hidden.is_none());

    // Hidden tab panels are present with hidden:true
    let specs = dom.els.iter().find(|e| e.text.as_deref() == Some("Weight: 2.5 lbs")).unwrap();
    assert_eq!(specs.hidden, Some(true));

    let reviews = dom.els.iter().find(|e| e.text.as_deref() == Some("Great product! 5 stars.")).unwrap();
    assert_eq!(reviews.hidden, Some(true));

    // Agent sees all tab content without clicking — no JS needed
    assert!(dom.els.iter().any(|e| e.text.as_deref() == Some("Dimensions: 12 x 8 x 1 inches")));
}

#[test]
fn test_hidden_compact_output_markers() {
    let html = r#"
    <html><head><style>
        .hidden { display: none; }
    </style></head>
    <body>
        <button>Visible</button>
        <button class="hidden">Secret</button>
    </body></html>
    "#;

    let dom = browsy_core::parse(html, 1920.0, 1080.0);
    let compact = output::to_compact_string(&dom);

    // Hidden elements are prefixed with ! in compact format
    assert!(compact.contains("\"Visible\""));
    assert!(compact.contains("!"));
    assert!(compact.contains("\"Secret\""));
}

#[test]
fn test_visible_method_filters_correctly() {
    let html = r#"
    <html><head><style>
        .off { display: none; }
    </style></head>
    <body>
        <a href="/a">Link A</a>
        <a href="/b" class="off">Link B</a>
        <button>Click Me</button>
        <button class="off">Hidden Button</button>
    </body></html>
    "#;

    let dom = browsy_core::parse(html, 1920.0, 1080.0);

    let all = &dom.els;
    let visible = dom.visible();

    // All 4 elements exist
    assert_eq!(all.iter().filter(|e| e.tag == "a" || e.tag == "button").count(), 4);
    // Only 2 are visible
    assert_eq!(visible.iter().filter(|e| e.tag == "a" || e.tag == "button").count(), 2);
    assert!(visible.iter().any(|e| e.text.as_deref() == Some("Link A")));
    assert!(visible.iter().any(|e| e.text.as_deref() == Some("Click Me")));
    assert!(!visible.iter().any(|e| e.text.as_deref() == Some("Link B")));
}

#[test]
fn test_name_field_on_form_inputs() {
    let html = r#"
    <html><body>
        <form>
            <input type="text" name="username" placeholder="Username" />
            <input type="email" name="email" placeholder="Email" />
            <textarea name="bio" placeholder="Bio"></textarea>
            <select name="color">
                <option value="red">Red</option>
            </select>
            <button>Submit</button>
        </form>
    </body></html>
    "#;

    let dom = browsy_core::parse(html, 1920.0, 1080.0);

    let username = dom.els.iter().find(|e| e.ph.as_deref() == Some("Username")).unwrap();
    assert_eq!(username.name.as_deref(), Some("username"));

    let email = dom.els.iter().find(|e| e.ph.as_deref() == Some("Email")).unwrap();
    assert_eq!(email.name.as_deref(), Some("email"));

    let bio = dom.els.iter().find(|e| e.tag == "textarea").unwrap();
    assert_eq!(bio.name.as_deref(), Some("bio"));

    let select = dom.els.iter().find(|e| e.tag == "select").unwrap();
    assert_eq!(select.name.as_deref(), Some("color"));

    // Buttons should not have name field populated via this logic
    let button = dom.els.iter().find(|e| e.tag == "button").unwrap();
    assert_eq!(button.name, None);
}

#[test]
fn test_alert_detection_role() {
    let html = r#"
    <html><body>
        <div role="alert">Something went wrong!</div>
        <div role="status">Saved successfully.</div>
        <p>Normal paragraph.</p>
    </body></html>
    "#;

    let dom = browsy_core::parse(html, 1920.0, 1080.0);

    let alert = dom.els.iter().find(|e| e.text.as_deref() == Some("Something went wrong!")).unwrap();
    assert_eq!(alert.alert_type.as_deref(), Some("alert"));

    let status = dom.els.iter().find(|e| e.text.as_deref() == Some("Saved successfully.")).unwrap();
    assert_eq!(status.alert_type.as_deref(), Some("status"));

    let normal = dom.els.iter().find(|e| e.text.as_deref() == Some("Normal paragraph.")).unwrap();
    assert_eq!(normal.alert_type, None);

    // alerts() method
    let alerts = dom.alerts();
    assert_eq!(alerts.len(), 2);
}

#[test]
fn test_alert_detection_css_classes() {
    // Use <p> tags (emitted as text elements) instead of bare <div>s
    let html = r#"
    <html><body>
        <p class="alert alert-danger">Error occurred</p>
        <p class="success-message">Profile saved</p>
        <p class="warning-box">Low disk space</p>
        <p class="text-normal">Just text</p>
    </body></html>
    "#;

    let dom = browsy_core::parse(html, 1920.0, 1080.0);

    let error_el = dom.els.iter().find(|e| e.text.as_deref() == Some("Error occurred")).unwrap();
    assert!(error_el.alert_type.is_some());

    let success = dom.els.iter().find(|e| e.text.as_deref() == Some("Profile saved")).unwrap();
    assert_eq!(success.alert_type.as_deref(), Some("success"));

    let warning = dom.els.iter().find(|e| e.text.as_deref() == Some("Low disk space")).unwrap();
    assert_eq!(warning.alert_type.as_deref(), Some("warning"));
}

#[test]
fn test_table_extraction() {
    let html = r#"
    <html><body>
        <table>
            <tr><th>Name</th><th>Age</th><th>City</th></tr>
            <tr><td>Alice</td><td>30</td><td>NYC</td></tr>
            <tr><td>Bob</td><td>25</td><td>LA</td></tr>
        </table>
    </body></html>
    "#;

    let dom = browsy_core::parse(html, 1920.0, 1080.0);
    let tables = dom.tables();

    assert_eq!(tables.len(), 1);
    assert_eq!(tables[0].headers, vec!["Name", "Age", "City"]);
    assert_eq!(tables[0].rows.len(), 2);
    assert_eq!(tables[0].rows[0], vec!["Alice", "30", "NYC"]);
    assert_eq!(tables[0].rows[1], vec!["Bob", "25", "LA"]);
}

#[test]
fn test_page_type_login() {
    let html = r#"
    <html><body>
        <h1>Login</h1>
        <input type="text" placeholder="Username" />
        <input type="password" placeholder="Password" />
        <button>Sign In</button>
    </body></html>
    "#;

    let dom = browsy_core::parse(html, 1920.0, 1080.0);
    assert_eq!(dom.page_type, output::PageType::Login);
}

#[test]
fn test_page_type_search() {
    let html = r#"
    <html><body>
        <input type="search" placeholder="Search..." />
        <button>Go</button>
    </body></html>
    "#;

    let dom = browsy_core::parse(html, 1920.0, 1080.0);
    assert_eq!(dom.page_type, output::PageType::Search);
}

#[test]
fn test_page_type_form() {
    let html = r#"
    <html><body>
        <h1>Register</h1>
        <input type="text" name="name" placeholder="Name" />
        <input type="email" name="email" placeholder="Email" />
        <textarea name="message"></textarea>
        <button>Submit</button>
    </body></html>
    "#;

    let dom = browsy_core::parse(html, 1920.0, 1080.0);
    assert_eq!(dom.page_type, output::PageType::Form);
}

#[test]
fn test_page_type_error() {
    let html = r#"
    <html><head><title>404 Not Found</title></head>
    <body><h1>Page Not Found</h1></body></html>
    "#;

    let dom = browsy_core::parse(html, 1920.0, 1080.0);
    assert_eq!(dom.page_type, output::PageType::Error);
}

#[test]
fn test_pagination_detection() {
    let html = r#"
    <html><body>
        <div class="pagination">
            <a href="/page/1">1</a>
            <a href="/page/2">2</a>
            <a href="/page/3">3</a>
            <a href="/page/2">Next</a>
            <a href="/page/0">Previous</a>
        </div>
    </body></html>
    "#;

    let dom = browsy_core::parse(html, 1920.0, 1080.0);
    let pagination = dom.pagination();

    assert!(pagination.is_some());
    let p = pagination.unwrap();
    assert_eq!(p.next.as_deref(), Some("/page/2"));
    assert_eq!(p.prev.as_deref(), Some("/page/0"));
    assert!(p.pages.len() >= 3);
}

#[test]
fn test_compact_output_form_markers() {
    let html = r#"
    <html><body>
        <input type="text" name="email" value="test@test.com" required />
        <input type="checkbox" name="agree" checked />
    </body></html>
    "#;

    let dom = browsy_core::parse(html, 1920.0, 1080.0);
    let compact = output::to_compact_string(&dom);

    // Should contain name marker
    assert!(compact.contains("[email]"), "compact should contain [email], got: {}", compact);
    // Should contain value marker
    assert!(compact.contains("[=test@test.com]"), "compact should contain [=test@test.com], got: {}", compact);
    // Should contain required marker
    assert!(compact.contains("[*]"), "compact should contain [*], got: {}", compact);
    // Should contain checked marker
    assert!(compact.contains("[v]"), "compact should contain [v], got: {}", compact);
}

// --- Semantic position tests ---

#[test]
fn test_compact_output_semantic_position() {
    // Test 1: Duplicate "Read more" links get region labels
    let html = r#"
    <html>
    <body style="margin: 0;">
        <div style="width: 1920px;">
            <div style="height: 200px;">
                <a href="/post/1">Read more</a>
            </div>
            <div style="height: 200px; margin-top: 200px;">
                <a href="/post/2">Read more</a>
            </div>
        </div>
    </body>
    </html>
    "#;

    let dom = browsy_core::parse(html, 1920.0, 1080.0);
    let compact = output::to_compact_string(&dom);

    // Both "Read more" links share (tag=a, text="Read more") so they need disambiguation
    let read_more_lines: Vec<&str> = compact.lines()
        .filter(|l| l.contains("\"Read more\""))
        .collect();
    assert_eq!(read_more_lines.len(), 2, "Should have 2 'Read more' lines, got: {}", compact);
    assert!(read_more_lines.iter().all(|l| l.contains("@")),
        "All duplicate elements should have region labels, got:\n{}", compact);

    // Test 2: Unique elements have NO position data
    let html2 = r#"
    <html><body>
        <a href="/about">About Us</a>
        <a href="/contact">Contact</a>
        <button>Sign In</button>
    </body></html>
    "#;

    let dom2 = browsy_core::parse(html2, 1920.0, 1080.0);
    let compact2 = output::to_compact_string(&dom2);

    // No element shares (tag, text, href) so no @ labels
    assert!(!compact2.contains("@"),
        "Unique elements should have no position data, got:\n{}", compact2);

    // Test 3: Wide input gets size hint
    let html3 = r#"
    <html>
    <body style="margin: 0;">
        <input type="text" placeholder="Search" style="width: 1200px;" />
        <input type="text" placeholder="ZIP" style="width: 100px;" />
        <input type="text" placeholder="Name" style="width: 500px;" />
    </body>
    </html>
    "#;

    let dom3 = browsy_core::parse(html3, 1920.0, 1080.0);
    let compact3 = output::to_compact_string(&dom3);

    // 1200/1920 = 62.5% → wide
    assert!(compact3.contains("wide"),
        "1200px input on 1920px viewport should be 'wide', got:\n{}", compact3);
    // 100/1920 = 5.2% → narrow
    assert!(compact3.contains("narrow"),
        "100px input on 1920px viewport should be 'narrow', got:\n{}", compact3);
    // 500/1920 = 26% → no size hint (normal range)
    let name_line = compact3.lines().find(|l| l.contains("\"Name\"")).unwrap();
    assert!(!name_line.contains("wide") && !name_line.contains("narrow") && !name_line.contains("full"),
        "500px input should have no size hint, got: {}", name_line);
}

// --- Suggested action tests ---

#[test]
fn test_suggested_action_login() {
    let html = r#"
    <html><body>
        <h1>Login</h1>
        <form>
            <input type="email" name="email" placeholder="Email" />
            <input type="password" name="password" placeholder="Password" />
            <button>Sign In</button>
        </form>
    </body></html>
    "#;

    let dom = browsy_core::parse(html, 1920.0, 1080.0);
    assert_eq!(dom.page_type, output::PageType::Login);

    let login = dom.suggested_actions.iter().find(|a| matches!(a, output::SuggestedAction::Login { .. }));
    assert!(login.is_some(), "Should detect login action, got: {:?}", dom.suggested_actions);

    if let Some(output::SuggestedAction::Login { username_id, password_id, submit_id, .. }) = login {
        // Verify the IDs point to real elements
        assert!(dom.get(*username_id).is_some(), "username_id should exist");
        assert!(dom.get(*password_id).is_some(), "password_id should exist");
        assert!(dom.get(*submit_id).is_some(), "submit_id should exist");

        let username = dom.get(*username_id).unwrap();
        assert!(username.input_type.as_deref() == Some("email") || username.input_type.as_deref() == Some("text"));

        let password = dom.get(*password_id).unwrap();
        assert_eq!(password.input_type.as_deref(), Some("password"));

        let submit = dom.get(*submit_id).unwrap();
        assert_eq!(submit.tag, "button");
    }
}

#[test]
fn test_suggested_action_enter_code() {
    let html = r#"
    <html><head><title>Verify Your Identity</title></head>
    <body>
        <h1>Enter verification code</h1>
        <form>
            <input type="text" name="code" placeholder="Enter code" />
            <button>Verify</button>
        </form>
    </body></html>
    "#;

    let dom = browsy_core::parse(html, 1920.0, 1080.0);
    assert_eq!(dom.page_type, output::PageType::TwoFactorAuth);

    let code_action = dom.suggested_actions.iter().find(|a| matches!(a, output::SuggestedAction::EnterCode { .. }));
    assert!(code_action.is_some(), "Should detect enter code action, got: {:?}", dom.suggested_actions);

    if let Some(output::SuggestedAction::EnterCode { input_id, submit_id, .. }) = code_action {
        let input = dom.get(*input_id).unwrap();
        assert_eq!(input.tag, "input");
        let submit = dom.get(*submit_id).unwrap();
        assert_eq!(submit.tag, "button");
    }
}

#[test]
fn test_suggested_action_search() {
    let html = r#"
    <html><body>
        <form>
            <input type="search" name="q" placeholder="Search..." />
            <button>Search</button>
        </form>
    </body></html>
    "#;

    let dom = browsy_core::parse(html, 1920.0, 1080.0);

    let search = dom.suggested_actions.iter().find(|a| matches!(a, output::SuggestedAction::Search { .. }));
    assert!(search.is_some(), "Should detect search action, got: {:?}", dom.suggested_actions);
}

#[test]
fn test_page_type_2fa() {
    let html = r#"
    <html><head><title>Two-Factor Authentication</title></head>
    <body>
        <h1>Verify your identity</h1>
        <input type="text" name="code" placeholder="Enter code" />
        <button>Submit</button>
    </body></html>
    "#;

    let dom = browsy_core::parse(html, 1920.0, 1080.0);
    assert_eq!(dom.page_type, output::PageType::TwoFactorAuth);
}

#[test]
fn test_page_type_oauth() {
    let html = r#"
    <html><head><title>Authorize Application</title></head>
    <body>
        <h1>Authorize MyApp</h1>
        <p>MyApp wants to access your account.</p>
        <button>Allow</button>
        <button>Deny</button>
    </body></html>
    "#;

    let dom = browsy_core::parse(html, 1920.0, 1080.0);
    assert_eq!(dom.page_type, output::PageType::OAuthConsent);
}

#[test]
fn test_page_type_inbox() {
    let html = r#"
    <html><head><title>Inbox - Mail</title></head>
    <body>
        <nav><a href="/">Home</a></nav>
        <main>
            <a href="/m/1">Meeting tomorrow</a>
            <a href="/m/2">Invoice #123</a>
            <a href="/m/3">Welcome aboard</a>
            <a href="/m/4">Security alert</a>
            <a href="/m/5">Newsletter</a>
            <a href="/m/6">Shipping update</a>
            <a href="/m/7">Payment received</a>
            <a href="/m/8">New comment</a>
            <a href="/m/9">Reminder</a>
            <a href="/m/10">Account update</a>
        </main>
    </body></html>
    "#;

    let dom = browsy_core::parse(html, 1920.0, 1080.0);
    assert_eq!(dom.page_type, output::PageType::Inbox);
}

#[test]
fn test_page_type_email_body() {
    let html = r#"
    <html><body>
        <p>From: noreply@example.com</p>
        <p>To: user@example.com</p>
        <p>Subject: Your verification code</p>
        <p>Date: 2026-01-15</p>
        <p>Your verification code is 482901. This code expires in 10 minutes.</p>
    </body></html>
    "#;

    let dom = browsy_core::parse(html, 1920.0, 1080.0);
    assert_eq!(dom.page_type, output::PageType::EmailBody);
}

#[test]
fn test_page_type_dashboard() {
    let html = r#"
    <html><head><title>Dashboard</title></head>
    <body>
        <nav><a href="/home">Home</a><a href="/settings">Settings</a></nav>
        <main>
            <h1>Welcome back</h1>
            <p>Here's your overview.</p>
        </main>
    </body></html>
    "#;

    let dom = browsy_core::parse(html, 1920.0, 1080.0);
    assert_eq!(dom.page_type, output::PageType::Dashboard);
}

#[test]
fn test_page_type_captcha() {
    let html = r#"
    <html><head><title>Verify you're human</title></head>
    <body>
        <h1>Complete the CAPTCHA</h1>
        <p>Please solve the challenge below.</p>
    </body></html>
    "#;

    let dom = browsy_core::parse(html, 1920.0, 1080.0);
    assert_eq!(dom.page_type, output::PageType::Captcha);
}

#[test]
fn test_find_codes_in_email() {
    let html = r#"
    <html><body>
        <p>From: noreply@example.com</p>
        <p>Your verification code is 482901. This code expires in 10 minutes.</p>
    </body></html>
    "#;

    let dom = browsy_core::parse(html, 1920.0, 1080.0);
    let codes = dom.find_codes();
    assert!(codes.contains(&"482901".to_string()), "Should find code 482901, got: {:?}", codes);
}

#[test]
fn test_find_codes_no_false_positives() {
    let html = r#"
    <html><body>
        <p>Page 12345 of results</p>
        <p>Order #67890 placed</p>
        <p>Copyright 2026</p>
    </body></html>
    "#;

    let dom = browsy_core::parse(html, 1920.0, 1080.0);
    let codes = dom.find_codes();
    assert!(codes.is_empty(), "Should not find codes without keyword context, got: {:?}", codes);
}

#[test]
fn test_consent_action_detection() {
    let html = r#"
    <html><head><title>Authorize Application</title></head>
    <body>
        <h1>Authorize MyApp</h1>
        <p>MyApp wants to access your data.</p>
        <button>Allow</button>
        <button>Deny</button>
    </body></html>
    "#;

    let dom = browsy_core::parse(html, 1920.0, 1080.0);
    let consent = dom.suggested_actions.iter().find(|a| matches!(a, output::SuggestedAction::Consent { .. }));
    assert!(consent.is_some(), "Should detect consent action");

    if let Some(output::SuggestedAction::Consent { approve_ids, deny_ids }) = consent {
        assert!(!approve_ids.is_empty(), "Should have approve buttons");
        assert!(!deny_ids.is_empty(), "Should have deny buttons");
    }
}
