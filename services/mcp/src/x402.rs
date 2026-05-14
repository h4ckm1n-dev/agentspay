//! x402 client with two settlement back-ends.
//!
//! 1. **Real mode** (`AGENTSPAY_NETWORK=solana-devnet`, default):
//!    sign a real SPL-token USDC `transfer_checked` transaction with the
//!    local [`AgentWallet`] and base64-encode it into the `payload.transaction`
//!    field of the Solana exact-scheme x402 payload. The demo seller then
//!    relays the signed transaction to the configured Solana RPC.
//!
//! 2. **Sandbox mode** (`AGENTSPAY_NETWORK=sandbox`):
//!    preserve the v0.2 wire format with a constant fake
//!    `signature: "sandbox-signature-no-crypto"` and a synthetic EIP-3009-style
//!    authorization. No on-chain activity. Used by the offline
//!    `examples/paid-endpoint/` demo against `/quote/:symbol`.
//!
//! The flow is split into [`X402Client::prepare`] (HTTP probe + parse 402
//! challenge + agent-side cap) and [`X402Client::complete`] (sign + retry).
//! The caller injects a budget check between them — see `main.rs`.

use std::sync::Arc;

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use chrono::Utc;
use reqwest::{header::HeaderMap, StatusCode};
use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use thiserror::Error;
use uuid::Uuid;

use crate::{
    solana::{build_usdc_transfer_tx, default_rpc_url, encode_transaction_b64},
    wallet::AgentWallet,
};

pub const SANDBOX_NETWORK: &str = "sandbox";
pub const SOLANA_DEVNET_NETWORK: &str = "solana-devnet";
pub const SOLANA_MAINNET_NETWORK: &str = "solana-mainnet";

/// Sandbox-mode placeholder address used in the EIP-3009-style authorization.
pub const SANDBOX_FROM_ADDRESS: &str = "agentspay_local_agent";
/// Sandbox-mode placeholder signature.
pub const SANDBOX_SIGNATURE: &str = "sandbox-signature-no-crypto";
pub const X_PAYMENT: &str = "X-Payment";
pub const X_PAYMENT_RESPONSE: &str = "x-payment-response";

#[derive(Debug, Error)]
pub enum X402Error {
    #[error("http request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("response body could not be read: {0}")]
    Body(String),
    #[error("server returned 402 without a parseable body: {0}")]
    Malformed402(String),
    #[error("server response missing required accepts[] entries")]
    NoAcceptsEntries,
    #[error("server response missing extra.decimals; cannot compute fiat amount")]
    MissingDecimals,
    #[error("server returned unexpected status {status} on retry: {body}")]
    UnexpectedRetryStatus { status: StatusCode, body: String },
    #[error("amount {required_usdc} exceeds max_amount_usdc cap {cap_usdc}")]
    AmountAboveCap {
        required_usdc: String,
        cap_usdc: String,
    },
    #[error("server quoted amount {0} cannot be parsed as a non-negative integer")]
    BadAmount(String),
    #[error("X-PAYMENT-RESPONSE header is missing on the settled response")]
    MissingPaymentResponse,
    #[error("X-PAYMENT-RESPONSE header is not valid base64 JSON: {0}")]
    BadPaymentResponse(String),
    #[error(
        "server requirement network={requirement:?} does not match \
        the locally configured network={configured:?}"
    )]
    NetworkMismatch {
        requirement: String,
        configured: String,
    },
    #[error("requirement.payTo {0:?} is not a valid Solana pubkey: {1}")]
    InvalidPayTo(String, String),
    #[error("failed to serialize x402 payload: {0}")]
    Serialize(String),
    #[error("solana transaction build failed: {0}")]
    SolanaBuild(String),
    #[error("agent wallet is not configured but the active network requires real signing")]
    WalletMissing,
}

// ---------------------------------------------------------------------------
// 402 challenge shapes (parsed from the seller)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct PaymentRequiredBody {
    #[serde(rename = "x402Version", default)]
    pub x402_version: Option<u8>,
    /// Optional human-readable error from the seller. Surfaced in tracing,
    /// not consumed structurally — kept so the JSON shape matches the spec.
    #[serde(default)]
    #[allow(dead_code)]
    pub error: Option<String>,
    pub accepts: Vec<PaymentRequirement>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)] // several fields are part of the wire shape but unused in this client.
