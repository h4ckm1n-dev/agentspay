//! Environment-variable configuration for agentspay-web-shim.

use std::{env, net::SocketAddr, path::PathBuf, time::Duration};

#[derive(Debug, Clone)]
pub struct Config {
    pub listen_addr: SocketAddr,
    pub mcp_binary: PathBuf,
    pub paid_endpoint_url: String,
    pub devnet_wallet_path: PathBuf,
    pub devnet_ledger_path: PathBuf,
    pub session_ttl: Duration,
    pub subprocess_timeout: Duration,
    /// `None` → run with in-memory session+ratelimit stores. Phase 4 sets it.
    pub redis_url: Option<String>,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            listen_addr: env::var("AGENTSPAY_SHIM_LISTEN_ADDR")
                .unwrap_or_else(|_| "0.0.0.0:8080".to_string())
                .parse()?,
            mcp_binary: env::var("AGENTSPAY_MCP_BINARY")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("/usr/local/bin/agentspay-mcp")),
            paid_endpoint_url: env::var("AGENTSPAY_PAID_ENDPOINT_URL")
                .unwrap_or_else(|_| "http://localhost:3001".to_string()),
            devnet_wallet_path: env::var("AGENTSPAY_DEVNET_WALLET_PATH")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("/data/devnet-wallet.json")),
            devnet_ledger_path: env::var("AGENTSPAY_DEVNET_LEDGER_PATH")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("/data/devnet-ledger.db")),
            session_ttl: Duration::from_secs(30 * 60),
            subprocess_timeout: Duration::from_secs(10),
            redis_url: env::var("AGENTSPAY_REDIS_URL").ok(),
        })
    }
}
