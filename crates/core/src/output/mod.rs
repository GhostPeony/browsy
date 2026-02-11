use crate::css::{Display, Visibility};
use crate::dom::NodeType;
use crate::layout::LayoutNode;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

/// The Spatial DOM — the primary output of agentbrowser.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpatialDom {
    pub url: String,
    pub title: String,
    pub vp: [f32; 2],
    pub scroll: [f32; 2],
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub suggested_actions: Vec<SuggestedAction>,
    #[serde(default, skip_serializing_if = "PageType::is_other")]
    pub page_type: PageType,
    pub els: Vec<SpatialElement>,
    /// O(1) lookup: element ID → index in `els`.
    #[serde(skip)]
    id_index: HashMap<u32, usize>,
}

/// A single element in the Spatial DOM.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
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
    /// HTML name attribute (for form fields: input, textarea, select)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Associated label text (from <label for="id">)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Alert type: "alert", "status", "error", "success", "warning"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alert_type: Option<String>,
    /// Whether the element is hidden (display:none, visibility:hidden, aria-hidden, hidden attr).
    /// Hidden elements are still included so agents can see dropdown menus, accordion panels,
    /// modal content, tabs, etc. without JS execution.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hidden: Option<bool>,
    /// Bounds: [x, y, width, height]
    pub b: [i32; 4],
}

/// A suggested action for the agent based on page content analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum SuggestedAction {
    Login {
        username_id: u32,
        password_id: u32,
        submit_id: u32,
        #[serde(skip_serializing_if = "Option::is_none")]
        remember_me_id: Option<u32>,
    },
    EnterCode {
        input_id: u32,
        submit_id: u32,
        #[serde(skip_serializing_if = "Option::is_none")]
        code_length: Option<usize>,
    },
    Search {
        input_id: u32,
        submit_id: u32,
    },
    Consent {
        approve_ids: Vec<u32>,
        deny_ids: Vec<u32>,
    },
    SelectFromList {
        items: Vec<u32>,
    },
}

impl SpatialDom {
    /// Deserialize from JSON and rebuild the ID index.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        let mut dom: SpatialDom = serde_json::from_str(json)?;
        dom.rebuild_index();
        Ok(dom)
    }

    /// O(1) element lookup by ID.
    pub fn get(&self, id: u32) -> Option<&SpatialElement> {
        self.id_index.get(&id).map(|&idx| &self.els[idx])
    }

    /// Rebuild the ID index (call after mutating `els`).
    pub fn rebuild_index(&mut self) {
        self.id_index = self.els.iter().enumerate().map(|(i, e)| (e.id, i)).collect();
    }

    /// Return only visible (non-hidden) elements.
    pub fn visible(&self) -> Vec<&SpatialElement> {
        self.els.iter().filter(|e| e.hidden != Some(true)).collect()
    }

    /// Return elements whose top edge is within the viewport (above the fold).
    pub fn above_fold(&self) -> Vec<&SpatialElement> {
        let fold_y = self.vp[1] as i32;
        self.els.iter().filter(|e| e.b[1] < fold_y).collect()
    }

    /// Return elements whose top edge is below the viewport fold.
    pub fn below_fold(&self) -> Vec<&SpatialElement> {
        let fold_y = self.vp[1] as i32;
        self.els.iter().filter(|e| e.b[1] >= fold_y).collect()
    }

    /// Return a new SpatialDom with only above-fold elements (for token-limited contexts).
    pub fn filter_above_fold(&self) -> SpatialDom {
        let fold_y = self.vp[1] as i32;
        let els: Vec<SpatialElement> = self.els.iter().filter(|e| e.b[1] < fold_y).cloned().collect();
        let id_index = els.iter().enumerate().map(|(i, e)| (e.id, i)).collect();
        SpatialDom {
            url: self.url.clone(),
            title: self.title.clone(),
            vp: self.vp,
            scroll: self.scroll,
            suggested_actions: self.suggested_actions.clone(),
            page_type: self.page_type.clone(),
            els,
            id_index,
        }
    }
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

/// Landmark tags — emitted as structural markers (role only, no recursive text).
const LANDMARK_TAGS: &[&str] = &["nav", "main", "header", "footer", "aside", "section", "form"];

