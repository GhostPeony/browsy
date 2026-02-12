//! A2A (Agent-to-Agent) protocol support.
//!
//! Implements agent card discovery and task execution endpoints per the
//! A2A protocol specification. Browsy acts as an autonomous skill provider:
//! orchestrating agents describe a browsing goal, browsy handles multi-step
//! navigation using page intelligence, and streams progress via SSE.

use std::convert::Infallible;
use std::sync::Arc;

use axum::{
    extract::State,
    http::StatusCode,
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse,
    },
    routing::{get, post},
    Json, Router,
};
use browsy_core::fetch::{Session, SessionConfig};
use browsy_core::output::{self, PageType, SuggestedAction};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::AppState;

/// Returns an axum Router with A2A protocol routes.
pub fn a2a_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/.well-known/agent.json", get(agent_card))
        .route("/a2a/tasks", post(create_task))
        .route("/a2a/tasks/{task_id}", get(get_task))
}

// ---------------------------------------------------------------------------
// Agent Card
// ---------------------------------------------------------------------------

async fn agent_card() -> impl IntoResponse {
    Json(serde_json::json!({
        "name": "browsy",
        "description": "Zero-render browser for AI agents. Navigates websites, fills forms, extracts structured data without rendering pixels.",
        "url": "http://localhost:3847",
        "version": "0.1.0",
        "capabilities": {
            "streaming": true,
            "pushNotifications": false
        },
        "skills": [
            {
                "id": "web-browse",
                "name": "Browse & Extract",
                "description": "Navigate to a URL, interact with the page, and extract content. Handles multi-step flows like login, search, and form submission autonomously.",
                "tags": ["web", "browsing", "scraping", "forms"],
                "examples": [
                    "Go to example.com and extract the pricing table",
                    "Search for 'browsy' on DuckDuckGo and return the top 5 results",
                    "Log in to this dashboard with these credentials and download the report"
                ]
            }
        ],
        "defaultInputModes": ["text/plain"],
        "defaultOutputModes": ["text/plain", "application/json"]
    }))
}

// ---------------------------------------------------------------------------
// Task Types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct CreateTaskRequest {
    goal: String,
    #[serde(default)]
    params: TaskParams,
}

