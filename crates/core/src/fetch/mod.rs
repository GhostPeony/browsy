//! HTTP fetching and real page loading.
//! Gated behind the "fetch" feature flag.

use crate::output::SpatialDom;
use reqwest::blocking::Client;
use url::Url;

/// Configuration for page fetching.
pub struct FetchConfig {
    /// Viewport width in pixels.
    pub viewport_width: f32,
    /// Viewport height in pixels.
    pub viewport_height: f32,
    /// User-Agent header.
    pub user_agent: String,
    /// Request timeout in seconds.
    pub timeout_secs: u64,
    /// Whether to fetch external CSS stylesheets linked via <link> tags.
    pub fetch_css: bool,
}

impl Default for FetchConfig {
    fn default() -> Self {
        Self {
            viewport_width: 1920.0,
            viewport_height: 1080.0,
            user_agent: format!(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36"
            ),
            timeout_secs: 30,
            fetch_css: true,
        }
    }
}

/// Fetch a URL and parse it into a SpatialDom.
pub fn fetch(url: &str, config: &FetchConfig) -> Result<SpatialDom, FetchError> {
    let parsed_url = Url::parse(url).map_err(|e| FetchError::InvalidUrl(e.to_string()))?;

    let client = Client::builder()
        .user_agent(&config.user_agent)
        .timeout(std::time::Duration::from_secs(config.timeout_secs))
        .build()
        .map_err(|e| FetchError::Network(e.to_string()))?;

    // Fetch HTML
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

    // Parse DOM
    let dom_tree = crate::dom::parse_html(&html);

    // If CSS fetching is enabled, fetch external stylesheets
    let external_css = if config.fetch_css {
        fetch_external_css(&dom_tree, &parsed_url, &client)
    } else {
        String::new()
    };

    // Compute styles with both embedded and external CSS
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

/// Fetch external CSS stylesheets referenced by <link> tags.
fn fetch_external_css(
    dom: &crate::dom::DomNode,
    base_url: &Url,
    client: &Client,
) -> String {
    let mut css = String::new();
    collect_link_hrefs(dom, &mut |href| {
        // Resolve relative URLs
        if let Ok(css_url) = base_url.join(href) {
            if let Ok(resp) = client.get(css_url.as_str()).send() {
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

/// Recursively find <link rel="stylesheet"> tags and collect their hrefs.
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

#[derive(Debug)]
pub enum FetchError {
    InvalidUrl(String),
    Network(String),
    HttpError(u16),
}

impl std::fmt::Display for FetchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FetchError::InvalidUrl(e) => write!(f, "Invalid URL: {}", e),
            FetchError::Network(e) => write!(f, "Network error: {}", e),
            FetchError::HttpError(code) => write!(f, "HTTP error: {}", code),
        }
    }
}

impl std::error::Error for FetchError {}
