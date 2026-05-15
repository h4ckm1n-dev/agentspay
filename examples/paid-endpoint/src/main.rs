//! AgentsPay demo paid endpoint.
//!
//! Two routes share the same Axum service on `127.0.0.1:3001`:
//!
//! * `GET /quote/:symbol` — **sandbox mode**. Returns 402, then accepts any
//!   non-empty `X-PAYMENT` header on retry. No blockchain. Used by the
//!   offline regression tests in tandem with `AGENTSPAY_NETWORK=sandbox`.
//!
//! * `GET /real-quote/:symbol` — **real Solana devnet**. Returns 402, then
//!   expects an `X-PAYMENT` header whose `payload.transaction` is the base64
//!   of a `bincode`-serialized signed Solana `Transaction`. The seller
//!   submits the transaction to the configured RPC and emits an
//!   `X-PAYMENT-RESPONSE` header containing the real on-chain signature.
//!
//! The recipient address (`payTo`) is configurable via
//! `AGENTSPAY_DEMO_PAYTO`. When unset, an ephemeral keypair is generated at
//! startup and its pubkey is logged so the operator can fund it for ATA
//! rent.

use std::{
    collections::HashMap,
    env, fs,
    net::SocketAddr,
    path::{Path as StdPath, PathBuf},
    sync::Arc,
};

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Default bind address. We use `0.0.0.0` (not `127.0.0.1`) so the
/// container's port-forward can actually reach the process. On a developer
/// laptop this still answers on `localhost:3001` because the kernel accepts
/// loopback packets on any local IP.
const LISTEN_ADDR: &str = "0.0.0.0:3001";
const X402_VERSION: u8 = 1;
const SCHEME: &str = "exact";
const SANDBOX_NETWORK: &str = "sandbox";
const SOLANA_DEVNET_NETWORK: &str = "solana-devnet";
/// `100000` base units of USDC (6 decimals) = 0.10 USDC per quote.
const MAX_AMOUNT_REQUIRED: &str = "100000";
const USDC_DECIMALS: u8 = 6;
const MAX_TIMEOUT_SECONDS: u32 = 60;
/// Circle USDC mint on Solana devnet.
const USDC_MINT_DEVNET: &str = "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU";
const DEFAULT_RPC: &str = "https://api.devnet.solana.com";
const RPC_URL_ENV: &str = "AGENTSPAY_SOLANA_RPC_URL";
const PROVIDER_KEYPAIR_ENV: &str = "AGENTSPAY_PROVIDER_KEYPAIR";
const DEMO_PAYTO_ENV: &str = "AGENTSPAY_DEMO_PAYTO";
/// Default location for the persistent demo-provider keypair, relative to `$HOME`.
const DEFAULT_PROVIDER_KEYPAIR_SUBPATH: &str = ".agentspay/provider-keypair.json";
/// Toggle: when `true`, forward verify+settle to the public facilitator
/// instead of submitting the signed transaction directly to a Solana RPC.
const USE_FACILITATOR_ENV: &str = "AGENTSPAY_USE_FACILITATOR";
/// Default facilitator URL (Coinbase-hosted public facilitator). Override via
/// `AGENTSPAY_FACILITATOR_URL`.
const DEFAULT_FACILITATOR_URL: &str = "https://x402.org/facilitator";
const FACILITATOR_URL_ENV: &str = "AGENTSPAY_FACILITATOR_URL";

// SAFETY: the hardcoded fallback address below is used **only** when neither
// `AGENTSPAY_DEMO_PAYTO` nor `AGENTSPAY_PROVIDER_KEYPAIR` is set. It is a
// well-known throwaway devnet receiver pubkey (the all-1s base58 string is
// invalid for SPL operations, so we instead pick a valid 32-byte pubkey).
// Operators are expected to override it via env in any real demo run.
const FALLBACK_PAYTO: &str = "6dnXxM4S6ZxN3CdEqcyJxYC2sBmrCxnX3yHJ9JT3KweK";

