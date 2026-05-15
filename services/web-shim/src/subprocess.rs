//! Spawn `agentspay-mcp`, pipe a JSON-RPC tools/call sequence into stdin,
//! parse the response, return the inner JSON value.
//!
//! The contract with `agentspay-mcp` is the MCP 2025-06-18 stdio transport.
//! We always send three messages in order: initialize, notifications/initialized,
//! tools/call. Then we close stdin and read line-delimited JSON from stdout
//! until we see the response for our tools/call id.

use std::{path::Path, process::Stdio, time::Duration};

use serde_json::{json, Value};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::Command,
    time::timeout,
};

use crate::error::ShimError;

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)] // `SolanaDevnet` is exercised in Phase 3.
pub enum NetworkMode {
    Sandbox,
    SolanaDevnet,
}

impl NetworkMode {
    fn as_env(self) -> &'static str {
        match self {
            Self::Sandbox => "sandbox",
            Self::SolanaDevnet => "solana-devnet",
        }
    }
}

pub struct McpCall<'a> {
    pub binary: &'a Path,
    pub network: NetworkMode,
    pub keypair_path: &'a Path,
    pub db_url: &'a str,
    pub tool: &'a str,
    pub args: Value,
    pub timeout: Duration,
}

pub async fn run(call: McpCall<'_>) -> Result<Value, ShimError> {
    let mut cmd = Command::new(call.binary);
    cmd.env("AGENTSPAY_NETWORK", call.network.as_env())
        .env("AGENTSPAY_KEYPAIR_PATH", call.keypair_path)
        .env("AGENTSPAY_DATABASE_URL", call.db_url)
        .env("RUST_LOG", "agentspay_mcp=warn")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        // Forward subprocess stderr to the shim's stderr so the pretty
        // agentspay-mcp banner + per-tool-call status lines surface in
        // `docker compose logs shim` (and in operator-attached terminals).
        // Native MCP-host installs already see this output — this brings
        // the Docker-stack experience in line.
        .stderr(Stdio::inherit());

    let mut child = cmd
        .spawn()
        .map_err(|e| ShimError::SubprocessFailed(format!("spawn failed: {e}")))?;

    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| ShimError::SubprocessFailed("subprocess has no stdin".into()))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| ShimError::SubprocessFailed("subprocess has no stdout".into()))?;

    let init = json!({
        "jsonrpc":"2.0","id":1,"method":"initialize",
        "params":{
            "protocolVersion":"2025-06-18",
            "capabilities":{},
            "clientInfo":{"name":"agentspay-web-shim","version":env!("CARGO_PKG_VERSION")}
        }
    });
    let initialized = json!({
        "jsonrpc":"2.0","method":"notifications/initialized","params":{}
    });
    let tool_call = json!({
        "jsonrpc":"2.0","id":42,"method":"tools/call",
        "params":{"name":call.tool,"arguments":call.args}
    });

    let payload = format!("{init}\n{initialized}\n{tool_call}\n");
    stdin
        .write_all(payload.as_bytes())
        .await
        .map_err(|e| ShimError::SubprocessFailed(format!("stdin write: {e}")))?;
    // NOTE: we deliberately do NOT drop stdin here. The rmcp server treats
    // stdin EOF as a shutdown signal and exits before async tool handlers
    // finish. We hold stdin open for the duration of the read and kill the
    // child once we have our response (or on timeout).

    let reader = BufReader::new(stdout);
    let mut lines = reader.lines();

    let read_fut = async {
        while let Some(line) = lines.next_line().await.transpose() {
            let line =
                line.map_err(|e| ShimError::SubprocessFailed(format!("stdout read: {e}")))?;
            if line.trim().is_empty() {
                continue;
            }
            let msg: Value = serde_json::from_str(&line)
                .map_err(|e| ShimError::MalformedMcp(format!("{e}: {line}")))?;
            if msg.get("id").and_then(Value::as_i64) == Some(42) {
                if let Some(err) = msg.get("error") {
                    return Err(ShimError::MalformedMcp(format!("mcp error: {err}")));
                }
                return Ok(msg.get("result").cloned().unwrap_or(Value::Null));
            }
        }
        Err(ShimError::MalformedMcp(
            "subprocess stdout closed without tools/call response".into(),
        ))
    };

    let result = timeout(call.timeout, read_fut).await;

    // We have our answer (or timed out). Drop stdin, then kill + reap the
    // child so it never lingers past this request.
    drop(stdin);
    let _ = child.start_kill();
    let _ = child.wait().await;

    match result {
        Ok(inner) => inner,
        Err(_) => Err(ShimError::SubprocessTimeout(call.timeout.as_secs())),
    }
}
