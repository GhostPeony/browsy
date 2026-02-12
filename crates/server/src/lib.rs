//! REST API + A2A server for browsy.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::{
    extract::{Query, State},
    http::{HeaderMap, HeaderValue, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use browsy_core::fetch::{FetchError, SearchEngine, Session, SessionConfig};
use browsy_core::output;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

mod a2a;

// ---------------------------------------------------------------------------
// Session management
// ---------------------------------------------------------------------------

/// A session entry with last-access tracking.
struct SessionEntry {
    session: Session,
    last_access: Instant,
}

/// Shared server state.
pub struct AppState {
    sessions: Mutex<HashMap<String, SessionEntry>>,
    config: ServerConfig,
}

/// Server configuration.
pub struct ServerConfig {
    pub port: u16,
    pub session_timeout: Duration,
    pub allow_private_network: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port: 3847,
            session_timeout: Duration::from_secs(30 * 60),
            allow_private_network: false,
        }
    }
}

impl AppState {
    pub fn new(config: ServerConfig) -> Self {
        Self {
            sessions: Mutex::new(HashMap::new()),
            config,
        }
    }

    /// Get or create a session from the X-Browsy-Session header.
    /// Returns the session token.
    fn get_or_create_session(&self, headers: &HeaderMap) -> Result<String, StatusCode> {
        let token = headers
            .get("X-Browsy-Session")
            .and_then(|v| v.to_str().ok())
            .map(String::from);

        let mut sessions = self.sessions.lock().unwrap();

        // Purge expired sessions
        let timeout = self.config.session_timeout;
        sessions.retain(|_, entry| entry.last_access.elapsed() < timeout);

        if let Some(ref t) = token {
            if sessions.contains_key(t) {
                sessions.get_mut(t).unwrap().last_access = Instant::now();
                return Ok(t.clone());
            }
        }

        // Create new session
        let mut session_config = SessionConfig::default();
        session_config.allow_private_network = self.config.allow_private_network;
        let session = Session::with_config(session_config)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let new_token = Uuid::new_v4().to_string();
        sessions.insert(
            new_token.clone(),
            SessionEntry {
                session,
                last_access: Instant::now(),
            },
        );
        Ok(new_token)
    }

    /// Execute a closure with the session for the given token.
    fn with_session<F, R>(&self, token: &str, f: F) -> Result<R, StatusCode>
    where
        F: FnOnce(&mut Session) -> R,
    {
        let mut sessions = self.sessions.lock().unwrap();
        let entry = sessions.get_mut(token).ok_or(StatusCode::BAD_REQUEST)?;
        entry.last_access = Instant::now();
        Ok(f(&mut entry.session))
    }
}

// ---------------------------------------------------------------------------
// Request / response types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct BrowseParams {
    pub url: String,
    pub format: Option<String>,
    pub scope: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ClickParams {
    pub id: u32,
}

#[derive(Debug, Deserialize)]
pub struct TypeTextParams {
    pub id: u32,
    pub text: String,
}

#[derive(Debug, Deserialize)]
pub struct CheckParams {
    pub id: u32,
}

#[derive(Debug, Deserialize)]
pub struct SelectParams {
    pub id: u32,
    pub value: String,
}