const X_PAYMENT: &str = "x-payment";
const X_PAYMENT_RESPONSE: &str = "x-payment-response";

// ---------------------------------------------------------------------------
// Shared state
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct AppState {
    /// The address that should receive USDC from agents on `/real-quote/*`.
    pay_to: String,
    rpc: Arc<RpcClient>,
    http: reqwest::Client,
    /// `Some(base_url)` when verify+settle should be forwarded to a public
    /// facilitator instead of being submitted to the local RPC directly.
    /// Set via `AGENTSPAY_USE_FACILITATOR=true` + optional
    /// `AGENTSPAY_FACILITATOR_URL`.
    facilitator: Option<String>,
}

// ---------------------------------------------------------------------------
// x402 payload shapes
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
struct PaymentRequirement<'a> {
    scheme: &'a str,
    network: &'a str,
    #[serde(rename = "maxAmountRequired")]
    max_amount_required: &'a str,
    resource: String,
    description: &'a str,
    #[serde(rename = "mimeType")]
    mime_type: &'a str,
    #[serde(rename = "payTo")]
    pay_to: &'a str,
    #[serde(rename = "maxTimeoutSeconds")]
    max_timeout_seconds: u32,
    asset: &'a str,
    extra: Value,
}

#[derive(Debug, Serialize)]
struct PaymentRequiredBody<'a> {
    #[serde(rename = "x402Version")]
    x402_version: u8,
    error: &'a str,
    accepts: Vec<PaymentRequirement<'a>>,
}

/// x402 Solana payload as sent in the `X-PAYMENT` header (after base64).
#[derive(Debug, Deserialize)]
struct SolanaPaymentPayload {
    #[serde(rename = "x402Version")]
    #[allow(dead_code)]
    x402_version: u8,
    #[allow(dead_code)]
    scheme: String,
    network: String,
    payload: SolanaInner,
}

#[derive(Debug, Deserialize)]
struct SolanaInner {
    /// Base64 of a `bincode`-serialized signed Solana `Transaction`.
    transaction: String,
}

#[derive(Debug, Serialize)]
struct PaymentResponseBody<'a> {
    success: bool,
    transaction: String,
    network: &'a str,
    payer: String,
}

// ---------------------------------------------------------------------------
// Mock quote data
// ---------------------------------------------------------------------------

fn mock_quotes() -> HashMap<&'static str, f64> {
    HashMap::from([
        ("AAPL", 195.42),
        ("MSFT", 421.18),
        ("GOOG", 178.91),
        ("NVDA", 132.05),
        ("AMZN", 215.77),
    ])
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn health() -> impl IntoResponse {
    tracing::info!(method = "GET", path = "/health", "request");
    (StatusCode::OK, Json(json!({"status": "ok"})))
}

/// Sandbox: accept any non-empty X-PAYMENT header and emit a synthetic
/// payment-response. No blockchain involvement.
async fn sandbox_quote(
    State(state): State<AppState>,
    Path(symbol): Path<String>,
    headers: HeaderMap,
) -> Response {
    let path = format!("/quote/{symbol}");
    tracing::info!(
        method = "GET",
        path = %path,
        x_payment_present = headers.contains_key(X_PAYMENT),
        mode = "sandbox",
        "request"
    );

    let normalized = symbol.to_uppercase();
    let quotes = mock_quotes();
    let Some(price) = quotes.get(normalized.as_str()).copied() else {
        return symbol_not_supported(&normalized, &quotes);
    };

    let Some(raw) = headers.get(X_PAYMENT) else {
        return payment_required(
            &state.pay_to,
            &path,
            SANDBOX_NETWORK,
            "X-PAYMENT header is required",
        )
        .into_response();
    };
    let header_value = raw.to_str().unwrap_or("").trim();
    if header_value.is_empty() {
        return payment_required(
            &state.pay_to,
            &path,
            SANDBOX_NETWORK,
            "X-PAYMENT header is empty",
        )
        .into_response();
    }

    // Synthetic settlement metadata: no real transaction signature, but
    // we still emit the header so clients exercise the parse path.
    let response_body = PaymentResponseBody {
        success: true,
        transaction: format!("sandbox-tx-{}", uuid::Uuid::new_v4().simple()),
        network: SANDBOX_NETWORK,
        payer: "agentspay_local_agent".to_string(),
    };
    let body = json!({
        "symbol": normalized,
        "price": price,
        "currency": "USD",
        "timestamp_rfc3339": chrono::Utc::now().to_rfc3339(),
        "network": SANDBOX_NETWORK,
        "transaction": response_body.transaction,
    });
    let encoded = BASE64.encode(serde_json::to_vec(&response_body).unwrap_or_default());
    let mut response = (StatusCode::OK, Json(body)).into_response();
    if let Ok(value) = encoded.parse() {
        response.headers_mut().insert(X_PAYMENT_RESPONSE, value);
    }
    response
}

