//! Devnet handlers: wallet status + one-click on-chain trigger.

use std::net::SocketAddr;
use std::time::Duration;

use axum::{
    extract::{ConnectInfo, Query, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::{
    devnet_wallet::{self, WalletStatus},
    error::ShimError,
    latest_tx::LatestTx,
    state::AppState,
    subprocess::{self, McpCall, NetworkMode},
};

const TRIGGER_RL_MAX: u32 = 1;
const TRIGGER_RL_WINDOW: Duration = Duration::from_secs(3600);
const STATUS_RL_MAX: u32 = 60;
const STATUS_RL_WINDOW: Duration = Duration::from_secs(60);
const SYMBOLS: &[&str] = &["AAPL", "MSFT", "GOOG", "NVDA", "AMZN"];

pub async fn wallet_status(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<Json<WalletStatus>, ShimError> {
    let client_ip = super::client_ip(&headers, &addr);
    state
        .ratelimit
        .check(
            &format!("devnet-status:{client_ip}"),
            STATUS_RL_MAX,
            STATUS_RL_WINDOW,
        )
        .await
        .map_err(|retry_after_secs| ShimError::RateLimited { retry_after_secs })?;

    let pubkey = devnet_wallet::read_pubkey(&state.config.devnet_wallet_path)
        .await
        .map_err(|e| ShimError::Internal(format!("read wallet: {e}")))?;
    let status = devnet_wallet::fetch_status(&state.http, &pubkey)
        .await
        .map_err(|e| ShimError::Internal(format!("fetch status: {e}")))?;
    Ok(Json(status))
}

#[derive(Debug, Serialize)]
pub struct TriggerResponse {
    pub signature: String,
    pub explorer_url: String,
    pub symbol: String,
    pub amount_charged_usdc: String,
    pub body: Value,
    pub latency_ms: u128,
}

#[derive(Debug, Deserialize)]
pub struct TriggerRequest {
    pub symbol: Option<String>,
}

pub async fn trigger(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
    payload: Option<Json<TriggerRequest>>,
) -> Result<Json<TriggerResponse>, ShimError> {
    let client_ip = super::client_ip(&headers, &addr);
    state
        .ratelimit
        .check(
            &format!("devnet-trigger:{client_ip}"),
            TRIGGER_RL_MAX,
            TRIGGER_RL_WINDOW,
        )
        .await
        .map_err(|retry_after_secs| ShimError::RateLimited { retry_after_secs })?;

    let pubkey = devnet_wallet::read_pubkey(&state.config.devnet_wallet_path)
        .await
        .map_err(|e| ShimError::Internal(format!("read wallet: {e}")))?;
    let status = devnet_wallet::fetch_status(&state.http, &pubkey)
        .await
        .map_err(|e| ShimError::Internal(format!("fetch status: {e}")))?;
    if !status.healthy {
        tracing::warn!(message = ?status.message, "devnet wallet drained");
        return Err(ShimError::WalletDrained);
    }

    let requested = payload.as_ref().and_then(|Json(req)| req.symbol.as_deref());
    let symbol = requested_symbol(requested)?;
    let url = format!("{}/real-quote/{}", state.config.paid_endpoint_url, symbol);
    let db_url = format!(
        "sqlite://{}?mode=rwc",
        state.config.devnet_ledger_path.display()
    );

    let start = std::time::Instant::now();
    let mcp_result = subprocess::run(McpCall {
        binary: &state.config.mcp_binary,
        network: NetworkMode::SolanaDevnet,
        keypair_path: &state.config.devnet_wallet_path,
        db_url: &db_url,
        tool: "agentspay_pay_url",
        args: json!({"url": url, "max_amount_usdc": "0.50"}),
        timeout: Duration::from_secs(20),
    })
    .await?;

    let payload_text = mcp_result
        .pointer("/content/0/text")
        .and_then(Value::as_str)
        .ok_or_else(|| ShimError::MalformedMcp("missing content[0].text".into()))?;
    let payload: Value = serde_json::from_str(payload_text)
        .map_err(|e| ShimError::MalformedMcp(format!("inner json: {e}")))?;

    let signature = payload
        .get("transaction")
        .and_then(Value::as_str)
        .ok_or_else(|| ShimError::MalformedMcp("missing transaction".into()))?
        .to_string();
    let explorer_url = payload
        .get("explorer_url")
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| format!("https://solscan.io/tx/{signature}?cluster=devnet"));
    let amount_charged_usdc = payload
        .get("amount_charged_usdc")
        .and_then(Value::as_str)
        .unwrap_or("0.10")
        .to_string();
    let body_str = payload.get("body").and_then(Value::as_str).unwrap_or("{}");
    let body: Value = serde_json::from_str(body_str).unwrap_or(Value::Null);

    let latency_ms = start.elapsed().as_millis();

    // Update the hero counter cache.
    state
        .latest_tx
        .set(LatestTx {
            signature: signature.clone(),
            amount_usdc: amount_charged_usdc.clone(),
            explorer_url: explorer_url.clone(),
            at: std::time::SystemTime::now(),
        })
        .await;

    Ok(Json(TriggerResponse {
        signature,
        explorer_url,
        symbol,
        amount_charged_usdc,
        body,
        latency_ms,
    }))
}

#[derive(Debug, Deserialize)]
pub struct PaymentRequestQuery {
    pub symbol: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PaymentRequestResponse {
    pub symbol: String,
    pub url: String,
    pub status: u16,
    pub amount_usdc: Option<String>,
    pub pay_to: Option<String>,
    pub network: Option<String>,
    pub description: Option<String>,
    pub resource: Option<String>,
    pub body: Value,
}

pub async fn payment_request(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
    Query(query): Query<PaymentRequestQuery>,
) -> Result<Json<PaymentRequestResponse>, ShimError> {
    let client_ip = super::client_ip(&headers, &addr);
    state
        .ratelimit
        .check(
            &format!("devnet-payment-request:{client_ip}"),
            STATUS_RL_MAX,
            STATUS_RL_WINDOW,
        )
        .await
        .map_err(|retry_after_secs| ShimError::RateLimited { retry_after_secs })?;

    let symbol = requested_symbol(query.symbol.as_deref())?;
    let url = format!("{}/real-quote/{}", state.config.paid_endpoint_url, symbol);
    let response = state
        .http
        .get(&url)
        .send()
        .await
        .map_err(|e| ShimError::Internal(format!("request payment: {e}")))?;
    let status = response.status();
    let body: Value = response
        .json()
        .await
        .map_err(|e| ShimError::Internal(format!("read payment request: {e}")))?;

    if status != StatusCode::PAYMENT_REQUIRED {
        tracing::warn!(
            status = status.as_u16(),
            symbol,
            "paid endpoint did not return a 402 payment request"
        );
    }

    let requirement = body.pointer("/accepts/0").unwrap_or(&Value::Null);
    let amount_usdc = requirement
        .get("maxAmountRequired")
        .and_then(Value::as_str)
        .and_then(|amount| {
            let decimals = requirement
                .pointer("/extra/decimals")
                .and_then(Value::as_u64)
                .unwrap_or(6) as u32;
            amount_to_usdc(amount, decimals)
        });

    Ok(Json(PaymentRequestResponse {
        symbol,
        url,
        status: status.as_u16(),
        amount_usdc,
        pay_to: requirement
            .get("payTo")
            .and_then(Value::as_str)
            .map(str::to_string),
        network: requirement
            .get("network")
            .and_then(Value::as_str)
            .map(str::to_string),
        description: requirement
            .get("description")
            .and_then(Value::as_str)
            .map(str::to_string),
        resource: requirement
            .get("resource")
            .and_then(Value::as_str)
            .map(str::to_string),
        body,
    }))
}

fn requested_symbol(symbol: Option<&str>) -> Result<String, ShimError> {
    let Some(symbol) = symbol else {
        return Ok(pick_symbol().to_string());
    };
    let normalized = symbol.trim().to_uppercase();
    if SYMBOLS.contains(&normalized.as_str()) {
        return Ok(normalized);
    }
    Err(ShimError::BadRequest(format!(
        "unsupported symbol {symbol}; expected one of {}",
        SYMBOLS.join(", ")
    )))
}

fn amount_to_usdc(amount: &str, decimals: u32) -> Option<String> {
    let units = amount.parse::<u128>().ok()?;
    let scale = 10_u128.checked_pow(decimals)?;
    let whole = units / scale;
    let fractional = units % scale;
    if fractional == 0 {
        return Some(whole.to_string());
    }
    let mut frac = format!("{fractional:0width$}", width = decimals as usize);
    while frac.ends_with('0') {
        frac.pop();
    }
    Some(format!("{whole}.{frac}"))
}

fn pick_symbol() -> &'static str {
    let i = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as usize)
        % SYMBOLS.len();
    SYMBOLS[i]
}
