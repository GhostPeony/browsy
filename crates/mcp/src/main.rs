use std::sync::Mutex;

use browsy_core::fetch::{Session, SessionConfig};
use browsy_mcp::BrowsyServer;
use rmcp::ServiceExt;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create Session outside tokio runtime — reqwest::blocking::Client has its own
    // internal runtime that panics if dropped inside another tokio context.
    let config = SessionConfig::default();
    let session = Session::with_config(config)?;
    let session = std::sync::Arc::new(Mutex::new(session));

    let server = BrowsyServer::with_session(session.clone());

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(async move {
            let service = server.serve(rmcp::transport::stdio()).await?;
            service.waiting().await?;
            Ok::<(), Box<dyn std::error::Error>>(())
        })?;

    // Session's Arc drops here, outside tokio — safe for reqwest::blocking
    drop(session);
    Ok(())
}
