//! MCP server for browsy — exposes browse/click/type/search tools over stdio.

use std::sync::Mutex;

use browsy_core::fetch::{FetchError, Session, SearchEngine};
use browsy_core::output;

use rmcp::{
    handler::server::tool::ToolRouter,
    handler::server::wrapper::Parameters,
    model::{CallToolResult, Content, ServerCapabilities, ServerInfo, Implementation},
    tool, tool_handler, tool_router, ErrorData as McpError, ServerHandler,
};

use serde::Deserialize;

// --- Parameter structs ---

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct BrowseParams {
    #[schemars(description = "URL to navigate to")]
    pub url: String,
    #[schemars(description = "Output format: 'compact' (default) or 'json'")]
    pub format: Option<String>,
    #[schemars(description = "Scope: 'all' (default), 'visible', 'above_fold', or 'visible_above_fold'")]
    pub scope: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ClickParams {
    #[schemars(description = "Element ID to click")]
    pub id: u32,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct TypeTextParams {
    #[schemars(description = "Element ID of the text input")]
    pub id: u32,
    #[schemars(description = "Text to type into the input")]
    pub text: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CheckParams {
    #[schemars(description = "Element ID of the checkbox or radio button")]
    pub id: u32,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SelectParams {
    #[schemars(description = "Element ID of the select element")]
    pub id: u32,
    #[schemars(description = "Value to select")]
    pub value: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetPageParams {
    #[schemars(description = "Output format: 'compact' (default) or 'json'")]
    pub format: Option<String>,
    #[schemars(description = "Scope: 'all' (default), 'visible', 'above_fold', or 'visible_above_fold'")]
    pub scope: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SearchParams {
    #[schemars(description = "Search query")]
    pub query: String,
    #[schemars(description = "Search engine: 'duckduckgo' (default) or 'google'")]
    pub engine: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct FindParams {
    #[schemars(description = "Find elements containing this text")]
    pub text: Option<String>,
    #[schemars(description = "Find elements with this ARIA role")]
    pub role: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct LoginParams {
    #[schemars(description = "Username or email")]
    pub username: String,
    #[schemars(description = "Password")]
    pub password: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct EnterCodeParams {
    #[schemars(description = "Verification or 2FA code")]
    pub code: String,
}

// --- Output helpers ---

pub fn format_page(dom: &output::SpatialDom, format: Option<&str>) -> String {
    match format {
        Some("json") => serde_json::to_string_pretty(dom).unwrap_or_default(),
        _ => {
            let mut header = format!(
                "title: {}\nurl: {}\nels: {}\n---\n",
                dom.title,
                dom.url,
                dom.els.len()
            );
            header.push_str(&output::to_compact_string(dom));
            header
        }
    }
}

fn strip_hidden(mut dom: output::SpatialDom) -> output::SpatialDom {
    dom.els.retain(|e| e.hidden != Some(true));
    dom.rebuild_index();
    dom
}

fn apply_scope(dom: output::SpatialDom, scope: Option<&str>) -> output::SpatialDom {
    match scope.unwrap_or("all") {
        "visible" => strip_hidden(dom),
        "above_fold" => dom.filter_above_fold(),
        "visible_above_fold" => strip_hidden(dom).filter_above_fold(),
        _ => dom,
    }
}

fn captcha_warning(dom: &output::SpatialDom) -> Option<String> {
    if dom.page_type != output::PageType::Captcha {
        return None;
    }
    let detail = match dom.captcha {
        Some(ref c) => format!(" ({:?})", c.captcha_type),
        None => String::new(),
    };
    Some(format!("\u{26a0} CAPTCHA detected{detail} \u{2014} this page requires human verification to proceed.\n"))
}

fn blocked_warning(dom: &output::SpatialDom) -> Option<String> {
    let info = dom.blocked.as_ref()?;
    let mut text = format!("\u{26a0} Blocked detected ({})\n", info.reason);
    if !info.signals.is_empty() {
        text.push_str(&format!("signals: {}\n", info.signals.join(", ")));
    }
    if !info.recommendations.is_empty() {
        text.push_str("recommendations:\n");
        for rec in &info.recommendations {
            text.push_str(&format!("  - {}\n", rec));
        }
    }
    if info.require_human {
        text.push_str("requires_human: true\n");
    }
    let next_step = if info.require_human {
        "ask_human_to_solve"
    } else if info.signals.iter().any(|s| s == "rate_limit") {
        "backoff_and_retry"
    } else {
        "retry_with_guidance"
    };
    text.push_str(&format!("next_step: {}\n", next_step));
    Some(text)
}

fn err(msg: impl Into<String>) -> McpError {
    McpError::new(rmcp::model::ErrorCode::INVALID_PARAMS, msg.into(), None)
}

fn map_fetch_error(e: FetchError) -> McpError {
    match &e {
        FetchError::InvalidUrl(_) | FetchError::ActionError(_) | FetchError::BlockedUrl(_) =>
            McpError::new(rmcp::model::ErrorCode::INVALID_PARAMS, e.to_string(), None),
        FetchError::Network(_) | FetchError::HttpError(_) | FetchError::ResponseTooLarge(_, _) =>
            McpError::new(rmcp::model::ErrorCode::INTERNAL_ERROR, e.to_string(), None),
    }
}

// --- Server ---

#[derive(Clone)]
pub struct BrowsyServer {
    session: std::sync::Arc<Mutex<Session>>,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl BrowsyServer {
    pub fn with_session(session: std::sync::Arc<Mutex<Session>>) -> Self {
        Self {
            session,
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "Navigate to a URL and return the page content. Use this to browse websites.")]
    pub async fn browse(
        &self,
        Parameters(params): Parameters<BrowseParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut session = self.session.lock().unwrap();
        let dom = session.goto(&params.url).map_err(map_fetch_error)?;
        let mut text = blocked_warning(&dom).unwrap_or_default();
        text.push_str(&captcha_warning(&dom).unwrap_or_default());
        let scoped = apply_scope(dom, params.scope.as_deref());
        text.push_str(&format_page(&scoped, params.format.as_deref()));
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    #[tool(description = "Click an element by its ID. Links navigate to new pages, buttons submit forms.")]
    pub async fn click(
        &self,
        Parameters(params): Parameters<ClickParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut session = self.session.lock().unwrap();
        let dom = session.click(params.id).map_err(map_fetch_error)?;
        let mut text = blocked_warning(&dom).unwrap_or_default();
        text.push_str(&captcha_warning(&dom).unwrap_or_default());
        text.push_str(&format_page(&dom, None));
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    #[tool(description = "Type text into an input field or textarea by element ID.")]
    pub async fn type_text(
        &self,
        Parameters(params): Parameters<TypeTextParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut session = self.session.lock().unwrap();
        session.type_text(params.id, &params.text).map_err(map_fetch_error)?;
        Ok(CallToolResult::success(vec![Content::text(format!(
            "Typed {:?} into element {}",
            params.text, params.id
        ))]))
    }

    #[tool(description = "Check a checkbox or radio button by element ID.")]
    pub async fn check(
        &self,
        Parameters(params): Parameters<CheckParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut session = self.session.lock().unwrap();
        session.check(params.id).map_err(map_fetch_error)?;
        Ok(CallToolResult::success(vec![Content::text(format!(
            "Checked element {}",
            params.id
        ))]))
    }

    #[tool(description = "Uncheck a checkbox or radio button by element ID.")]
    pub async fn uncheck(
        &self,
        Parameters(params): Parameters<CheckParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut session = self.session.lock().unwrap();
        session.uncheck(params.id).map_err(map_fetch_error)?;
        Ok(CallToolResult::success(vec![Content::text(format!(
            "Unchecked element {}",
            params.id
        ))]))
    }

    #[tool(description = "Select an option in a dropdown/select element by element ID and value.")]
    pub async fn select(
        &self,
        Parameters(params): Parameters<SelectParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut session = self.session.lock().unwrap();
        session.select(params.id, &params.value).map_err(map_fetch_error)?;
        Ok(CallToolResult::success(vec![Content::text(format!(
            "Selected {:?} in element {}",
            params.value, params.id
        ))]))
    }

    #[tool(description = "Get the current page DOM with form state (typed values, checked states). Use after type_text/check/select to see the updated form.")]
    pub async fn get_page(
        &self,
        Parameters(params): Parameters<GetPageParams>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.session.lock().unwrap();
        let dom = session.dom().ok_or_else(|| err("No page loaded"))?;
        let scoped = apply_scope(dom, params.scope.as_deref());
        let text = format_page(&scoped, params.format.as_deref());
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    #[tool(description = "Search the web and return structured results with title, URL, and snippet.")]
    pub async fn search(
        &self,
        Parameters(params): Parameters<SearchParams>,
    ) -> Result<CallToolResult, McpError> {
        let engine = match params.engine.as_deref() {
            Some("google") => SearchEngine::Google,
            _ => SearchEngine::DuckDuckGo,
        };
        let mut session = self.session.lock().unwrap();
        let results = session.search_with(&params.query, engine).map_err(map_fetch_error)?;
        let json = serde_json::to_string_pretty(&results).unwrap_or_default();
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "Go back to the previous page in browsing history.")]
    pub async fn back(&self) -> Result<CallToolResult, McpError> {
        let mut session = self.session.lock().unwrap();
        let dom = session.back().map_err(map_fetch_error)?;
        let text = format_page(&dom, None);
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    #[tool(description = "Log in using detected login form fields. Requires a page with a login form loaded.")]
    pub async fn login(
        &self,
        Parameters(params): Parameters<LoginParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut session = self.session.lock().unwrap();
        let dom = session.login(&params.username, &params.password).map_err(map_fetch_error)?;
        let text = format_page(&dom, None);
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    #[tool(description = "Enter a verification or 2FA code into the detected code input field.")]
    pub async fn enter_code(
        &self,
        Parameters(params): Parameters<EnterCodeParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut session = self.session.lock().unwrap();
        let dom = session.enter_code(&params.code).map_err(map_fetch_error)?;
        let text = format_page(&dom, None);
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    #[tool(description = "Find elements on the current page by text content or ARIA role.")]
    pub async fn find(
        &self,
        Parameters(params): Parameters<FindParams>,
    ) -> Result<CallToolResult, McpError> {
        let session = self.session.lock().unwrap();
        let mut results: Vec<browsy_core::output::SpatialElement> = params
            .text
            .as_deref()
            .map(|t| session.find_by_text(t).into_iter().cloned().collect())
            .unwrap_or_default();

        if let Some(ref role) = params.role {
            let mut dedupe: std::collections::HashSet<u32> =
                results.iter().map(|r| r.id).collect();
            for el in session.find_by_role(role) {
                if dedupe.insert(el.id) {
                    results.push(el.clone());
                }
            }
        }

        let json = serde_json::to_string_pretty(&results).unwrap_or_default();
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "Extract structured table data from the current page. Returns headers and rows.")]
    pub async fn tables(&self) -> Result<CallToolResult, McpError> {
        let session = self.session.lock().unwrap();
        let dom = session.dom().ok_or_else(|| err("No page loaded"))?;
        let tables = dom.tables();
        let json = serde_json::to_string_pretty(&tables).unwrap_or_default();
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(description = "Get page metadata: page type, suggested actions (login/search/consent), alerts, pagination, title, and URL.")]
    pub async fn page_info(&self) -> Result<CallToolResult, McpError> {
        let session = self.session.lock().unwrap();
        let dom = session.dom().ok_or_else(|| err("No page loaded"))?;
        let mut info = serde_json::json!({
            "title": dom.title,
            "url": dom.url,
            "page_type": format!("{:?}", dom.page_type),
            "suggested_actions": dom.suggested_actions,
            "alerts": dom.alerts().iter().map(|a| {
                serde_json::json!({
                    "id": a.id,
                    "type": a.alert_type,
                    "text": a.text,
                })
            }).collect::<Vec<_>>(),
            "pagination": dom.pagination(),
        });
        if let Some(ref captcha) = dom.captcha {
            info.as_object_mut().unwrap().insert(
                "captcha".to_string(),
                serde_json::to_value(captcha).unwrap_or_default(),
            );
        }
        if let Some(ref blocked) = dom.blocked {
            info.as_object_mut().unwrap().insert(
                "blocked".to_string(),
                serde_json::to_value(blocked).unwrap_or_default(),
            );
        }
        let text = serde_json::to_string_pretty(&info).unwrap_or_default();
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }
}

#[tool_handler]
impl ServerHandler for BrowsyServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "browsy: zero-render browser for AI agents. Use browse to navigate, \
                 then interact with elements by ID. Elements are listed with [id:tag \"text\"] format. \
                 Size hints (narrow/wide/full) appear on form elements. \
                 Position (@top, @mid-L, etc.) appears only to disambiguate duplicate elements."
                    .to_string(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation {
                name: "browsy-mcp".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                ..Default::default()
            },
            ..Default::default()
        }
    }
}
