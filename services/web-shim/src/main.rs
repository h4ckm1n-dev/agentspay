//! agentspay-web-shim — HTTP bridge between the marketing website and the
//! local `agentspay-mcp` binary.
//!
//! Spawns the real MCP server as a subprocess per request, pipes JSON-RPC
//! over stdio, and exposes the responses as JSON over HTTPS. Holds the
//! rate-limited devnet wallet for the one-click "trigger a real TX" demo.

use std::net::SocketAddr;

use axum::{routing::get, Json, Router};
use serde_json::json;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main(flavor = "multi_thread")]
async fn main() -> anyhow::Result<()> {
    init_tracing();

    let app = Router::new().route("/api/health", get(health));
    let addr: SocketAddr = "0.0.0.0:8080".parse()?;
    tracing::info!(%addr, "agentspay-web-shim listening");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn health() -> Json<serde_json::Value> {
    Json(json!({
        "status": "ok",
        "service": "agentspay-web-shim",
        "version": env!("CARGO_PKG_VERSION"),
        "environment": std::env::var("AGENTSPAY_ENV").unwrap_or_else(|_| "sandbox".to_string()),
    }))
}

fn init_tracing() {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("agentspay_web_shim=info,tower_http=info"));
    tracing_subscriber::registry()
        .with(env_filter)
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(std::io::stderr)
                .with_ansi(false),
        )
        .init();
}
