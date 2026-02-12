use std::sync::Arc;
use browsy_server::{AppState, ServerConfig, build_router};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ServerConfig::default();
    let port = config.port;
    let state = Arc::new(AppState::new(config));
    let app = build_router(state);

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(async move {
            let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await?;
            eprintln!("browsy server listening on http://localhost:{port}");
            axum::serve(listener, app).await?;
            Ok::<(), Box<dyn std::error::Error>>(())
        })?;

    Ok(())
}
