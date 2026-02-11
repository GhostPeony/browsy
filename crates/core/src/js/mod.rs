//! Pattern-based JavaScript inference.
//!
//! Instead of running a JS engine, we detect common JS patterns
//! (toggle visibility, class changes, tab switching) and simulate
//! their effects on the DOM. This gives agents a way to "click"
//! elements and see updated DOM state without actual JS execution.

use crate::dom::{DomNode, NodeType};

/// A detected interactive behavior on the page.
#[derive(Debug, Clone)]
pub struct JsBehavior {
    /// The element ID (from spatial DOM) that triggers this behavior.
    pub trigger_id: u32,
    /// What happens when the trigger is activated.
    pub action: JsAction,
}

/// An inferred JS action.
#[derive(Debug, Clone)]
pub enum JsAction {
    /// Toggle visibility of a target element (show/hide).
    ToggleVisibility {
        /// CSS selector or ID of the target element.
        target: String,
    },
    /// Toggle a CSS class on a target element.
    ToggleClass {
        target: String,
        class: String,
    },
    /// Switch tabs: show one panel, hide siblings.
    TabSwitch {
        /// The panel to show.
        show_target: String,
        /// Sibling panels to hide.
        hide_targets: Vec<String>,
    },
    /// Submit a form (already handled by Session, but noted here).
    FormSubmit {
        form_selector: String,
    },
    /// Navigate to a URL (already handled by Session click).
    Navigate {
        url: String,
    },
}

/// Analyze the DOM for common JS patterns and return detected behaviors.
pub fn detect_behaviors(dom: &DomNode) -> Vec<JsBehavior> {
    let mut behaviors = Vec::new();
    let mut id_counter = 1u32;

    detect_behaviors_recursive(dom, &mut behaviors, &mut id_counter);

    behaviors
}

fn detect_behaviors_recursive(
    node: &DomNode,
    behaviors: &mut Vec<JsBehavior>,
    id_counter: &mut u32,
) {
    if node.node_type == NodeType::Element {
        // Check for onclick handlers
        if let Some(onclick) = node.get_attr("onclick") {
            if let Some(action) = parse_onclick(onclick) {
                behaviors.push(JsBehavior {
                    trigger_id: *id_counter,
                    action,
                });
            }
        }

        // Check for data-toggle patterns (Bootstrap-style)
        if let Some(toggle) = node.get_attr("data-toggle") {
            if let Some(target) = node.get_attr("data-target")
                .or_else(|| node.get_attr("href"))
            {
                let action = match toggle {
                    "collapse" | "dropdown" | "modal" => JsAction::ToggleVisibility {
                        target: target.to_string(),
                    },
                    "tab" | "pill" => {
                        if let Some(target_id) = target.strip_prefix('#') {
                            JsAction::TabSwitch {
                                show_target: target_id.to_string(),
                                hide_targets: Vec::new(),
                            }
                        } else {
                            JsAction::ToggleVisibility {
                                target: target.to_string(),
                            }
                        }
                    }
                    _ => JsAction::ToggleVisibility {
                        target: target.to_string(),
                    },
                };
                behaviors.push(JsBehavior {
                    trigger_id: *id_counter,
                    action,
                });
            }
        }

        // Check for aria-controls (accessibility pattern for toggles)
        if let Some(controls) = node.get_attr("aria-controls") {
            if node.get_attr("aria-expanded").is_some() {
                behaviors.push(JsBehavior {
                    trigger_id: *id_counter,
                    action: JsAction::ToggleVisibility {
                        target: format!("#{}", controls),
                    },
                });
            }
        }

        // Check for role="tab" with aria-controls
        if node.get_attr("role") == Some("tab") {
            if let Some(controls) = node.get_attr("aria-controls") {
                behaviors.push(JsBehavior {
                    trigger_id: *id_counter,
                    action: JsAction::TabSwitch {
                        show_target: controls.to_string(),
                        hide_targets: Vec::new(),
                    },
                });
            }
        }

        // Increment ID for elements that would be in the spatial DOM
        let tag = node.tag.as_str();
        let is_counted = is_interactive_tag(tag)
            || is_text_tag(tag)
            || node.attributes.contains_key("role")
            || node.attributes.contains_key("onclick")
            || node.attributes.get("tabindex").is_some();
        if is_counted {
            *id_counter += 1;
        }
    }

    for child in &node.children {
        detect_behaviors_recursive(child, behaviors, id_counter);
    }
}