/// Real Solana devnet: submit the signed transaction supplied by the agent
/// and wait for confirmation.
async fn real_quote(
    State(state): State<AppState>,
    Path(symbol): Path<String>,
    headers: HeaderMap,
) -> Response {
    let path = format!("/real-quote/{symbol}");
    tracing::info!(
        method = "GET",
        path = %path,
        x_payment_present = headers.contains_key(X_PAYMENT),
        mode = "solana-devnet",
        "request"
    );

    let normalized = symbol.to_uppercase();
    let quotes = mock_quotes();
    let Some(price) = quotes.get(normalized.as_str()).copied() else {
        return symbol_not_supported(&normalized, &quotes);
    };

    let Some(raw) = headers.get(X_PAYMENT) else {
        return payment_required(
            &state.pay_to,
            &path,
            SOLANA_DEVNET_NETWORK,
            "X-PAYMENT header is required",
        )
        .into_response();
    };

    let payload = match decode_solana_payment(raw.to_str().unwrap_or("")) {
        Ok(p) => p,
        Err(reason) => {
            tracing::warn!(reason = %reason, "rejecting invalid X-PAYMENT header");
            return payment_required(
                &state.pay_to,
                &path,
                SOLANA_DEVNET_NETWORK,
                "X-PAYMENT header is invalid",
            )
            .into_response();
        }
    };

    let tx_bytes = match BASE64.decode(payload.payload.transaction.trim()) {
        Ok(b) => b,
        Err(e) => {
            tracing::warn!(error = %e, "X-PAYMENT.payload.transaction is not valid base64");
            return payment_required(
                &state.pay_to,
                &path,
                SOLANA_DEVNET_NETWORK,
                "payload.transaction is not valid base64",
            )
            .into_response();
        }
    };
    let tx: Transaction = match bincode::deserialize(&tx_bytes) {
        Ok(t) => t,
        Err(e) => {
            tracing::warn!(error = %e, "X-PAYMENT transaction is not a valid bincode Transaction");
            return payment_required(
                &state.pay_to,
                &path,
                SOLANA_DEVNET_NETWORK,
                "payload.transaction is not a valid Solana Transaction",
            )
            .into_response();
        }
    };

    let payer = tx
        .message
        .account_keys
        .first()
        .map(|p| p.to_string())
        .unwrap_or_default();

    // Branch: forward verify+settle to facilitator, or submit to RPC directly.
    let signature = if let Some(facilitator_url) = state.facilitator.as_deref() {
        tracing::info!(
            network = SOLANA_DEVNET_NETWORK,
            facilitator = facilitator_url,
            payer = %payer,
            "forwarding payment to facilitator (verify + settle)"
        );
        let raw_b64 = headers
            .get(X_PAYMENT)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .trim();
        let raw_bytes = match BASE64.decode(raw_b64) {
            Ok(b) => b,
            Err(e) => {
                tracing::warn!(error = %e, "facilitator path: X-PAYMENT base64 decode");
                return facilitator_failure(&state.pay_to, &path, format!("invalid base64: {e}"));
            }
        };
        let payment_payload: Value = match serde_json::from_slice(&raw_bytes) {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(error = %e, "facilitator path: X-PAYMENT json parse");
                return facilitator_failure(
                    &state.pay_to,
                    &path,
                    format!("invalid payload json: {e}"),
                );
            }
        };
        let resource = format!("http://{LISTEN_ADDR}{path}");
        let requirements = json!({
            "scheme": SCHEME,
            "network": SOLANA_DEVNET_NETWORK,
            "maxAmountRequired": MAX_AMOUNT_REQUIRED,
            "resource": resource,
            "description": "Realtime stock quote (devnet demo)",
            "mimeType": "application/json",
            "payTo": state.pay_to,
            "maxTimeoutSeconds": MAX_TIMEOUT_SECONDS,
            "asset": USDC_MINT_DEVNET,
            "extra": { "decimals": USDC_DECIMALS },
        });
        match submit_via_facilitator(
            &state.http,
            facilitator_url,
            &payment_payload,
            &requirements,
        )
        .await
        {
            Ok(tx_sig) => match tx_sig.parse() {
                Ok(sig) => sig,
                Err(e) => {
                    tracing::warn!(error = %e, signature = %tx_sig, "facilitator returned malformed signature");
                    return facilitator_failure(
                        &state.pay_to,
                        &path,
                        format!("facilitator returned malformed signature: {tx_sig}"),
                    );
                }
            },
            Err(e) => {
                tracing::warn!(error = %e, "facilitator submission failed");
                return facilitator_failure(&state.pay_to, &path, e.to_string());
            }
        }
    } else {
        tracing::info!(
            network = SOLANA_DEVNET_NETWORK,
            payer = %payer,
            "submitting payment transaction to solana"
        );
        match state
            .rpc
            .send_and_confirm_transaction_with_spinner_and_commitment(
                &tx,
                CommitmentConfig::confirmed(),
            )
            .await
        {
            Ok(sig) => sig,
            Err(e) => {
                tracing::warn!(error = %e, "solana send_and_confirm_transaction failed");
                let resource = format!("http://{LISTEN_ADDR}{path}");
                let body = PaymentRequiredBody {
                    x402_version: X402_VERSION,
                    error: "solana RPC rejected the signed transaction",
                    accepts: vec![accepts_entry(
                        &state.pay_to,
                        resource,
                        SOLANA_DEVNET_NETWORK,
                    )],
                };
                let mut response = (StatusCode::PAYMENT_REQUIRED, Json(body)).into_response();
                response.headers_mut().insert(
                    "x-error-detail",
                    e.to_string().parse().unwrap_or_else(|_| {
                        "solana-rpc-error".parse().expect("static header parses")
                    }),
                );
                return response;
            }
        }
    };

    let body = json!({
        "symbol": normalized,
        "price": price,
        "currency": "USD",
        "timestamp_rfc3339": chrono::Utc::now().to_rfc3339(),
        "network": SOLANA_DEVNET_NETWORK,
        "transaction": signature.to_string(),
        "explorer_url": format!("https://solscan.io/tx/{signature}?cluster=devnet"),
    });

    let response_body = PaymentResponseBody {
        success: true,
        transaction: signature.to_string(),
        network: SOLANA_DEVNET_NETWORK,
        payer,
    };
    let encoded = BASE64.encode(serde_json::to_vec(&response_body).unwrap_or_default());

    let mut response = (StatusCode::OK, Json(body)).into_response();
    if let Ok(value) = encoded.parse() {
        response.headers_mut().insert(X_PAYMENT_RESPONSE, value);
    }
    tracing::info!(
        signature = %signature,
        payer = %response_body.payer,
        "payment confirmed on solana devnet"
    );
    response
}

