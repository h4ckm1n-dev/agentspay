//! Stats handlers — read-only views of the in-memory hero counter cache.

use axum::{extract::State, Json};
use serde_json::{json, Value};

use crate::state::AppState;

pub async fn latest_tx(State(state): State<AppState>) -> Json<Value> {
    match state.latest_tx.get().await {
        Some(v) => Json(serde_json::to_value(v).unwrap_or(Value::Null)),
        None => Json(json!({"signature": null, "age_seconds": null})),
    }
}