pub struct PaymentRequirement {
    pub scheme: String,
    pub network: String,
    #[serde(rename = "maxAmountRequired")]
    pub max_amount_required: String,
    #[serde(default)]
    pub resource: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(rename = "mimeType", default)]
    pub mime_type: Option<String>,
    #[serde(rename = "payTo")]
    pub pay_to: String,
    #[serde(rename = "maxTimeoutSeconds", default)]
    pub max_timeout_seconds: Option<u32>,
    #[serde(default)]
    pub asset: Option<String>,
    #[serde(default)]
    pub extra: Option<serde_json::Value>,
}

impl PaymentRequirement {
    pub fn decimals(&self) -> Result<u32, X402Error> {
        self.extra
            .as_ref()
            .and_then(|v| v.get("decimals"))
            .and_then(|v| v.as_u64())
            .map(|v| v as u32)
            .ok_or(X402Error::MissingDecimals)
    }

    /// Convert `max_amount_required` (base units) to a decimal string in USDC.
    pub fn required_usdc(&self) -> Result<String, X402Error> {
        let decimals = self.decimals()?;
        let base: u128 = self
            .max_amount_required
            .parse()
            .map_err(|_| X402Error::BadAmount(self.max_amount_required.clone()))?;
        Ok(format_fixed_point(base, decimals))
    }

    /// Parse `max_amount_required` as a `u64` of token base units. Used by
    /// the Solana signing path.
    pub fn amount_base_units(&self) -> Result<u64, X402Error> {
        self.max_amount_required
            .parse::<u64>()
            .map_err(|_| X402Error::BadAmount(self.max_amount_required.clone()))
    }
}

// ---------------------------------------------------------------------------
// Outgoing payload shapes
// ---------------------------------------------------------------------------

/// Sandbox-mode payload — preserves the original EIP-3009 envelope used by
/// the Coinbase-CDP-style demo seller. Real cryptography is intentionally
/// omitted; the seller accepts any non-empty X-Payment header.
#[derive(Debug, Serialize)]
struct SandboxPaymentPayload<'a> {
    #[serde(rename = "x402Version")]
    x402_version: u8,
    scheme: &'a str,
    network: &'a str,
    payload: SandboxInner<'a>,
}

#[derive(Debug, Serialize)]
struct SandboxInner<'a> {
    signature: &'a str,
    authorization: Authorization<'a>,
}

#[derive(Debug, Serialize)]
struct Authorization<'a> {
    from: &'a str,
    to: &'a str,
    value: &'a str,
    #[serde(rename = "validAfter")]
    valid_after: String,
    #[serde(rename = "validBefore")]
    valid_before: String,
    nonce: String,
}

/// Solana exact-scheme payload. The `transaction` field is the base64 of a
/// `bincode`-serialized signed Solana `Transaction` performing an SPL token
/// transfer to `requirement.payTo`. The facilitator (or demo seller, in our
/// case) submits it to devnet RPC.
#[derive(Debug, Serialize)]
struct SolanaPaymentPayload<'a> {
    #[serde(rename = "x402Version")]
    x402_version: u8,
    scheme: &'a str,
    network: &'a str,
    payload: SolanaInner,
}

#[derive(Debug, Serialize)]
struct SolanaInner {
    /// Base64 of a `bincode`-serialized signed Solana `Transaction`.
    transaction: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)] // success/network/payer parsed for spec-conformance, logged via Debug only.
pub struct PaymentResponseHeader {
    #[serde(default)]
    pub success: bool,
    #[serde(default)]
    pub transaction: String,
    #[serde(default)]
    pub network: String,
    #[serde(default)]
    pub payer: String,
}

// ---------------------------------------------------------------------------
// Public outcomes
// ---------------------------------------------------------------------------

pub struct PaidResponse {
    pub body: String,
    pub status: StatusCode,
    pub settlement: Option<PaymentResponseHeader>,
    pub requirement: Option<PaymentRequirement>,
}

pub struct PreparedPayment {
    pub requirement: PaymentRequirement,
    pub x402_version: u8,
}

pub enum PreparedOrSettled {
    NoPaymentRequired(PaidResponse),
    PaymentRequired(PreparedPayment),
}

// ---------------------------------------------------------------------------
// Network mode
// ---------------------------------------------------------------------------

