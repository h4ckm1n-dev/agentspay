//! agentspay-web-shim — HTTP bridge between the marketing website and the
//! local `agentspay-mcp` binary.
//!
//! Spawns the real MCP server as a subprocess per request, pipes JSON-RPC
//! over stdio, and exposes the responses as JSON over HTTPS. Holds the
//! rate-limited devnet wallet for the one-click "trigger a real TX" demo.

use std::sync::Arc;

use axum::{routing::get, Json, Router};
use serde_json::json;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

mod config;
mod error;
mod session;
mod state;
mod subprocess;

use crate::{config::Config, session::SessionStore, state::AppState};

#[tokio::main(flavor = "multi_thread")]
async fn main() -> anyhow::Result<()> {
    init_tracing();

    let config = Config::from_env()?;
    let listen_addr = config.listen_addr;
    let sessions = SessionStore::new_in_memory(config.session_ttl);
    let state = AppState {
        config: Arc::new(config),
        sessions: Arc::new(sessions),
        http: reqwest::Client::new(),
    };

    let app = Router::new()
        .route("/api/health", get(health))
        .with_state(state);

    tracing::info!(addr = %listen_addr, "agentspay-web-shim listening");

    let listener = tokio::net::TcpListener::bind(listen_addr).await?;
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
