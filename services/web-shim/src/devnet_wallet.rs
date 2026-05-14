//! Live status check of the shim's devnet wallet (SOL + USDC balances)
//! via the public Solana devnet RPC.

use std::path::Path;

use serde::Serialize;

const RPC_URL: &str = "https://api.devnet.solana.com";
const USDC_DEVNET_MINT: &str = "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU";
const MIN_SOL: f64 = 0.05;
const MIN_USDC: f64 = 2.0;

#[derive(Debug, Clone, Serialize)]
pub struct WalletStatus {
    pub pubkey: String,
    pub sol_balance: f64,
    pub usdc_balance: f64,
    pub healthy: bool,
    pub message: Option<String>,
}

pub async fn read_pubkey(path: &Path) -> anyhow::Result<String> {
    let raw = tokio::fs::read_to_string(path).await?;
    let bytes: Vec<u8> = serde_json::from_str(&raw)?;
    if bytes.len() != 64 {
        anyhow::bail!("keypair file must contain 64 bytes, got {}", bytes.len());
    }
    // Solana ed25519 keypair JSON = secret(32) || public(32). Encode last 32 as base58.
    Ok(bs58::encode(&bytes[32..]).into_string())
}

pub async fn fetch_status(http: &reqwest::Client, pubkey: &str) -> anyhow::Result<WalletStatus> {
    let sol_lamports = rpc_call_u64(http, "getBalance", serde_json::json!([pubkey])).await?;
    let sol_balance = sol_lamports as f64 / 1_000_000_000.0;

    let usdc_balance = fetch_usdc_balance(http, pubkey).await.unwrap_or(0.0);

    let mut warns = Vec::new();
    if sol_balance < MIN_SOL {
        warns.push(format!("SOL {sol_balance:.4} < {MIN_SOL}"));
    }
    if usdc_balance < MIN_USDC {
        warns.push(format!("USDC {usdc_balance:.2} < {MIN_USDC}"));
    }
    let healthy = warns.is_empty();
    let message = if healthy {
        None
    } else {
        Some(warns.join("; "))
    };

    Ok(WalletStatus {
        pubkey: pubkey.to_string(),
        sol_balance,
        usdc_balance,
        healthy,
        message,
    })
}

async fn rpc_call_u64(
    http: &reqwest::Client,
    method: &str,
    params: serde_json::Value,
) -> anyhow::Result<u64> {
    let resp: serde_json::Value = http
        .post(RPC_URL)
        .json(&serde_json::json!({
            "jsonrpc":"2.0","id":1,"method":method,"params":params
        }))
        .send()
        .await?
        .json()
        .await?;
    resp.get("result")
        .and_then(|r| r.get("value"))
        .and_then(|v| v.as_u64())
        .ok_or_else(|| anyhow::anyhow!("rpc {method} returned {resp}"))
}

async fn fetch_usdc_balance(http: &reqwest::Client, pubkey: &str) -> anyhow::Result<f64> {
    let resp: serde_json::Value = http
        .post(RPC_URL)
        .json(&serde_json::json!({
            "jsonrpc":"2.0","id":1,"method":"getTokenAccountsByOwner",
            "params":[pubkey,{"mint":USDC_DEVNET_MINT},{"encoding":"jsonParsed"}]
        }))
        .send()
        .await?
        .json()
        .await?;
    let accounts = resp.pointer("/result/value").and_then(|v| v.as_array());
    let total: f64 = accounts
        .map(|a| a.iter())
        .into_iter()
        .flatten()
        .filter_map(|acc| {
            acc.pointer("/account/data/parsed/info/tokenAmount/uiAmountString")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse::<f64>().ok())
        })
        .sum();
    Ok(total)
}