fn symbol_not_supported(normalized: &str, quotes: &HashMap<&'static str, f64>) -> Response {
    (
        StatusCode::NOT_FOUND,
        Json(json!({
            "error": "symbol not supported",
            "symbol": normalized,
            "supported": quotes.keys().collect::<Vec<_>>(),
        })),
    )
        .into_response()
}

fn accepts_entry<'a>(
    pay_to: &'a str,
    resource: String,
    network: &'a str,
) -> PaymentRequirement<'a> {
    PaymentRequirement {
        scheme: SCHEME,
        network,
        max_amount_required: MAX_AMOUNT_REQUIRED,
        resource,
        description: "Realtime stock quote (devnet demo)",
        mime_type: "application/json",
        pay_to,
        max_timeout_seconds: MAX_TIMEOUT_SECONDS,
        asset: USDC_MINT_DEVNET,
        extra: json!({ "decimals": USDC_DECIMALS }),
    }
}

fn payment_required(pay_to: &str, path: &str, network: &str, error: &'static str) -> Response {
    let resource = format!("http://{LISTEN_ADDR}{path}");
    let body = PaymentRequiredBody {
        x402_version: X402_VERSION,
        error,
        accepts: vec![accepts_entry(pay_to, resource, network)],
    };
    (StatusCode::PAYMENT_REQUIRED, Json(body)).into_response()
}

