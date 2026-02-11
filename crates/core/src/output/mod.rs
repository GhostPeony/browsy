use crate::css::{Display, Visibility};
use crate::dom::NodeType;
use crate::layout::LayoutNode;
use serde::Serialize;

/// The Spatial DOM — the primary output of agentbrowser.
#[derive(Debug, Serialize)]
pub struct SpatialDom {
    pub url: String,
    pub title: String,
    pub vp: [f32; 2],
    pub scroll: [f32; 2],
    pub els: Vec<SpatialElement>,
}

/// A single element in the Spatial DOM.
#[derive(Debug, Serialize)]
pub struct SpatialElement {
    pub id: u32,
    pub tag: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ph: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub href: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub val: Option<String>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub input_type: Option<String>,
    /// Bounds: [x, y, width, height]
    pub b: [i32; 4],
}

/// Tags that are always interactive.
const INTERACTIVE_TAGS: &[&str] = &[
    "a", "button", "input", "select", "textarea", "details", "summary",
];

/// Tags that bear meaningful text content for the agent.
const TEXT_TAGS: &[&str] = &[
    "h1", "h2", "h3", "h4", "h5", "h6", "p", "label", "span", "li", "td", "th", "dt", "dd",
    "figcaption", "blockquote", "pre", "code", "em", "strong", "b", "i", "mark", "small",
];

/// Generate the Spatial DOM from a laid-out tree.
pub fn generate_spatial_dom(
    root: &LayoutNode,
    viewport_width: f32,
    viewport_height: f32,
) -> SpatialDom {
    let mut els = Vec::new();
    let mut id_counter = 1u32;

    collect_elements(root, &mut els, &mut id_counter, false);

    // Extract title from the tree
    let title = find_title(root).unwrap_or_default();

    SpatialDom {
        url: String::new(), // Set by caller
        title,
        vp: [viewport_width, viewport_height],
        scroll: [0.0, 0.0],
        els,
    }
}

fn collect_elements(
    node: &LayoutNode,
    els: &mut Vec<SpatialElement>,
    id_counter: &mut u32,
    parent_hidden: bool,
) {
    // Determine if this node is hidden (cascades to children)
    let is_hidden = parent_hidden
        || node.style.display == Display::None
        || node.style.visibility == Visibility::Hidden;

    if is_hidden {
        // Still recurse so we don't miss children that might override,
        // but mark them as hidden too
        for child in &node.children {
            collect_elements(child, els, id_counter, true);
        }
        return;
    }

    // Skip zero-size elements (likely invisible)
    if node.bounds.width <= 0.0 && node.bounds.height <= 0.0 && node.node_type == NodeType::Element
    {
        for child in &node.children {
            collect_elements(child, els, id_counter, is_hidden);
        }
        return;
    }

    let tag = node.tag.as_str();
    let is_interactive = INTERACTIVE_TAGS.contains(&tag)
        || node.attributes.contains_key("onclick")
        || node.attributes.contains_key("role")
        || node.attributes.get("tabindex").is_some();

    let is_text = TEXT_TAGS.contains(&tag);

    // Elements with role attribute
    let has_role = node.attributes.contains_key("role");

    if is_interactive || is_text || has_role {
        let text_content = if !node.text_content.is_empty() {
            node.text_content.clone()
        } else {
            collect_visible_text(node)
        };
        let text = if text_content.is_empty() {
            None
        } else {
            Some(text_content)
        };

        let role = determine_role(node);
        let ph = node.attributes.get("placeholder").cloned();
        let href = node.attributes.get("href").cloned();
        let val = node.attributes.get("value").cloned();
        let input_type = if tag == "input" {
            node.attributes.get("type").cloned()
        } else {
            None
        };

        let el = SpatialElement {
            id: *id_counter,
            tag: tag.to_string(),
            role,
            text,
            ph,
            href,
            val,
            input_type,
            b: [
                node.bounds.x.round() as i32,
                node.bounds.y.round() as i32,
                node.bounds.width.round() as i32,
                node.bounds.height.round() as i32,
            ],
        };

        *id_counter += 1;
        els.push(el);
    }

    // Recurse into children
    for child in &node.children {
        collect_elements(child, els, id_counter, is_hidden);
    }
}

fn collect_visible_text(node: &LayoutNode) -> String {
    let mut result = String::new();
    collect_text_recursive(node, &mut result);
    result.trim().to_string()
}

fn collect_text_recursive(node: &LayoutNode, out: &mut String) {
    if node.node_type == NodeType::Text {
        let t = node.text.trim();
        if !t.is_empty() {
            if !out.is_empty() && !out.ends_with(' ') {
                out.push(' ');
            }
            out.push_str(t);
        }
        return;
    }
    for child in &node.children {
        collect_text_recursive(child, out);
    }
}

fn determine_role(node: &LayoutNode) -> Option<String> {
    // Explicit ARIA role
    if let Some(role) = node.attributes.get("role") {
        return Some(role.clone());
    }

    // Implicit roles from tag
    match node.tag.as_str() {
        "a" => Some("link".to_string()),
        "button" => Some("button".to_string()),
        "input" => {
            let input_type = node
                .attributes
                .get("type")
                .map(|s| s.as_str())
                .unwrap_or("text");
            match input_type {
                "checkbox" => Some("checkbox".to_string()),
                "radio" => Some("radio".to_string()),
                "submit" | "button" => Some("button".to_string()),
                "search" => Some("searchbox".to_string()),
                _ => Some("textbox".to_string()),
            }
        }
        "select" => Some("combobox".to_string()),
        "textarea" => Some("textbox".to_string()),
        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => Some("heading".to_string()),
        "nav" => Some("navigation".to_string()),
        _ => None,
    }
}

fn find_title(node: &LayoutNode) -> Option<String> {
    if node.tag == "title" {
        let text = collect_visible_text(node);
        if !text.is_empty() {
            return Some(text);
        }
    }
    for child in &node.children {
        if let Some(title) = find_title(child) {
            return Some(title);
        }
    }
    None
}

/// Generate the compact string format for extreme token budgets.
pub fn to_compact_string(dom: &SpatialDom) -> String {
    let mut lines = Vec::new();
    for el in &dom.els {
        let mut parts = Vec::new();
        parts.push(format!("{}:{}", el.id, el.tag));

        if let Some(ref t) = el.input_type {
            if t != "text" {
                parts.last_mut().unwrap().push_str(&format!(":{}", t));
            }
        }

        if let Some(ref text) = el.text {
            parts.push(format!("\"{}\"", text));
        } else if let Some(ref ph) = el.ph {
            parts.push(format!("\"{}\"", ph));
        }

        if let Some(ref href) = el.href {
            parts.push(format!("->{}", href));
        }

        parts.push(format!("@{},{} {}x{}", el.b[0], el.b[1], el.b[2], el.b[3]));

        lines.push(format!("[{}]", parts.join(" ")));
    }
    lines.join("\n")
}
