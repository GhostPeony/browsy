//! HTTP fetching, session management, and agent actions.
//! Gated behind the "fetch" feature flag.

mod session;

pub use session::{Session, SessionConfig, SearchEngine, SearchResult, SearchPage, InputPurpose, extract_search_results_from, extract_google_results_from};

use crate::output::SpatialDom;
use reqwest::blocking::Client;
use reqwest::redirect::Policy;
use serde::Serialize;
use std::io::Read;
use std::net::{Ipv4Addr, Ipv6Addr};
use url::Url;

/// Legacy standalone fetch — use Session for new code.
pub fn fetch(url: &str, config: &FetchConfig) -> Result<SpatialDom, FetchError> {
    let parsed_url = Url::parse(url).map_err(|e| FetchError::InvalidUrl(e.to_string()))?;
    if !is_url_allowed(&parsed_url, config.allow_private_network, config.allow_non_http) {
        return Err(FetchError::BlockedUrl(parsed_url.to_string()));
    }

    let allow_private = config.allow_private_network;
    let allow_non_http = config.allow_non_http;
    let max_redirects = config.max_redirects;
    let client = Client::builder()
        .user_agent(&config.user_agent)
        .timeout(std::time::Duration::from_secs(config.timeout_secs))
        .redirect(Policy::custom(move |attempt| {
            if attempt.previous().len() >= max_redirects {
                return attempt.stop();
            }
            if !is_url_allowed(attempt.url(), allow_private, allow_non_http) {
                return attempt.stop();
            }
            attempt.follow()
        }))
        .build()
        .map_err(|e| FetchError::Network(e.to_string()))?;

    let response = client
        .get(parsed_url.as_str())
        .send()
        .map_err(|e| FetchError::Network(e.to_string()))?;

    let status = response.status();
    if !status.is_success() {
        return Err(FetchError::HttpError(status.as_u16()));
    }

    let html = read_response_text_limited(response, config.max_response_bytes)?;

    let dom_tree = crate::dom::parse_html(&html);
    let external_css = if config.fetch_css {
        fetch_external_css(
            &dom_tree,
            &parsed_url,
            &client,
            &config.blocked_patterns,
            config.max_css_bytes_total,
            config.max_css_bytes_per_file,
            config.allow_private_network,
            config.allow_non_http,
        )
    } else {
        String::new()
    };

    let styled = if external_css.is_empty() {
        crate::css::compute_styles_with_viewport(&dom_tree, config.viewport_width, config.viewport_height)
    } else {
        crate::css::compute_styles_with_external_and_viewport(&dom_tree, &external_css, config.viewport_width, config.viewport_height)
    };

    let laid_out =
        crate::layout::compute_layout(&styled, config.viewport_width, config.viewport_height);
    let mut spatial = crate::output::generate_spatial_dom(
        &laid_out,
        config.viewport_width,
        config.viewport_height,
    );
    spatial.url = url.to_string();
    crate::output::resolve_urls(&mut spatial, url);
    Ok(spatial)
}

pub struct FetchConfig {
    pub viewport_width: f32,
    pub viewport_height: f32,
    pub user_agent: String,
    pub timeout_secs: u64,
    pub fetch_css: bool,
    pub max_response_bytes: usize,
    pub max_css_bytes_total: usize,
    pub max_css_bytes_per_file: usize,
    pub max_redirects: usize,
    pub allow_private_network: bool,
    pub allow_non_http: bool,
    pub blocked_patterns: Vec<String>,
}

impl Default for FetchConfig {
    fn default() -> Self {
        Self {
            viewport_width: 1920.0,
            viewport_height: 1080.0,
            user_agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36".to_string(),
            timeout_secs: 30,
            fetch_css: true,
            max_response_bytes: 5 * 1024 * 1024,
            max_css_bytes_total: 1024 * 1024,
            max_css_bytes_per_file: 256 * 1024,
            max_redirects: 10,
            allow_private_network: false,
            allow_non_http: false,
            blocked_patterns: default_blocked_patterns(),
        }
    }
}

#[derive(Debug, Serialize)]
pub enum FetchError {
    InvalidUrl(String),
    BlockedUrl(String),
    Network(String),
    HttpError(u16),
    ActionError(String),
    ResponseTooLarge(u64, usize),
}

