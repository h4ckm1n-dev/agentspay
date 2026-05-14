//! AgentsPay MCP server (v0.3 — Week 3 Step 1).
//!
//! Exposes five MCP tools — `agentspay_balance`, `agentspay_pay_url`,
//! `agentspay_set_budget`, `agentspay_audit_log`, `agentspay_topup_info` —
//! over stdio so MCP hosts (Claude Code, Cursor, Cline) can hand any agent
//! a budget-controlled USDC wallet that pays real x402 endpoints on
//! Solana devnet.
//!
//! The previous sandbox path is preserved behind `AGENTSPAY_NETWORK=sandbox`
//! so the offline `examples/paid-endpoint/` demo against `/quote/:symbol`
//! continues to work. The default network is `solana-devnet`.

mod db;
mod entities;
mod migration;
mod repo;
mod solana;
mod wallet;
mod x402;

use std::{sync::Arc, time::Duration};

use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{
        CallToolResult, Content, Implementation, ProtocolVersion, ServerCapabilities, ServerInfo,
    },
    schemars, tool, tool_handler, tool_router,
    transport::stdio,
    ErrorData as McpError, ServerHandler, ServiceExt,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use thiserror::Error;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use uuid::Uuid;

use crate::{
    repo::{new_audit, new_ledger_entry, LedgerRepo},
    wallet::AgentWallet,
    x402::{NetworkMode, PaidResponse, PreparedOrSettled, X402Client, X402Error},
};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const DEFAULT_AGENT_ID: &str = "default";
const DEFAULT_AUDIT_LIMIT: u32 = 20;
const MAX_AUDIT_LIMIT: u32 = 100;
const CURRENCY: &str = "USDC";
const HTTP_USER_AGENT: &str = "agentspay-mcp/0.3.0";
const HTTP_TIMEOUT_SECS: u64 = 30;
/// Budget shown when the user has not called `agentspay_set_budget` yet.
const DEFAULT_DAILY_BUDGET_USD: f64 = 50.0;
const CIRCLE_FAUCET_URL: &str = "https://faucet.circle.com";
const SOL_FAUCET_URL: &str = "https://faucet.solana.com";
const TOPUP_INSTRUCTIONS: &str = "1. Open the faucet URL in a browser. \
    2. Select 'Solana Devnet'. \
    3. Paste the pubkey. \
    4. Solve the captcha and request 10 USDC. \
    Funds arrive in ~30 seconds. You also need a small amount of SOL for \
    transaction fees — visit https://faucet.solana.com with the same pubkey.";