/// POST `{paymentPayload, paymentRequirements}` to `{facilitator}/verify`,
/// then `{facilitator}/settle`. Returns the on-chain transaction signature
/// from the settle response (`transaction` field).
///
/// Per the coinbase/x402 facilitator spec:
/// - `/verify` returns `{ isValid: bool, invalidReason?: string, payer?: string }`.
/// - `/settle` returns `{ success: bool, errorReason?: string, transaction?: string, network: string, payer: string }`.
async fn submit_via_facilitator(
    http: &reqwest::Client,
    base_url: &str,
    payment_payload: &Value,
    payment_requirements: &Value,
) -> anyhow::Result<String> {
    let body = json!({
        "x402Version": X402_VERSION,
        "paymentPayload": payment_payload,
        "paymentRequirements": payment_requirements,
    });

    // Phase 1: verify.
    let verify_url = format!("{}/verify", base_url.trim_end_matches('/'));
    let resp = http
        .post(&verify_url)
        .json(&body)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("verify request failed: {e}"))?;
    let status = resp.status();
    let verify_body: Value = resp
        .json()
        .await
        .map_err(|e| anyhow::anyhow!("verify response not JSON ({status}): {e}"))?;
    if !status.is_success() {
        return Err(anyhow::anyhow!(
            "facilitator /verify returned {status}: {}",
            verify_body
        ));
    }
    let is_valid = verify_body
        .get("isValid")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if !is_valid {
        let reason = verify_body
            .get("invalidReason")
            .and_then(Value::as_str)
            .unwrap_or("(no reason)");
        return Err(anyhow::anyhow!("facilitator rejected payment: {reason}"));
    }

    // Phase 2: settle.
    let settle_url = format!("{}/settle", base_url.trim_end_matches('/'));
    let resp = http
        .post(&settle_url)
        .json(&body)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("settle request failed: {e}"))?;
    let status = resp.status();
    let settle_body: Value = resp
        .json()
        .await
        .map_err(|e| anyhow::anyhow!("settle response not JSON ({status}): {e}"))?;
    if !status.is_success() {
        return Err(anyhow::anyhow!(
            "facilitator /settle returned {status}: {}",
            settle_body
        ));
    }
    let success = settle_body
        .get("success")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if !success {
        let reason = settle_body
            .get("errorReason")
            .and_then(Value::as_str)
            .unwrap_or("(no reason)");
        return Err(anyhow::anyhow!("facilitator settle failed: {reason}"));
    }
    let tx = settle_body
        .get("transaction")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("facilitator settle missing transaction field"))?
        .to_string();
    Ok(tx)
}

