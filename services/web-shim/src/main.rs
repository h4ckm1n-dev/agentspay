//! agentspay-web-shim — HTTP bridge between the marketing website and the
//! local `agentspay-mcp` binary.
//!
//! Spawns the real MCP server as a subprocess per request, pipes JSON-RPC
//! over stdio, and exposes the responses as JSON over HTTPS. Holds the
//! rate-limited devnet wallet for the one-click "trigger a real TX" demo.

use std::{sync::Arc, time::Duration};

use axum::{
    routing::{get, post},
    Router,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

mod config;
mod error;
mod handlers;
mod session;
mod state;
mod subprocess;

use crate::{
    config::Config,
    handlers::{health, sandbox},
    session::SessionStore,
    state::AppState,
};

#[tokio::main(flavor = "multi_thread")]
async fn main() -> anyhow::Result<()> {
    init_tracing();

    let config = Config::from_env()?;
    let listen_addr = config.listen_addr;
    let sessions = Arc::new(SessionStore::new_in_memory(config.session_ttl));
    let state = AppState {
        config: Arc::new(config),
        sessions: sessions.clone(),
        http: reqwest::Client::new(),
    };

    // Background sweep: every 60s, drop expired sessions and log if any were swept.
    tokio::spawn(sweep_loop(sessions));

    let app = Router::new()
        .route("/api/health", get(health::health))
        .route("/api/sandbox/session", post(sandbox::create_session))
        .route("/api/sandbox/call", post(sandbox::call_tool))
        .with_state(state);

    tracing::info!(addr = %listen_addr, "agentspay-web-shim listening");

    let listener = tokio::net::TcpListener::bind(listen_addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn sweep_loop(sessions: Arc<SessionStore>) {
    let mut ticker = tokio::time::interval(Duration::from_secs(60));
    // First tick fires immediately — skip it so we don't log a no-op at startup.
    ticker.tick().await;
    loop {
        ticker.tick().await;
        let (active, swept) = sessions.sweep().await;
        if swept > 0 {
            tracing::info!(active, swept, "session sweep");
        }
    }
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