// ---------------------------------------------------------------------------
// Domain wire shapes (MCP responses)
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, schemars::JsonSchema)]
struct BalanceResponse {
    available_usdc: String,
    budget_remaining_today_usdc: String,
    daily_cap_usdc: String,
    per_call_cap_usdc: String,
    today_spent_usdc: String,
    currency: String,
    environment: String,
    /// Base58 Solana public key the agent will sign x402 payments with.
    solana_pubkey: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct PayUrlRequest {
    /// HTTP/HTTPS URL of the x402-priced endpoint to call.
    url: String,
    /// Maximum amount of USDC to authorise for this call (decimal string).
    max_amount_usdc: String,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
struct PayUrlResponse {
    status: String,
    payment_id: String,
    endpoint: String,
    amount_charged_usdc: String,
    ledger_entry_id: String,
    transaction: String,
    /// The resource body returned by the upstream endpoint, as a string.
    body: String,
    /// `"paid"` when settlement happened, `"none"` for endpoints that
    /// served 200 without ever issuing a 402 challenge.
    payment_status: String,
    /// `"sandbox"`, `"solana-devnet"`, or `"solana-mainnet"`.
    network: String,
    /// Solscan URL for the on-chain TX when applicable; empty otherwise.
    explorer_url: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct SetBudgetRequest {
    /// Daily spending cap in USD (must be > 0).
    daily_usd: f64,
    /// Per-call spending cap in USD (must be > 0).
    per_call_usd: f64,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
struct SetBudgetResponse {
    agent_id: String,
    daily_usd: f64,
    per_call_usd: f64,
    updated_at_rfc3339: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct AuditLogRequest {
    /// Number of entries to return (default 20, max 100).
    #[serde(default)]
    limit: Option<u32>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
struct AuditEntryDto {
    id: String,
    timestamp_rfc3339: String,
    tool: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    endpoint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    amount_usdc: Option<String>,
    status: String,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
struct AuditLogResponse {
    entries: Vec<AuditEntryDto>,
    total: u64,
    returned: usize,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
struct TopupInfoResponse {
    pubkey: String,
    network: String,
    faucet_url: String,
    sol_faucet_url: String,
    instructions: String,
}

// ---------------------------------------------------------------------------
// Server
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct AgentsPayServer {
    repo: Arc<LedgerRepo>,
    wallet: Arc<AgentWallet>,
    network: NetworkMode,
    http: reqwest::Client,
    /// Serializes the budget-check → settle → record path so two concurrent
    /// `pay_url` invocations cannot both pass a budget check against the
    /// same uncommitted view of `today_spending`. Held for the duration of
    /// a single pay_url; non-pay tools never acquire it.
    pay_lock: Arc<tokio::sync::Mutex<()>>,
    tool_router: ToolRouter<Self>,
}

impl AgentsPayServer {
    fn new(
        repo: Arc<LedgerRepo>,
        wallet: Arc<AgentWallet>,
        network: NetworkMode,
        http: reqwest::Client,
    ) -> Self {
        Self {
            repo,
            wallet,
            network,
            http,
            pay_lock: Arc::new(tokio::sync::Mutex::new(())),
            tool_router: Self::tool_router(),
        }
    }

    fn environment(&self) -> &'static str {
        self.network.wire_name()
    }
}

// ---------------------------------------------------------------------------
// Tool router
// ---------------------------------------------------------------------------

#[tool_router]
impl AgentsPayServer {
    #[tool(
        description = "Get current USDC balance, today's spending, and remaining daily budget for the agent."
    )]
    async fn agentspay_balance(&self) -> Result<CallToolResult, McpError> {
        let wallet_row = self
            .repo
            .get_or_init_default_wallet()
            .await
            .map_err(db_err)?;
        let budget = self
            .repo
            .get_budget(DEFAULT_AGENT_ID)
            .await
            .map_err(db_err)?;
        let today_spent = self
            .repo
            .today_spending(DEFAULT_AGENT_ID)
            .await
            .map_err(db_err)?;

        let (daily_cap, per_call_cap) = match budget.as_ref() {
            Some(b) => (b.daily_usd, b.per_call_usd),
            None => (DEFAULT_DAILY_BUDGET_USD, DEFAULT_DAILY_BUDGET_USD),
        };
        let remaining = (daily_cap - today_spent).max(0.0);

        let response = BalanceResponse {
            available_usdc: wallet_row.available_usdc,
            budget_remaining_today_usdc: format!("{remaining:.2}"),
            daily_cap_usdc: format!("{daily_cap:.2}"),
            per_call_cap_usdc: format!("{per_call_cap:.2}"),
            today_spent_usdc: format!("{today_spent:.2}"),
            currency: CURRENCY.to_string(),
            environment: self.environment().to_string(),
            solana_pubkey: self.wallet.pubkey_base58(),
        };

        json_result(&response)
    }

    #[tool(description = "Pay for an x402-priced URL up to max_amount_usdc. \
        On solana-devnet (default) this signs a real SPL USDC transfer with the local keypair. \
        In sandbox mode it sends a placeholder payload.")]
    async fn agentspay_pay_url(
        &self,
        Parameters(req): Parameters<PayUrlRequest>,
    ) -> Result<CallToolResult, McpError> {
        // ---- 1. Validate inputs.
        let parsed = url::Url::parse(&req.url).map_err(|e| {
            McpError::invalid_params(
                format!("url is not a valid URL: {e}"),
                Some(json!({ "field": "url", "value": req.url })),
            )
        })?;
        if !matches!(parsed.scheme(), "http" | "https") {
            return Err(McpError::invalid_params(
                "url scheme must be http or https",
                Some(json!({ "field": "url", "scheme": parsed.scheme() })),
            ));
        }
        let max_amount: f64 = req.max_amount_usdc.parse().map_err(|e| {
            McpError::invalid_params(
                format!("max_amount_usdc must parse as a decimal number: {e}"),
                Some(json!({ "field": "max_amount_usdc", "value": req.max_amount_usdc })),
            )
        })?;
        if !max_amount.is_finite() || max_amount <= 0.0 {
            return Err(McpError::invalid_params(
                "max_amount_usdc must be a positive finite number",
                Some(json!({ "field": "max_amount_usdc", "value": req.max_amount_usdc })),
            ));
        }

        // ---- 2. Serialize the budget-check + settle + ledger-write path.
        let _guard = self.pay_lock.lock().await;

        // ---- 3. Phase 1: probe.
        let client = X402Client::for_mode(
            &self.http,
            self.network.clone(),
            Some(Arc::clone(&self.wallet)),
        )
        .map_err(|e| McpError::internal_error(format!("x402 client init: {e}"), None))?;
        let prepared = match client.prepare(&req.url, max_amount).await {
            Ok(PreparedOrSettled::NoPaymentRequired(paid)) => {
                let _ = self
                    .repo
                    .insert_audit_entry(new_audit(
                        DEFAULT_AGENT_ID,
                        "agentspay_pay_url",
                        Some(req.url.clone()),
                        None,
                        "ok payment_status=none".to_string(),
                    ))
                    .await;
                return reply_paid_response(req.url, paid, "0.00".to_string(), &self.network);
            }
            Ok(PreparedOrSettled::PaymentRequired(p)) => p,
            Err(X402Error::AmountAboveCap {
                required_usdc,
                cap_usdc,
            }) => {
                let _ = self
                    .repo
                    .insert_audit_entry(new_audit(
                        DEFAULT_AGENT_ID,
                        "agentspay_pay_url",
                        Some(req.url.clone()),
                        Some(req.max_amount_usdc.clone()),
                        format!("rejected reason=above-call-cap required={required_usdc} cap={cap_usdc}"),
                    ))
                    .await;
                return Err(McpError::invalid_params(
                    format!(
                        "endpoint requires more than max_amount_usdc \
                        (required={required_usdc}, cap={cap_usdc})"
                    ),
                    Some(json!({
                        "required_usdc": required_usdc,
                        "max_amount_usdc": cap_usdc,
                        "reason": "above_call_cap",
                    })),
                ));
            }
            Err(e) => {
                tracing::warn!(error = %e, url = %req.url, "agentspay_pay_url prepare failed");
                let _ = self
                    .repo
                    .insert_audit_entry(new_audit(
                        DEFAULT_AGENT_ID,
                        "agentspay_pay_url",
                        Some(req.url.clone()),
                        Some(req.max_amount_usdc.clone()),
                        format!("error {e}"),
                    ))
                    .await;
                return Err(McpError::internal_error(
                    format!("x402 payment flow failed: {e}"),
                    Some(json!({ "url": req.url })),
                ));
            }
        };

        // ---- 4. Budget enforcement.
        let required_usdc_str = prepared
            .requirement
            .required_usdc()
            .map_err(|e| McpError::internal_error(format!("malformed amount: {e}"), None))?;
        let required_usdc: f64 = required_usdc_str.parse().unwrap_or(f64::INFINITY);

        if let Some(budget) = self
            .repo
            .get_budget(DEFAULT_AGENT_ID)
            .await
            .map_err(db_err)?
        {
            if required_usdc > budget.per_call_usd {
                let _ = self
                    .repo
                    .insert_audit_entry(new_audit(
                        DEFAULT_AGENT_ID,
                        "agentspay_pay_url",
                        Some(req.url.clone()),
                        Some(required_usdc_str.clone()),
                        format!(
                            "rejected reason=per-call-budget required={required_usdc_str} per_call_cap={:.2}",
                            budget.per_call_usd
                        ),
                    ))
                    .await;
                return Err(McpError::invalid_params(
                    format!(
                        "endpoint price exceeds configured per-call budget \
                        (required={required_usdc_str} per_call_cap={:.2})",
                        budget.per_call_usd
                    ),
                    Some(json!({
                        "required_usdc": required_usdc_str,
                        "per_call_cap_usd": budget.per_call_usd,
                        "reason": "per_call_budget",
                    })),
                ));
            }

            let today_spend = self
                .repo
                .today_spending(DEFAULT_AGENT_ID)
                .await
                .map_err(db_err)?;
            if today_spend + required_usdc > budget.daily_usd {
                let _ = self
                    .repo
                    .insert_audit_entry(new_audit(
                        DEFAULT_AGENT_ID,
                        "agentspay_pay_url",
                        Some(req.url.clone()),
                        Some(required_usdc_str.clone()),
                        format!(
                            "rejected reason=daily-budget required={required_usdc_str} \
                            today_spent={today_spend:.2} daily_cap={:.2}",
                            budget.daily_usd
                        ),
                    ))
                    .await;
                return Err(McpError::invalid_params(
                    format!(
                        "this call would exceed today's remaining budget \
                        (required={required_usdc_str} today_spent={today_spend:.2} \
                        daily_cap={:.2})",
                        budget.daily_usd
                    ),
                    Some(json!({
                        "required_usdc": required_usdc_str,
                        "today_spent_usd": today_spend,
                        "daily_cap_usd": budget.daily_usd,
                        "reason": "daily_budget",
                    })),
                ));
            }
        }

        // ---- 5. Phase 2: sign + retry.
        let PaidResponse {
            body,
            status,
            settlement,
            requirement,
        } = match client.complete(&req.url, prepared).await {
            Ok(ok) => ok,
            Err(e) => {
                tracing::warn!(error = %e, url = %req.url, "agentspay_pay_url complete failed");
                let _ = self
                    .repo
                    .insert_audit_entry(new_audit(
                        DEFAULT_AGENT_ID,
                        "agentspay_pay_url",
                        Some(req.url.clone()),
                        Some(required_usdc_str.clone()),
                        format!("error post-budget {e}"),
                    ))
                    .await;
                return Err(McpError::internal_error(
                    format!("x402 payment flow failed: {e}"),
                    Some(json!({ "url": req.url })),
                ));
            }
        };

        // ---- 6. Persist ledger + audit.
        let payment_id = format!("pay_{}", Uuid::new_v4().simple());
        let (amount_usdc_str, transaction, payment_status) = match (&settlement, &requirement) {
            (Some(settlement), Some(req_obj)) => {
                let amount = req_obj
                    .required_usdc()
                    .unwrap_or_else(|_| "0.00".to_string());
                (amount, settlement.transaction.clone(), "paid".to_string())
            }
            _ => ("0.00".to_string(), String::new(), "none".to_string()),
        };

        let ledger_active = new_ledger_entry(
            DEFAULT_AGENT_ID,
            req.url.clone(),
            amount_usdc_str.clone(),
            payment_id.clone(),
            transaction.clone(),
            format!("ok status={status}"),
        );
        let audit_active = new_audit(
            DEFAULT_AGENT_ID,
            "agentspay_pay_url",
            Some(req.url.clone()),
            Some(amount_usdc_str.clone()),
            format!(
                "ok payment_status={payment_status} network={}",
                self.environment()
            ),
        );

        let (ledger_model, _audit_model) = self
            .repo
            .record_paid_call(ledger_active, audit_active)
            .await
            .map_err(db_err)?;

        // ---- 7. Reply.
        let explorer_url = explorer_url_for(&self.network, &transaction);
        let response = PayUrlResponse {
            status: "ok".to_string(),
            payment_id,
            endpoint: req.url,
            amount_charged_usdc: amount_usdc_str,
            ledger_entry_id: ledger_model.id,
            transaction,
            body,
            payment_status,
            network: self.environment().to_string(),
            explorer_url,
        };
        json_result(&response)
    }

    #[tool(description = "Set daily and per-call USDC spending budget for the agent.")]
    async fn agentspay_set_budget(
        &self,
        Parameters(req): Parameters<SetBudgetRequest>,
    ) -> Result<CallToolResult, McpError> {
        validate_positive_amount("daily_usd", req.daily_usd)?;
        validate_positive_amount("per_call_usd", req.per_call_usd)?;

        let budget = self
            .repo
            .upsert_budget(DEFAULT_AGENT_ID, req.daily_usd, req.per_call_usd)
            .await
            .map_err(db_err)?;

        let _ = self
            .repo
            .insert_audit_entry(new_audit(
                DEFAULT_AGENT_ID,
                "agentspay_set_budget",
                None,
                None,
                format!(
                    "budget-updated daily={:.2} per_call={:.2}",
                    req.daily_usd, req.per_call_usd
                ),
            ))
            .await;

        let response = SetBudgetResponse {
            agent_id: budget.agent_id,
            daily_usd: budget.daily_usd,
            per_call_usd: budget.per_call_usd,
            updated_at_rfc3339: budget.updated_at.to_rfc3339(),
        };
        json_result(&response)
    }

    #[tool(description = "Return the most recent audit log entries (default 20, max 100).")]
    async fn agentspay_audit_log(
        &self,
        Parameters(req): Parameters<AuditLogRequest>,
    ) -> Result<CallToolResult, McpError> {
        let limit = req
            .limit
            .unwrap_or(DEFAULT_AUDIT_LIMIT)
            .clamp(1, MAX_AUDIT_LIMIT);

        let rows = self.repo.recent_audit(limit).await.map_err(db_err)?;
        let total = self.repo.count_audit().await.map_err(db_err)?;

        let entries: Vec<AuditEntryDto> = rows
            .into_iter()
            .map(|row| AuditEntryDto {
                id: row.id,
                timestamp_rfc3339: row.created_at.to_rfc3339(),
                tool: row.tool,
                endpoint: row.endpoint,
                amount_usdc: row.amount_usdc,
                status: row.status,
            })
            .collect();
        let returned = entries.len();

        let response = AuditLogResponse {
            entries,
            total,
            returned,
        };
        json_result(&response)
    }

    #[tool(
        description = "Get the agent's Solana pubkey plus faucet URL and step-by-step instructions \
        for funding it with devnet USDC. The MCP server cannot self-fund (Circle's faucet requires \
        a manual web captcha)."
    )]
    async fn agentspay_topup_info(&self) -> Result<CallToolResult, McpError> {
        let response = TopupInfoResponse {
            pubkey: self.wallet.pubkey_base58(),
            network: self.environment().to_string(),
            faucet_url: CIRCLE_FAUCET_URL.to_string(),
            sol_faucet_url: SOL_FAUCET_URL.to_string(),
            instructions: TOPUP_INSTRUCTIONS.to_string(),
        };
        json_result(&response)
    }
}

// ---------------------------------------------------------------------------
// Server handler
// ---------------------------------------------------------------------------

#[tool_handler]
impl ServerHandler for AgentsPayServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2025_06_18,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation {
                name: env!("CARGO_PKG_NAME").to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                title: None,
                description: None,
                website_url: None,
                icons: None,
            },
            instructions: Some(
                "AgentsPay v0.3 — budget-controlled USDC wallet for autonomous agents. \
                Signs real Solana devnet SPL token transfers by default. Tools: \
                agentspay_balance, agentspay_pay_url, agentspay_set_budget, \
                agentspay_audit_log, agentspay_topup_info."
                    .to_string(),
            ),
        }
    }
}

