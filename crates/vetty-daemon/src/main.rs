use std::sync::Arc;

use anyhow::Result;
use axum::routing::get;
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;

mod events;
mod rest;
mod proxy;
mod vsock;
mod ws;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_target(false)
        .init();

    let store = Arc::new(events::EventStore::new());

    let port = std::env::var("VETTY_DAEMON_PORT")
        .unwrap_or_else(|_| "9876".to_string())
        .parse::<u16>()?;
    let listener = TcpListener::bind(format!("127.0.0.1:{port}")).await?;
    tracing::info!("vetty-daemon listening on http://127.0.0.1:{port}");

    proxy::start_proxy_backend(port).await?;

    let vsock_store = store.clone();
    let vsock_path = std::env::var("VETTY_VSOCK_PATH").unwrap_or_else(|_| "/tmp/vetty_v.sock".to_string());
    tokio::spawn(async move {
        if let Err(err) = vsock::listen_vsock(&vsock_path, vsock_store).await {
            tracing::error!("vsock listener failed: {err}");
        }
    });

    let app = rest::rest_router()
        .route("/ws/events", get(ws::ws_handler))
        .layer(CorsLayer::permissive())
        .with_state(store);

    axum::serve(listener, app).await?;
    Ok(())
}
