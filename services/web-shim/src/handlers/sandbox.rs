//! Sandbox session + tool-call handlers.

use std::time::Duration;

use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    error::ShimError,
    state::AppState,
    subprocess::{self, McpCall, NetworkMode},
};

#[derive(Debug, Serialize)]
pub struct SessionResponse {
    pub session_id: String,
    pub expires_in_secs: u64,
}

pub async fn create_session(
    State(state): State<AppState>,
) -> Result<Json<SessionResponse>, ShimError> {
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
