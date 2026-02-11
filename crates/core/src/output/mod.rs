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
#[derive(Debug, Serialize, Clone, PartialEq)]
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
    /// ARIA: whether the element is disabled
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disabled: Option<bool>,
    /// ARIA: whether the element is checked (checkbox/radio)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checked: Option<bool>,
    /// ARIA: whether expanded (dropdown, accordion)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expanded: Option<bool>,
    /// ARIA: whether selected (tab, option)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected: Option<bool>,
    /// ARIA: whether required
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
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

/// Tags that commonly wrap interactive elements (dedup candidates).
const WRAPPER_TAGS: &[&str] = &["li", "td", "th", "span", "p", "dt", "dd"];

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
    // aria-hidden="true" hides the element and all children
    let aria_hidden = node
        .attributes
        .get("aria-hidden")
        .map(|v| v == "true")
        .unwrap_or(false);

    // Determine if this node is hidden (cascades to children)
    let is_hidden = parent_hidden
        || node.style.display == Display::None
        || node.style.visibility == Visibility::Hidden
        || aria_hidden;

    if is_hidden {
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
    let has_role = node.attributes.contains_key("role");

    // Image with alt text
    let is_img_with_alt = tag == "img" && node.attributes.contains_key("alt");

    let should_emit = is_interactive || is_text || has_role || is_img_with_alt;

    if should_emit {
        // Skip text-only elements with trivial content (just punctuation/separators)
        if is_text && !is_interactive && !has_role {
            let text_content = if !node.text_content.is_empty() {
                &node.text_content
            } else {
                ""
            };
            if is_trivial_text(text_content) {
                for child in &node.children {
                    collect_elements(child, els, id_counter, is_hidden);
                }
                return;
            }
        }
        // Deduplication: for wrapper tags that only wrap interactive children,
        // skip the wrapper and let the children carry the text.
        let has_interactive = has_interactive_descendants(node);
        let is_wrapper = WRAPPER_TAGS.contains(&tag);

        if is_wrapper && !is_interactive && has_interactive {
            let own_text = collect_own_text(node);
            if own_text.is_empty() || is_trivial_text(&own_text) {
                for child in &node.children {
                    collect_elements(child, els, id_counter, is_hidden);
                }
                return;
            }
            emit_element(node, els, id_counter, Some(own_text));
        } else if is_text && !is_interactive && has_interactive {
            let own_text = collect_own_text(node);
            if own_text.is_empty() || is_trivial_text(&own_text) {
                for child in &node.children {
                    collect_elements(child, els, id_counter, is_hidden);
                }
                return;
            }
            emit_element(node, els, id_counter, Some(own_text));
        } else {
            emit_element(node, els, id_counter, None);
        }
    }

    // Recurse into children
    for child in &node.children {
        collect_elements(child, els, id_counter, is_hidden);
    }
}