/// Which settlement back-end to use. Selected at MCP startup from
/// `AGENTSPAY_NETWORK`; defaults to [`NetworkMode::SolanaDevnet`].
#[derive(Clone, Debug)]
pub enum NetworkMode {
    /// Original v0.2 sandbox path: fake signature, no cryptography. Kept so
    /// the offline `examples/paid-endpoint/` `/quote/:symbol` route still
    /// works after the v0.3 cutover.
    Sandbox,
    /// Real Solana devnet signing using the [`AgentWallet`] keypair.
    SolanaDevnet,
    /// Real Solana mainnet signing. Same code path as devnet; we never
    /// hardcode mainnet anywhere by default, but the variant exists so an
    /// operator can opt in via the env var.
    SolanaMainnet,
}

impl NetworkMode {
    pub fn wire_name(&self) -> &'static str {
        match self {
            NetworkMode::Sandbox => SANDBOX_NETWORK,
            NetworkMode::SolanaDevnet => SOLANA_DEVNET_NETWORK,
            NetworkMode::SolanaMainnet => SOLANA_MAINNET_NETWORK,
        }
    }

    /// True when the active mode signs a real Solana transaction.
    pub fn is_real_solana(&self) -> bool {
        matches!(self, NetworkMode::SolanaDevnet | NetworkMode::SolanaMainnet)
    }

    /// Parse from the `AGENTSPAY_NETWORK` env var. Unknown values fall back
    /// to [`NetworkMode::SolanaDevnet`] with a warning so a misconfigured
    /// env var doesn't silently regress to sandbox mode.
    pub fn from_env() -> Self {
        match std::env::var("AGENTSPAY_NETWORK")
            .ok()
            .as_deref()
            .map(str::trim)
        {
            Some(SANDBOX_NETWORK) => NetworkMode::Sandbox,
            Some(SOLANA_DEVNET_NETWORK) => NetworkMode::SolanaDevnet,
            Some(SOLANA_MAINNET_NETWORK) => NetworkMode::SolanaMainnet,
            Some(other) if !other.is_empty() => {
                tracing::warn!(value = %other, "unknown AGENTSPAY_NETWORK, defaulting to solana-devnet");
                NetworkMode::SolanaDevnet
            }
            _ => NetworkMode::SolanaDevnet,
        }
    }
}

// ---------------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------------

pub struct X402Client<'a> {
    http: &'a reqwest::Client,
    mode: NetworkMode,
    /// `None` only in sandbox mode. Required for any real Solana signing.
    wallet: Option<Arc<AgentWallet>>,
    /// Lazily-constructed devnet RPC client used to fetch a recent blockhash
    /// when building the SPL transfer transaction. Cheap to recreate, but
    /// passed by reference so the caller controls the lifetime.
    rpc: Option<Arc<RpcClient>>,
}

impl<'a> X402Client<'a> {
    /// Construct a sandbox-mode client (preserves v0.2 behavior).
    pub fn sandbox(http: &'a reqwest::Client) -> Self {
        Self {
            http,
            mode: NetworkMode::Sandbox,
            wallet: None,
            rpc: None,
        }
    }

    /// Construct a real-signing client.
    pub fn solana(
        http: &'a reqwest::Client,
        mode: NetworkMode,
        wallet: Arc<AgentWallet>,
        rpc: Arc<RpcClient>,
    ) -> Self {
        debug_assert!(mode.is_real_solana(), "use Self::sandbox for sandbox mode");
        Self {
            http,
            mode,
            wallet: Some(wallet),
            rpc: Some(rpc),
        }
    }

    /// Pick the right constructor based on `mode`.
    pub fn for_mode(
        http: &'a reqwest::Client,
        mode: NetworkMode,
        wallet: Option<Arc<AgentWallet>>,
    ) -> Result<Self, X402Error> {
        match mode {
            NetworkMode::Sandbox => Ok(Self::sandbox(http)),
            NetworkMode::SolanaDevnet | NetworkMode::SolanaMainnet => {
                let wallet = wallet.ok_or(X402Error::WalletMissing)?;
                let rpc = Arc::new(RpcClient::new(default_rpc_url()));
                Ok(Self::solana(http, mode, wallet, rpc))
            }
        }
    }

    #[allow(dead_code)]
    pub fn mode(&self) -> &NetworkMode {
        &self.mode
    }