// ---------------------------------------------------------------------------
// Binary-level error type (transport / startup failures)
// ---------------------------------------------------------------------------

#[derive(Debug, Error)]
enum McpServerError {
    #[error("serve failure: {0}")]
    Serve(String),
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn validate_positive_amount(field: &'static str, value: f64) -> Result<(), McpError> {
    if !value.is_finite() || value <= 0.0 {
        return Err(McpError::invalid_params(
            format!("{field} must be a positive finite number"),
            Some(json!({ "field": field, "value": value })),
        ));
    }
    Ok(())
}

fn db_err(e: sea_orm::DbErr) -> McpError {
    McpError::internal_error(format!("database error: {e}"), None)
}

fn json_result<T: Serialize>(value: &T) -> Result<CallToolResult, McpError> {
    let content = Content::json(value).map_err(|e| {
        McpError::internal_error(format!("failed to encode response payload: {e}"), None)
    })?;
    Ok(CallToolResult::success(vec![content]))
}

fn reply_paid_response(
    url: String,
    paid: PaidResponse,
    amount_charged_usdc: String,
    network: &NetworkMode,
) -> Result<CallToolResult, McpError> {
    let response = PayUrlResponse {
        status: "ok".to_string(),
        payment_id: format!("free_{}", Uuid::new_v4().simple()),
        endpoint: url,
        amount_charged_usdc,
        ledger_entry_id: String::new(),
        transaction: String::new(),
        body: paid.body,
        payment_status: "none".to_string(),
        network: network.wire_name().to_string(),
        explorer_url: String::new(),
    };
    json_result(&response)
}

/// Build a Solscan URL for a given TX signature, or an empty string for
/// sandbox / non-Solana modes (or when the signature is empty).
fn explorer_url_for(mode: &NetworkMode, signature: &str) -> String {
    if signature.is_empty() {
        return String::new();
    }
    match mode {
        NetworkMode::SolanaDevnet => {
            format!("https://solscan.io/tx/{signature}?cluster=devnet")
        }
        NetworkMode::SolanaMainnet => format!("https://solscan.io/tx/{signature}"),
        NetworkMode::Sandbox => String::new(),
    }
}

fn init_tracing() {
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("agentspay_mcp=info"));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(std::io::stderr)
                .with_ansi(false),
        )
        .init();
}

