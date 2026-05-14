//! Sandbox session + tool-call handlers.

use std::time::Duration;

use axum::{
    extract::{ConnectInfo, State},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::net::SocketAddr;

use crate::{
    error::ShimError,
    state::AppState,
    subprocess::{self, McpCall, NetworkMode},
};

const SESSION_RL_MAX: u32 = 30;
const SESSION_RL_WINDOW: Duration = Duration::from_secs(60);
const CALL_RL_MAX: u32 = 60;
const CALL_RL_WINDOW: Duration = Duration::from_secs(60);

#[derive(Debug, Serialize)]
pub struct SessionResponse {
    pub session_id: String,
    pub expires_in_secs: u64,
}

pub async fn create_session(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<AppState>,
) -> Result<Json<SessionResponse>, ShimError> {
    state
        .ratelimit
        .check(
            &format!("sandbox-session:{}", addr.ip()),
            SESSION_RL_MAX,
            SESSION_RL_WINDOW,
        )
        .await
        .map_err(|retry_after_secs| ShimError::RateLimited { retry_after_secs })?;
    let session = state
        .sessions
        .create()
        .await
        .map_err(|e| ShimError::Internal(format!("session create: {e}")))?;
    Ok(Json(SessionResponse {
        session_id: session.id.clone(),
        expires_in_secs: state.config.session_ttl.as_secs(),
    }))
}

#[derive(Debug, Deserialize)]
pub struct CallRequest {
    pub session_id: String,
    pub tool: String,
    #[serde(default)]
    pub args: Value,
}

#[derive(Debug, Serialize)]
pub struct CallResponse {
    pub session_id: String,
    pub tool: String,
    pub result: Value,
    pub latency_ms: u128,
}

const ALLOWED_TOOLS: &[&str] = &[
    "agentspay_balance",
    "agentspay_pay_url",
    "agentspay_set_budget",
    "agentspay_audit_log",
    "agentspay_topup_info",
];

pub async fn call_tool(
    State(state): State<AppState>,
    Json(req): Json<CallRequest>,
) -> Result<Json<CallResponse>, ShimError> {
    if !ALLOWED_TOOLS.contains(&req.tool.as_str()) {
        return Err(ShimError::BadRequest(format!(
            "tool not allowed: {} (allowed: {})",
            req.tool,
            ALLOWED_TOOLS.join(", ")
        )));
    }

    state
        .ratelimit
        .check(
            &format!("sandbox-call:{}", req.session_id),
            CALL_RL_MAX,
            CALL_RL_WINDOW,
        )
        .await
        .map_err(|retry_after_secs| ShimError::RateLimited { retry_after_secs })?;

    let session = state
        .sessions
        .get(&req.session_id)
        .await
        .ok_or(ShimError::SessionGone)?;

    let start = std::time::Instant::now();
    let result = subprocess::run(McpCall {
        binary: &state.config.mcp_binary,
        network: NetworkMode::Sandbox,
        keypair_path: &session.keypair_path(),
        db_url: &session.db_url(),
        tool: &req.tool,
        args: req.args,
        timeout: Duration::from_secs(10),
    })
    .await?;

    Ok(Json(CallResponse {
        session_id: req.session_id,
        tool: req.tool,
        result,
        latency_ms: start.elapsed().as_millis(),
    }))
}
