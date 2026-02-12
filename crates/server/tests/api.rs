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

fn test_server_limited(max_sessions: usize) -> TestServer {
    let config = ServerConfig {
        allow_private_network: true,
        max_sessions,
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

// ---------------------------------------------------------------------------
// CORS headers
// ---------------------------------------------------------------------------

#[tokio::test]
async fn cors_headers_present() {
    let server = test_server();
    let res = server.get("/health").await;
    res.assert_status_ok();
    let header = res
        .headers()
        .get("access-control-allow-origin")
        .expect("access-control-allow-origin header should be present");
    assert_eq!(header.to_str().unwrap(), "*");
}

// ---------------------------------------------------------------------------
// Session limit enforcement
// ---------------------------------------------------------------------------

#[tokio::test]
async fn session_limit_enforced() {
    let server = test_server_limited(2);

    // First two requests should succeed (each creates a new session).
    // Using an invalid URL — browse will return 400, but the session is still
    // created before the browse attempt.
    let res1 = server
        .post("/api/browse")
        .json(&json!({ "url": "https://example.com" }))
        .await;
    let token1 = res1
        .headers()
        .get("x-browsy-session")
        .expect("first request should return session token");
    assert!(!token1.is_empty());

    let res2 = server
        .post("/api/browse")
        .json(&json!({ "url": "https://example.com" }))
        .await;
    let token2 = res2
        .headers()
        .get("x-browsy-session")
        .expect("second request should return session token");
    assert!(!token2.is_empty());

    // Third request without a session header should be rejected (limit is 2).
    let res3 = server
        .post("/api/browse")
        .json(&json!({ "url": "https://example.com" }))
        .await;
    res3.assert_status(StatusCode::SERVICE_UNAVAILABLE);
}

// ---------------------------------------------------------------------------
// Session reuse via X-Browsy-Session header
// ---------------------------------------------------------------------------

#[tokio::test]
async fn session_reuse_with_header() {
    let server = test_server();

    // Create a session by browsing.
    let res1 = server
        .post("/api/browse")
        .json(&json!({ "url": "https://example.com" }))
        .await;
    let token = res1
        .headers()
        .get("x-browsy-session")
        .expect("should return session token")
        .to_str()
        .unwrap()
        .to_string();
    assert!(!token.is_empty());

    // Reuse the session token on a second request.
    let res2 = server
        .post("/api/browse")
        .add_header(
            http::header::HeaderName::from_static("x-browsy-session"),
            http::header::HeaderValue::from_str(&token).unwrap(),
        )
        .json(&json!({ "url": "https://example.com" }))
        .await;
    let token2 = res2
        .headers()
        .get("x-browsy-session")
        .expect("should return session token on reuse")
        .to_str()
        .unwrap()
        .to_string();

    assert_eq!(token, token2, "reused session should return the same token");
}

// ---------------------------------------------------------------------------
// Invalid session token creates a new session
// ---------------------------------------------------------------------------

#[tokio::test]
async fn invalid_session_token_creates_new_session() {
    let server = test_server();

    let res = server
        .post("/api/browse")
        .add_header(
            http::header::HeaderName::from_static("x-browsy-session"),
            http::header::HeaderValue::from_static("nonexistent-token"),
        )
        .json(&json!({ "url": "https://example.com" }))
        .await;

    // The server should not fail — it creates a new session.
    let token = res
        .headers()
        .get("x-browsy-session")
        .expect("should return a new session token")
        .to_str()
        .unwrap()
        .to_string();
    assert!(!token.is_empty());
    assert_ne!(
        token, "nonexistent-token",
        "should generate a new token, not echo back the invalid one"
    );
}

// ---------------------------------------------------------------------------
// Click/type/check/select without a prior browse return non-2xx
// ---------------------------------------------------------------------------

#[tokio::test]
async fn click_without_browse_returns_error() {
    let server = test_server();
    let res = server
        .post("/api/click")
        .json(&json!({ "id": 1 }))
        .await;
    assert!(
        !res.status_code().is_success(),
        "click without browse should not succeed, got {}",
        res.status_code()
    );
}

#[tokio::test]
async fn type_without_browse_returns_error() {
    let server = test_server();
    let res = server
        .post("/api/type")
        .json(&json!({ "id": 1, "text": "hello" }))
        .await;
    assert!(
        !res.status_code().is_success(),
        "type without browse should not succeed, got {}",
        res.status_code()
    );
}

#[tokio::test]
async fn check_without_browse_returns_error() {
    let server = test_server();
    let res = server
        .post("/api/check")
        .json(&json!({ "id": 1 }))
        .await;
    assert!(
        !res.status_code().is_success(),
        "check without browse should not succeed, got {}",
        res.status_code()
    );
}

#[tokio::test]
async fn select_without_browse_returns_error() {
    let server = test_server();
    let res = server
        .post("/api/select")
        .json(&json!({ "id": 1, "value": "opt" }))
        .await;
    assert!(
        !res.status_code().is_success(),
        "select without browse should not succeed, got {}",
        res.status_code()
    );
}