/// Generate the Spatial DOM from a laid-out tree.
pub fn generate_spatial_dom(
    root: &LayoutNode,
    viewport_width: f32,
    viewport_height: f32,
) -> SpatialDom {
    let mut els = Vec::new();
    let mut id_counter = 1u32;

    // Collect label associations: HTML id -> label text
    let label_map = collect_label_associations(root);

    collect_elements(root, &mut els, &mut id_counter, false, &label_map);

    // Extract title from the tree
    let title = find_title(root).unwrap_or_default();

    let id_index = els.iter().enumerate().map(|(i, e)| (e.id, i)).collect();
    let mut dom = SpatialDom {
        url: String::new(), // Set by caller
        title,
        vp: [viewport_width, viewport_height],
        scroll: [0.0, 0.0],
        suggested_actions: Vec::new(),
        page_type: PageType::Other,
        els,
        id_index,
    };

    // Detect page type and suggested actions
    dom.page_type = detect_page_type(&dom);
    dom.suggested_actions = detect_suggested_actions(&dom);

    dom
}

/// Walk the tree to find <label for="xxx"> elements and map input IDs to label text.
fn collect_label_associations(root: &LayoutNode) -> std::collections::HashMap<String, String> {
    let mut map = std::collections::HashMap::new();
    collect_labels_recursive(root, &mut map);
    map
}

fn collect_labels_recursive(node: &LayoutNode, map: &mut std::collections::HashMap<String, String>) {
    if node.tag == "label" {
        if let Some(for_id) = node.attributes.get("for") {
            let text = if !node.text_content.is_empty() {
                node.text_content.clone()
            } else {
                collect_visible_text(node)
            };
            if !text.is_empty() {
                map.insert(for_id.clone(), text);
            }
        }
    }
    for child in &node.children {
        collect_labels_recursive(child, map);
    }
}

/// Resolve all relative URLs in the SpatialDom against a base URL.
pub fn resolve_urls(dom: &mut SpatialDom, base_url: &str) {
    // Try to parse base URL; if invalid, skip resolution
    let base = match url::Url::parse(base_url) {
        Ok(u) => u,
        Err(_) => return,
    };

    for el in &mut dom.els {
        if let Some(ref href) = el.href {
            // Skip already-absolute URLs, javascript:, mailto:, tel:, data:, #anchors
            if href.starts_with("http://")
                || href.starts_with("https://")
                || href.starts_with("javascript:")
                || href.starts_with("mailto:")
                || href.starts_with("tel:")
                || href.starts_with("data:")
                || href.starts_with('#')
            {
                continue;
            }
            // Resolve relative URL
            if let Ok(resolved) = base.join(href) {
                el.href = Some(resolved.to_string());
            }
        }
    }
}

