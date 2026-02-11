//! HTTP fetching, session management, and agent actions.
//! Gated behind the "fetch" feature flag.

use crate::output::{SpatialDom, SpatialElement};
use reqwest::blocking::Client;
use std::collections::HashMap;
use std::sync::Arc;
use url::Url;

/// Configuration for a browsy session.
pub struct SessionConfig {
    /// Viewport width in pixels.
    pub viewport_width: f32,
    /// Viewport height in pixels.
    pub viewport_height: f32,
    /// User-Agent header.
    pub user_agent: String,
    /// Request timeout in seconds.
    pub timeout_secs: u64,
    /// Whether to fetch external CSS stylesheets.
    pub fetch_css: bool,
    /// Blocked URL patterns (substrings). Requests matching these are skipped.
    pub blocked_patterns: Vec<String>,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            viewport_width: 1920.0,
            viewport_height: 1080.0,
            user_agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36".to_string(),
            timeout_secs: 30,
            fetch_css: true,
            blocked_patterns: default_blocked_patterns(),
        }
    }
}

/// Default patterns to block (ads, trackers, fonts, large images).
fn default_blocked_patterns() -> Vec<String> {
    [
        // Ad networks
        "doubleclick.net", "googlesyndication.com", "googleadservices.com",
        "adservice.google", "ads.facebook", "analytics.google",
        // Trackers
        "google-analytics.com", "googletagmanager.com", "facebook.net/en_US/fbevents",
        "hotjar.com", "segment.io", "mixpanel.com", "amplitude.com",
        // Fonts (unnecessary for layout)
        "fonts.googleapis.com", "fonts.gstatic.com", "use.typekit.net",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}

/// A browsing session with cookie persistence and page state.
pub struct Session {
    client: Client,
    config: SessionConfig,
    /// Current page URL.
    current_url: Option<Url>,
    /// Current page DOM.
    current_dom: Option<SpatialDom>,
    /// Previous page DOM (for delta output).
    previous_dom: Option<SpatialDom>,
    /// Navigation history (URLs).
    history: Vec<String>,
    /// Current form field values (element_id -> value).
    form_values: HashMap<u32, String>,
    /// Raw HTML of current page (for form extraction).
    current_html: Option<String>,
}

impl Session {
    /// Create a new session with default config.
    pub fn new() -> Result<Self, FetchError> {
        Self::with_config(SessionConfig::default())
    }

    /// Create a new session with custom config.
    pub fn with_config(config: SessionConfig) -> Result<Self, FetchError> {
        let cookie_store = Arc::new(reqwest::cookie::Jar::default());
        let client = Client::builder()
            .user_agent(&config.user_agent)
            .timeout(std::time::Duration::from_secs(config.timeout_secs))
            .cookie_provider(cookie_store)
            .build()
            .map_err(|e| FetchError::Network(e.to_string()))?;

        Ok(Self {
            client,
            config,
            current_url: None,
            current_dom: None,
            previous_dom: None,
            history: Vec::new(),
            form_values: HashMap::new(),
            current_html: None,
        })
    }

    /// Navigate to a URL and return the Spatial DOM.
    pub fn goto(&mut self, url: &str) -> Result<&SpatialDom, FetchError> {
        let parsed_url = Url::parse(url).map_err(|e| FetchError::InvalidUrl(e.to_string()))?;

        let response = self
            .client
            .get(parsed_url.as_str())
            .send()
            .map_err(|e| FetchError::Network(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            return Err(FetchError::HttpError(status.as_u16()));
        }

        let html = response
            .text()
            .map_err(|e| FetchError::Network(e.to_string()))?;

        self.load_html(&html, url)?;

        // Track history
        self.history.push(url.to_string());
        self.current_url = Some(parsed_url);

        Ok(self.current_dom.as_ref().unwrap())
    }

    /// Load HTML content directly (without fetching).
    pub fn load_html(&mut self, html: &str, url: &str) -> Result<&SpatialDom, FetchError> {
        let dom_tree = crate::dom::parse_html(html);

        // Fetch external CSS if enabled
        let external_css = if self.config.fetch_css {
            if let Ok(base_url) = Url::parse(url) {
                fetch_external_css(&dom_tree, &base_url, &self.client, &self.config.blocked_patterns)
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        let styled = if external_css.is_empty() {
            crate::css::compute_styles(&dom_tree)
        } else {
            crate::css::compute_styles_with_external(&dom_tree, &external_css)
        };

        let laid_out = crate::layout::compute_layout(
            &styled,
            self.config.viewport_width,
            self.config.viewport_height,
        );
        let mut spatial = crate::output::generate_spatial_dom(
            &laid_out,
            self.config.viewport_width,
            self.config.viewport_height,
        );
        spatial.url = url.to_string();

        // Shift previous DOM
        self.previous_dom = self.current_dom.take();
        self.current_dom = Some(spatial);
        self.current_html = Some(html.to_string());
        self.form_values.clear();

        Ok(self.current_dom.as_ref().unwrap())
    }

    /// Load from a pre-parsed DOM tree (used after JS actions modify the DOM).
    fn load_html_from_dom(&mut self, dom_tree: crate::dom::DomNode, url: &str) -> Result<&SpatialDom, FetchError> {
        let styled = crate::css::compute_styles(&dom_tree);
        let laid_out = crate::layout::compute_layout(
            &styled,
            self.config.viewport_width,
            self.config.viewport_height,
        );
        let mut spatial = crate::output::generate_spatial_dom(
            &laid_out,
            self.config.viewport_width,
            self.config.viewport_height,
        );
        spatial.url = url.to_string();

        self.previous_dom = self.current_dom.take();
        self.current_dom = Some(spatial);
        self.form_values.clear();

        Ok(self.current_dom.as_ref().unwrap())
    }

    /// Get the current page DOM.
    pub fn dom(&self) -> Option<&SpatialDom> {
        self.current_dom.as_ref()
    }

    /// Get the delta between current and previous page.
    pub fn delta(&self) -> Option<crate::output::DeltaDom> {
        match (&self.previous_dom, &self.current_dom) {
            (Some(old), Some(new)) => Some(crate::output::diff(old, new)),
            _ => None,
        }
    }

    /// Get detected JS behaviors for the current page.
    pub fn behaviors(&self) -> Vec<crate::js::JsBehavior> {
        self.current_html
            .as_ref()
            .map(|html| {
                let dom_tree = crate::dom::parse_html(html);
                crate::js::detect_behaviors(&dom_tree)
            })
            .unwrap_or_default()
    }

    /// Find an element by ID in the current DOM.
    pub fn element(&self, id: u32) -> Option<&SpatialElement> {
        self.current_dom
            .as_ref()
            .and_then(|dom| dom.els.iter().find(|e| e.id == id))
    }

    /// Find elements by text content.
    pub fn find_by_text(&self, text: &str) -> Vec<&SpatialElement> {
        self.current_dom
            .as_ref()
            .map(|dom| {
                dom.els
                    .iter()
                    .filter(|e| e.text.as_deref().map(|t| t.contains(text)).unwrap_or(false))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Find elements by role.
    pub fn find_by_role(&self, role: &str) -> Vec<&SpatialElement> {
        self.current_dom
            .as_ref()
            .map(|dom| {
                dom.els
                    .iter()
                    .filter(|e| e.role.as_deref() == Some(role))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Click an element by ID. For links, navigates to href. For buttons
    /// inside forms, submits the form. Returns the new DOM.
    pub fn click(&mut self, id: u32) -> Result<&SpatialDom, FetchError> {
        let (tag, href, is_submit) = {
            let el = self.element(id).ok_or_else(|| {
                FetchError::ActionError(format!("Element {} not found", id))
            })?;
            let is_submit = el.tag == "button"
                || (el.tag == "input"
                    && el.input_type.as_deref().map(|t| t == "submit").unwrap_or(false));
            (el.tag.clone(), el.href.clone(), is_submit)
        };

        if tag == "a" {
            if let Some(href) = href {
                // Resolve relative URL
                let target = if let Some(ref base) = self.current_url {
                    base.join(&href)
                        .map(|u| u.to_string())
                        .unwrap_or(href)
                } else {
                    href
                };
                return self.goto(&target);
            }
        }

        // Check for JS behaviors BEFORE form submit (onclick takes priority)
        if let Some(html) = &self.current_html {
            let dom_tree = crate::dom::parse_html(html);
            let behaviors = crate::js::detect_behaviors(&dom_tree);
            if let Some(behavior) = behaviors.iter().find(|b| b.trigger_id == id) {
                match &behavior.action {
                    crate::js::JsAction::Navigate { url } => {
                        let target = if let Some(ref base) = self.current_url {
                            base.join(url)
                                .map(|u| u.to_string())
                                .unwrap_or_else(|_| url.clone())
                        } else {
                            url.clone()
                        };
                        return self.goto(&target);
                    }
                    action => {
                        // Apply the JS action to the DOM and re-render
                        let modified = crate::js::apply_action(&dom_tree, action);
                        let html_url = self.current_url.as_ref()
                            .map(|u| u.to_string())
                            .unwrap_or_default();
                        return self.load_html_from_dom(modified, &html_url);
                    }
                }
            }
        }

        // Form submission (after JS check, so onclick takes priority)
        if is_submit {
            return self.submit_form(id);
        }

        // For other elements, return current DOM
        self.current_dom
            .as_ref()
            .ok_or_else(|| FetchError::ActionError("No page loaded".to_string()))
    }

    /// Type text into an input/textarea field.
    pub fn type_text(&mut self, id: u32, text: &str) -> Result<(), FetchError> {
        let el = self.element(id).ok_or_else(|| {
            FetchError::ActionError(format!("Element {} not found", id))
        })?;

        if el.tag != "input" && el.tag != "textarea" {
            return Err(FetchError::ActionError(format!(
                "Element {} ({}) is not a text input",
                id, el.tag
            )));
        }

        self.form_values.insert(id, text.to_string());
        Ok(())
    }

    /// Select an option in a select element.
    pub fn select(&mut self, id: u32, value: &str) -> Result<(), FetchError> {
        let el = self.element(id).ok_or_else(|| {
            FetchError::ActionError(format!("Element {} not found", id))
        })?;

        if el.tag != "select" {
            return Err(FetchError::ActionError(format!(
                "Element {} ({}) is not a select",
                id, el.tag
            )));
        }

        self.form_values.insert(id, value.to_string());
        Ok(())
    }

    /// Go back in navigation history.
    pub fn back(&mut self) -> Result<&SpatialDom, FetchError> {
        if self.history.len() < 2 {
            return Err(FetchError::ActionError("No history to go back to".to_string()));
        }
        // Remove current
        self.history.pop();
        // Navigate to previous
        let prev = self.history.last().unwrap().clone();
        self.goto(&prev)
    }

    /// Get current URL.
    pub fn url(&self) -> Option<&str> {
        self.current_url.as_ref().map(|u| u.as_str())
    }

    /// Submit a form by finding the enclosing form and POSTing its values.
    fn submit_form(&mut self, button_id: u32) -> Result<&SpatialDom, FetchError> {
        let html = self.current_html.as_ref().ok_or_else(|| {
            FetchError::ActionError("No page loaded".to_string())
        })?.clone();

        let base_url = self.current_url.clone().ok_or_else(|| {
            FetchError::ActionError("No URL loaded".to_string())
        })?;

        // Parse the HTML to find forms and their fields
        let dom_tree = crate::dom::parse_html(&html);
        let forms = extract_forms(&dom_tree);

        // Find which form contains our button (by matching text/position)
        let _button_el = self.element(button_id).ok_or_else(|| {
            FetchError::ActionError(format!("Button {} not found", button_id))
        })?.clone();

        // Use the first form (most common case), or the form with action
        let form = forms.first().ok_or_else(|| {
            FetchError::ActionError("No form found on page".to_string())
        })?;

        // Build form data
        let mut form_data: Vec<(String, String)> = Vec::new();

        // Add form fields from DOM defaults
        for field in &form.fields {
            if let Some(name) = &field.name {
                let value = self
                    .form_values
                    .values()
                    .find(|_| {
                        // Match by placeholder/type
                        false
                    })
                    .cloned()
                    .or(field.value.clone())
                    .unwrap_or_default();
                form_data.push((name.clone(), value));
            }
        }

        // Override with typed values — match by element order
        let dom = self.current_dom.as_ref().unwrap();
        let inputs: Vec<&SpatialElement> = dom
            .els
            .iter()
            .filter(|e| e.tag == "input" || e.tag == "textarea" || e.tag == "select")
            .collect();

        for (i, input) in inputs.iter().enumerate() {
            if let Some(typed_value) = self.form_values.get(&input.id) {
                if i < form_data.len() {
                    form_data[i].1 = typed_value.clone();
                } else if let Some(name) = get_input_name_by_index(&dom_tree, i) {
                    form_data.push((name, typed_value.clone()));
                }
            }
        }

        // Determine form method and action
        let method = form.method.as_deref().unwrap_or("get").to_lowercase();
        let action = form.action.as_deref().unwrap_or("");
        let target_url = base_url
            .join(action)
            .map_err(|e| FetchError::InvalidUrl(e.to_string()))?;

        // Submit
        let response = if method == "post" {
            self.client
                .post(target_url.as_str())
                .form(&form_data)
                .send()
                .map_err(|e| FetchError::Network(e.to_string()))?
        } else {
            self.client
                .get(target_url.as_str())
                .query(&form_data)
                .send()
                .map_err(|e| FetchError::Network(e.to_string()))?
        };

        let new_url = response.url().to_string();
        let html = response
            .text()
            .map_err(|e| FetchError::Network(e.to_string()))?;

        self.history.push(new_url.clone());
        self.current_url = Some(Url::parse(&new_url).unwrap_or(target_url));
        self.load_html(&html, &new_url)
    }
}

// Legacy standalone function for backward compatibility
/// Fetch a URL and parse it into a SpatialDom.
pub fn fetch(url: &str, config: &FetchConfig) -> Result<SpatialDom, FetchError> {
    let client = Client::builder()
        .user_agent(&config.user_agent)
        .timeout(std::time::Duration::from_secs(config.timeout_secs))
        .build()
        .map_err(|e| FetchError::Network(e.to_string()))?;

    let parsed_url = Url::parse(url).map_err(|e| FetchError::InvalidUrl(e.to_string()))?;

    let response = client
        .get(parsed_url.as_str())
        .send()
        .map_err(|e| FetchError::Network(e.to_string()))?;

    let status = response.status();
    if !status.is_success() {
        return Err(FetchError::HttpError(status.as_u16()));
    }

    let html = response
        .text()
        .map_err(|e| FetchError::Network(e.to_string()))?;

    let dom_tree = crate::dom::parse_html(&html);
    let external_css = if config.fetch_css {
        fetch_external_css(&dom_tree, &parsed_url, &client, &[])
    } else {
        String::new()
    };

    let styled = if external_css.is_empty() {
        crate::css::compute_styles(&dom_tree)
    } else {
        crate::css::compute_styles_with_external(&dom_tree, &external_css)
    };

    let laid_out =
        crate::layout::compute_layout(&styled, config.viewport_width, config.viewport_height);
    let mut spatial = crate::output::generate_spatial_dom(
        &laid_out,
        config.viewport_width,
        config.viewport_height,
    );
    spatial.url = url.to_string();
    Ok(spatial)
}

/// Legacy config type (use SessionConfig for new code).
pub struct FetchConfig {
    pub viewport_width: f32,
    pub viewport_height: f32,
    pub user_agent: String,
    pub timeout_secs: u64,
    pub fetch_css: bool,
}

impl Default for FetchConfig {
    fn default() -> Self {
        Self {
            viewport_width: 1920.0,
            viewport_height: 1080.0,
            user_agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36".to_string(),
            timeout_secs: 30,
            fetch_css: true,
        }
    }
}

/// Fetch external CSS stylesheets referenced by <link> tags.
fn fetch_external_css(
    dom: &crate::dom::DomNode,
    base_url: &Url,
    client: &Client,
    blocked: &[String],
) -> String {
    let mut css = String::new();
    collect_link_hrefs(dom, &mut |href| {
        if let Ok(css_url) = base_url.join(href) {
            let url_str = css_url.as_str();
            // Check against blocked patterns
            if blocked.iter().any(|p| url_str.contains(p.as_str())) {
                return;
            }
            if let Ok(resp) = client.get(url_str).send() {
                if resp.status().is_success() {
                    if let Ok(text) = resp.text() {
                        css.push_str(&text);
                        css.push('\n');
                    }
                }
            }
        }
    });
    css
}

fn collect_link_hrefs(node: &crate::dom::DomNode, callback: &mut dyn FnMut(&str)) {
    if node.tag == "link" {
        let is_stylesheet = node
            .get_attr("rel")
            .map(|r| r.to_lowercase().contains("stylesheet"))
            .unwrap_or(false);
        if is_stylesheet {
            if let Some(href) = node.get_attr("href") {
                callback(href);
            }
        }
    }
    for child in &node.children {
        collect_link_hrefs(child, callback);
    }
}

/// A form extracted from the DOM.
struct FormInfo {
    action: Option<String>,
    method: Option<String>,
    fields: Vec<FormField>,
}

struct FormField {
    name: Option<String>,
    value: Option<String>,
    #[allow(dead_code)]
    field_type: String,
}

/// Extract forms from the DOM tree.
fn extract_forms(node: &crate::dom::DomNode) -> Vec<FormInfo> {
    let mut forms = Vec::new();
    collect_forms(node, &mut forms);
    forms
}

fn collect_forms(node: &crate::dom::DomNode, forms: &mut Vec<FormInfo>) {
    if node.tag == "form" {
        let action = node.get_attr("action").map(|s| s.to_string());
        let method = node.get_attr("method").map(|s| s.to_string());
        let mut fields = Vec::new();
        collect_form_fields(node, &mut fields);
        forms.push(FormInfo {
            action,
            method,
            fields,
        });
    }
    for child in &node.children {
        collect_forms(child, forms);
    }
}

fn collect_form_fields(node: &crate::dom::DomNode, fields: &mut Vec<FormField>) {
    match node.tag.as_str() {
        "input" => {
            let field_type = node.get_attr("type").unwrap_or("text").to_string();
            // Skip submit/button/hidden types for value collection
            if field_type != "submit" && field_type != "button" {
                fields.push(FormField {
                    name: node.get_attr("name").map(|s| s.to_string()),
                    value: node.get_attr("value").map(|s| s.to_string()),
                    field_type,
                });
            }
        }
        "textarea" => {
            fields.push(FormField {
                name: node.get_attr("name").map(|s| s.to_string()),
                value: Some(node.text_content()),
                field_type: "textarea".to_string(),
            });
        }
        "select" => {
            // Find selected option
            let selected_value = find_selected_option(node);
            fields.push(FormField {
                name: node.get_attr("name").map(|s| s.to_string()),
                value: selected_value,
                field_type: "select".to_string(),
            });
        }
        _ => {}
    }
    for child in &node.children {
        collect_form_fields(child, fields);
    }
}

fn find_selected_option(node: &crate::dom::DomNode) -> Option<String> {
    for child in &node.children {
        if child.tag == "option" {
            if child.attributes.contains_key("selected") {
                return child.get_attr("value").map(|s| s.to_string());
            }
        }
    }
    // Default: first option
    for child in &node.children {
        if child.tag == "option" {
            return child.get_attr("value").map(|s| s.to_string());
        }
    }
    None
}

fn get_input_name_by_index(node: &crate::dom::DomNode, target_index: usize) -> Option<String> {
    let mut names = Vec::new();
    collect_input_names(node, &mut names);
    names.get(target_index).cloned()
}

fn collect_input_names(node: &crate::dom::DomNode, names: &mut Vec<String>) {
    if (node.tag == "input" || node.tag == "textarea" || node.tag == "select")
        && node.get_attr("type").unwrap_or("text") != "submit"
    {
        names.push(node.get_attr("name").unwrap_or("").to_string());
    }
    for child in &node.children {
        collect_input_names(child, names);
    }
}

#[derive(Debug)]
pub enum FetchError {
    InvalidUrl(String),
    Network(String),
    HttpError(u16),
    ActionError(String),
}

impl std::fmt::Display for FetchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FetchError::InvalidUrl(e) => write!(f, "Invalid URL: {}", e),
            FetchError::Network(e) => write!(f, "Network error: {}", e),
            FetchError::HttpError(code) => write!(f, "HTTP error: {}", code),
            FetchError::ActionError(e) => write!(f, "Action error: {}", e),
        }
    }
}

impl std::error::Error for FetchError {}
