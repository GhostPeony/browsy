//! Browsing session with cookie persistence, navigation, and agent actions.

use super::{
    FetchError,
    FetchConfig,
    fetch_external_css,
    extract_forms,
    find_form_index_for_button,
    is_url_allowed,
    read_response_text_limited,
    fetch_html_with_retry,
};
use crate::output::{CaptchaInfo, PageType, SpatialDom, SpatialElement, SuggestedAction};
use reqwest::blocking::Client;
use reqwest::redirect::Policy;
use reqwest::header::{RETRY_AFTER, USER_AGENT};
use serde::{Serialize, Deserialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use url::Url;

/// Configuration for a browsy session.
pub struct SessionConfig {
    pub viewport_width: f32,
    pub viewport_height: f32,
    pub user_agent: String,
    pub timeout_secs: u64,
    pub fetch_css: bool,
    pub blocked_patterns: Vec<String>,
    pub max_response_bytes: usize,
    pub max_css_bytes_total: usize,
    pub max_css_bytes_per_file: usize,
    pub max_redirects: usize,
    pub allow_private_network: bool,
    pub allow_non_http: bool,
    pub retry_attempts: usize,
    pub retry_delay_ms: u64,
    pub retry_on_blocked: bool,
    pub retry_user_agents: Vec<String>,
}

impl Default for SessionConfig {
    fn default() -> Self {
        let fetch = FetchConfig::default();
        Self {
            viewport_width: 1920.0,
            viewport_height: 1080.0,
            user_agent: fetch.user_agent,
            timeout_secs: 30,
            fetch_css: true,
            blocked_patterns: super::default_blocked_patterns(),
            max_response_bytes: fetch.max_response_bytes,
            max_css_bytes_total: fetch.max_css_bytes_total,
            max_css_bytes_per_file: fetch.max_css_bytes_per_file,
            max_redirects: fetch.max_redirects,
            allow_private_network: fetch.allow_private_network,
            allow_non_http: fetch.allow_non_http,
            retry_attempts: fetch.retry_attempts,
            retry_delay_ms: fetch.retry_delay_ms,
            retry_on_blocked: fetch.retry_on_blocked,
            retry_user_agents: fetch.retry_user_agents,
        }
    }
}

/// Semantic purpose of an input field.
#[derive(Debug, Clone, PartialEq)]
pub enum InputPurpose {
    Password,
    Email,
    Username,
    VerificationCode,
    Search,
    Phone,
}

/// A browsing session with cookie persistence and page state.
pub struct Session {
    client: Client,
    config: SessionConfig,
    current_url: Option<Url>,
    current_dom: Option<SpatialDom>,
    previous_dom: Option<SpatialDom>,
    history: Vec<String>,
    form_values: HashMap<u32, String>,
    checked_ids: HashSet<u32>,
    unchecked_ids: HashSet<u32>,
    current_html: Option<String>,
}

impl Session {
    pub fn new() -> Result<Self, FetchError> {
        Self::with_config(SessionConfig::default())
    }

    pub fn with_config(config: SessionConfig) -> Result<Self, FetchError> {
        let cookie_store = Arc::new(reqwest::cookie::Jar::default());
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
            checked_ids: HashSet::new(),
            unchecked_ids: HashSet::new(),
            current_html: None,
        })
    }

    /// Navigate to a URL and return the Spatial DOM.
    pub fn goto(&mut self, url: &str) -> Result<SpatialDom, FetchError> {
        let parsed_url = Url::parse(url).map_err(|e| FetchError::InvalidUrl(e.to_string()))?;
        if !is_url_allowed(&parsed_url, self.config.allow_private_network, self.config.allow_non_http) {
            return Err(FetchError::BlockedUrl(parsed_url.to_string()));
        }

        let html = self.fetch_html_with_retry(&parsed_url)?;

        let dom = self.load_html(&html, url)?;
        self.history.push(url.to_string());
        self.current_url = Some(parsed_url);

        Ok(dom)
    }

    /// Load HTML content directly (without fetching).
    pub fn load_html(&mut self, html: &str, url: &str) -> Result<SpatialDom, FetchError> {
        let dom_tree = crate::dom::parse_html(html);

        let external_css = if self.config.fetch_css {
            if let Ok(base_url) = Url::parse(url) {
                fetch_external_css(
                    &dom_tree,
                    &base_url,
                    &self.client,
                    &self.config.blocked_patterns,
                    self.config.max_css_bytes_total,
                    self.config.max_css_bytes_per_file,
                    self.config.allow_private_network,
                    self.config.allow_non_http,
                )
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        let styled = if external_css.is_empty() {
            crate::css::compute_styles_with_viewport(&dom_tree, self.config.viewport_width, self.config.viewport_height)
        } else {
            crate::css::compute_styles_with_external_and_viewport(&dom_tree, &external_css, self.config.viewport_width, self.config.viewport_height)
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
        crate::output::resolve_urls(&mut spatial, url);

        let result = spatial.clone();
        self.previous_dom = self.current_dom.take();
        self.current_dom = Some(spatial);
        self.current_html = Some(html.to_string());
        self.form_values.clear();
        self.checked_ids.clear();
        self.unchecked_ids.clear();

        Ok(result)
    }

    /// Load from a pre-parsed DOM tree (used after JS actions modify the DOM).
    fn load_html_from_dom(&mut self, dom_tree: crate::dom::DomNode, url: &str) -> Result<SpatialDom, FetchError> {
        let styled = crate::css::compute_styles_with_viewport(&dom_tree, self.config.viewport_width, self.config.viewport_height);
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

        let result = spatial.clone();
        self.previous_dom = self.current_dom.take();
        self.current_dom = Some(spatial);
        self.form_values.clear();
        self.checked_ids.clear();
        self.unchecked_ids.clear();

        Ok(result)
    }

    /// Return the current DOM with form state (typed values, checked state) overlaid.
    pub fn dom(&self) -> Option<SpatialDom> {
        let dom = self.current_dom.as_ref()?;
        let mut result = dom.clone();

        // Overlay typed form values
        for el in &mut result.els {
            if let Some(val) = self.form_values.get(&el.id) {
                el.val = Some(val.clone());
            }
            // Overlay checked/unchecked state
            if self.checked_ids.contains(&el.id) {
                el.checked = Some(true);
            } else if self.unchecked_ids.contains(&el.id) {
                el.checked = Some(false);
            }
        }

        Some(result)
    }

    /// Return a reference to the raw DOM without form state overlay.
    pub fn dom_ref(&self) -> Option<&SpatialDom> {
        self.current_dom.as_ref()
    }

    pub fn delta(&self) -> Option<crate::output::DeltaDom> {
        match (&self.previous_dom, &self.current_dom) {
            (Some(old), Some(new)) => Some(crate::output::diff(old, new)),
            _ => None,
        }
    }

    pub fn behaviors(&self) -> Vec<crate::js::JsBehavior> {
        self.current_html
            .as_ref()
            .map(|html| {
                let dom_tree = crate::dom::parse_html(html);
                crate::js::detect_behaviors(&dom_tree)
            })
            .unwrap_or_default()
    }

    pub fn element(&self, id: u32) -> Option<&SpatialElement> {
        self.current_dom.as_ref().and_then(|dom| dom.get(id))
    }

    /// Return an owned copy of an element by ID (for FFI consumers).
    pub fn element_owned(&self, id: u32) -> Option<SpatialElement> {
        self.element(id).cloned()
    }

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

    fn resolve_url(&self, relative: &str) -> String {
        match &self.current_url {
            Some(base) => base.join(relative).map(|u| u.to_string()).unwrap_or_else(|_| relative.to_string()),
            None => relative.to_string(),
        }
    }

    /// Click an element. Links navigate, buttons submit forms, JS behaviors are simulated.
    pub fn click(&mut self, id: u32) -> Result<SpatialDom, FetchError> {
        let (tag, href, is_submit) = {
            let el = self.element(id).ok_or_else(|| {
                FetchError::ActionError(format!("Element {} not found", id))
            })?;
            let is_submit = el.tag == "button"
                || (el.tag == "input" && el.input_type.as_deref() == Some("submit"));
            (el.tag.clone(), el.href.clone(), is_submit)
        };

        if tag == "a" {
            if let Some(href) = href {
                let trimmed = href.trim();
                let lower = trimmed.to_lowercase();
                if lower.starts_with('#')
                    || lower.starts_with("javascript:")
                    || lower.starts_with("mailto:")
                    || lower.starts_with("tel:")
                    || lower.starts_with("data:")
                {
                    return self.current_dom
                        .as_ref()
                        .cloned()
                        .ok_or_else(|| FetchError::ActionError("No page loaded".to_string()));
                }
                let target = self.resolve_url(trimmed);
                return self.goto(&target);
            }
        }

        // Check JS behaviors before form submit
        if let Some(html) = &self.current_html {
            let dom_tree = crate::dom::parse_html(html);
            let behaviors = crate::js::detect_behaviors(&dom_tree);
            if let Some(behavior) = behaviors.iter().find(|b| b.trigger_id == id) {
                match &behavior.action {
                    crate::js::JsAction::Navigate { url } => {
                        let target = self.resolve_url(url);
                        return self.goto(&target);
                    }
                    action => {
                        let modified = crate::js::apply_action(&dom_tree, action);
                        let html_url = self.current_url.as_ref()
                            .map(|u| u.to_string())
                            .unwrap_or_default();
                        return self.load_html_from_dom(modified, &html_url);
                    }
                }
            }
        }

        if is_submit {
            return self.submit_form(id);
        }

        self.current_dom
            .as_ref()
            .cloned()
            .ok_or_else(|| FetchError::ActionError("No page loaded".to_string()))
    }

    pub fn type_text(&mut self, id: u32, text: &str) -> Result<(), FetchError> {
        let el = self.element(id).ok_or_else(|| {
            FetchError::ActionError(format!("Element {} not found", id))
        })?;
        if el.tag != "input" && el.tag != "textarea" {
            return Err(FetchError::ActionError(format!(
                "Element {} ({}) is not a text input", id, el.tag
            )));
        }
        self.form_values.insert(id, text.to_string());
        Ok(())
    }

    fn require_checkable(&self, id: u32) -> Result<&SpatialElement, FetchError> {
        let el = self.element(id).ok_or_else(|| {
            FetchError::ActionError(format!("Element {} not found", id))
        })?;
        let is_checkable = el.input_type.as_deref() == Some("checkbox")
            || el.input_type.as_deref() == Some("radio");
        if !is_checkable {
            return Err(FetchError::ActionError(format!(
                "Element {} is not a checkbox or radio", id
            )));
        }
        Ok(el)
    }

    /// Check a checkbox or radio button.
    pub fn check(&mut self, id: u32) -> Result<(), FetchError> {
        self.require_checkable(id)?;
        self.checked_ids.insert(id);
        self.unchecked_ids.remove(&id);
        Ok(())
    }

    /// Uncheck a checkbox or radio button.
    pub fn uncheck(&mut self, id: u32) -> Result<(), FetchError> {
        self.require_checkable(id)?;
        self.unchecked_ids.insert(id);
        self.checked_ids.remove(&id);
        Ok(())
    }

    /// Toggle a checkbox or radio button based on its current state.
    pub fn toggle(&mut self, id: u32) -> Result<(), FetchError> {
        let el = self.require_checkable(id)?;
        let currently_checked = if self.checked_ids.contains(&id) {
            true
        } else if self.unchecked_ids.contains(&id) {
            false
        } else {
            el.checked == Some(true)
        };
        if currently_checked {
            self.unchecked_ids.insert(id);
            self.checked_ids.remove(&id);
        } else {
            self.checked_ids.insert(id);
            self.unchecked_ids.remove(&id);
        }
        Ok(())
    }

    pub fn select(&mut self, id: u32, value: &str) -> Result<(), FetchError> {
        let el = self.element(id).ok_or_else(|| {
            FetchError::ActionError(format!("Element {} not found", id))
        })?;
        if el.tag != "select" {
            return Err(FetchError::ActionError(format!(
                "Element {} ({}) is not a select", id, el.tag
            )));
        }
        self.form_values.insert(id, value.to_string());
        Ok(())
    }

    pub fn back(&mut self) -> Result<SpatialDom, FetchError> {
        if self.history.len() < 2 {
            return Err(FetchError::ActionError("No history to go back to".to_string()));
        }
        self.history.pop();
        let prev = self.history.last().unwrap().clone();
        self.goto(&prev)
    }

    pub fn url(&self) -> Option<&str> {
        self.current_url.as_ref().map(|u| u.as_str())
    }

    // --- Findability methods ---

    /// Case-insensitive substring match on element text.
    pub fn find_by_text_fuzzy(&self, text: &str) -> Vec<&SpatialElement> {
        let needle = text.to_lowercase();
        self.current_dom
            .as_ref()
            .map(|dom| {
                dom.els
                    .iter()
                    .filter(|e| {
                        e.text.as_ref().map(|t| t.to_lowercase().contains(&needle)).unwrap_or(false)
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Find an input element by its semantic purpose.
    pub fn find_input_by_purpose(&self, purpose: InputPurpose) -> Option<&SpatialElement> {
        let dom = self.current_dom.as_ref()?;
        dom.els.iter().find(|e| {
            if e.hidden == Some(true) || e.tag != "input" {
                return false;
            }
            let input_type = e.input_type.as_deref().unwrap_or("text");
            let name = e.name.as_deref().unwrap_or("").to_lowercase();
            let label = e.label.as_deref().unwrap_or("").to_lowercase();
            let ph = e.ph.as_deref().unwrap_or("").to_lowercase();

            match purpose {
                InputPurpose::Password => input_type == "password",
                InputPurpose::Email => {
                    input_type == "email"
                        || name.contains("email")
                        || label.contains("email")
                }
                InputPurpose::Username => {
                    (input_type == "text" || input_type == "email")
                        && (name.contains("user") || name.contains("login")
                            || label.contains("user") || label.contains("login"))
                }
                InputPurpose::VerificationCode => {
                    (input_type == "text" || input_type == "number" || input_type == "tel")
                        && (name.contains("code") || name.contains("otp") || name.contains("verify")
                            || label.contains("code") || label.contains("otp") || label.contains("verify")
                            || ph.contains("code") || ph.contains("otp") || ph.contains("verify"))
                }
                InputPurpose::Search => {
                    input_type == "search"
                        || e.role.as_deref() == Some("searchbox")
                        || name.contains("search")
                }
                InputPurpose::Phone => {
                    input_type == "tel"
                        || name.contains("phone")
                        || label.contains("phone")
                }
            }
        })
    }

    /// Find the nearest button to an input element.
    pub fn find_nearest_button(&self, input_id: u32) -> Option<&SpatialElement> {
        let dom = self.current_dom.as_ref()?;
        let btn_id = crate::output::find_nearest_submit_button(dom, input_id)?;
        dom.get(btn_id)
    }

    // --- Compound actions ---

    /// Fill in a login form and submit it.
    pub fn login(&mut self, username: &str, password: &str) -> Result<SpatialDom, FetchError> {
        let (uid, pid, sid) = {
            let dom = self.dom_ref().ok_or_else(|| {
                FetchError::ActionError("No page loaded".to_string())
            })?;
            dom.suggested_actions.iter().find_map(|a| match a {
                SuggestedAction::Login { username_id, password_id, submit_id, .. } => {
                    Some((*username_id, *password_id, *submit_id))
                }
                _ => None,
            }).ok_or_else(|| {
                FetchError::ActionError("No login form detected".to_string())
            })?
        };
        self.type_text(uid, username)?;
        self.type_text(pid, password)?;
        self.click(sid)
    }

    /// Fill in a verification code and submit it.
    pub fn enter_code(&mut self, code: &str) -> Result<SpatialDom, FetchError> {
        let (input_id, submit_id) = {
            let dom = self.dom_ref().ok_or_else(|| {
                FetchError::ActionError("No page loaded".to_string())
            })?;
            dom.suggested_actions.iter().find_map(|a| match a {
                SuggestedAction::EnterCode { input_id, submit_id, .. } => {
                    Some((*input_id, *submit_id))
                }
                _ => None,
            }).ok_or_else(|| {
                FetchError::ActionError("No verification code form detected".to_string())
            })?
        };
        self.type_text(input_id, code)?;
        self.click(submit_id)
    }

    /// Extract a verification code from the current page.
    pub fn find_verification_code(&self) -> Option<String> {
        self.dom_ref()?.find_codes().into_iter().next()
    }

    /// Returns true if the current page is a CAPTCHA challenge.
    pub fn is_captcha(&self) -> bool {
        self.current_dom.as_ref()
            .map(|dom| dom.page_type == PageType::Captcha)
            .unwrap_or(false)
    }

    /// Returns CAPTCHA info if a CAPTCHA was detected on the current page.
    pub fn captcha_info(&self) -> Option<&CaptchaInfo> {
        self.current_dom.as_ref()?.captcha.as_ref()
    }

    /// Search the web using DuckDuckGo and return structured results.
    pub fn search(&mut self, query: &str) -> Result<Vec<SearchResult>, FetchError> {
        self.search_with(query, SearchEngine::DuckDuckGo)
    }

    /// Search using a specific engine.
    pub fn search_with(&mut self, query: &str, engine: SearchEngine) -> Result<Vec<SearchResult>, FetchError> {
        let encoded: String = url::form_urlencoded::Serializer::new(String::new())
            .append_pair("q", query)
            .finish();

        let url = match engine {
            SearchEngine::DuckDuckGo => format!("https://html.duckduckgo.com/html/?{}", encoded),
            SearchEngine::Google => format!("https://www.google.com/search?{}&num=10", encoded),
        };

        let parsed_url = Url::parse(&url).map_err(|e| FetchError::InvalidUrl(e.to_string()))?;
        let html = self.fetch_html_with_retry(&parsed_url)?;

        let dom = crate::dom::parse_html(&html);
        match engine {
            SearchEngine::DuckDuckGo => Ok(extract_ddg_results(&dom)),
            SearchEngine::Google => Ok(extract_google_results(&dom)),
        }
    }

    /// Search and browse the top N results, returning each page's SpatialDom.
    pub fn search_and_read(&mut self, query: &str, n: usize) -> Result<Vec<SearchPage>, FetchError> {
        self.search_and_read_with(query, n, SearchEngine::DuckDuckGo)
    }

    /// Search with a specific engine and browse the top N results.
    pub fn search_and_read_with(&mut self, query: &str, n: usize, engine: SearchEngine) -> Result<Vec<SearchPage>, FetchError> {
        let results = self.search_with(query, engine)?;
        let mut pages = Vec::new();

        for result in results.into_iter().take(n) {
            let dom = self.goto(&result.url).ok();
            pages.push(SearchPage { result, dom });
        }

        Ok(pages)
    }

    fn submit_form(&mut self, button_id: u32) -> Result<SpatialDom, FetchError> {
        let html = self.current_html.as_ref().ok_or_else(|| {
            FetchError::ActionError("No page loaded".to_string())
        })?.clone();

        let base_url = self.current_url.clone().ok_or_else(|| {
            FetchError::ActionError("No URL loaded".to_string())
        })?;

        let dom_tree = crate::dom::parse_html(&html);
        let forms = extract_forms(&dom_tree);

        let button_el = self.element(button_id).ok_or_else(|| {
            FetchError::ActionError(format!("Button {} not found", button_id))
        })?.clone();

        if forms.is_empty() {
            return Err(FetchError::ActionError("No form found on page".to_string()));
        }

        // Find the form containing this button by matching name/text
        let form_idx = find_form_index_for_button(
            &forms,
            button_el.name.as_deref(),
            button_el.text.as_deref(),
        );
        let form = &forms[form_idx];

        // Check if the button has a formaction that overrides the form action
        let button_formaction = form.buttons.iter().find(|b| {
            b.name.as_deref() == button_el.name.as_deref()
                || b.value.as_deref() == button_el.text.as_deref()
                || b.text.as_deref() == button_el.text.as_deref()
        }).and_then(|b| b.formaction.clone());

        // Build form data from parsed form fields with default values
        let mut form_data: Vec<(String, String)> = Vec::new();
        for field in &form.fields {
            if let Some(name) = &field.name {
                // For checkbox/radio, only include if checked
                if field.field_type == "checkbox" || field.field_type == "radio" {
                    if field.checked {
                        form_data.push((name.clone(), field.value.clone().unwrap_or_else(|| "on".to_string())));
                    }
                } else {
                    form_data.push((name.clone(), field.value.clone().unwrap_or_default()));
                }
            }
        }

        // Overlay typed values and checkbox state from session (scoped to this form)
        let dom = self.current_dom.as_ref().ok_or_else(|| {
            FetchError::ActionError("No page loaded".to_string())
        })?;
        let inputs: Vec<&SpatialElement> = dom
            .els
            .iter()
            .filter(|e| e.tag == "input" || e.tag == "textarea" || e.tag == "select")
            .collect();

        let mut name_to_indices: HashMap<&str, Vec<usize>> = HashMap::new();
        for (i, field) in form.fields.iter().enumerate() {
            if let Some(name) = field.name.as_deref() {
                name_to_indices.entry(name).or_default().push(i);
            }
        }

        for input in inputs.iter() {
            let name = match input.name.as_deref() {
                Some(n) => n,
                None => continue,
            };
            if !name_to_indices.contains_key(name) {
                continue;
            }
            // Overlay typed text values
            if let Some(typed_value) = self.form_values.get(&input.id) {
                if let Some(indices) = name_to_indices.get(name) {
                    for idx in indices {
                        if *idx < form_data.len() {
                            form_data[*idx].1 = typed_value.clone();
                        }
                    }
                }
            }

            // Overlay checkbox/radio checked state
            let is_checkable = input.input_type.as_deref() == Some("checkbox")
                || input.input_type.as_deref() == Some("radio");
            if is_checkable {
                let val = input.val.clone().unwrap_or_else(|| "on".to_string());
                if self.checked_ids.contains(&input.id) {
                    // Ensure it's in form_data
                    if let Some(indices) = name_to_indices.get(name) {
                        for idx in indices {
                            if *idx < form_data.len() {
                                form_data[*idx].1 = val.clone();
                            }
                        }
                    } else {
                        form_data.push((name.to_string(), val));
                    }
                } else if self.unchecked_ids.contains(&input.id) {
                    // Remove from form_data
                    form_data.retain(|(n, _)| n != name);
                }
            }
        }

        let method = form.method.as_deref().unwrap_or("get").to_lowercase();
        let action_str = button_formaction.as_deref()
            .or(form.action.as_deref())
            .unwrap_or("");
        let target_url = base_url
            .join(action_str)
            .map_err(|e| FetchError::InvalidUrl(e.to_string()))?;
        if !is_url_allowed(&target_url, self.config.allow_private_network, self.config.allow_non_http) {
            return Err(FetchError::BlockedUrl(target_url.to_string()));
        }

        let (new_url, html) = self.submit_with_retry(&target_url, &method, &form_data)?;

        self.history.push(new_url.clone());
        self.current_url = Some(Url::parse(&new_url).unwrap_or(target_url));
        self.load_html(&html, &new_url)
    }

    fn fetch_html_with_retry(&self, url: &Url) -> Result<String, FetchError> {
        let cfg = FetchConfig {
            viewport_width: self.config.viewport_width,
            viewport_height: self.config.viewport_height,
            user_agent: self.config.user_agent.clone(),
            timeout_secs: self.config.timeout_secs,
            fetch_css: self.config.fetch_css,
            max_response_bytes: self.config.max_response_bytes,
            max_css_bytes_total: self.config.max_css_bytes_total,
            max_css_bytes_per_file: self.config.max_css_bytes_per_file,
            max_redirects: self.config.max_redirects,
            allow_private_network: self.config.allow_private_network,
            allow_non_http: self.config.allow_non_http,
            blocked_patterns: self.config.blocked_patterns.clone(),
            retry_attempts: self.config.retry_attempts,
            retry_delay_ms: self.config.retry_delay_ms,
            retry_on_blocked: self.config.retry_on_blocked,
            retry_user_agents: self.config.retry_user_agents.clone(),
        };
        fetch_html_with_retry(&self.client, url, &cfg)
    }

    fn submit_with_retry(
        &self,
        target_url: &Url,
        method: &str,
        form_data: &[(String, String)],
    ) -> Result<(String, String), FetchError> {
        let mut attempts = 0usize;
        let max_attempts = self.config.retry_attempts + 1;

        while attempts < max_attempts {
            let ua = self
                .config
                .retry_user_agents
                .get(attempts)
                .cloned()
                .unwrap_or_else(|| self.config.user_agent.clone());

            let response = if method == "post" {
                self.client
                    .post(target_url.as_str())
                    .header(USER_AGENT, ua)
                    .form(form_data)
                    .send()
            } else {
                self.client
                    .get(target_url.as_str())
                    .header(USER_AGENT, ua)
                    .query(form_data)
                    .send()
            }
            .map_err(|e| FetchError::Network(e.to_string()))?;

            let status = response.status();
            let retry_after = response.headers().get(RETRY_AFTER).and_then(parse_retry_after);
            if !status.is_success() {
                if matches!(status.as_u16(), 403 | 408 | 429 | 500 | 502 | 503 | 504)
                    && attempts + 1 < max_attempts
                {
                    attempts += 1;
                    let delay = retry_delay_ms(self.config.retry_delay_ms, attempts - 1, retry_after);
                    thread::sleep(Duration::from_millis(delay));
                    continue;
                }
                return Err(FetchError::HttpError(status.as_u16()));
            }

            let new_url = response.url().to_string();
            let html = read_response_text_limited(response, self.config.max_response_bytes)?;
            if self.config.retry_on_blocked {
                let lower = html.to_lowercase();
                let blocked = lower.contains("captcha")
                    || lower.contains("access denied")
                    || lower.contains("verify you are a human")
                    || lower.contains("unusual traffic")
                    || lower.contains("cloudflare")
                    || lower.contains("perimeterx")
                    || lower.contains("datadome");
                if blocked && attempts + 1 < max_attempts {
                    attempts += 1;
                    let delay = retry_delay_ms(self.config.retry_delay_ms, attempts - 1, None);
                    thread::sleep(Duration::from_millis(delay));
                    continue;
                }
            }
            return Ok((new_url, html));
        }

        Err(FetchError::Network("Retry attempts exhausted".to_string()))
    }
}

fn retry_delay_ms(base_ms: u64, attempt: usize, retry_after_secs: Option<u64>) -> u64 {
    let base = base_ms.max(50);
    let exp = 1u64 << attempt.min(6);
    let mut delay = base.saturating_mul(exp).min(30_000);
    if let Some(secs) = retry_after_secs {
        delay = delay.max(secs.saturating_mul(1000));
    }
    delay
}

fn parse_retry_after(value: &reqwest::header::HeaderValue) -> Option<u64> {
    let s = value.to_str().ok()?;
    s.trim().parse::<u64>().ok()
}

/// Search engine to use.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SearchEngine {
    /// DuckDuckGo HTML-only endpoint (most reliable, no JS needed).
    DuckDuckGo,
    /// Google (may be blocked or return CAPTCHAs for automated requests).
    Google,
}

/// A single search result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
}

/// A search result paired with its fetched page (if successful).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchPage {
    pub result: SearchResult,
    pub dom: Option<SpatialDom>,
}

/// Parse search results from a DOM tree. Public for testing.
pub fn extract_search_results_from(dom: &crate::dom::DomNode) -> Vec<SearchResult> {
    extract_ddg_results(dom)
}

/// Parse search results from a Google DOM tree. Public for testing.
pub fn extract_google_results_from(dom: &crate::dom::DomNode) -> Vec<SearchResult> {
    extract_google_results(dom)
}

// ---- DuckDuckGo parser ----

fn extract_ddg_results(dom: &crate::dom::DomNode) -> Vec<SearchResult> {
    let mut results = Vec::new();
    find_ddg_result_nodes(dom, &mut results);
    results
}

fn find_ddg_result_nodes(node: &crate::dom::DomNode, results: &mut Vec<SearchResult>) {
    let classes = node.get_attr("class").unwrap_or("");
    let is_result = classes.split_whitespace().any(|c| c == "result")
        && !classes.contains("result--ad");

    if is_result && node.tag == "div" {
        let mut title = String::new();
        let mut url = String::new();
        let mut snippet = String::new();
        extract_ddg_fields(node, &mut title, &mut url, &mut snippet);

        if !title.is_empty() || !url.is_empty() {
            let resolved_url = decode_redirect_url(&url, "uddg").unwrap_or(url);
            results.push(SearchResult {
                title: title.trim().to_string(),
                url: resolved_url,
                snippet: snippet.trim().to_string(),
            });
        }
    }

    for child in &node.children {
        find_ddg_result_nodes(child, results);
    }
}

fn extract_ddg_fields(
    node: &crate::dom::DomNode,
    title: &mut String,
    url: &mut String,
    snippet: &mut String,
) {
    let classes = node.get_attr("class").unwrap_or("");

    if classes.split_whitespace().any(|c| c == "result__a") && node.tag == "a" {
        *title = node.text_content();
        if let Some(href) = node.get_attr("href") {
            *url = href.to_string();
        }
    }

    if classes.split_whitespace().any(|c| c == "result__snippet") {
        *snippet = node.text_content();
    }

    if classes.split_whitespace().any(|c| c == "result__url") && url.is_empty() {
        if let Some(href) = node.get_attr("href") {
            *url = href.to_string();
        }
    }

    for child in &node.children {
        extract_ddg_fields(child, title, url, snippet);
    }
}

// ---- Google parser ----
// Uses structural patterns (h3 for titles, parent a for URLs) rather than
// fragile class names that Google changes every few months.

fn extract_google_results(dom: &crate::dom::DomNode) -> Vec<SearchResult> {
    let mut results = Vec::new();
    let search_root = find_node_by_id(dom, "rso").unwrap_or(dom);
    find_google_links(search_root, &mut results);
    results
}

/// Find result links: anchor tags that contain an h3 (Google's consistent pattern).
/// For each match, extract title from h3, URL from href, snippet from sibling divs.
fn find_google_links(node: &crate::dom::DomNode, results: &mut Vec<SearchResult>) {
    if node.tag == "a" && has_h3_descendant(node) {
        if let Some(href) = node.get_attr("href") {
            if href.starts_with("http") || href.starts_with("/url?") {
                // Skip ads
                if href.contains("google.com/aclk") || href.contains("googleadservices") {
                    return;
                }

                let title = find_h3_text(node);
                if !title.is_empty() {
                    let resolved = decode_redirect_url(&href, "q")
                        .unwrap_or_else(|| href.to_string());

                    // Skip internal Google links
                    if resolved.contains("google.com/search") || resolved.starts_with("/search") {
                        return;
                    }

                    // Walk up to find snippet in nearby sibling divs
                    // (we'll do a second pass below)
                    results.push(SearchResult {
                        title: title.trim().to_string(),
                        url: resolved,
                        snippet: String::new(),
                    });
                    return;
                }
            }
        }
    }

    for child in &node.children {
        find_google_links(child, results);
    }

    // Second pass: fill in snippets by matching result URLs to nearby text
    if node.tag == "div" && !results.is_empty() {
        fill_google_snippets(node, results);
    }
}

/// Find the text of the first h3 descendant.
fn find_h3_text(node: &crate::dom::DomNode) -> String {
    if node.tag == "h3" {
        return node.text_content();
    }
    for child in &node.children {
        let text = find_h3_text(child);
        if !text.is_empty() {
            return text;
        }
    }
    String::new()
}

/// Fill snippets for results by looking for text-bearing divs near result links.
fn fill_google_snippets(node: &crate::dom::DomNode, results: &mut [SearchResult]) {
    // Look for snippet-class divs and match them to results by proximity
    for result in results.iter_mut() {
        if !result.snippet.is_empty() {
            continue;
        }
        // Search the DOM for snippet text associated with this result
        let mut snippet = String::new();
        find_snippet_near_title(node, &result.title, &mut snippet);
        result.snippet = snippet.trim().to_string();
    }
}

fn find_snippet_near_title(node: &crate::dom::DomNode, title: &str, snippet: &mut String) {
    if !snippet.is_empty() {
        return;
    }

    let classes = node.get_attr("class").unwrap_or("");
    let is_snippet = classes.contains("VwiC3b")
        || classes.contains("yDYNvb")
        || classes.contains("lEBKkf")
        || classes.contains("st");

    if is_snippet && (node.tag == "div" || node.tag == "span") {
        let text = node.text_content();
        if !text.is_empty() && text != title && text.len() > 20 {
            *snippet = text;
            return;
        }
    }

    for child in &node.children {
        find_snippet_near_title(child, title, snippet);
    }
}

fn has_h3_descendant(node: &crate::dom::DomNode) -> bool {
    if node.tag == "h3" {
        return true;
    }
    node.children.iter().any(|c| has_h3_descendant(c))
}

fn find_node_by_id<'a>(node: &'a crate::dom::DomNode, id: &str) -> Option<&'a crate::dom::DomNode> {
    if node.get_attr("id") == Some(id) {
        return Some(node);
    }
    for child in &node.children {
        if let Some(found) = find_node_by_id(child, id) {
            return Some(found);
        }
    }
    None
}

// ---- Shared helpers ----

/// Decode a redirect URL by extracting a query parameter.
/// Works for both DDG (?uddg=...) and Google (?q=...).
fn decode_redirect_url(redirect_url: &str, param: &str) -> Option<String> {
    // Normalize to a full URL for parsing
    let full = if redirect_url.starts_with("//") {
        format!("https:{}", redirect_url)
    } else if redirect_url.starts_with('/') {
        // Relative URL like /url?q=... — prepend a dummy base
        format!("https://dummy.local{}", redirect_url)
    } else {
        redirect_url.to_string()
    };

    if let Ok(parsed) = Url::parse(&full) {
        for (key, value) in parsed.query_pairs() {
            if key == param {
                return Some(value.into_owned());
            }
        }
    }

    if redirect_url.starts_with("http") {
        Some(redirect_url.to_string())
    } else {
        None
    }
}
