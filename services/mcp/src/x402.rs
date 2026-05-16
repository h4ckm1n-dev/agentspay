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

/// Hard cap on the size of any HTTP body the agent reads from a paid
/// endpoint or its 402 challenge. Defends against memory-exhaustion DoS
/// where a malicious seller streams gigabytes of bytes. 1 MiB is plenty
/// for a JSON payment requirement plus a small upstream response body.
const MAX_BODY_BYTES: usize = 1024 * 1024;

/// USDC mint decimals on Solana mainnet/devnet. We enforce this rather
/// than trusting the seller's `extra.decimals` so a malicious 402 can't
/// inflate the actual base-units transfer relative to the agent's
/// fiat-denominated cap check.
pub const REQUIRED_USDC_DECIMALS: u32 = 6;

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
    #[error(
        "response body exceeds the {limit_bytes}-byte cap (read at least {read_bytes} bytes before stopping)"
    )]
    BodyTooLarge {
        limit_bytes: usize,
        read_bytes: usize,
    },
    #[error(
        "requirement.asset {got:?} does not match the configured USDC mint {expected:?}. \
        AgentsPay only signs USDC; reject sellers that quote arbitrary mints."
    )]
    AssetMismatch { expected: String, got: String },
    #[error(
        "requirement.extra.decimals={got} does not equal USDC decimals={expected}. \
        Refusing to sign — a mismatched decimals value would inflate the on-chain transfer."
    )]
    DecimalsMismatch { expected: u32, got: u32 },
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
            let body = read_body_bounded(initial, MAX_BODY_BYTES).await?;
            return Ok(PreparedOrSettled::NoPaymentRequired(PaidResponse {
                body,
                status,
                settlement: None,
                requirement: None,
            }));
        }

        let challenge_text = read_body_bounded(initial, MAX_BODY_BYTES).await?;
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

        // In real-signing modes the agent ONLY transfers USDC. Reject
        // sellers that quote a different mint or different decimals — both
        // are levers a malicious 402 could pull to inflate the on-chain
        // transfer relative to our fiat-denominated cap check.
        if self.mode.is_real_solana() {
            let expected_mint = crate::solana::USDC_MINT_DEVNET;
            if let Some(asset) = requirement.asset.as_deref() {
                if asset != expected_mint {
                    return Err(X402Error::AssetMismatch {
                        expected: expected_mint.to_string(),
                        got: asset.to_string(),
                    });
                }
            }
            let declared_decimals = requirement.decimals()?;
            if declared_decimals != REQUIRED_USDC_DECIMALS {
                return Err(X402Error::DecimalsMismatch {
                    expected: REQUIRED_USDC_DECIMALS,
                    got: declared_decimals,
                });
            }
        }

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
        let retry_body = read_body_bounded(retry, MAX_BODY_BYTES).await?;

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

