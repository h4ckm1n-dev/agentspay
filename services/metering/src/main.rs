use std::{env, net::SocketAddr};

use agentspay_proto::{
    common::{Environment, Money},
    metering::{
        metering_service_server::{MeteringService, MeteringServiceServer},
        GetEndpointPriceRequest, GetEndpointPriceResponse, RecordUsageRequest, RecordUsageResponse,
    },
};
use tonic::{transport::Server, Request, Response, Status};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use uuid::Uuid;

#[derive(Default)]
struct MeteringGrpc;

#[tonic::async_trait]
impl MeteringService for MeteringGrpc {
    async fn record_usage(
        &self,
        request: Request<RecordUsageRequest>,
    ) -> Result<Response<RecordUsageResponse>, Status> {
        let request = request.into_inner();
        if request.idempotency_key.trim().is_empty() {
            return Err(Status::invalid_argument("idempotency_key is required"));
        }

        Ok(Response::new(RecordUsageResponse {
            usage_event_id: id("usage"),
            organization_id: request.organization_id,
            endpoint_id: request.endpoint_id,
            units: request.units,
            environment: normalize_environment(request.environment),
        }))
    }

    async fn get_endpoint_price(
        &self,
        request: Request<GetEndpointPriceRequest>,
    ) -> Result<Response<GetEndpointPriceResponse>, Status> {
        let request = request.into_inner();

        Ok(Response::new(GetEndpointPriceResponse {
            endpoint_id: request.endpoint_id,
            pricing_model: "fixed".to_string(),
            price: Some(Money {
                amount: "0.002".to_string(),
                currency: "USDC".to_string(),
            }),
            environment: normalize_environment(request.environment),
        }))
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();
    let addr = env::var("METERING_GRPC_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:50053".to_string())
        .parse::<SocketAddr>()?;

    tracing::info!(%addr, "starting agentspay metering service");
    Server::builder()
        .add_service(MeteringServiceServer::new(MeteringGrpc))
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
