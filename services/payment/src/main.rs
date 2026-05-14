use std::{env, net::SocketAddr};

use agentspay_proto::{
    common::{Environment, Money},
    payment::{
        payment_service_server::{PaymentService, PaymentServiceServer},
        GetBalanceRequest, GetBalanceResponse, SettlePaymentRequest, SettlePaymentResponse,
        VerifyPaymentRequest, VerifyPaymentResponse,
    },
};
use tonic::{transport::Server, Request, Response, Status};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use uuid::Uuid;

#[derive(Default)]
struct PaymentGrpc;

#[tonic::async_trait]
impl PaymentService for PaymentGrpc {
    async fn verify_payment(
        &self,
        request: Request<VerifyPaymentRequest>,
    ) -> Result<Response<VerifyPaymentResponse>, Status> {
        let request = request.into_inner();
        if request.idempotency_key.trim().is_empty() {
            return Err(Status::invalid_argument("idempotency_key is required"));
        }

        Ok(Response::new(VerifyPaymentResponse {
            valid: true,
            payment_id: id("pay"),
            authorization_id: id("authz"),
            policy_decision_id: id("policy"),
            audit_proof_id: id("audit"),
            ledger_state: "authorized".to_string(),
            reason: "sandbox payment accepted".to_string(),
            environment: request
                .context
                .map(|context| normalize_environment(context.environment))
                .unwrap_or(Environment::Sandbox as i32),
        }))
    }

    async fn settle_payment(
        &self,
        request: Request<SettlePaymentRequest>,
    ) -> Result<Response<SettlePaymentResponse>, Status> {
        let request = request.into_inner();
        if request.idempotency_key.trim().is_empty() {
            return Err(Status::invalid_argument("idempotency_key is required"));
        }

        Ok(Response::new(SettlePaymentResponse {
            settled: true,
            settlement_id: id("settle"),
            transaction_id: id("txn"),
            ledger_entry_id: id("ledger"),
            status: "settled".to_string(),
            audit_proof_id: id("audit"),
            environment: request
                .context
                .map(|context| normalize_environment(context.environment))
                .unwrap_or(Environment::Sandbox as i32),
        }))
    }

    async fn get_balance(
        &self,
        request: Request<GetBalanceRequest>,
    ) -> Result<Response<GetBalanceResponse>, Status> {
        let request = request.into_inner();
        let environment = normalize_environment(request.environment);

        Ok(Response::new(GetBalanceResponse {
            available: Some(Money {
                amount: "1000.00".to_string(),
                currency: "USDC".to_string(),
            }),
            pending: Some(Money {
                amount: "0.00".to_string(),
                currency: "USDC".to_string(),
            }),
            environment,
        }))
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();
    let addr = env::var("PAYMENT_GRPC_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:50052".to_string())
        .parse::<SocketAddr>()?;

    tracing::info!(%addr, "starting agentspay payment service");
    Server::builder()
        .add_service(PaymentServiceServer::new(PaymentGrpc))
        .serve(addr)
        .await?;

    Ok(())
}

fn normalize_environment(environment: i32) -> i32 {
    if Environment::try_from(environment).unwrap_or(Environment::Unspecified)
        == Environment::Unspecified
    {
        Environment::Sandbox as i32
    } else {
        environment
    }
}

fn id(prefix: &str) -> String {
    format!("{prefix}_{}", Uuid::new_v4().simple())
}

fn init_tracing() {
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "agentspay=info".into());

    tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer())
        .init();
}