fn facilitator_failure(pay_to: &str, path: &str, detail: String) -> Response {
    let resource = format!("http://{LISTEN_ADDR}{path}");
    let body = PaymentRequiredBody {
        x402_version: X402_VERSION,
        error: "facilitator rejected the signed payment",
        accepts: vec![accepts_entry(pay_to, resource, SOLANA_DEVNET_NETWORK)],
    };
    let mut response = (StatusCode::PAYMENT_REQUIRED, Json(body)).into_response();
    if let Ok(value) = detail.parse() {
        response.headers_mut().insert("x-error-detail", value);
    }
    response
}

fn decode_solana_payment(raw: &str) -> Result<SolanaPaymentPayload, String> {
    if raw.is_empty() {
        return Err("empty header".into());
    }
    let bytes = BASE64
        .decode(raw.trim())
        .map_err(|e| format!("base64 decode failed: {e}"))?;
    let parsed: SolanaPaymentPayload =
        serde_json::from_slice(&bytes).map_err(|e| format!("json parse failed: {e}"))?;
    if parsed.network != SOLANA_DEVNET_NETWORK {
        return Err(format!(
            "unsupported network: expected {SOLANA_DEVNET_NETWORK}, got {}",
            parsed.network
        ));
    }
    Ok(parsed)
}

// ---------------------------------------------------------------------------
// Pay-to resolution
// ---------------------------------------------------------------------------

/// Resolve the recipient `payTo` address using this precedence:
///   1. `AGENTSPAY_DEMO_PAYTO` (explicit; for production demos).
///   2. `AGENTSPAY_PROVIDER_KEYPAIR` (load a JSON keypair, use its pubkey).
///   3. **Persistent default**: load-or-create at `~/.agentspay/provider-keypair.json`
///      with mode 0600. Stable across restarts so other devs can pin a single
///      `payTo` for the demo without setting any env vars.
///   4. As a last resort, the documented [`FALLBACK_PAYTO`] string.
fn resolve_pay_to() -> anyhow::Result<String> {
    if let Ok(addr) = env::var(DEMO_PAYTO_ENV) {
        let trimmed = addr.trim();
        if !trimmed.is_empty() {
            tracing::info!(pay_to = %trimmed, source = "AGENTSPAY_DEMO_PAYTO", "resolved demo pay_to");
            return Ok(trimmed.to_string());
        }
    }
    if let Ok(path) = env::var(PROVIDER_KEYPAIR_ENV) {
        let kp = load_keypair_from_path(StdPath::new(&path))?;
        let pubkey = kp.pubkey().to_string();
        tracing::info!(pay_to = %pubkey, source = "AGENTSPAY_PROVIDER_KEYPAIR", path = %path, "resolved demo pay_to");
        return Ok(pubkey);
    }

    // Persistent default. First run generates a keypair and writes it 0600;
    // subsequent runs reuse it so the demo address stays stable.
    let default_path = default_provider_keypair_path()?;
    let kp = load_or_create_provider_keypair(&default_path)?;
    let pubkey = kp.pubkey().to_string();
    tracing::info!(
        pay_to = %pubkey,
        source = "persistent-default",
        path = %default_path.display(),
        fund_with_sol = "https://faucet.solana.com",
        fund_with_usdc = "https://faucet.circle.com",
        "resolved demo pay_to from persistent provider keypair; fund this address for SPL ATA rent. Override with AGENTSPAY_DEMO_PAYTO."
    );
    if pubkey.is_empty() {
        // Belt-and-suspenders: this should be unreachable but if so, the
        // fallback string keeps the route returning a valid 402.
        return Ok(FALLBACK_PAYTO.to_string());
    }
    Ok(pubkey)
}

fn default_provider_keypair_path() -> anyhow::Result<PathBuf> {
    let home = env::var_os("HOME").map(PathBuf::from).ok_or_else(|| {
        anyhow::anyhow!("$HOME is not set; cannot pick default provider keypair path")
    })?;
    Ok(home.join(DEFAULT_PROVIDER_KEYPAIR_SUBPATH))
}