#[derive(Debug, Default, Deserialize)]
#[allow(dead_code)]
struct TaskParams {
    url: Option<String>,
    credentials: Option<Credentials>,
    search_query: Option<String>,
    extract: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct Credentials {
    username: String,
    password: String,
}

#[derive(Debug, Clone, Serialize)]
struct TaskStatus {
    id: String,
    status: String,
    steps: Vec<TaskStep>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct TaskStep {
    action: String,
    detail: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    page_type: Option<String>,
}

// ---------------------------------------------------------------------------
// Create Task — POST /a2a/tasks
// ---------------------------------------------------------------------------

async fn create_task(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateTaskRequest>,
) -> Sse<impl futures::Stream<Item = Result<Event, Infallible>>> {
    let task_id = Uuid::new_v4().to_string();
    let (tx, rx) = tokio::sync::mpsc::channel::<Result<Event, Infallible>>(32);

    let allow_private = state.config.allow_private_network;

    // Spawn the task execution on a blocking thread (reqwest::blocking)
    let task_id_clone = task_id.clone();
    tokio::task::spawn_blocking(move || {
        execute_task(task_id_clone, req, allow_private, tx);
    });

    let stream = tokio_stream::wrappers::ReceiverStream::new(rx);
    Sse::new(stream).keep_alive(KeepAlive::new().interval(std::time::Duration::from_secs(15)))
}

/// Parse a URL from the goal text or params.
fn extract_url(goal: &str, params: &TaskParams) -> Option<String> {
    // Explicit URL param takes priority
    if let Some(ref url) = params.url {
        return Some(url.clone());
    }

    // Try to find a URL in the goal text
    for word in goal.split_whitespace() {
        let candidate = word.trim_matches(|c: char| c == '\'' || c == '"' || c == ',' || c == '.');
        if candidate.starts_with("http://") || candidate.starts_with("https://") {
            return Some(candidate.to_string());
        }
        // Bare domain detection (e.g. "example.com")
        if candidate.contains('.') && !candidate.contains(' ') && candidate.len() > 3 {
            let parts: Vec<&str> = candidate.split('.').collect();
            if parts.len() >= 2 && parts.last().map_or(false, |tld| tld.len() >= 2) {
                return Some(format!("https://{candidate}"));
            }
        }
    }
    None
}

/// Detect intent from goal text.
fn detect_intent(goal: &str) -> TaskIntent {
    let lower = goal.to_lowercase();
    if lower.contains("search") || lower.contains("find") || lower.contains("look up") {
        TaskIntent::Search
    } else if lower.contains("log in") || lower.contains("login") || lower.contains("sign in") {
        TaskIntent::Login
    } else if lower.contains("extract") || lower.contains("scrape") || lower.contains("get the") {
        TaskIntent::Extract
    } else if lower.contains("table") || lower.contains("pricing") || lower.contains("data") {
        TaskIntent::ExtractTables
    } else if lower.contains("fill") || lower.contains("submit") || lower.contains("form") {
        TaskIntent::FillForm
    } else {
        TaskIntent::Browse
    }
}

#[derive(Debug)]
enum TaskIntent {
    Browse,
    Search,
    Login,
    Extract,
    ExtractTables,
    FillForm,
}

/// Execute a browsing task, sending SSE events along the way.
fn execute_task(
    task_id: String,
    req: CreateTaskRequest,
    allow_private: bool,
    tx: tokio::sync::mpsc::Sender<Result<Event, Infallible>>,
) {
    let mut steps: Vec<TaskStep> = Vec::new();

    let send_status = |status: &str, steps: &[TaskStep], result: Option<serde_json::Value>, error: Option<String>| {
        let payload = TaskStatus {
            id: task_id.clone(),
            status: status.to_string(),
            steps: steps.to_vec(),
            result,
            error,
        };
        let _ = tx.blocking_send(Ok(
            Event::default()
                .event("status")
                .data(serde_json::to_string(&payload).unwrap_or_default()),
        ));
    };

    // 1. Parse the goal
    let intent = detect_intent(&req.goal);
    let url = extract_url(&req.goal, &req.params);
    let search_query = req.params.search_query.clone().or_else(|| {
        // Try extracting search query from goal: "search for X" / "search 'X'"
        let lower = req.goal.to_lowercase();
        if let Some(pos) = lower.find("search for ").or_else(|| lower.find("search '")) {
            let start = if lower[pos..].starts_with("search for ") {
                pos + 11
            } else {
                pos + 8
            };
            let rest = &req.goal[start..];
            let query = rest
                .trim_matches(|c: char| c == '\'' || c == '"')
                .split(&['.', '!'][..])
                .next()
                .unwrap_or(rest)
                .trim();
            if !query.is_empty() {
                return Some(query.to_string());
            }
        }
        None
    });

    send_status("working", &steps, None, None);

    // 2. Create session
    let mut config = SessionConfig::default();
    config.allow_private_network = allow_private;
    let mut session = match Session::with_config(config) {
        Ok(s) => s,
        Err(e) => {
            send_status("failed", &steps, None, Some(format!("Session creation failed: {e}")));
            return;
        }
    };

    // 3. Execute based on intent
    match intent {
        TaskIntent::Search => {
            let query = match search_query {
                Some(q) => q,
                None => {
                    send_status("failed", &steps, None, Some("No search query found in goal".into()));
                    return;
                }
            };

            steps.push(TaskStep {
                action: "search".into(),
                detail: format!("Searching for: {query}"),
                page_type: None,
            });
            send_status("working", &steps, None, None);

            match session.search(&query) {
                Ok(results) => {
                    let result = serde_json::to_value(&results).unwrap_or_default();
                    steps.push(TaskStep {
                        action: "complete".into(),
                        detail: format!("Found {} results", results.len()),
                        page_type: Some("SearchResults".into()),
                    });
                    send_status("completed", &steps, Some(result), None);
                }
                Err(e) => {
                    send_status("failed", &steps, None, Some(e.to_string()));
                }
            }
        }

        TaskIntent::Login => {
            let url = match url {
                Some(u) => u,
                None => {
                    send_status("failed", &steps, None, Some("No URL found for login".into()));
                    return;
                }
            };
            let creds = match req.params.credentials {
                Some(c) => c,
                None => {
                    send_status("failed", &steps, None, Some("No credentials provided for login".into()));
                    return;
                }
            };

            // Navigate to URL
            steps.push(TaskStep {
                action: "browse".into(),
                detail: format!("Navigating to {url}"),
                page_type: None,
            });
            send_status("working", &steps, None, None);

            let dom = match session.goto(&url) {
                Ok(d) => d,
                Err(e) => {
                    send_status("failed", &steps, None, Some(e.to_string()));
                    return;
                }
            };

            steps.last_mut().unwrap().page_type = Some(format!("{:?}", dom.page_type));
            send_status("working", &steps, None, None);

            // Login
            steps.push(TaskStep {
                action: "login".into(),
                detail: format!("Logging in as {}", creds.username),
                page_type: None,
            });
            send_status("working", &steps, None, None);

            match session.login(&creds.username, &creds.password) {
                Ok(dom) => {
                    let page_type = format!("{:?}", dom.page_type);
                    let compact = output::to_compact_string(&dom);
                    steps.push(TaskStep {
                        action: "complete".into(),
                        detail: format!("Login result: {page_type}"),
                        page_type: Some(page_type),
                    });
                    send_status(
                        "completed",
                        &steps,
                        Some(serde_json::json!({
                            "title": dom.title,
                            "url": dom.url,
                            "page_type": format!("{:?}", dom.page_type),
                            "content": compact,
                        })),
                        None,
                    );
                }
                Err(e) => {
                    send_status("failed", &steps, None, Some(e.to_string()));
                }
            }
        }

        TaskIntent::ExtractTables | TaskIntent::Extract | TaskIntent::Browse | TaskIntent::FillForm => {
            let url = match url {
                Some(u) => u,
                None => {
                    // For search-like intent without a URL, try search
                    if let Some(query) = search_query {
                        steps.push(TaskStep {
                            action: "search".into(),
                            detail: format!("No URL found, searching: {query}"),
                            page_type: None,
                        });
                        send_status("working", &steps, None, None);
                        match session.search(&query) {
                            Ok(results) => {
                                let result = serde_json::to_value(&results).unwrap_or_default();
                                send_status("completed", &steps, Some(result), None);
                            }
                            Err(e) => {
                                send_status("failed", &steps, None, Some(e.to_string()));
                            }
                        }
                        return;
                    }
                    send_status("failed", &steps, None, Some("No URL or search query found in goal".into()));
                    return;
                }
            };

            // Navigate
            steps.push(TaskStep {
                action: "browse".into(),
                detail: format!("Navigating to {url}"),
                page_type: None,
            });
            send_status("working", &steps, None, None);

            let dom = match session.goto(&url) {
                Ok(d) => d,
                Err(e) => {
                    send_status("failed", &steps, None, Some(e.to_string()));
                    return;
                }
            };

            let page_type = format!("{:?}", dom.page_type);
            steps.last_mut().unwrap().page_type = Some(page_type.clone());
            send_status("working", &steps, None, None);

            // Handle cookie consent if detected
            for action in &dom.suggested_actions {
                if let SuggestedAction::CookieConsent { accept_id, .. } = action {
                    steps.push(TaskStep {
                        action: "click".into(),
                        detail: "Accepting cookie consent".into(),
                        page_type: None,
                    });
                    send_status("working", &steps, None, None);
                    let _ = session.click(*accept_id);
                    break;
                }
            }

            // Handle CAPTCHA / blocked
            if dom.page_type == PageType::Captcha {
                send_status(
                    "failed",
                    &steps,
                    None,
                    Some("CAPTCHA detected — requires human verification".into()),
                );
                return;
            }
            if dom.page_type == PageType::Blocked {
                send_status(
                    "failed",
                    &steps,
                    None,
                    Some("Access blocked by target site".into()),
                );
                return;
            }

            // Get final DOM (may have changed after cookie consent)
            let final_dom = session.dom().unwrap_or(dom);

            // Build result based on intent
            let mut result = serde_json::json!({
                "title": final_dom.title,
                "url": final_dom.url,
                "page_type": format!("{:?}", final_dom.page_type),
            });

            // Extract tables if requested or if intent is table extraction
            let tables = final_dom.tables();
            if !tables.is_empty() {
                result.as_object_mut().unwrap().insert(
                    "tables".into(),
                    serde_json::to_value(&tables).unwrap_or_default(),
                );
            }

            // Always include compact content
            result.as_object_mut().unwrap().insert(
                "content".into(),
                serde_json::Value::String(output::to_compact_string(&final_dom)),
            );

            // Include page intelligence
            if !final_dom.suggested_actions.is_empty() {
                result.as_object_mut().unwrap().insert(
                    "suggested_actions".into(),
                    serde_json::to_value(&final_dom.suggested_actions).unwrap_or_default(),
                );
            }

            steps.push(TaskStep {
                action: "complete".into(),
                detail: format!(
                    "Extracted content ({} elements, {} tables)",
                    final_dom.els.len(),
                    tables.len()
                ),
                page_type: Some(format!("{:?}", final_dom.page_type)),
            });
            send_status("completed", &steps, Some(result), None);
        }
    }
}

// ---------------------------------------------------------------------------
// Get Task — GET /a2a/tasks/:task_id (status polling stub)
// ---------------------------------------------------------------------------

async fn get_task(
    axum::extract::Path(task_id): axum::extract::Path<String>,
) -> impl IntoResponse {
    // Task results are streamed via SSE. This endpoint provides a minimal
    // acknowledgment that the task ID format is valid.
    if Uuid::parse_str(&task_id).is_err() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Invalid task ID" })),
        );
    }

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "id": task_id,
            "status": "unknown",
            "message": "Task results are delivered via SSE stream on POST /a2a/tasks. Use the SSE event stream for real-time status."
        })),
    )
}
