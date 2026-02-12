//! Integration tests for A2A agent card discovery and task endpoints.

use std::sync::Arc;

use axum_test::TestServer;
use browsy_server::{AppState, ServerConfig, build_router};
use http::StatusCode;
use serde_json::Value;

fn test_server() -> TestServer {
    let state = Arc::new(AppState::new(ServerConfig::default()));
    let app = build_router(state);
    TestServer::new(app).unwrap()
}

#[tokio::test]
async fn agent_card_discovery() {
    let server = test_server();

    let response = server.get("/.well-known/agent.json").await;

    response.assert_status_ok();
    response.assert_header("content-type", "application/json");

    let body: Value = response.json();
    assert_eq!(body["name"], "browsy");
    assert_eq!(body["url"], "http://localhost:3847");
    assert_eq!(body["version"], "0.1.0");
}

#[tokio::test]
async fn agent_card_has_required_fields() {
    let server = test_server();

    let body: Value = server.get("/.well-known/agent.json").await.json();

    // Top-level required fields
    assert!(body["name"].is_string(), "name must be a string");
    assert!(body["description"].is_string(), "description must be a string");
    assert!(body["url"].is_string(), "url must be a string");
    assert!(body["version"].is_string(), "version must be a string");
    assert!(body["capabilities"].is_object(), "capabilities must be an object");
    assert!(body["skills"].is_array(), "skills must be an array");
    assert!(
        body["defaultInputModes"].is_array(),
        "defaultInputModes must be an array"
    );
    assert!(
        body["defaultOutputModes"].is_array(),
        "defaultOutputModes must be an array"
    );

    // Capabilities required fields
    let caps = &body["capabilities"];
    assert!(caps["streaming"].is_boolean(), "streaming must be a boolean");
    assert!(
        caps["pushNotifications"].is_boolean(),
        "pushNotifications must be a boolean"
    );

    // Skills structure
    let skills = body["skills"].as_array().unwrap();
    assert!(!skills.is_empty(), "must have at least one skill");

    let skill = &skills[0];
    assert!(skill["id"].is_string(), "skill id must be a string");
    assert!(skill["name"].is_string(), "skill name must be a string");
    assert!(
        skill["description"].is_string(),
        "skill description must be a string"
    );
    assert!(skill["tags"].is_array(), "skill tags must be an array");
    assert!(
        skill["examples"].is_array(),
        "skill examples must be an array"
    );
}

#[tokio::test]
async fn agent_card_skill_content() {
    let server = test_server();

    let body: Value = server.get("/.well-known/agent.json").await.json();

    let skill = &body["skills"][0];
    assert_eq!(skill["id"], "web-browse");
    assert_eq!(skill["name"], "Browse & Extract");

    let tags: Vec<&str> = skill["tags"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert!(tags.contains(&"web"));
    assert!(tags.contains(&"browsing"));
    assert!(tags.contains(&"scraping"));
    assert!(tags.contains(&"forms"));

    let examples = skill["examples"].as_array().unwrap();
    assert_eq!(examples.len(), 3);
}

// ---------------------------------------------------------------------------
// Task endpoint tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn task_invalid_id_returns_400() {
    let server = test_server();

    let response = server.get("/a2a/tasks/not-a-uuid").await;
    response.assert_status(StatusCode::BAD_REQUEST);

    let body: Value = response.json();
    assert!(
        body["error"].is_string(),
        "error field should be present in response"
    );
}

#[tokio::test]
async fn task_valid_uuid_returns_unknown_status() {
    let server = test_server();

    let uuid = "550e8400-e29b-41d4-a716-446655440000";
    let response = server.get(&format!("/a2a/tasks/{uuid}")).await;
    response.assert_status_ok();

    let body: Value = response.json();
    assert_eq!(body["id"], uuid);
    assert_eq!(body["status"], "unknown");
}
