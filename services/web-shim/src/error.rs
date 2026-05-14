//! Typed errors that map to HTTP responses.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum ShimError {
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error("session not found or expired")]
    SessionGone,
    #[error("rate limited (retry in {retry_after_secs}s)")]
    RateLimited { retry_after_secs: u64 },
    #[error("subprocess timed out after {0}s")]
    SubprocessTimeout(u64),
    #[error("subprocess failed: {0}")]
    SubprocessFailed(String),
    #[error("malformed mcp response: {0}")]
    MalformedMcp(String),
    #[error("devnet wallet drained or unhealthy")]
    WalletDrained,
    #[error("internal error: {0}")]
    Internal(String),
}

impl IntoResponse for ShimError {
    fn into_response(self) -> Response {
        let request_id = Uuid::new_v4().to_string();
        let (status, code) = match &self {
            Self::BadRequest(_) => (StatusCode::BAD_REQUEST, "bad_request"),
            Self::SessionGone => (StatusCode::GONE, "session_gone"),
            Self::RateLimited { .. } => (StatusCode::TOO_MANY_REQUESTS, "rate_limited"),
            Self::SubprocessTimeout(_) => (StatusCode::GATEWAY_TIMEOUT, "subprocess_timeout"),
            Self::SubprocessFailed(_) => (StatusCode::BAD_GATEWAY, "subprocess_failed"),
            Self::MalformedMcp(_) => (StatusCode::BAD_GATEWAY, "malformed_mcp"),
            Self::WalletDrained => (StatusCode::SERVICE_UNAVAILABLE, "wallet_drained"),
            Self::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, "internal"),
        };
        tracing::warn!(error = %self, request_id = %request_id, "request failed");
        let mut response = (
            status,
            Json(json!({
                "code": code,
                "message": self.to_string(),
                "request_id": request_id,
            })),
        )
            .into_response();
        if let Self::RateLimited { retry_after_secs } = &self {
            if let Ok(v) = retry_after_secs.to_string().parse() {
                response.headers_mut().insert("retry-after", v);
            }
        }
        response
    }
}
