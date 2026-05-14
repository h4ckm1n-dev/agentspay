use std::{env, net::SocketAddr};

use agentspay_proto::auth::{
    auth_service_server::{AuthService, AuthServiceServer},
    CreateAgentRequest, CreateAgentResponse, ValidateApiKeyRequest, ValidateApiKeyResponse,
};
use agentspay_proto::common::Environment;
use tonic::{transport::Server, Request, Response, Status};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use uuid::Uuid;

#[derive(Default)]
struct AuthGrpc;

#[tonic::async_trait]
impl AuthService for AuthGrpc {
    async fn validate_api_key(
        &self,
        request: Request<ValidateApiKeyRequest>,
    ) -> Result<Response<ValidateApiKeyResponse>, Status> {
        let request = request.into_inner();
        if request.api_key.trim().is_empty() {
            return Err(Status::unauthenticated("api_key is required"));
        }

        Ok(Response::new(ValidateApiKeyResponse {
            active: true,
            organization_id: "org_sandbox".to_string(),
            api_key_id: id("key"),
            scopes: vec![scope_or_default(request.required_scope)],
            environment: normalize_environment(request.environment),
        }))
    }

    async fn create_agent(
        &self,
        request: Request<CreateAgentRequest>,
    ) -> Result<Response<CreateAgentResponse>, Status> {
        let request = request.into_inner();
        if request.idempotency_key.trim().is_empty() {
            return Err(Status::invalid_argument("idempotency_key is required"));
        }

        Ok(Response::new(CreateAgentResponse {
            agent_id: id("agent"),
            organization_id: request.organization_id,
            name: request.name,
            scopes: request.scopes,
            environment: normalize_environment(request.environment),
        }))
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();
    let addr = env::var("AUTH_GRPC_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:50051".to_string())
        .parse::<SocketAddr>()?;

    tracing::info!(%addr, "starting agentspay auth service");
    Server::builder()
        .add_service(AuthServiceServer::new(AuthGrpc))
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

fn scope_or_default(scope: String) -> String {
    if scope.trim().is_empty() {
        "payments:read".to_string()
    } else {
        scope
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
