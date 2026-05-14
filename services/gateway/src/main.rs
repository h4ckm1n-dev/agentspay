use std::{env, net::SocketAddr};

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use uuid::Uuid;

#[derive(Clone)]
struct AppState {
    environment: String,
    version: &'static str,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();

    let port = env::var("PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(8080);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let app = build_router(AppState {
        environment: env::var("AGENTSPAY_ENV").unwrap_or_else(|_| "sandbox".to_string()),
        version: env!("CARGO_PKG_VERSION"),
    });

    tracing::info!(%addr, "starting agentspay gateway");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/v1/health", get(health))
        .route("/v1/status", get(status))
        .route("/.well-known/agentspay", get(discovery))
        .route("/x402/supported", get(x402_supported))
        .route("/x402/verify", post(x402_verify))
        .route("/x402/settle", post(x402_settle))
        .with_state(state)
}

async fn health(State(state): State<AppState>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        service: "agentspay-gateway",
        version: state.version,
        environment: state.environment,
    })
}

async fn status(State(state): State<AppState>) -> Json<StatusResponse> {
    Json(StatusResponse {
        status: "operational",
        environment: state.environment,
        ledger_mode: "sandbox-ledger",
        settlement_rail: "solana-usdc-sandbox",
        services: vec![
            ServiceStatus::stubbed("auth"),
            ServiceStatus::stubbed("payment"),
            ServiceStatus::stubbed("metering"),
        ],
    })
}

async fn discovery(State(state): State<AppState>) -> Json<DiscoveryDocument> {
    Json(DiscoveryDocument {
        name: "AgentsPay",
        version: state.version,
        environment: state.environment,
        capabilities: vec![
            "x402.verify",
            "x402.settle",
            "x402.supported",
            "sandbox-ledger",
            "audit-proof-stub",
        ],
        endpoints: DiscoveryEndpoints {
            health: "/v1/health",
            status: "/v1/status",
            supported: "/x402/supported",
            verify: "/x402/verify",
            settle: "/x402/settle",
        },
        rails: vec![SettlementRail {
            rail: "solana-usdc",
            network: "devnet",
            environment: "sandbox",
            status: "stubbed",
        }],
    })
}

async fn x402_supported(State(state): State<AppState>) -> Json<X402SupportedResponse> {
    Json(X402SupportedResponse {
        x402_version: "1",
        environment: state.environment,
        schemes: vec![SupportedScheme {
            scheme: "exact",
            network: "solana-devnet",
            asset: "USDC",
            max_timeout_seconds: 300,
        }],
        facilitator: FacilitatorEndpoints {
            supported_url: "/x402/supported",
            verify_url: "/x402/verify",
            settle_url: "/x402/settle",
        },
    })
}

async fn x402_verify(
    State(state): State<AppState>,
    Json(request): Json<X402VerifyRequest>,
) -> Result<Json<X402VerifyResponse>, ApiError> {
    require_idempotency_key(&request.idempotency_key)?;

    Ok(Json(X402VerifyResponse {
        valid: true,
        environment: state.environment,
        payment_id: prefixed_id("pay"),
        authorization_id: prefixed_id("authz"),
        policy_decision_id: prefixed_id("policy"),
        audit_proof_id: prefixed_id("audit"),
        ledger_state: "authorized",
        reason: "sandbox payment accepted",
        requirement: request.requirement,
    }))
}

async fn x402_settle(
    State(state): State<AppState>,
    Json(request): Json<X402SettleRequest>,
) -> Result<Json<X402SettleResponse>, ApiError> {
    require_idempotency_key(&request.idempotency_key)?;

    Ok(Json(X402SettleResponse {
        settled: true,
        environment: state.environment,
        payment_id: request.payment_id,
        authorization_id: request.authorization_id,
        settlement_id: prefixed_id("settle"),
        transaction_id: prefixed_id("txn"),
        ledger_entry_id: prefixed_id("ledger"),
        audit_proof_id: prefixed_id("audit"),
        status: "settled",
        rail: "sandbox-ledger",
    }))
}

fn require_idempotency_key(value: &str) -> Result<(), ApiError> {
    if value.trim().is_empty() {
        Err(ApiError::MissingIdempotencyKey)
    } else {
        Ok(())
    }
}

