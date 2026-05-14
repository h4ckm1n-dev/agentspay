//! Environment-variable configuration for agentspay-web-shim.

use std::{env, net::SocketAddr, path::PathBuf, time::Duration};

#[derive(Debug, Clone)]
pub struct Config {
    pub listen_addr: SocketAddr,
    pub mcp_binary: PathBuf,
    /// Used by Phase 2 webhook/server-side calls to the demo merchant.
    #[allow(dead_code)]
    pub paid_endpoint_url: String,
    /// Phase 3 mounts the rate-limited devnet wallet from here.
    #[allow(dead_code)]
    pub devnet_wallet_path: PathBuf,
    /// Phase 3 records real-tx audit entries here.
    #[allow(dead_code)]
    pub devnet_ledger_path: PathBuf,
    pub session_ttl: Duration,
    /// Phase 1 currently hardcodes 10s in the handler; Phase 2 will read this.
    #[allow(dead_code)]
    pub subprocess_timeout: Duration,
    /// `None` → run with in-memory session+ratelimit stores. Phase 4 sets it.
    #[allow(dead_code)]
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