fn load_keypair_from_path(path: &StdPath) -> anyhow::Result<Keypair> {
    let raw = fs::read_to_string(path)
        .map_err(|e| anyhow::anyhow!("reading provider keypair {}: {e}", path.display()))?;
    let bytes: Vec<u8> = serde_json::from_str(&raw)
        .map_err(|e| anyhow::anyhow!("parsing provider keypair JSON at {}: {e}", path.display()))?;
    Keypair::try_from(bytes.as_slice())
        .map_err(|e| anyhow::anyhow!("invalid provider keypair bytes at {}: {e}", path.display()))
}

fn load_or_create_provider_keypair(path: &StdPath) -> anyhow::Result<Keypair> {
    if path.exists() {
        return load_keypair_from_path(path);
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| anyhow::anyhow!("creating dir {}: {e}", parent.display()))?;
    }
    let kp = Keypair::new();
    let bytes = kp.to_bytes().to_vec();
    let serialized = serde_json::to_string(&bytes)
        .map_err(|e| anyhow::anyhow!("serializing provider keypair: {e}"))?;
    fs::write(path, &serialized)
        .map_err(|e| anyhow::anyhow!("writing provider keypair to {}: {e}", path.display()))?;
    set_owner_only_permissions(path)?;
    Ok(kp)
}

#[cfg(unix)]
fn set_owner_only_permissions(path: &StdPath) -> anyhow::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = fs::metadata(path)
        .map_err(|e| anyhow::anyhow!("stat {}: {e}", path.display()))?
        .permissions();
    perms.set_mode(0o600);
    fs::set_permissions(path, perms)
        .map_err(|e| anyhow::anyhow!("chmod 0600 on {}: {e}", path.display()))
}

#[cfg(not(unix))]
fn set_owner_only_permissions(_path: &StdPath) -> anyhow::Result<()> {
    Ok(())
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

fn init_tracing() {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("agentspay_paid_endpoint_demo=info,tower_http=info"));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stdout))
        .init();
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> anyhow::Result<()> {
    init_tracing();

    let pay_to = resolve_pay_to()?;

    let rpc_url = env::var(RPC_URL_ENV).unwrap_or_else(|_| DEFAULT_RPC.to_string());
    let rpc = Arc::new(RpcClient::new(rpc_url.clone()));

    let http = reqwest::Client::builder()
        .user_agent("agentspay-paid-endpoint-demo/0.3.0")
        .build()
        .map_err(|e| anyhow::anyhow!("building http client: {e}"))?;

    let facilitator = match env::var(USE_FACILITATOR_ENV).as_deref() {
        Ok("true") | Ok("1") | Ok("yes") => {
            let url = env::var(FACILITATOR_URL_ENV)
                .unwrap_or_else(|_| DEFAULT_FACILITATOR_URL.to_string());
            tracing::info!(facilitator_url = %url, "facilitator mode enabled (verify+settle forwarded)");
            Some(url)
        }
        _ => {
            tracing::info!(rpc_url = %rpc_url, "direct-RPC mode (no facilitator)");
            None
        }
    };
    tracing::info!(rpc_url = %rpc_url, pay_to = %pay_to, "solana RPC configured");

    let state = AppState {
        pay_to,
        rpc,
        http,
        facilitator,
    };

    let app = Router::new()
        .route("/health", get(health))
        .route("/quote/:symbol", get(sandbox_quote))
        .route("/real-quote/:symbol", get(real_quote))
        .with_state(state);

    let addr: SocketAddr = LISTEN_ADDR.parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!(
        %addr,
        routes = "/health, /quote/:symbol (sandbox), /real-quote/:symbol (solana-devnet)",
        "agentspay-paid-endpoint-demo listening"
    );

    axum::serve(listener, app).await?;
    Ok(())
}