    /// Phase 1: GET + parse 402 + apply agent cap.
    pub async fn prepare(
        &self,
        url: &str,
        max_amount_usdc: f64,
    ) -> Result<PreparedOrSettled, X402Error> {
        let initial = self.http.get(url).send().await?;
        let status = initial.status();

        if status != StatusCode::PAYMENT_REQUIRED {
            let body = initial
                .text()
                .await
                .map_err(|e| X402Error::Body(e.to_string()))?;
            return Ok(PreparedOrSettled::NoPaymentRequired(PaidResponse {
                body,
                status,
                settlement: None,
                requirement: None,
            }));
        }

        let challenge_text = initial
            .text()
            .await
            .map_err(|e| X402Error::Body(e.to_string()))?;
        let challenge: PaymentRequiredBody = serde_json::from_str(&challenge_text)
            .map_err(|e| X402Error::Malformed402(format!("{e}: {challenge_text}")))?;

        // Prefer the first accept that matches our active network so an
        // ambiguous seller (e.g. one that lists both sandbox and devnet)
        // can still settle with us. Falls back to the first entry — the
        // network-mismatch check in `complete()` then yields a clear error.
        let configured = self.mode.wire_name();
        let requirement = challenge
            .accepts
            .iter()
            .find(|r| r.network == configured)
            .cloned()
            .or_else(|| challenge.accepts.into_iter().next())
            .ok_or(X402Error::NoAcceptsEntries)?;

        let required_str = requirement.required_usdc()?;
        let required_f64: f64 = required_str.parse().unwrap_or(f64::INFINITY);
        if required_f64 > max_amount_usdc {
            return Err(X402Error::AmountAboveCap {
                required_usdc: required_str,
                cap_usdc: format!("{max_amount_usdc}"),
            });
        }

        Ok(PreparedOrSettled::PaymentRequired(PreparedPayment {
            requirement,
            x402_version: challenge.x402_version.unwrap_or(1),
        }))
    }

    /// Phase 2: sign + retry. Strategy varies by [`NetworkMode`].
    pub async fn complete(
        &self,
        url: &str,
        prepared: PreparedPayment,
    ) -> Result<PaidResponse, X402Error> {
        let encoded = self.build_payment_header(&prepared).await?;
        let PreparedPayment { requirement, .. } = prepared;

        let retry = self
            .http
            .get(url)
            .header(X_PAYMENT, &encoded)
            .send()
            .await?;
        let retry_status = retry.status();
        let retry_headers = retry.headers().clone();
        let retry_body = retry
            .text()
            .await
            .map_err(|e| X402Error::Body(e.to_string()))?;

        if !retry_status.is_success() {
            return Err(X402Error::UnexpectedRetryStatus {
                status: retry_status,
                body: retry_body,
            });
        }

        let settlement = parse_payment_response(&retry_headers)?;
        Ok(PaidResponse {
            body: retry_body,
            status: retry_status,
            settlement: Some(settlement),
            requirement: Some(requirement),
        })
    }

