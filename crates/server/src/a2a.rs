//! A2A (Agent-to-Agent) protocol support.
//!
//! Implements the agent card discovery endpoint at `/.well-known/agent.json`,
//! allowing other agents and orchestrators to discover browsy's capabilities.

use std::sync::Arc;

use axum::{response::IntoResponse, routing::get, Json, Router};
use serde_json::json;

use crate::AppState;

/// Returns an axum Router with A2A protocol routes.
///
/// Currently exposes:
/// - `GET /.well-known/agent.json` — agent card discovery
pub fn a2a_routes() -> Router<Arc<AppState>> {
    Router::new().route("/.well-known/agent.json", get(agent_card))
}

/// Handler for the A2A agent card discovery endpoint.
///
/// Returns a JSON document describing browsy's identity, capabilities,
/// and available skills per the A2A protocol specification.
async fn agent_card() -> impl IntoResponse {
    Json(json!({
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
