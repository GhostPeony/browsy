//! REST API + A2A server for browsy.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::{
    extract::State,
    http::{HeaderMap, HeaderValue, StatusCode},
    response::IntoResponse,
    routing::get,
    Router,
};
use browsy_core::fetch::{Session, SessionConfig};
use uuid::Uuid;

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
    pub fn get_or_create_session(&self, headers: &HeaderMap) -> Result<String, StatusCode> {
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
        sessions.insert(new_token.clone(), SessionEntry {
            session,
            last_access: Instant::now(),
        });
        Ok(new_token)
    }

    /// Execute a closure with the session for the given token.
    pub fn with_session<F, R>(&self, token: &str, f: F) -> Result<R, StatusCode>
    where
        F: FnOnce(&mut Session) -> R,
    {
        let mut sessions = self.sessions.lock().unwrap();
        let entry = sessions.get_mut(token).ok_or(StatusCode::BAD_REQUEST)?;
        entry.last_access = Instant::now();
        Ok(f(&mut entry.session))
    }
}

/// Build the axum router.
pub fn build_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(health))
        .with_state(state)
}

async fn health() -> &'static str {
    "ok"
}