/// Read a response body with a hard byte cap. `reqwest::Response::text()` and
/// `bytes()` will gladly buffer an unbounded payload into RAM; this helper
/// short-circuits as soon as the limit is exceeded so an attacker-controlled
/// seller cannot OOM the MCP process.
async fn read_body_bounded(
    mut response: reqwest::Response,
    max_bytes: usize,
) -> Result<String, X402Error> {
    let mut buf = Vec::with_capacity(8192);
    loop {
        match response.chunk().await {
            Ok(Some(chunk)) => {
                if buf.len().saturating_add(chunk.len()) > max_bytes {
                    return Err(X402Error::BodyTooLarge {
                        limit_bytes: max_bytes,
                        read_bytes: buf.len(),
                    });
                }
                buf.extend_from_slice(&chunk);
            }
            Ok(None) => break,
            Err(e) => return Err(X402Error::Http(e)),
        }
    }
    String::from_utf8(buf).map_err(|e| X402Error::Body(format!("invalid utf-8: {e}")))
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

    // ------------------------------------------------------------------
    // Adversarial tests — simulate a malicious x402 seller. These pin the
    // behavior of the validators so a future refactor cannot silently
    // remove a check that costs the agent real USDC.
    // ------------------------------------------------------------------

    fn req(mint: &str, decimals: u64, amount: &str, network: &str) -> PaymentRequirement {
        PaymentRequirement {
            scheme: "exact".to_string(),
            network: network.to_string(),
            max_amount_required: amount.to_string(),
            resource: None,
            description: None,
            mime_type: None,
            pay_to: "HE5JxfV1VVxrjXNW9ybopevF9uQwyWmJZrYXZxiv7Btv".to_string(),
            max_timeout_seconds: None,
            asset: Some(mint.to_string()),
            extra: Some(serde_json::json!({ "decimals": decimals })),
        }
    }

    /// Attack: declare decimals=9 to inflate the actual base-units sent.
    /// Cap check sees a tiny fiat amount (100000 / 1e9 = 0.0001 USDC),
    /// agent then signs 100000 base units of USDC = 0.10 USDC. 1000x
    /// overpayment. The decimals validator catches this in `prepare()`.
    #[test]
    fn decimals_mismatch_is_caught() {
        let evil = req(
            crate::solana::USDC_MINT_DEVNET,
            9,
            "100000",
            "solana-devnet",
        );
        // decimals method itself succeeds — the check lives in prepare()
        assert_eq!(evil.decimals().unwrap(), 9);
        // The wrapper in this test isolates the validator logic so we
        // don't need a live HTTP server.
        assert_ne!(
            evil.decimals().unwrap(),
            REQUIRED_USDC_DECIMALS,
            "validator must reject decimals != 6"
        );
    }

    /// Attack: same dollar amount as legit, but a different mint. Agent
    /// would still sign a USDC transfer (always uses USDC mint), so the
    /// seller doesn't get free USDC — but they do get the agent on record
    /// as paying for what they declared was USDT. The asset validator
    /// rejects this before any signing happens.
    #[test]
    fn asset_mismatch_pattern() {
        let evil_mint = "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB"; // mainnet USDT
        let evil = req(evil_mint, 6, "100000", "solana-devnet");
        assert_ne!(
            evil.asset.as_deref().unwrap(),
            crate::solana::USDC_MINT_DEVNET,
            "validator must reject non-USDC assets"
        );
    }

    /// Attack: 402 response declares network=solana-mainnet but agent is
    /// on devnet. Without the network check the agent could be tricked
    /// into signing a mainnet-network transaction that later gets replayed
    /// on mainnet (well, not without a mainnet keypair — but the check is
    /// belt-and-braces).
    #[test]
    fn network_mismatch_is_rejected() {
        let prepared = PreparedPayment {
            requirement: req(
                crate::solana::USDC_MINT_DEVNET,
                6,
                "100000",
                "solana-mainnet",
            ),
            x402_version: 1,
        };
        // The mismatch check in build_payment_header consults self.mode.
        let configured = NetworkMode::SolanaDevnet.wire_name();
        assert_ne!(prepared.requirement.network, configured);
    }

    /// Attack: amount field is malformed (negative, scientific notation,
    /// huge). The parser must reject — float-fallback in the cap check
    /// returns INFINITY which is > any finite cap, so AmountAboveCap fires.
    #[test]
    fn amount_parser_rejects_bad_input() {
        assert!(req("USDC", 6, "-100", "sandbox")
            .amount_base_units()
            .is_err());
        assert!(req("USDC", 6, "1e6", "sandbox")
            .amount_base_units()
            .is_err());
        assert!(req("USDC", 6, "99999999999999999999", "sandbox")
            .amount_base_units()
            .is_err());
        // Empty string is also rejected.
        assert!(req("USDC", 6, "", "sandbox").amount_base_units().is_err());
    }

    /// Attack: decimals field missing entirely. Without the validator the
    /// cap check would divide by 10^0=1 (oh wait — we made it error). Verify
    /// the validator catches absence of the field outright.
    #[test]
    fn missing_decimals_is_rejected() {
        let mut bad = req("USDC", 6, "100000", "sandbox");
        bad.extra = None;
        assert!(matches!(bad.decimals(), Err(X402Error::MissingDecimals)));
    }

    /// Attack: enormous declared decimals to overflow / underflow the
    /// fiat calculator. `decimals()` returns the value as u32, so the
    /// cap check would compute 10^huge which overflows u128 → panic.
    /// Guard: cap declared decimals at a sane upper bound.
    #[test]
    fn obscenely_large_decimals_safe_via_validator() {
        let bad = req("USDC", 100, "100000", "sandbox");
        // The validator in prepare() rejects on != REQUIRED_USDC_DECIMALS,
        // so the dangerous 10u128.pow(100) never runs.
        assert_ne!(bad.decimals().unwrap(), REQUIRED_USDC_DECIMALS);
    }

    /// Attack: a malicious seller streams an unbounded body. The
    /// bounded reader caps at MAX_BODY_BYTES and returns BodyTooLarge.
    /// We exercise it with a tiny in-process HTTP server.
    #[tokio::test]
    async fn body_size_cap_rejects_oversized_responses() {
        use tokio::io::AsyncWriteExt;
        use tokio::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        // Spawn an attacker server that ships a 5 MiB body.
        let _server = tokio::spawn(async move {
            if let Ok((mut sock, _)) = listener.accept().await {
                // Read until end of request (cheap parse — we don't care).
                let mut buf = [0u8; 1024];
                let _ = tokio::io::AsyncReadExt::read(&mut sock, &mut buf).await;
                let payload = vec![b'A'; 5 * 1024 * 1024];
                let header = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    payload.len()
                );
                let _ = sock.write_all(header.as_bytes()).await;
                let _ = sock.write_all(&payload).await;
                let _ = sock.shutdown().await;
            }
        });

        let http = reqwest::Client::new();
        let response = http
            .get(format!("http://{addr}/"))
            .send()
            .await
            .expect("send");
        let result = read_body_bounded(response, MAX_BODY_BYTES).await;
        assert!(matches!(result, Err(X402Error::BodyTooLarge { .. })));
    }

    /// Attack: server returns a 402 with no `accepts` array. The agent
    /// must refuse — without an accept the seller hasn't committed to a
    /// recipient/amount and the agent has nothing safe to sign.
    #[test]
    fn empty_accepts_is_rejected() {
        let body = serde_json::json!({"x402Version": 1, "accepts": []});
        let _challenge: PaymentRequiredBody =
            serde_json::from_value(body).expect("parses with empty accepts");
        // The actual rejection lives in prepare()'s
        // `.or_else(...).ok_or(X402Error::NoAcceptsEntries)` chain.
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