fn emit_element(
    node: &LayoutNode,
    els: &mut Vec<SpatialElement>,
    id_counter: &mut u32,
    text_override: Option<String>,
) {
    let tag = node.tag.as_str();

    let text = if let Some(t) = text_override {
        if t.is_empty() { None } else { Some(t) }
    } else if tag == "img" {
        // Use alt text for images
        node.attributes.get("alt").cloned().filter(|s| !s.is_empty())
    } else {
        let text_content = if !node.text_content.is_empty() {
            node.text_content.clone()
        } else {
            collect_visible_text(node)
        };
        if text_content.is_empty() {
            node.attributes.get("aria-label").cloned()
        } else {
            Some(text_content)
        }
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

    let disabled = parse_bool_attr(node, "disabled")
        .or_else(|| parse_bool_attr(node, "aria-disabled"));
    let checked = parse_bool_attr(node, "checked")
        .or_else(|| parse_aria_bool(node, "aria-checked"));
    let expanded = parse_aria_bool(node, "aria-expanded");
    let selected = parse_bool_attr(node, "selected")
        .or_else(|| parse_aria_bool(node, "aria-selected"));
    let required = parse_bool_attr(node, "required")
        .or_else(|| parse_aria_bool(node, "aria-required"));

    let el = SpatialElement {
        id: *id_counter,
        tag: tag.to_string(),
        role,
        text,
        ph,
        href,
        val,
        input_type,
        disabled,
        checked,
        expanded,
        selected,
        required,
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

/// Check if a node has any interactive descendants.
fn has_interactive_descendants(node: &LayoutNode) -> bool {
    for child in &node.children {
        let tag = child.tag.as_str();
        if INTERACTIVE_TAGS.contains(&tag)
            || child.attributes.contains_key("onclick")
            || child.attributes.contains_key("role")
            || child.attributes.get("tabindex").is_some()
        {
            return true;
        }
        if has_interactive_descendants(child) {
            return true;
        }
    }
    false
}

/// Collect text directly owned by this node, NOT from interactive or text-tag descendants.
/// This gives us text that wouldn't be captured by child elements emitted separately.
fn collect_own_text(node: &LayoutNode) -> String {
    let mut result = String::new();
    // Iterate children (not node itself) so the tag-based skip applies to children only
    for child in &node.children {
        collect_own_text_recursive(child, &mut result);
    }
    result.trim().to_string()
}

fn collect_own_text_recursive(node: &LayoutNode, out: &mut String) {
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
    // Skip children that will be emitted as their own elements
    let tag = node.tag.as_str();
    if INTERACTIVE_TAGS.contains(&tag)
        || TEXT_TAGS.contains(&tag)
        || node.attributes.contains_key("onclick")
        || node.attributes.contains_key("role")
        || node.attributes.get("tabindex").is_some()
    {
        return;
    }
    for child in &node.children {
        collect_own_text_recursive(child, out);
    }
}

/// Check if text is trivial (only punctuation, separators, or whitespace).
/// These elements add noise without conveying meaningful content.
fn is_trivial_text(text: &str) -> bool {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return true;
    }
    // Skip if all chars are just separators/punctuation
    trimmed.chars().all(|c| matches!(c, '|' | '·' | '•' | '-' | '–' | '—' | '/' | '\\' | ',' | '.' | ':' | ';' | '(' | ')' | '[' | ']' | '{' | '}' | ' ' | '\t' | '\n'))
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
        "main" => Some("main".to_string()),
        "aside" => Some("complementary".to_string()),
        "header" => Some("banner".to_string()),
        "footer" => Some("contentinfo".to_string()),
        "form" => Some("form".to_string()),
        "section" => Some("region".to_string()),
        "img" => Some("img".to_string()),
        _ => None,
    }
}

/// Parse a boolean HTML attribute (present = true).
fn parse_bool_attr(node: &LayoutNode, attr: &str) -> Option<bool> {
    if node.attributes.contains_key(attr) {
        Some(true)
    } else {
        None
    }
}

/// Parse an ARIA boolean attribute (value = "true" or "false").
fn parse_aria_bool(node: &LayoutNode, attr: &str) -> Option<bool> {
    node.attributes.get(attr).map(|v| v == "true")
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

/// Delta output — only the changes between two SpatialDoms.
#[derive(Debug, Serialize)]
pub struct DeltaDom {
    /// Elements that were added or changed.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub changed: Vec<SpatialElement>,
    /// IDs of elements that were removed.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub removed: Vec<u32>,
}

/// Compute the diff between two SpatialDoms.
/// Returns only added/changed/removed elements.
pub fn diff(old: &SpatialDom, new: &SpatialDom) -> DeltaDom {
    let mut changed = Vec::new();
    let mut removed = Vec::new();

    // Build lookup of old elements by a content key (tag + text + href + bounds)
    // We match by content similarity, not by ID (since IDs are assigned sequentially)
    let old_set: std::collections::HashSet<ElementKey> = old.els.iter().map(ElementKey::from).collect();
    let new_set: std::collections::HashSet<ElementKey> = new.els.iter().map(ElementKey::from).collect();

    // Elements in new but not in old → added/changed
    for el in &new.els {
        let key = ElementKey::from(el);
        if !old_set.contains(&key) {
            changed.push(el.clone());
        }
    }

    // Elements in old but not in new → removed
    for el in &old.els {
        let key = ElementKey::from(el);
        if !new_set.contains(&key) {
            removed.push(el.id);
        }
    }

    DeltaDom { changed, removed }
}

/// Generate compact string format for a delta.
pub fn delta_to_compact_string(delta: &DeltaDom) -> String {
    let mut lines = Vec::new();

    if !delta.removed.is_empty() {
        lines.push(format!("-[{}]", delta.removed.iter().map(|id| id.to_string()).collect::<Vec<_>>().join(",")));
    }

    for el in &delta.changed {
        let mut parts = Vec::new();
        parts.push(format!("+{}:{}", el.id, el.tag));

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

#[derive(Hash, PartialEq, Eq)]
struct ElementKey {
    tag: String,
    text: Option<String>,
    ph: Option<String>,
    href: Option<String>,
    input_type: Option<String>,
    bounds: [i32; 4],
}

impl From<&SpatialElement> for ElementKey {
    fn from(el: &SpatialElement) -> Self {
        Self {
            tag: el.tag.clone(),
            text: el.text.clone(),
            ph: el.ph.clone(),
            href: el.href.clone(),
            input_type: el.input_type.clone(),
            bounds: el.b,
        }
    }
}