/// Parse an onclick attribute value into a JsAction.
fn parse_onclick(onclick: &str) -> Option<JsAction> {
    let onclick = onclick.trim();

    // Pattern: toggle/show/hide element by ID
    // e.g., document.getElementById('foo').style.display = 'none'
    // e.g., document.getElementById('menu').classList.toggle('hidden')
    // e.g., $('#dropdown').toggle()
    // e.g., toggle('panel')

    // getElementById with classList operations (check before generic toggle)
    if let Some(id) = extract_element_id(onclick) {
        if let Some(class) = extract_class_toggle(onclick) {
            return Some(JsAction::ToggleClass {
                target: format!("#{}", id),
                class,
            });
        }
        if onclick.contains("style.display") || onclick.contains(".toggle(") {
            return Some(JsAction::ToggleVisibility {
                target: format!("#{}", id),
            });
        }
        return Some(JsAction::ToggleVisibility {
            target: format!("#{}", id),
        });
    }

    // jQuery-style: $(...).toggle(), $(...).show(), $(...).hide()
    if let Some(selector) = extract_jquery_selector(onclick) {
        if onclick.contains(".toggle(") || onclick.contains(".show(")
            || onclick.contains(".hide(")
        {
            return Some(JsAction::ToggleVisibility {
                target: selector,
            });
        }
        if onclick.contains(".addClass(") || onclick.contains(".removeClass(")
            || onclick.contains(".toggleClass(")
        {
            if let Some(class) = extract_jquery_class(onclick) {
                return Some(JsAction::ToggleClass {
                    target: selector,
                    class,
                });
            }
        }
    }

    // Simple function call: toggle('id'), showPanel('id')
    if let Some(id) = extract_function_arg(onclick) {
        return Some(JsAction::ToggleVisibility {
            target: format!("#{}", id),
        });
    }

    // location.href = '...' or window.location = '...'
    if onclick.contains("location") {
        if let Some(url) = extract_url_assignment(onclick) {
            return Some(JsAction::Navigate { url });
        }
    }

    None
}

/// Extract element ID from getElementById('id') or similar patterns.
fn extract_element_id(s: &str) -> Option<String> {
    // getElementById('id') or getElementById("id")
    if let Some(start) = s.find("getElementById(") {
        let rest = &s[start + 15..];
        return extract_quoted_string(rest);
    }
    None
}

/// Extract class name from classList.toggle('class') pattern.
fn extract_class_toggle(s: &str) -> Option<String> {
    for method in &["classList.toggle(", "classList.add(", "classList.remove("] {
        if let Some(start) = s.find(method) {
            let rest = &s[start + method.len()..];
            return extract_quoted_string(rest);
        }
    }
    None
}

/// Extract jQuery selector from $('selector') or jQuery('selector').
fn extract_jquery_selector(s: &str) -> Option<String> {
    for prefix in &["$('", "$(\"", "jQuery('", "jQuery(\""] {
        if let Some(start) = s.find(prefix) {
            let rest = &s[start + prefix.len()..];
            let quote = if prefix.ends_with('\'') { '\'' } else { '"' };
            if let Some(end) = rest.find(quote) {
                return Some(rest[..end].to_string());
            }
        }
    }
    None
}