impl std::fmt::Display for FetchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FetchError::InvalidUrl(e) => write!(f, "Invalid URL: {}", e),
            FetchError::BlockedUrl(u) => write!(f, "Blocked URL: {}", u),
            FetchError::Network(e) => write!(f, "Network error: {}", e),
            FetchError::HttpError(code) => write!(f, "HTTP error: {}", code),
            FetchError::ActionError(e) => write!(f, "Action error: {}", e),
            FetchError::ResponseTooLarge(found, max) => write!(f, "Response too large: {} bytes (max {})", found, max),
        }
    }
}

impl std::error::Error for FetchError {}

// --- Shared helpers used by both fetch() and Session ---

fn default_blocked_patterns() -> Vec<String> {
    [
        "doubleclick.net", "googlesyndication.com", "googleadservices.com",
        "adservice.google", "ads.facebook", "analytics.google",
        "google-analytics.com", "googletagmanager.com", "facebook.net/en_US/fbevents",
        "hotjar.com", "segment.io", "mixpanel.com", "amplitude.com",
        "fonts.googleapis.com", "fonts.gstatic.com", "use.typekit.net",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}

fn fetch_external_css(
    dom: &crate::dom::DomNode,
    base_url: &Url,
    client: &Client,
    blocked: &[String],
    max_total_bytes: usize,
    max_per_file_bytes: usize,
    allow_private: bool,
    allow_non_http: bool,
) -> String {
    let mut css = String::new();
    let mut remaining = max_total_bytes;
    collect_link_hrefs(dom, &mut |href| {
        if remaining == 0 {
            return;
        }
        if let Ok(css_url) = base_url.join(href) {
            if !is_url_allowed(&css_url, allow_private, allow_non_http) {
                return;
            }
            let url_str = css_url.as_str();
            if blocked.iter().any(|p| url_str.contains(p.as_str())) {
                return;
            }
            if let Ok(resp) = client.get(url_str).send() {
                if resp.status().is_success() {
                    let limit = remaining.min(max_per_file_bytes);
                    if limit == 0 {
                        return;
                    }
                    if let Ok(text) = read_response_text_limited(resp, limit) {
                        remaining = remaining.saturating_sub(text.len());
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

// --- Form extraction helpers ---

pub(crate) struct FormInfo {
    pub action: Option<String>,
    pub method: Option<String>,
    pub fields: Vec<FormField>,
    pub buttons: Vec<FormButton>,
}

pub(crate) struct FormField {
    pub name: Option<String>,
    pub value: Option<String>,
    pub field_type: String,
    /// Whether this field is a checkbox/radio that is checked by default
    pub checked: bool,
}

pub(crate) struct FormButton {
    pub name: Option<String>,
    pub value: Option<String>,
    pub formaction: Option<String>,
    pub text: Option<String>,
}

pub(crate) fn extract_forms(node: &crate::dom::DomNode) -> Vec<FormInfo> {
    let mut forms = Vec::new();
    collect_forms(node, &mut forms);
    forms
}

fn collect_forms(node: &crate::dom::DomNode, forms: &mut Vec<FormInfo>) {
    if node.tag == "form" {
        let action = node.get_attr("action").map(|s| s.to_string());
        let method = node.get_attr("method").map(|s| s.to_string());
        let mut fields = Vec::new();
        let mut buttons = Vec::new();
        collect_form_fields(node, &mut fields, &mut buttons);
        forms.push(FormInfo { action, method, fields, buttons });
    }
    for child in &node.children {
        collect_forms(child, forms);
    }
}

fn collect_form_fields(node: &crate::dom::DomNode, fields: &mut Vec<FormField>, buttons: &mut Vec<FormButton>) {
    match node.tag.as_str() {
        "input" => {
            let field_type = node.get_attr("type").unwrap_or("text").to_string();
            if field_type == "submit" || field_type == "button" || field_type == "image" {
                buttons.push(FormButton {
                    name: node.get_attr("name").map(|s| s.to_string()),
                    value: node.get_attr("value").map(|s| s.to_string()),
                    formaction: node.get_attr("formaction").map(|s| s.to_string()),
                    text: node.get_attr("value").map(|s| s.to_string()),
                });
            } else {
                let checked = (field_type == "checkbox" || field_type == "radio")
                    && node.attributes.contains_key("checked");
                fields.push(FormField {
                    name: node.get_attr("name").map(|s| s.to_string()),
                    value: node.get_attr("value").map(|s| s.to_string()),
                    field_type,
                    checked,
                });
            }
        }
        "button" => {
            buttons.push(FormButton {
                name: node.get_attr("name").map(|s| s.to_string()),
                value: node.get_attr("value").map(|s| s.to_string()),
                formaction: node.get_attr("formaction").map(|s| s.to_string()),
                text: Some(node.text_content()),
            });
        }
        "textarea" => {
            fields.push(FormField {
                name: node.get_attr("name").map(|s| s.to_string()),
                value: Some(node.text_content()),
                field_type: "textarea".to_string(),
                checked: false,
            });
        }
        "select" => {
            let selected_value = find_selected_option(node);
            fields.push(FormField {
                name: node.get_attr("name").map(|s| s.to_string()),
                value: selected_value,
                field_type: "select".to_string(),
                checked: false,
            });
        }
        _ => {}
    }
    for child in &node.children {
        collect_form_fields(child, fields, buttons);
    }
}

/// Find which form index contains a button matching the given button name/value.
/// Falls back to form index 0 if no match found.
pub(crate) fn find_form_index_for_button(
    forms: &[FormInfo],
    button_name: Option<&str>,
    button_text: Option<&str>,
) -> usize {
    for (i, form) in forms.iter().enumerate() {
        for btn in &form.buttons {
            if let Some(name) = button_name {
                if btn.name.as_deref() == Some(name) {
                    return i;
                }
            }
            if let Some(text) = button_text {
                if btn.value.as_deref() == Some(text) {
                    return i;
                }
                if btn.text.as_deref() == Some(text) {
                    return i;
                }
            }
        }
    }
    0
}

fn find_selected_option(node: &crate::dom::DomNode) -> Option<String> {
    for child in &node.children {
        if child.tag == "option" && child.attributes.contains_key("selected") {
            return child.get_attr("value").map(|s| s.to_string());
        }
    }
    for child in &node.children {
        if child.tag == "option" {
            return child.get_attr("value").map(|s| s.to_string());
        }
    }
    None
}

pub(crate) fn read_response_text_limited(
    response: reqwest::blocking::Response,
    max_bytes: usize,
) -> Result<String, FetchError> {
    if let Some(len) = response.content_length() {
        if len > max_bytes as u64 {
            return Err(FetchError::ResponseTooLarge(len, max_bytes));
        }
    }
    let mut buf = Vec::new();
    let mut limited = response.take(max_bytes as u64 + 1);
    limited
        .read_to_end(&mut buf)
        .map_err(|e| FetchError::Network(e.to_string()))?;
    if buf.len() > max_bytes {
        return Err(FetchError::ResponseTooLarge(buf.len() as u64, max_bytes));
    }
    Ok(String::from_utf8_lossy(&buf).to_string())
}

pub(crate) fn is_url_allowed(url: &Url, allow_private: bool, allow_non_http: bool) -> bool {
    if !allow_non_http && !matches!(url.scheme(), "http" | "https") {
        return false;
    }
    if allow_private {
        return true;
    }
    if let Some(host) = url.host_str() {
        if is_local_hostname(host) {
            return false;
        }
    }
    match url.host() {
        Some(url::Host::Ipv4(ip)) => !is_private_ipv4(ip),
        Some(url::Host::Ipv6(ip)) => !is_private_ipv6(ip),
        Some(url::Host::Domain(_)) => true,
        None => false,
    }
}

fn is_local_hostname(host: &str) -> bool {
    let h = host.to_lowercase();
    h == "localhost" || h.ends_with(".localhost") || h.ends_with(".local")
}

fn is_private_ipv4(ip: Ipv4Addr) -> bool {
    let oct = ip.octets();
    match oct {
        [10, ..] => true,
        [127, ..] => true,
        [169, 254, ..] => true,
        [172, b, ..] if (16..=31).contains(&b) => true,
        [192, 168, ..] => true,
        [100, b, ..] if (64..=127).contains(&b) => true,
        [0, ..] => true,
        [224..=239, ..] => true,
        [240..=255, ..] => true,
        [192, 0, 2, ..] => true,
        [198, 51, 100, ..] => true,
        [203, 0, 113, ..] => true,
        _ => false,
    }
}

fn is_private_ipv6(ip: Ipv6Addr) -> bool {
    if ip.is_loopback() || ip.is_unspecified() {
        return true;
    }
    let segments = ip.segments();
    let first = segments[0];
    (first & 0xfe00) == 0xfc00
        || (first & 0xffc0) == 0xfe80
        || (first & 0xff00) == 0xff00
}