    async fn build_payment_header(&self, prepared: &PreparedPayment) -> Result<String, X402Error> {
        let configured = self.mode.wire_name();
        if prepared.requirement.network != configured {
            return Err(X402Error::NetworkMismatch {
                requirement: prepared.requirement.network.clone(),
                configured: configured.to_string(),
            });
        }

        match self.mode {
            NetworkMode::Sandbox => Ok(build_sandbox_header(prepared)),
            NetworkMode::SolanaDevnet | NetworkMode::SolanaMainnet => {
                let wallet = self.wallet.as_ref().ok_or(X402Error::WalletMissing)?;
                let rpc = self.rpc.as_ref().ok_or(X402Error::WalletMissing)?;
                build_solana_header(wallet.as_ref(), rpc.as_ref(), prepared).await
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Payload builders
// ---------------------------------------------------------------------------

fn build_sandbox_header(prepared: &PreparedPayment) -> String {
    let now = Utc::now().timestamp();
    let auth = Authorization {
        from: SANDBOX_FROM_ADDRESS,
        to: &prepared.requirement.pay_to,
        value: &prepared.requirement.max_amount_required,
        valid_after: now.to_string(),
        valid_before: (now + 60).to_string(),
        nonce: Uuid::new_v4().to_string(),
    };
    let payload = SandboxPaymentPayload {
        x402_version: prepared.x402_version,
        scheme: &prepared.requirement.scheme,
        network: &prepared.requirement.network,
        payload: SandboxInner {
            signature: SANDBOX_SIGNATURE,
            authorization: auth,
        },
    };
    // serde_json::to_vec on a struct with only owned/owned-borrowed strings
    // cannot fail in practice.
    let bytes = serde_json::to_vec(&payload).expect("sandbox payload serializes");
    BASE64.encode(bytes)
}

async fn build_solana_header(
    wallet: &AgentWallet,
    rpc: &RpcClient,
    prepared: &PreparedPayment,
) -> Result<String, X402Error> {
    let recipient: Pubkey = prepared.requirement.pay_to.parse().map_err(
        |e: solana_sdk::pubkey::ParsePubkeyError| {
            X402Error::InvalidPayTo(prepared.requirement.pay_to.clone(), e.to_string())
        },
    )?;
    let amount = prepared.requirement.amount_base_units()?;

    tracing::info!(
        payer = %wallet.pubkey_base58(),
        recipient = %recipient,
        amount_base_units = amount,
        network = %prepared.requirement.network,
        "signing solana usdc transfer for x402 payment"
    );

    let tx = build_usdc_transfer_tx(rpc, wallet.keypair(), &recipient, amount)
        .await
        .map_err(|e| X402Error::SolanaBuild(e.to_string()))?;
    let encoded_tx =
        encode_transaction_b64(&tx).map_err(|e| X402Error::Serialize(e.to_string()))?;

    let payload = SolanaPaymentPayload {
        x402_version: prepared.x402_version,
        scheme: &prepared.requirement.scheme,
        network: &prepared.requirement.network,
        payload: SolanaInner {
            transaction: encoded_tx,
        },
    };
    let bytes = serde_json::to_vec(&payload).map_err(|e| X402Error::Serialize(e.to_string()))?;
    Ok(BASE64.encode(bytes))
}

fn parse_payment_response(headers: &HeaderMap) -> Result<PaymentResponseHeader, X402Error> {
    let raw = headers
        .get(X_PAYMENT_RESPONSE)
        .ok_or(X402Error::MissingPaymentResponse)?;
    let raw_str = raw
        .to_str()
        .map_err(|e| X402Error::BadPaymentResponse(e.to_string()))?;
    let bytes = BASE64
        .decode(raw_str.trim())
        .map_err(|e| X402Error::BadPaymentResponse(format!("base64 decode failed: {e}")))?;
    let parsed: PaymentResponseHeader = serde_json::from_slice(&bytes)
        .map_err(|e| X402Error::BadPaymentResponse(format!("json parse failed: {e}")))?;
    Ok(parsed)
}

/// Format `base_units / 10^decimals` as a fixed-point decimal string.
fn format_fixed_point(base_units: u128, decimals: u32) -> String {
    if decimals == 0 {
        return base_units.to_string();
    }
    let divisor = 10u128.pow(decimals);
    let whole = base_units / divisor;
    let frac = base_units % divisor;
    let frac_str = format!("{frac:0width$}", width = decimals as usize);
    let trimmed = frac_str.trim_end_matches('0');
    if trimmed.is_empty() {
        format!("{whole}.00")
    } else if trimmed.len() == 1 {
        format!("{whole}.{trimmed}0")
    } else {
        format!("{whole}.{trimmed}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_point_basic() {
        assert_eq!(format_fixed_point(100_000, 6), "0.10");
        assert_eq!(format_fixed_point(500_000, 6), "0.50");
        assert_eq!(format_fixed_point(1_000_000, 6), "1.00");
        assert_eq!(format_fixed_point(1_500_000, 6), "1.50");
        assert_eq!(format_fixed_point(0, 6), "0.00");
        assert_eq!(format_fixed_point(1, 6), "0.000001");
    }

    #[test]
    fn network_mode_wire_names() {
        assert_eq!(NetworkMode::Sandbox.wire_name(), "sandbox");
        assert_eq!(NetworkMode::SolanaDevnet.wire_name(), "solana-devnet");
        assert_eq!(NetworkMode::SolanaMainnet.wire_name(), "solana-mainnet");
    }

    #[test]
    fn sandbox_header_round_trips_as_json() {
        let prepared = PreparedPayment {
            requirement: PaymentRequirement {
                scheme: "exact".to_string(),
                network: "sandbox".to_string(),
                max_amount_required: "100000".to_string(),
                resource: None,
                description: None,
                mime_type: None,
                pay_to: "FakeRecipient1111111111111111111111111111111".to_string(),
                max_timeout_seconds: None,
                asset: None,
                extra: Some(serde_json::json!({ "decimals": 6 })),
            },
            x402_version: 1,
        };
        let encoded = build_sandbox_header(&prepared);
        let decoded = BASE64.decode(&encoded).expect("base64");
        let parsed: serde_json::Value = serde_json::from_slice(&decoded).expect("json");
        assert_eq!(parsed["network"], "sandbox");
        assert_eq!(parsed["payload"]["signature"], SANDBOX_SIGNATURE);
    }
}
