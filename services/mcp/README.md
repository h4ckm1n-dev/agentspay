# agentspay-mcp

**A budget-controlled USDC wallet for AI agents, as a local MCP server.**

`agentspay-mcp` gives an AI coding agent (Claude Code, Cursor, Cline, Zed) a
Solana USDC wallet it can use to pay [x402](https://www.x402.org/)-priced APIs —
but **per-call and daily caps are checked before the keypair ever signs**, so the
agent can spend without being able to drain the wallet. The keypair lives on your
machine (mode 0600); there is no account, email, OTP, or custodial enclave.

> Status: **v0.3, Solana devnet.** Mainnet is a `NetworkMode` variant gated behind
> a compliance review — the shipped product signs on devnet only.

<!-- mcp-name: io.github.h4ckm1n-dev/agentspay -->
mcp-name: io.github.h4ckm1n-dev/agentspay

## Install

```bash
cargo install agentspay-mcp
```

This builds and places `agentspay-mcp` on your PATH. Point your MCP host at it —
for Claude Code:

```jsonc
// .mcp.json
{
  "mcpServers": {
    "agentspay": { "command": "agentspay-mcp" }
  }
}
```

## The 5 tools

| Tool | What it does |
|---|---|
| `agentspay_balance` | On-chain USDC + SOL, today's spend, remaining daily budget, pubkey. |
| `agentspay_pay_url` | Handles the 402 challenge, checks caps + payee allowlist, signs, retries, records the ledger entry. Optional `idempotency_key` prevents double-payment on retry. |
| `agentspay_set_budget` | Sets daily / per-call USDC caps, plus an optional recipient allowlist and mandate expiry. |
| `agentspay_audit_log` | Recent ledger rows, including rejected attempts. |
| `agentspay_topup_info` | Pubkey + faucet URLs for funding the devnet wallet. |

## Safety model

Per-call + daily caps enforced before signing (default 50/50 USDC until set).
Optional payee allowlist and mandate expiry. SSRF guard on `pay_url` (blocks
loopback / RFC1918 / link-local / cloud-metadata). Asset + decimals validators
reject malicious x402 sellers that try to inflate the transfer. 1 MiB response
body cap. See [SECURITY-AUDIT.md](https://github.com/h4ckm1n-dev/agentspay/blob/main/SECURITY-AUDIT.md).

## Configuration

| Variable | Default | Purpose |
|---|---|---|
| `AGENTSPAY_NETWORK` | `solana-devnet` | `sandbox` / `solana-devnet` / `solana-mainnet` |
| `AGENTSPAY_KEYPAIR_PATH` | `~/.agentspay/keypair.json` | Solana keypair (0600 on first run) |
| `AGENTSPAY_DATABASE_URL` | `sqlite://~/.agentspay/agentspay-mcp.db` | SeaORM ledger DB |
| `AGENTSPAY_SOLANA_RPC_URL` | `https://api.devnet.solana.com` | RPC endpoint |

Full docs and a live devnet demo: <https://github.com/h4ckm1n-dev/agentspay>.

## License

Licensed under the Apache License, Version 2.0.
