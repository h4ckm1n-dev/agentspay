//! Stats handlers — read-only public proof views.

use axum::{
    extract::{Query, State},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::error::ShimError;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct TransactionsQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub limit: Option<u64>,
}

pub async fn latest_tx(State(state): State<AppState>) -> Json<Value> {
    match state.latest_tx.get().await {
        Some(v) => Json(serde_json::to_value(v).unwrap_or(Value::Null)),
        None => Json(json!({"signature": null, "age_seconds": null})),
    }
}

pub async fn transactions(
    State(state): State<AppState>,
    Query(query): Query<TransactionsQuery>,
) -> Result<Json<Value>, ShimError> {
    let page = query.page.unwrap_or(1);
    let page_size = query.limit.or(query.page_size).unwrap_or(10);
    let txs = state
        .latest_tx
        .ledger_transactions(page, page_size)
        .await
        .map_err(|err| ShimError::Internal(format!("read transactions: {err}")))?;

    Ok(Json(serde_json::to_value(txs).unwrap_or(Value::Null)))
}
