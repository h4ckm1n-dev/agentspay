use axum::Json;
use serde_json::{json, Value};

pub async fn health() -> Json<Value> {
    Json(json!({
        "status": "ok",
        "service": "agentspay-web-shim",
        "version": env!("CARGO_PKG_VERSION"),
        "environment": std::env::var("AGENTSPAY_ENV").unwrap_or_else(|_| "sandbox".to_string()),
    }))
}