#[derive(Debug, Deserialize)]
pub struct SearchParams {
    pub query: String,
    pub engine: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LoginParams {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct EnterCodeParams {
    pub code: String,
}

#[derive(Debug, Deserialize)]
pub struct FindParams {
    pub text: Option<String>,
    pub role: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GetPageQuery {
    pub format: Option<String>,
    pub scope: Option<String>,
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
}

// ---------------------------------------------------------------------------
// Output helpers (mirroring browsy-mcp)
// ---------------------------------------------------------------------------

fn format_page(dom: &output::SpatialDom, format: Option<&str>) -> String {
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
    Some(format!(
        "\u{26a0} CAPTCHA detected{detail} \u{2014} this page requires human verification to proceed.\n"
    ))
}

// ---------------------------------------------------------------------------
// Error mapping
// ---------------------------------------------------------------------------

fn map_fetch_error(e: FetchError) -> (StatusCode, Json<ErrorResponse>) {
    let status = match &e {
        FetchError::InvalidUrl(_) | FetchError::ActionError(_) | FetchError::BlockedUrl(_) => {
            StatusCode::BAD_REQUEST
        }
        FetchError::Network(_)
        | FetchError::HttpError(_)
        | FetchError::ResponseTooLarge(_, _) => StatusCode::INTERNAL_SERVER_ERROR,
    };
    (
        status,
        Json(ErrorResponse {
            error: e.to_string(),
        }),
    )
}

// ---------------------------------------------------------------------------
// Response builder
// ---------------------------------------------------------------------------

/// Build a JSON response with the X-Browsy-Session header attached.
fn session_response<T: Serialize>(
    token: &str,
    status: StatusCode,
    body: T,
) -> impl IntoResponse {
    let mut headers = HeaderMap::new();
    if let Ok(val) = HeaderValue::from_str(token) {
        headers.insert("X-Browsy-Session", val);
    }
    (status, headers, Json(body))
}

fn session_text_response(token: &str, status: StatusCode, text: String) -> impl IntoResponse {
    let mut headers = HeaderMap::new();
    if let Ok(val) = HeaderValue::from_str(token) {
        headers.insert("X-Browsy-Session", val);
    }
    (status, headers, text)
}

// ---------------------------------------------------------------------------
// Blocking helper
// ---------------------------------------------------------------------------

/// Run a closure on a blocking thread and return its response.
///
/// `browsy_core::Session` uses `reqwest::blocking::Client` which has its own
/// internal tokio Runtime. This Runtime cannot be created or dropped inside
/// another async context. All session operations must therefore run on a
/// dedicated blocking thread.
async fn run_blocking<F>(f: F) -> axum::response::Response
where
    F: FnOnce() -> axum::response::Response + Send + 'static,
{
    match tokio::task::spawn_blocking(f).await {
        Ok(response) => response,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

/// Build the axum router.
pub fn build_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/api/browse", post(browse))
        .route("/api/click", post(click))
        .route("/api/type", post(type_text))
        .route("/api/check", post(check))
        .route("/api/uncheck", post(uncheck))
        .route("/api/select", post(select))
        .route("/api/search", post(search))
        .route("/api/login", post(login))
        .route("/api/enter-code", post(enter_code))
        .route("/api/find", post(find))
        .route("/api/page", get(get_page))
        .route("/api/page-info", get(page_info))
        .route("/api/tables", get(tables))
        .route("/api/back", post(back))
        .merge(a2a::a2a_routes())
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn health() -> &'static str {
    "ok"
}

/// POST /api/browse  { url, format?, scope? }
async fn browse(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(params): Json<BrowseParams>,
) -> axum::response::Response {
    run_blocking(move || {
        let token = match state.get_or_create_session(&headers) {
            Ok(t) => t,
            Err(s) => {
                return session_text_response("", s, "session creation failed".into())
                    .into_response()
            }
        };

        let result = state.with_session(&token, |session| session.goto(&params.url));
        match result {
            Ok(Ok(dom)) => {
                let mut text = captcha_warning(&dom).unwrap_or_default();
                let scoped = apply_scope(dom, params.scope.as_deref());
                text.push_str(&format_page(&scoped, params.format.as_deref()));
                session_text_response(&token, StatusCode::OK, text).into_response()
            }
            Ok(Err(e)) => {
                let (status, body) = map_fetch_error(e);
                session_response(&token, status, body.0).into_response()
            }
            Err(s) => session_text_response("", s, "session error".into()).into_response(),
        }
    })
    .await
}

/// POST /api/click  { id }
async fn click(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(params): Json<ClickParams>,
) -> axum::response::Response {
    run_blocking(move || {
        let token = match state.get_or_create_session(&headers) {
            Ok(t) => t,
            Err(s) => {
                return session_text_response("", s, "session creation failed".into())
                    .into_response()
            }
        };

        let result = state.with_session(&token, |session| session.click(params.id));
        match result {
            Ok(Ok(dom)) => {
                let mut text = captcha_warning(&dom).unwrap_or_default();
                text.push_str(&format_page(&dom, None));
                session_text_response(&token, StatusCode::OK, text).into_response()
            }
            Ok(Err(e)) => {
                let (status, body) = map_fetch_error(e);
                session_response(&token, status, body.0).into_response()
            }
            Err(s) => session_text_response("", s, "session error".into()).into_response(),
        }
    })
    .await
}

/// POST /api/type  { id, text }
async fn type_text(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(params): Json<TypeTextParams>,
) -> axum::response::Response {
    run_blocking(move || {
        let token = match state.get_or_create_session(&headers) {
            Ok(t) => t,
            Err(s) => {
                return session_text_response("", s, "session creation failed".into())
                    .into_response()
            }
        };

        let result =
            state.with_session(&token, |session| session.type_text(params.id, &params.text));
        match result {
            Ok(Ok(())) => {
                let body = serde_json::json!({
                    "ok": true,
                    "message": format!("Typed {:?} into element {}", params.text, params.id)
                });
                session_response(&token, StatusCode::OK, body).into_response()
            }
            Ok(Err(e)) => {
                let (status, body) = map_fetch_error(e);
                session_response(&token, status, body.0).into_response()
            }
            Err(s) => session_text_response("", s, "session error".into()).into_response(),
        }
    })
    .await
}

/// POST /api/check  { id }
async fn check(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(params): Json<CheckParams>,
) -> axum::response::Response {
    run_blocking(move || {
        let token = match state.get_or_create_session(&headers) {
            Ok(t) => t,
            Err(s) => {
                return session_text_response("", s, "session creation failed".into())
                    .into_response()
            }
        };

        let result = state.with_session(&token, |session| session.check(params.id));
        match result {
            Ok(Ok(())) => {
                let body = serde_json::json!({
                    "ok": true,
                    "message": format!("Checked element {}", params.id)
                });
                session_response(&token, StatusCode::OK, body).into_response()
            }
            Ok(Err(e)) => {
                let (status, body) = map_fetch_error(e);
                session_response(&token, status, body.0).into_response()
            }
            Err(s) => session_text_response("", s, "session error".into()).into_response(),
        }
    })
    .await
}

/// POST /api/uncheck  { id }
async fn uncheck(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(params): Json<CheckParams>,
) -> axum::response::Response {
    run_blocking(move || {
        let token = match state.get_or_create_session(&headers) {
            Ok(t) => t,
            Err(s) => {
                return session_text_response("", s, "session creation failed".into())
                    .into_response()
            }
        };

        let result = state.with_session(&token, |session| session.uncheck(params.id));
        match result {
            Ok(Ok(())) => {
                let body = serde_json::json!({
                    "ok": true,
                    "message": format!("Unchecked element {}", params.id)
                });
                session_response(&token, StatusCode::OK, body).into_response()
            }
            Ok(Err(e)) => {
                let (status, body) = map_fetch_error(e);
                session_response(&token, status, body.0).into_response()
            }
            Err(s) => session_text_response("", s, "session error".into()).into_response(),
        }
    })
    .await
}

/// POST /api/select  { id, value }
async fn select(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(params): Json<SelectParams>,
) -> axum::response::Response {
    run_blocking(move || {
        let token = match state.get_or_create_session(&headers) {
            Ok(t) => t,
            Err(s) => {
                return session_text_response("", s, "session creation failed".into())
                    .into_response()
            }
        };

        let result =
            state.with_session(&token, |session| session.select(params.id, &params.value));
        match result {
            Ok(Ok(())) => {
                let body = serde_json::json!({
                    "ok": true,
                    "message": format!("Selected {:?} in element {}", params.value, params.id)
                });
                session_response(&token, StatusCode::OK, body).into_response()
            }
            Ok(Err(e)) => {
                let (status, body) = map_fetch_error(e);
                session_response(&token, status, body.0).into_response()
            }
            Err(s) => session_text_response("", s, "session error".into()).into_response(),
        }
    })
    .await
}

/// POST /api/search  { query, engine? }
async fn search(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(params): Json<SearchParams>,
) -> axum::response::Response {
    run_blocking(move || {
        let token = match state.get_or_create_session(&headers) {
            Ok(t) => t,
            Err(s) => {
                return session_text_response("", s, "session creation failed".into())
                    .into_response()
            }
        };

        let engine = match params.engine.as_deref() {
            Some("google") => SearchEngine::Google,
            _ => SearchEngine::DuckDuckGo,
        };

        let result =
            state.with_session(&token, |session| session.search_with(&params.query, engine));
        match result {
            Ok(Ok(results)) => {
                session_response(&token, StatusCode::OK, results).into_response()
            }
            Ok(Err(e)) => {
                let (status, body) = map_fetch_error(e);
                session_response(&token, status, body.0).into_response()
            }
            Err(s) => session_text_response("", s, "session error".into()).into_response(),
        }
    })
    .await
}

/// POST /api/login  { username, password }
async fn login(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(params): Json<LoginParams>,
) -> axum::response::Response {
    run_blocking(move || {
        let token = match state.get_or_create_session(&headers) {
            Ok(t) => t,
            Err(s) => {
                return session_text_response("", s, "session creation failed".into())
                    .into_response()
            }
        };

        let result = state
            .with_session(&token, |session| session.login(&params.username, &params.password));
        match result {
            Ok(Ok(dom)) => {
                let text = format_page(&dom, None);
                session_text_response(&token, StatusCode::OK, text).into_response()
            }
            Ok(Err(e)) => {
                let (status, body) = map_fetch_error(e);
                session_response(&token, status, body.0).into_response()
            }
            Err(s) => session_text_response("", s, "session error".into()).into_response(),
        }
    })
    .await
}

/// POST /api/enter-code  { code }
async fn enter_code(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(params): Json<EnterCodeParams>,
) -> axum::response::Response {
    run_blocking(move || {
        let token = match state.get_or_create_session(&headers) {
            Ok(t) => t,
            Err(s) => {
                return session_text_response("", s, "session creation failed".into())
                    .into_response()
            }
        };

        let result = state.with_session(&token, |session| session.enter_code(&params.code));
        match result {
            Ok(Ok(dom)) => {
                let text = format_page(&dom, None);
                session_text_response(&token, StatusCode::OK, text).into_response()
            }
            Ok(Err(e)) => {
                let (status, body) = map_fetch_error(e);
                session_response(&token, status, body.0).into_response()
            }
            Err(s) => session_text_response("", s, "session error".into()).into_response(),
        }
    })
    .await
}

/// POST /api/find  { text?, role? }
async fn find(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(params): Json<FindParams>,
) -> axum::response::Response {
    run_blocking(move || {
        let token = match state.get_or_create_session(&headers) {
            Ok(t) => t,
            Err(s) => {
                return session_text_response("", s, "session creation failed".into())
                    .into_response()
            }
        };

        let result = state.with_session(&token, |session| {
            let mut results: Vec<output::SpatialElement> = params
                .text
                .as_deref()
                .map(|t| session.find_by_text(t).into_iter().cloned().collect())
                .unwrap_or_default();

            if let Some(ref role) = params.role {
                let role_results: Vec<_> = session
                    .find_by_role(role)
                    .into_iter()
                    .filter(|el| !results.iter().any(|r| r.id == el.id))
                    .cloned()
                    .collect();
                results.extend(role_results);
            }

            results
        });

        match result {
            Ok(results) => {
                session_response(&token, StatusCode::OK, results).into_response()
            }
            Err(s) => session_text_response("", s, "session error".into()).into_response(),
        }
    })
    .await
}

/// GET /api/page  ?scope=&format=
async fn get_page(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(params): Query<GetPageQuery>,
) -> axum::response::Response {
    run_blocking(move || {
        let token = match state.get_or_create_session(&headers) {
            Ok(t) => t,
            Err(s) => {
                return session_text_response("", s, "session creation failed".into())
                    .into_response()
            }
        };

        let result = state.with_session(&token, |session| session.dom());
        match result {
            Ok(Some(dom)) => {
                let scoped = apply_scope(dom, params.scope.as_deref());
                let text = format_page(&scoped, params.format.as_deref());
                session_text_response(&token, StatusCode::OK, text).into_response()
            }
            Ok(None) => {
                let body = ErrorResponse {
                    error: "No page loaded".into(),
                };
                session_response(&token, StatusCode::BAD_REQUEST, body).into_response()
            }
            Err(s) => session_text_response("", s, "session error".into()).into_response(),
        }
    })
    .await
}

/// GET /api/page-info
async fn page_info(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> axum::response::Response {
    run_blocking(move || {
        let token = match state.get_or_create_session(&headers) {
            Ok(t) => t,
            Err(s) => {
                return session_text_response("", s, "session creation failed".into())
                    .into_response()
            }
        };

        let result = state.with_session(&token, |session| session.dom());
        match result {
            Ok(Some(dom)) => {
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
                session_response(&token, StatusCode::OK, info).into_response()
            }
            Ok(None) => {
                let body = ErrorResponse {
                    error: "No page loaded".into(),
                };
                session_response(&token, StatusCode::BAD_REQUEST, body).into_response()
            }
            Err(s) => session_text_response("", s, "session error".into()).into_response(),
        }
    })
    .await
}

/// GET /api/tables
async fn tables(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> axum::response::Response {
    run_blocking(move || {
        let token = match state.get_or_create_session(&headers) {
            Ok(t) => t,
            Err(s) => {
                return session_text_response("", s, "session creation failed".into())
                    .into_response()
            }
        };

        let result = state.with_session(&token, |session| session.dom());
        match result {
            Ok(Some(dom)) => {
                let table_data = dom.tables();
                session_response(&token, StatusCode::OK, table_data).into_response()
            }
            Ok(None) => {
                let body = ErrorResponse {
                    error: "No page loaded".into(),
                };
                session_response(&token, StatusCode::BAD_REQUEST, body).into_response()
            }
            Err(s) => session_text_response("", s, "session error".into()).into_response(),
        }
    })
    .await
}

/// POST /api/back
async fn back(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> axum::response::Response {
    run_blocking(move || {
        let token = match state.get_or_create_session(&headers) {
            Ok(t) => t,
            Err(s) => {
                return session_text_response("", s, "session creation failed".into())
                    .into_response()
            }
        };

        let result = state.with_session(&token, |session| session.back());
        match result {
            Ok(Ok(dom)) => {
                let text = format_page(&dom, None);
                session_text_response(&token, StatusCode::OK, text).into_response()
            }
            Ok(Err(e)) => {
                let (status, body) = map_fetch_error(e);
                session_response(&token, status, body.0).into_response()
            }
            Err(s) => session_text_response("", s, "session error".into()).into_response(),
        }
    })
    .await
}
