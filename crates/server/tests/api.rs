//! Integration tests for the browsy REST API.

use std::sync::Arc;

use axum_test::TestServer;
use browsy_server::{AppState, ServerConfig, build_router};
use http::StatusCode;
use serde_json::json;

fn test_server() -> TestServer {
    let config = ServerConfig {
        allow_private_network: true,
        ..Default::default()
    };
    let state = Arc::new(AppState::new(config));
    let app = build_router(state);
    TestServer::new(app).unwrap()
}

#[tokio::test]
async fn health_returns_ok() {
    let server = test_server();
    let res = server.get("/health").await;
    res.assert_status_ok();
    res.assert_text("ok");
}

#[tokio::test]
async fn browse_missing_url_returns_422() {
    let server = test_server();
    let res = server
        .post("/api/browse")
        .json(&json!({}))
        .await;
    res.assert_status(StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn browse_invalid_url_returns_400() {
    let server = test_server();
    let res = server
        .post("/api/browse")
        .json(&json!({ "url": "not-a-url" }))
        .await;
    res.assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn page_without_browse_returns_400() {
    let server = test_server();
    let res = server.get("/api/page").await;
    res.assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn page_info_without_browse_returns_400() {
    let server = test_server();
    let res = server.get("/api/page-info").await;
    res.assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn tables_without_browse_returns_400() {
    let server = test_server();
    let res = server.get("/api/tables").await;
    res.assert_status(StatusCode::BAD_REQUEST);
}