fn prefixed_id(prefix: &str) -> String {
    format!("{prefix}_{}", Uuid::new_v4().simple())
}

fn init_tracing() {
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "agentspay=info,tower_http=info".into());

    tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer())
        .init();
}

#[derive(Debug, Error)]
enum ApiError {
    #[error("idempotency_key is required")]
    MissingIdempotencyKey,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = match self {
            ApiError::MissingIdempotencyKey => StatusCode::BAD_REQUEST,
        };

        (
            status,
            Json(ErrorResponse {
                error: self.to_string(),
            }),
        )
            .into_response()
    }
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    service: &'static str,
    version: &'static str,
    environment: String,
}

#[derive(Serialize)]
struct StatusResponse {
    status: &'static str,
    environment: String,
    ledger_mode: &'static str,
    settlement_rail: &'static str,
    services: Vec<ServiceStatus>,
}

#[derive(Serialize)]
struct ServiceStatus {
    name: &'static str,
    status: &'static str,
    mode: &'static str,
}

impl ServiceStatus {
    fn stubbed(name: &'static str) -> Self {
        Self {
            name,
            status: "available",
            mode: "stubbed",
        }
    }
}

#[derive(Serialize)]
struct DiscoveryDocument {
    name: &'static str,
    version: &'static str,
    environment: String,
    capabilities: Vec<&'static str>,
    endpoints: DiscoveryEndpoints,
    rails: Vec<SettlementRail>,
}

#[derive(Serialize)]
struct DiscoveryEndpoints {
    health: &'static str,
    status: &'static str,
    supported: &'static str,
    verify: &'static str,
    settle: &'static str,
}

#[derive(Serialize)]
struct SettlementRail {
    rail: &'static str,
    network: &'static str,
    environment: &'static str,
    status: &'static str,
}

#[derive(Serialize)]
struct X402SupportedResponse {
    x402_version: &'static str,
    environment: String,
    schemes: Vec<SupportedScheme>,
    facilitator: FacilitatorEndpoints,
}

#[derive(Serialize)]
struct SupportedScheme {
    scheme: &'static str,
    network: &'static str,
    asset: &'static str,
    max_timeout_seconds: u16,
}

#[derive(Serialize)]
struct FacilitatorEndpoints {
    supported_url: &'static str,
    verify_url: &'static str,
    settle_url: &'static str,
}

#[derive(Debug, Deserialize, Serialize)]
struct X402VerifyRequest {
    idempotency_key: String,
    payment_payload: PaymentPayload,
    requirement: PaymentRequirement,
    agentspay: Option<AgentsPayContext>,
}

#[derive(Debug, Serialize)]
struct X402VerifyResponse {
    valid: bool,
    environment: String,
    payment_id: String,
    authorization_id: String,
    policy_decision_id: String,
    audit_proof_id: String,
    ledger_state: &'static str,
    reason: &'static str,
    requirement: PaymentRequirement,
}

#[derive(Debug, Deserialize, Serialize)]
struct X402SettleRequest {
    idempotency_key: String,
    payment_id: String,
    authorization_id: String,
    payment_payload: Option<PaymentPayload>,
    requirement: Option<PaymentRequirement>,
    agentspay: Option<AgentsPayContext>,
}

#[derive(Debug, Serialize)]
struct X402SettleResponse {
    settled: bool,
    environment: String,
    payment_id: String,
    authorization_id: String,
    settlement_id: String,
    transaction_id: String,
    ledger_entry_id: String,
    audit_proof_id: String,
    status: &'static str,
    rail: &'static str,
}

#[derive(Debug, Deserialize, Serialize)]
struct PaymentPayload {
    scheme: String,
    network: String,
    amount: String,
    currency: String,
    payer: String,
    payee: Option<String>,
    nonce: String,
    signature: Option<String>,
    sandbox_proof: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct PaymentRequirement {
    scheme: String,
    network: String,
    amount: String,
    currency: String,
    pay_to: String,
    resource: String,
    description: Option<String>,
    expires_at: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct AgentsPayContext {
    organization_id: Option<String>,
    agent_id: Option<String>,
    endpoint_id: Option<String>,
    budget_id: Option<String>,
    policy_decision_id: Option<String>,
    audit_proof_id: Option<String>,
    environment: Option<String>,
}