/// Extract class name from jQuery .addClass('class') etc.
fn extract_jquery_class(s: &str) -> Option<String> {
    for method in &[".addClass('", ".addClass(\"", ".removeClass('",
                    ".removeClass(\"", ".toggleClass('", ".toggleClass(\""] {
        if let Some(start) = s.find(method) {
            let rest = &s[start + method.len()..];
            let quote = if method.ends_with('\'') { '\'' } else { '"' };
            if let Some(end) = rest.find(quote) {
                return Some(rest[..end].to_string());
            }
        }
    }
    None
}

/// Extract a simple function argument: functionName('arg')
fn extract_function_arg(s: &str) -> Option<String> {
    // Match pattern: word('...')
    if let Some(paren) = s.find('(') {
        let before = &s[..paren];
        if before.chars().all(|c| c.is_alphanumeric() || c == '_') {
            let rest = &s[paren + 1..];
            return extract_quoted_string(rest);
        }
    }
    None
}

/// Extract URL from location.href = '...' or window.location = '...'
fn extract_url_assignment(s: &str) -> Option<String> {
    for pattern in &["location.href=", "location.href =", "location=", "location =",
                     "window.location.href=", "window.location.href =",
                     "window.location=", "window.location ="] {
        if let Some(start) = s.find(pattern) {
            let rest = &s[start + pattern.len()..].trim_start();
            if let Some(url) = extract_quoted_string(rest) {
                return Some(url);
            }
        }
    }
    None
}

/// Extract a quoted string (single or double quotes).
fn extract_quoted_string(s: &str) -> Option<String> {
    let s = s.trim();
    let quote = s.chars().next()?;
    if quote != '\'' && quote != '"' {
        return None;
    }
    let rest = &s[1..];
    if let Some(end) = rest.find(quote) {
        Some(rest[..end].to_string())
    } else {
        None
    }
}

/// Apply a JS action to a DOM tree, returning the modified tree.
/// This simulates the effect of the JS action without running JS.
pub fn apply_action(dom: &DomNode, action: &JsAction) -> DomNode {
    match action {
        JsAction::ToggleVisibility { target } => {
            let id = target.strip_prefix('#').unwrap_or(target);
            toggle_element_visibility(dom, id)
        }
        JsAction::ToggleClass { target, class } => {
            let id = target.strip_prefix('#').unwrap_or(target);
            toggle_element_class(dom, id, class)
        }
        JsAction::TabSwitch { show_target, hide_targets } => {
            let mut result = dom.clone();
            // Show the target
            result = set_element_visibility(&result, show_target, true);
            // Hide siblings
            for hide in hide_targets {
                result = set_element_visibility(&result, hide, false);
            }
            result
        }
        JsAction::FormSubmit { .. } | JsAction::Navigate { .. } => {
            // Handled by Session, not DOM manipulation
            dom.clone()
        }
    }
}

/// Toggle the display of an element by ID.
fn toggle_element_visibility(node: &DomNode, target_id: &str) -> DomNode {
    let mut result = node.clone();

    if result.node_type == NodeType::Element {
        if result.get_attr("id") == Some(target_id) {
            // Toggle: if hidden, show; if visible, hide
            let is_hidden = result.get_attr("style")
                .map(|s| s.contains("display: none") || s.contains("display:none"))
                .unwrap_or(false)
                || result.attributes.contains_key("hidden");

            if is_hidden {
                // Show: remove hidden attribute and display:none from style
                result.attributes.remove("hidden");
                if let Some(style) = result.attributes.get_mut("style") {
                    *style = style
                        .replace("display: none", "")
                        .replace("display:none", "")
                        .trim_matches(';')
                        .trim()
                        .to_string();
                }
            } else {
                // Hide: add display:none
                let current = result.attributes.get("style")
                    .map(|s| s.to_string())
                    .unwrap_or_default();
                if current.is_empty() {
                    result.attributes.insert("style".to_string(), "display: none".to_string());
                } else {
                    result.attributes.insert("style".to_string(), format!("{}; display: none", current));
                }
            }
            return result;
        }
    }

    // Recurse
    result.children = result.children.iter()
        .map(|c| toggle_element_visibility(c, target_id))
        .collect();

    result
}

