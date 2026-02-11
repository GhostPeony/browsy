//! Tests for JavaScript behavior detection and action simulation.

use browsy_core::js;

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

    assert!(!behaviors.is_empty());
    match &behaviors[0].action {
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

    // Before toggle: dropdown present but hidden
    let dom = browsy_core::parse(html, 1920.0, 1080.0);
    let opt_a = dom.els.iter().find(|e| e.href.as_deref() == Some("/a")).unwrap();
    assert_eq!(opt_a.hidden, Some(true));

    // Apply toggle
    let dom_tree = browsy_core::dom::parse_html(html);
    let action = js::JsAction::ToggleVisibility {
        target: "#dropdown".to_string(),
    };
    let modified = js::apply_action(&dom_tree, &action);

    let styled = browsy_core::css::compute_styles(&modified);
    let laid_out = browsy_core::layout::compute_layout(&styled, 1920.0, 1080.0);
    let dom2 = browsy_core::output::generate_spatial_dom(&laid_out, 1920.0, 1080.0);

    assert!(dom2.els.iter().any(|e| e.href.as_deref() == Some("/a")));
    assert!(dom2.els.iter().any(|e| e.href.as_deref() == Some("/b")));
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

    assert!(!behaviors.is_empty());
    match &behaviors[0].action {
        js::JsAction::ToggleClass { target, class } => {
            assert_eq!(target, "#panel");
            assert_eq!(class, "hidden");
        }
        other => panic!("Expected ToggleClass, got {:?}", other),
    }

    // Apply the class toggle
    let modified = js::apply_action(&dom_tree, &behaviors[0].action);

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
    assert!(!panel_class.contains("hidden"));
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

    assert!(!behaviors.is_empty());
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

    assert!(!behaviors.is_empty());
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

    let tab_groups = js::detect_tab_groups(&dom_tree);
    assert_eq!(tab_groups.len(), 1);
    assert_eq!(tab_groups[0].tabs.len(), 2);
    assert_eq!(tab_groups[0].tabs[0].label, "Tab 1");
    assert!(tab_groups[0].tabs[0].selected);
    assert!(!tab_groups[0].tabs[1].selected);

    let behaviors = js::detect_behaviors(&dom_tree);
    let tab_behaviors: Vec<_> = behaviors.iter()
        .filter(|b| matches!(&b.action, js::JsAction::TabSwitch { .. }))
        .collect();
    assert_eq!(tab_behaviors.len(), 2);
}