fn build_http_client() -> anyhow::Result<reqwest::Client> {
    let client = reqwest::Client::builder()
        .user_agent(HTTP_USER_AGENT)
        .timeout(Duration::from_secs(HTTP_TIMEOUT_SECS))
        .build()?;
    Ok(client)
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

#[tokio::main(flavor = "multi_thread")]
async fn main() -> anyhow::Result<()> {
    init_tracing();

    let url = db::resolved_url().await?;
    tracing::info!(database_url = %url, "opening agentspay-mcp database");
    let db = db::connect_and_migrate(&url).await?;

    let repo = Arc::new(LedgerRepo::new(db));
    let _ = repo.get_or_init_default_wallet().await?;

    // Load (or first-run-generate) the agent's Solana keypair. The pubkey
    // is logged at info level so a user immediately sees it on stderr.
    let keypair_path = wallet::resolved_path()?;
    let agent_wallet = AgentWallet::load_or_create(&keypair_path)?;
    let pubkey_b58 = agent_wallet.pubkey_base58();
    tracing::info!(
        path = %keypair_path.display(),
        solana_pubkey = %pubkey_b58,
        "agent wallet ready"
    );
    let agent_wallet = Arc::new(agent_wallet);

    // Backfill / refresh the wallet row's pubkey column.
    if let Err(e) = repo.set_solana_pubkey(DEFAULT_AGENT_ID, &pubkey_b58).await {
        tracing::warn!(error = %e, "failed to persist solana_pubkey on wallet row");
    }

    let network = NetworkMode::from_env();
    tracing::info!(network = %network.wire_name(), "x402 network mode");

    let http = build_http_client()?;
    let server = AgentsPayServer::new(repo, agent_wallet, network, http);

    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        "starting agentspay-mcp on stdio"
    );

    let service = server
        .serve(stdio())
        .await
        .map_err(|e| McpServerError::Serve(e.to_string()))?;

    service
        .waiting()
        .await
        .map_err(|e| McpServerError::Serve(e.to_string()))?;

    Ok(())
}