fn collect_elements(
    node: &LayoutNode,
    els: &mut Vec<SpatialElement>,
    id_counter: &mut u32,
    parent_hidden: bool,
    label_map: &std::collections::HashMap<String, String>,
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
        || aria_hidden
        || node.attributes.contains_key("hidden");

    // Skip zero-size visible elements (layout artifacts, not meaningful content)
    if !is_hidden
        && node.bounds.width <= 0.0
        && node.bounds.height <= 0.0
        && node.node_type == NodeType::Element
    {
        for child in &node.children {
            collect_elements(child, els, id_counter, is_hidden, label_map);
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
    let is_landmark = LANDMARK_TAGS.contains(&tag);

    // Image with alt text
    let is_img_with_alt = tag == "img" && node.attributes.contains_key("alt");

    let should_emit = is_interactive || is_text || has_role || is_img_with_alt || is_landmark;

    if should_emit {
        // Landmarks: emit as structural markers with role only, no recursive text.
        // Children carry the actual content — avoid duplicating it in the parent.
        let is_landmark_role = is_landmark || is_landmark_role_attr(node);
        if is_landmark_role {
            // Emit with empty text (role-only marker)
            emit_element(node, els, id_counter, Some(String::new()), is_hidden, label_map);
            for child in &node.children {
                collect_elements(child, els, id_counter, is_hidden, label_map);
            }
            return;
        }

        // Skip text-only elements with trivial content (just punctuation/separators)
        if is_text && !is_interactive && !has_role {
            let text_content = if !node.text_content.is_empty() {
                &node.text_content
            } else {
                ""
            };
            if is_trivial_text(text_content) {
                for child in &node.children {
                    collect_elements(child, els, id_counter, is_hidden, label_map);
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
                    collect_elements(child, els, id_counter, is_hidden, label_map);
                }
                return;
            }
            emit_element(node, els, id_counter, Some(own_text), is_hidden, label_map);
        } else if is_text && !is_interactive && has_interactive {
            let own_text = collect_own_text(node);
            if own_text.is_empty() || is_trivial_text(&own_text) {
                for child in &node.children {
                    collect_elements(child, els, id_counter, is_hidden, label_map);
                }
                return;
            }
            emit_element(node, els, id_counter, Some(own_text), is_hidden, label_map);
        } else {
            emit_element(node, els, id_counter, None, is_hidden, label_map);
        }
    }

    // Recurse into children
    for child in &node.children {
        collect_elements(child, els, id_counter, is_hidden, label_map);
    }
}

fn emit_element(
    node: &LayoutNode,
    els: &mut Vec<SpatialElement>,
    id_counter: &mut u32,
    text_override: Option<String>,
    is_hidden: bool,
    label_map: &std::collections::HashMap<String, String>,
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
        if !text_content.is_empty() {
            Some(text_content)
        } else {
            // Fallback chain for text-less interactive elements (links, buttons)
            node.attributes.get("aria-label").cloned().filter(|s| !s.is_empty())
                .or_else(|| node.attributes.get("title").cloned().filter(|s| !s.is_empty()))
                .or_else(|| find_child_img_alt(node))
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

    // HTML name attribute for form fields
    let name = if matches!(tag, "input" | "select" | "textarea") {
        node.attributes.get("name").cloned()
    } else {
        None
    };

    // Associate label via <label for="id">
    let label = if matches!(tag, "input" | "select" | "textarea") {
        node.attributes.get("id")
            .and_then(|id| label_map.get(id))
            .cloned()
    } else {
        None
    };

    // Alert type detection from role or CSS classes
    let alert_type = detect_alert_type(node);

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
        name,
        label,
        alert_type,
        hidden: if is_hidden { Some(true) } else { None },
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

/// Find alt text from child <img> or <title> from child <svg>.
/// Used as fallback for text-less links/buttons that contain only images or icons.
fn find_child_img_alt(node: &LayoutNode) -> Option<String> {
    for child in &node.children {
        // <img alt="...">
        if child.tag == "img" {
            if let Some(alt) = child.attributes.get("alt") {
                if !alt.is_empty() {
                    return Some(alt.clone());
                }
            }
        }
        // <svg> → aria-label (extracted from <title> during DOM parsing) or <title> descendant
        if child.tag == "svg" {
            if let Some(label) = child.attributes.get("aria-label") {
                if !label.is_empty() {
                    return Some(label.clone());
                }
            }
            if let Some(title) = find_svg_title(child) {
                return Some(title);
            }
        }
        // Recurse (e.g., <a><span><img alt="..."></span></a>)
        if let Some(alt) = find_child_img_alt(child) {
            return Some(alt);
        }
    }
    None
}

/// Find the text content of a <title> element inside an SVG.
fn find_svg_title(node: &LayoutNode) -> Option<String> {
    for child in &node.children {
        if child.tag == "title" {
            let text = collect_visible_text(child);
            if !text.is_empty() {
                return Some(text);
            }
        }
        if let Some(title) = find_svg_title(child) {
            return Some(title);
        }
    }
    None
}

/// ARIA landmark roles — elements with these roles should not collect recursive text.
const LANDMARK_ROLES: &[&str] = &[
    "navigation", "main", "banner", "contentinfo", "complementary", "region", "form",
];

/// Check if an element has an explicit ARIA role that is a landmark role.
fn is_landmark_role_attr(node: &LayoutNode) -> bool {
    node.attributes
        .get("role")
        .map(|r| LANDMARK_ROLES.contains(&r.as_str()))
        .unwrap_or(false)
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
        let hidden_marker = if el.hidden == Some(true) { "!" } else { "" };
        parts.push(format!("{}{}:{}", hidden_marker, el.id, el.tag));

        if let Some(ref t) = el.input_type {
            if t != "text" {
                parts.last_mut().unwrap().push_str(&format!(":{}", t));
            }
        }

        // Form state markers
        if let Some(ref n) = el.name {
            parts.push(format!("[{}]", n));
        }
        if el.checked == Some(true) {
            parts.push("[v]".to_string());
        }
        if el.required == Some(true) {
            parts.push("[*]".to_string());
        }
        if let Some(ref v) = el.val {
            if !v.is_empty() {
                parts.push(format!("[={}]", v));
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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

// --- Alert detection ---

/// Detect alert type from role attributes or CSS class names.
fn detect_alert_type(node: &LayoutNode) -> Option<String> {
    // Check role attribute first
    if let Some(role) = node.attributes.get("role") {
        match role.as_str() {
            "alert" => return Some("alert".to_string()),
            "status" => return Some("status".to_string()),
            _ => {}
        }
    }

    // Check CSS classes for alert patterns.
    // Only match compound alert classes (e.g. "alert-error", "msg-danger", "flash-success")
    // to avoid false positives on generic classes like "error" used for non-alert purposes
    // (e.g. old Reddit uses class="error" on loading placeholder spans).
    if let Some(classes) = node.attributes.get("class") {
        let lower = classes.to_lowercase();
        let class_list: Vec<&str> = lower.split_whitespace().collect();
        for cls in &class_list {
            // Require compound patterns: "alert-error", "msg-error", "form-error", etc.
            // A bare "error" class is too ambiguous.
            let is_error = (cls.contains("error") || cls.contains("danger"))
                && (cls.contains('-') || cls.contains('_') || cls.starts_with("alert") || cls.starts_with("msg"));
            if is_error {
                return Some("error".to_string());
            }
            let is_success = cls.contains("success")
                && (cls.contains('-') || cls.contains('_') || cls.starts_with("alert") || cls.starts_with("msg"));
            if is_success {
                return Some("success".to_string());
            }
            let is_warning = cls.contains("warning")
                && (cls.contains('-') || cls.contains('_') || cls.starts_with("alert") || cls.starts_with("msg"));
            if is_warning {
                return Some("warning".to_string());
            }
            // "alert" as a class by itself is typically intentional (Bootstrap, etc.)
            if *cls == "alert" || cls.starts_with("alert-") || cls.starts_with("alert_") {
                return Some("alert".to_string());
            }
            if cls.contains("notice") || cls.contains("flash") {
                return Some("alert".to_string());
            }
        }
    }

    None
}

impl SpatialDom {
    /// Return elements with an alert_type set.
    pub fn alerts(&self) -> Vec<&SpatialElement> {
        self.els.iter().filter(|e| e.alert_type.is_some()).collect()
    }
}

// --- Table extraction ---

/// Structured table data extracted from the Spatial DOM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableData {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

impl SpatialDom {
    /// Extract structured table data from the Spatial DOM.
    /// Groups `th` elements as headers and `td` elements as row cells,
    /// using Y coordinates to determine row grouping.
    pub fn tables(&self) -> Vec<TableData> {
        // Collect all th and td elements
        let ths: Vec<&SpatialElement> = self.els.iter().filter(|e| e.tag == "th").collect();
        let tds: Vec<&SpatialElement> = self.els.iter().filter(|e| e.tag == "td").collect();

        if ths.is_empty() && tds.is_empty() {
            return Vec::new();
        }

        // Group headers by Y coordinate
        let headers = group_by_row(&ths);
        // Group data cells by Y coordinate
        let data_rows = group_by_row(&tds);

        // For simplicity, treat all th cells as a single header row
        // and all td cells as data rows. If there are multiple tables,
        // we'd need to cluster by X/Y proximity, but for now a single
        // table per page is the common case.
        let header_texts: Vec<String> = if !headers.is_empty() {
            headers[0].iter().map(|e| e.text.clone().unwrap_or_default()).collect()
        } else {
            Vec::new()
        };

        let row_data: Vec<Vec<String>> = data_rows
            .iter()
            .map(|row| row.iter().map(|e| e.text.clone().unwrap_or_default()).collect())
            .collect();

        if header_texts.is_empty() && row_data.is_empty() {
            return Vec::new();
        }

        vec![TableData {
            headers: header_texts,
            rows: row_data,
        }]
    }
}

/// Group elements into rows by Y coordinate (elements at the same Y = same row).
fn group_by_row<'a>(elements: &[&'a SpatialElement]) -> Vec<Vec<&'a SpatialElement>> {
    if elements.is_empty() {
        return Vec::new();
    }

    let mut sorted: Vec<&SpatialElement> = elements.to_vec();
    sorted.sort_by_key(|e| (e.b[1], e.b[0]));

    let mut rows: Vec<Vec<&'a SpatialElement>> = Vec::new();
    let mut current_row: Vec<&SpatialElement> = vec![sorted[0]];
    let mut current_y = sorted[0].b[1];

    for &el in &sorted[1..] {
        // Elements within 5px of the same Y are considered the same row
        if (el.b[1] - current_y).abs() <= 5 {
            current_row.push(el);
        } else {
            rows.push(current_row);
            current_row = vec![el];
            current_y = el.b[1];
        }
    }
    rows.push(current_row);
    rows
}

// --- Page type detection ---

/// Detected page type for agent decision-making.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub enum PageType {
    Login,
    TwoFactorAuth,
    OAuthConsent,
    Captcha,
    Search,
    SearchResults,
    Inbox,
    EmailBody,
    Dashboard,
    Form,
    Article,
    List,
    Error,
    #[default]
    Other,
}

impl PageType {
    pub fn is_other(&self) -> bool {
        matches!(self, PageType::Other)
    }
}

// --- Pagination detection ---

/// Pagination links detected on the page.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pagination {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prev: Option<String>,
    /// Numbered page links: (label, url)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub pages: Vec<(String, String)>,
}

impl SpatialDom {
    /// Detect pagination links on the page.
    pub fn pagination(&self) -> Option<Pagination> {
        let links: Vec<&SpatialElement> = self.els.iter()
            .filter(|e| e.role.as_deref() == Some("link") && e.href.is_some())
            .collect();

        let mut next: Option<String> = None;
        let mut prev: Option<String> = None;
        let mut pages: Vec<(String, String)> = Vec::new();

        for link in &links {
            let text = link.text.as_deref().unwrap_or("").trim().to_lowercase();
            let href = link.href.as_deref().unwrap_or("");

            if text == "next" || text == "next page" || text == ">" || text == ">>"
                || text == "\u{203a}" || text == "\u{00bb}"
            {
                next = Some(href.to_string());
            } else if text == "previous" || text == "prev" || text == "prev page"
                || text == "<" || text == "<<"
                || text == "\u{2039}" || text == "\u{00ab}"
            {
                prev = Some(href.to_string());
            } else if text.chars().all(|c| c.is_ascii_digit()) && !text.is_empty() {
                pages.push((text.clone(), href.to_string()));
            }
        }

        if next.is_some() || prev.is_some() || !pages.is_empty() {
            Some(Pagination { next, prev, pages })
        } else {
            None
        }
    }
}

// --- Verification code extraction ---

impl SpatialDom {
    /// Extract verification codes (4-8 digit sequences) from page text near code-related keywords.
    pub fn find_codes(&self) -> Vec<String> {
        let code_keywords = [
            "verification code", "security code", "your code",
            "enter code", "otp", "passcode", "one-time",
        ];

        let mut codes = Vec::new();

        for el in &self.els {
            let text = match &el.text {
                Some(t) => t,
                None => continue,
            };

            let text_lower = text.to_lowercase();
            let has_keyword = code_keywords.iter().any(|kw| text_lower.contains(kw));

            // Proximity check: only match short label elements within 100px Y
            let near_keyword = if !has_keyword {
                let el_y = el.b[1];
                self.els.iter().any(|other| {
                    (other.b[1] - el_y).abs() < 100
                        && other.text.as_ref().map(|t| {
                            t.len() < 80 && {
                                let lower = t.to_lowercase();
                                code_keywords.iter().any(|kw| lower.contains(kw))
                            }
                        }).unwrap_or(false)
                })
            } else {
                false
            };

            if !has_keyword && !near_keyword {
                continue;
            }

            // Extract 4-8 digit sequences, filtering out year-like numbers
            let chars: Vec<char> = text.chars().collect();
            let mut i = 0;
            while i < chars.len() {
                if chars[i].is_ascii_digit() {
                    let start = i;
                    while i < chars.len() && chars[i].is_ascii_digit() {
                        i += 1;
                    }
                    let len = i - start;
                    if len >= 4 && len <= 8 {
                        let code: String = chars[start..i].iter().collect();
                        // Filter out year-like 4-digit numbers (1900-2099)
                        if len == 4 {
                            if let Ok(n) = code.parse::<u32>() {
                                if (1900..=2099).contains(&n) {
                                    continue;
                                }
                            }
                        }
                        if !codes.contains(&code) {
                            codes.push(code);
                        }
                    }
                } else {
                    i += 1;
                }
            }
        }

        codes
    }
}

// --- Search input matching ---

/// Check if an element is a search input (input or textarea used for search queries).
fn is_search_input(e: &SpatialElement) -> bool {
    let is_text_input = match e.tag.as_str() {
        "input" => !matches!(
            e.input_type.as_deref(),
            Some("checkbox") | Some("radio") | Some("hidden") | Some("submit")
            | Some("button") | Some("image") | Some("password")
        ),
        "textarea" => true,
        _ => return false,
    };
    if !is_text_input { return false; }

    e.input_type.as_deref() == Some("search")
        || e.role.as_deref() == Some("searchbox")
        || e.name.as_deref() == Some("q")
        || e.name.as_ref().map(|n| n.to_lowercase().contains("search")).unwrap_or(false)
        || e.ph.as_ref().map(|p| p.to_lowercase().contains("search")).unwrap_or(false)
        || e.label.as_ref().map(|l| l.to_lowercase().contains("search")).unwrap_or(false)
}

// --- Page type detection ---

fn detect_page_type(dom: &SpatialDom) -> PageType {
    let title_lower = dom.title.to_lowercase();

    let title_has = |keywords: &[&str]| keywords.iter().any(|kw| title_lower.contains(kw));
    let heading_has = |keywords: &[&str]| {
        dom.els.iter().any(|e| {
            e.role.as_deref() == Some("heading")
                && e.text.as_ref().map(|t| {
                    let lower = t.to_lowercase();
                    keywords.iter().any(|kw| lower.contains(kw))
                }).unwrap_or(false)
        })
    };

    // Error
    let has_error_alerts = dom.els.iter().any(|e| {
        e.alert_type.as_deref() == Some("error")
    });
    let title_has_error = title_has(&["404", "500", "403", "not found", "error"]);
    if has_error_alerts || title_has_error {
        return PageType::Error;
    }

    // Captcha
    if title_has(&["captcha", "verify you're human", "verify you are human", "robot"]) {
        return PageType::Captcha;
    }

    // Login
    let has_password = dom.els.iter().any(|e| {
        e.hidden != Some(true) && e.input_type.as_deref() == Some("password")
    });
    if has_password {
        return PageType::Login;
    }

    // TwoFactorAuth — use specific phrases to avoid false positives on programming content
    // (bare "code" matches any page about source code)
    let verification_keywords = &[
        "verification", "verify your", "enter code", "security code", "verification code",
        "2fa", "two-factor", "two factor", "otp", "one-time", "passcode",
    ];
    let has_verification_context = title_has(verification_keywords) || heading_has(verification_keywords);
    let has_code_input = dom.els.iter().any(|e| {
        e.hidden != Some(true) && e.tag == "input" && {
            let t = e.input_type.as_deref().unwrap_or("text");
            t == "text" || t == "number" || t == "tel"
        }
    });
    if has_verification_context && has_code_input {
        return PageType::TwoFactorAuth;
    }

    // OAuthConsent
    let oauth_keywords = &["authorize", "allow access", "grant permission", "oauth", "consent"];
    if title_has(oauth_keywords) || heading_has(oauth_keywords) {
        return PageType::OAuthConsent;
    }

    // Inbox
    let inbox_keywords = &["inbox", "mail", "messages"];
    let link_count = dom.els.iter()
        .filter(|e| e.hidden != Some(true) && e.role.as_deref() == Some("link"))
        .count();
    if title_has(inbox_keywords) && link_count >= 10 {
        return PageType::Inbox;
    }

    // EmailBody
    let email_markers = ["from:", "to:", "subject:", "date:"];
    let marker_count = email_markers.iter().filter(|marker| {
        dom.els.iter().any(|e| {
            e.text.as_ref().map(|t| t.to_lowercase().contains(*marker)).unwrap_or(false)
        })
    }).count();
    if marker_count >= 3 {
        return PageType::EmailBody;
    }

    // Dashboard
    let dashboard_keywords = &["dashboard", "welcome back", "overview"];
    let has_nav = dom.els.iter().any(|e| e.role.as_deref() == Some("navigation"));
    let has_main = dom.els.iter().any(|e| e.role.as_deref() == Some("main"));
    if (title_has(dashboard_keywords) || heading_has(dashboard_keywords)) && has_nav && has_main {
        return PageType::Dashboard;
    }

    // Article (before Search — many content pages have search bars)
    // When a page has many links (typical of list pages), require more long text to
    // classify as Article. This prevents content-heavy list pages (e.g. subreddits with
    // long post descriptions) from being misclassified.
    let headings = dom.els.iter().filter(|e| e.role.as_deref() == Some("heading")).count();
    let long_texts = dom.els.iter().filter(|e| {
        e.tag == "p" && e.text.as_ref().map(|t| t.len() > 100).unwrap_or(false)
    }).count();
    let long_text_threshold = if link_count >= 20 { 10 } else { 2 };
    if headings >= 3 && long_texts >= long_text_threshold {
        return PageType::Article;
    }

    // SearchResults — needs both a search input AND search-related title/heading context.
    // This must come before List, since search result pages have many links.
    let has_search_input = dom.els.iter().any(|e| {
        e.hidden != Some(true) && is_search_input(e)
    });
    let search_results_keywords = &["search results", "results for", "search:", "found"];
    let has_search_results_context = title_has(search_results_keywords)
        || heading_has(search_results_keywords)
        || title_has(&["search"]);
    if has_search_input && has_search_results_context && link_count >= 8 {
        return PageType::SearchResults;
    }

    // List (before Search — many list pages have search bars in nav)
    if link_count >= 10 {
        return PageType::List;
    }

    // Search — a search input without enough links to be a results page or list
    if has_search_input {
        return PageType::Search;
    }

    // Form — count data-entry inputs only (exclude checkboxes, radios, hidden, submit, button)
    let input_count = dom.els.iter().filter(|e| {
        match e.tag.as_str() {
            "textarea" | "select" => true,
            "input" => !matches!(
                e.input_type.as_deref(),
                Some("checkbox") | Some("radio") | Some("hidden") | Some("submit") | Some("button") | Some("image")
            ),
            _ => false,
        }
    }).count();
    if input_count >= 2 {
        return PageType::Form;
    }

    PageType::Other
}

// --- Suggested action detection ---

fn detect_suggested_actions(dom: &SpatialDom) -> Vec<SuggestedAction> {
    let mut actions = Vec::new();

    // Priority order: Login > EnterCode > Consent > Search > SelectFromList
    if let Some(a) = detect_login_action(dom) {
        actions.push(a);
    }
    if let Some(a) = detect_enter_code_action(dom) {
        actions.push(a);
    }
    if let Some(a) = detect_consent_action(dom) {
        actions.push(a);
    }
    if let Some(a) = detect_search_action(dom) {
        actions.push(a);
    }
    if let Some(a) = detect_select_from_list_action(dom) {
        actions.push(a);
    }

    actions
}

fn detect_login_action(dom: &SpatialDom) -> Option<SuggestedAction> {
    let password = dom.els.iter().find(|e| {
        e.hidden != Some(true) && e.input_type.as_deref() == Some("password")
    })?;
    let password_id = password.id;
    let password_y = password.b[1];

    // Find nearest text/email input within 500px Y distance
    let username = dom.els.iter()
        .filter(|e| e.hidden != Some(true) && e.tag == "input")
        .filter(|e| {
            let t = e.input_type.as_deref().unwrap_or("text");
            t == "text" || t == "email"
        })
        .filter(|e| (e.b[1] - password_y).abs() < 500)
        .min_by_key(|e| (e.b[1] - password_y).abs())?;
    let username_id = username.id;

    let submit_id = find_nearest_submit_button(dom, password_id)?;

    // Optional: find "remember me" checkbox
    let remember_me_id = dom.els.iter()
        .filter(|e| e.hidden != Some(true))
        .filter(|e| e.input_type.as_deref() == Some("checkbox"))
        .find(|e| {
            let label = e.label.as_deref().unwrap_or("").to_lowercase();
            let name = e.name.as_deref().unwrap_or("").to_lowercase();
            label.contains("remember") || name.contains("remember")
        })
        .map(|e| e.id);

    Some(SuggestedAction::Login {
        username_id,
        password_id,
        submit_id,
        remember_me_id,
    })
}

fn detect_enter_code_action(dom: &SpatialDom) -> Option<SuggestedAction> {
    let verification_keywords = [
        "verification", "verify your", "enter code", "security code", "verification code",
        "2fa", "two-factor", "two factor", "otp", "one-time", "passcode",
    ];

    let title_lower = dom.title.to_lowercase();
    let has_keyword_in_title = verification_keywords.iter().any(|kw| title_lower.contains(kw));

    let has_keyword_in_heading = dom.els.iter().any(|e| {
        e.role.as_deref() == Some("heading")
            && e.text.as_ref().map(|t| {
                let lower = t.to_lowercase();
                verification_keywords.iter().any(|kw| lower.contains(kw))
            }).unwrap_or(false)
    });

    if !has_keyword_in_title && !has_keyword_in_heading {
        return None;
    }

    // Don't emit EnterCode if there's a password field (that's Login)
    if dom.els.iter().any(|e| e.hidden != Some(true) && e.input_type.as_deref() == Some("password")) {
        return None;
    }

    let code_keywords = ["code", "otp", "pin", "verify"];

    // Find code-like inputs by name/label/placeholder
    let code_inputs: Vec<&SpatialElement> = dom.els.iter()
        .filter(|e| e.hidden != Some(true) && e.tag == "input")
        .filter(|e| {
            let t = e.input_type.as_deref().unwrap_or("text");
            t == "text" || t == "number" || t == "tel"
        })
        .filter(|e| {
            let name = e.name.as_deref().unwrap_or("").to_lowercase();
            let label = e.label.as_deref().unwrap_or("").to_lowercase();
            let ph = e.ph.as_deref().unwrap_or("").to_lowercase();
            code_keywords.iter().any(|kw| name.contains(kw) || label.contains(kw) || ph.contains(kw))
        })
        .collect();

    // Check for separate digit inputs (width < 60px, count 4-8)
    let narrow_inputs: Vec<&SpatialElement> = dom.els.iter()
        .filter(|e| e.hidden != Some(true) && e.tag == "input")
        .filter(|e| {
            let t = e.input_type.as_deref().unwrap_or("text");
            (t == "text" || t == "number" || t == "tel") && e.b[2] < 60
        })
        .collect();

    let (input_id, code_length);

    if narrow_inputs.len() >= 4 && narrow_inputs.len() <= 8 {
        code_length = Some(narrow_inputs.len());
        input_id = narrow_inputs[0].id;
    } else if !code_inputs.is_empty() {
        input_id = code_inputs[0].id;
        code_length = None;
    } else {
        // Fallback: any visible text/number/tel input on a verification page
        let any_input = dom.els.iter()
            .filter(|e| e.hidden != Some(true) && e.tag == "input")
            .find(|e| {
                let t = e.input_type.as_deref().unwrap_or("text");
                t == "text" || t == "number" || t == "tel"
            })?;
        input_id = any_input.id;
        code_length = None;
    }

    let submit_id = find_nearest_submit_button(dom, input_id)?;

    Some(SuggestedAction::EnterCode {
        input_id,
        submit_id,
        code_length,
    })
}

fn detect_search_action(dom: &SpatialDom) -> Option<SuggestedAction> {
    let search_input = dom.els.iter()
        .filter(|e| e.hidden != Some(true))
        .find(|e| is_search_input(e))?;

    let submit_id = find_nearest_submit_button(dom, search_input.id)?;

    Some(SuggestedAction::Search {
        input_id: search_input.id,
        submit_id,
    })
}

fn detect_consent_action(dom: &SpatialDom) -> Option<SuggestedAction> {
    let oauth_keywords = ["authorize", "allow access", "grant permission", "oauth", "consent"];

    let title_lower = dom.title.to_lowercase();
    let has_keyword_in_title = oauth_keywords.iter().any(|kw| title_lower.contains(kw));

    let has_keyword_in_heading = dom.els.iter().any(|e| {
        matches!(e.tag.as_str(), "h1" | "h2")
            && e.text.as_ref().map(|t| {
                let lower = t.to_lowercase();
                oauth_keywords.iter().any(|kw| lower.contains(kw))
            }).unwrap_or(false)
    });

    if !has_keyword_in_title && !has_keyword_in_heading {
        return None;
    }

    let approve_words = ["allow", "authorize", "accept", "approve", "grant"];
    let deny_words = ["deny", "cancel", "decline", "reject"];

    let approve_ids: Vec<u32> = dom.els.iter()
        .filter(|e| e.hidden != Some(true))
        .filter(|e| e.tag == "button" || e.role.as_deref() == Some("button"))
        .filter(|e| {
            e.text.as_ref().map(|t| {
                let lower = t.to_lowercase();
                approve_words.iter().any(|w| lower.contains(w))
            }).unwrap_or(false)
        })
        .map(|e| e.id)
        .collect();

    let deny_ids: Vec<u32> = dom.els.iter()
        .filter(|e| e.hidden != Some(true))
        .filter(|e| e.tag == "button" || e.role.as_deref() == Some("button"))
        .filter(|e| {
            e.text.as_ref().map(|t| {
                let lower = t.to_lowercase();
                deny_words.iter().any(|w| lower.contains(w))
            }).unwrap_or(false)
        })
        .map(|e| e.id)
        .collect();

    if approve_ids.is_empty() && deny_ids.is_empty() {
        return None;
    }

    Some(SuggestedAction::Consent { approve_ids, deny_ids })
}

fn detect_select_from_list_action(dom: &SpatialDom) -> Option<SuggestedAction> {
    let mut links: Vec<&SpatialElement> = dom.els.iter()
        .filter(|e| e.hidden != Some(true))
        .filter(|e| e.tag == "a" && e.href.is_some())
        .collect();

    if links.len() < 5 {
        return None;
    }

    links.sort_by_key(|e| e.b[1]);

    // Group into rows (within 30px Y = same row)
    let mut rows: Vec<Vec<u32>> = Vec::new();
    let mut current_row: Vec<u32> = vec![links[0].id];
    let mut current_y = links[0].b[1];

    for link in &links[1..] {
        if (link.b[1] - current_y).abs() <= 30 {
            current_row.push(link.id);
        } else {
            rows.push(current_row);
            current_row = vec![link.id];
            current_y = link.b[1];
        }
    }
    rows.push(current_row);

    if rows.len() < 5 {
        return None;
    }

    let items: Vec<u32> = rows.iter().map(|row| row[0]).collect();
    Some(SuggestedAction::SelectFromList { items })
}

/// Find the nearest visible submit button to an input element.
/// Prefers buttons below the input; scores by Manhattan distance (Y weighted 2x).
pub(crate) fn find_nearest_submit_button(dom: &SpatialDom, input_id: u32) -> Option<u32> {
    let input = dom.get(input_id)?;
    let input_y = input.b[1];
    let input_x = input.b[0];

    let mut best: Option<(u32, i32)> = None;

    for el in &dom.els {
        if el.hidden == Some(true) { continue; }
        let is_button = el.tag == "button"
            || (el.tag == "input" && el.input_type.as_deref() == Some("submit"));
        if !is_button { continue; }

        let dy = el.b[1] - input_y;
        let dx = (el.b[0] - input_x).abs();
        // Heavy penalty for buttons above the input
        let score = if dy < 0 { dy.abs() * 4 + dx } else { dy * 2 + dx };

        if best.is_none() || score < best.unwrap().1 {
            best = Some((el.id, score));
        }
    }

    best.map(|(id, _)| id)
}