/// Set element visibility explicitly.
fn set_element_visibility(node: &DomNode, target_id: &str, visible: bool) -> DomNode {
    let mut result = node.clone();

    if result.node_type == NodeType::Element {
        if result.get_attr("id") == Some(target_id) {
            if visible {
                result.attributes.remove("hidden");
                if let Some(style) = result.attributes.get_mut("style") {
                    *style = style
                        .replace("display: none", "")
                        .replace("display:none", "")
                        .trim_matches(';')
                        .trim()
                        .to_string();
                }
            } else {
                let current = result.attributes.get("style")
                    .map(|s| s.to_string())
                    .unwrap_or_default();
                if current.is_empty() {
                    result.attributes.insert("style".to_string(), "display: none".to_string());
                } else if !current.contains("display: none") && !current.contains("display:none") {
                    result.attributes.insert("style".to_string(), format!("{}; display: none", current));
                }
            }
            return result;
        }
    }

    result.children = result.children.iter()
        .map(|c| set_element_visibility(c, target_id, visible))
        .collect();

    result
}

/// Toggle a class on an element by ID.
fn toggle_element_class(node: &DomNode, target_id: &str, class: &str) -> DomNode {
    let mut result = node.clone();

    if result.node_type == NodeType::Element {
        if result.get_attr("id") == Some(target_id) {
            let current_classes = result.attributes.get("class")
                .map(|s| s.to_string())
                .unwrap_or_default();
            let class_list: Vec<&str> = current_classes.split_whitespace().collect();

            if class_list.contains(&class) {
                // Remove class
                let new_classes: Vec<&str> = class_list.into_iter()
                    .filter(|c| *c != class)
                    .collect();
                result.attributes.insert("class".to_string(), new_classes.join(" "));
            } else {
                // Add class
                let new = if current_classes.is_empty() {
                    class.to_string()
                } else {
                    format!("{} {}", current_classes, class)
                };
                result.attributes.insert("class".to_string(), new);
            }
            return result;
        }
    }

    result.children = result.children.iter()
        .map(|c| toggle_element_class(c, target_id, class))
        .collect();

    result
}

fn is_interactive_tag(tag: &str) -> bool {
    matches!(tag, "a" | "button" | "input" | "select" | "textarea" | "details" | "summary")
}

fn is_text_tag(tag: &str) -> bool {
    matches!(tag,
        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "p" | "label" | "span" |
        "li" | "td" | "th" | "dt" | "dd" | "figcaption" | "blockquote" |
        "pre" | "code" | "em" | "strong" | "b" | "i" | "mark" | "small"
    )
}

/// Detect tab groups from the DOM (role="tablist" patterns).
pub fn detect_tab_groups(dom: &DomNode) -> Vec<TabGroup> {
    let mut groups = Vec::new();
    find_tab_groups(dom, &mut groups);
    groups
}

/// A group of tabs with their associated panels.
#[derive(Debug, Clone)]
pub struct TabGroup {
    pub tabs: Vec<TabInfo>,
}

#[derive(Debug, Clone)]
pub struct TabInfo {
    pub id: String,
    pub label: String,
    pub panel_id: String,
    pub selected: bool,
}

fn find_tab_groups(node: &DomNode, groups: &mut Vec<TabGroup>) {
    if node.get_attr("role") == Some("tablist") {
        let mut tabs = Vec::new();
        for child in &node.children {
            if child.get_attr("role") == Some("tab") {
                let id = child.get_attr("id").unwrap_or("").to_string();
                let label = child.text_content();
                let panel_id = child.get_attr("aria-controls").unwrap_or("").to_string();
                let selected = child.get_attr("aria-selected") == Some("true");
                tabs.push(TabInfo { id, label, panel_id, selected });
            }
        }
        if !tabs.is_empty() {
            groups.push(TabGroup { tabs });
        }
    }

    for child in &node.children {
        find_tab_groups(child, groups);
    }
}
