# AgentsPay

Budget-controlled USDC for autonomous AI agents.

AgentsPay is a local MCP server that lets Claude Code, Cursor, Cline, or Zed pay x402-priced APIs without handing the agent a credit card or an unrestricted keypair. It enforces per-call and daily caps before signing, records every attempt in SQLite, and can settle real USDC on Solana devnet.

Status: v0.3, devnet. Mainnet is intentionally gated behind a later compliance review.

## Proof

These are real Solana devnet settlements signed through this stack:

| Symbol | Amount | Receipt |
|---|---:|---|
| AAPL | 0.10 USDC | [4pGRMVgu7j5...hv6jYau](https://solscan.io/tx/4pGRMVgu7j5itCs7Vf6G9FTQW2Q1B2SjCEKHszLjvF9eVagWvtWq8aJWuYz1JNpBQr4CsbYRXSb9aWAu5hv6jYau?cluster=devnet) |
| GOOG | 0.10 USDC | [3EUyjsdN7Y2...h2BFJU](https://solscan.io/tx/3EUyjsdN7Y2ZHTUFMaNn3Y3TyMsGcK673Bis5oMw49RgTPGXhUJJiRYyG2JYrkkQypfszJH9FuRBPScTmXh2BFJU?cluster=devnet) |
| GOOG | 0.10 USDC | [ogEatB8NTZ3...FmAFFkJ](https://solscan.io/tx/ogEatB8NTZ3KiLufnWwVjU25jBWygwLNdhNJqKHZPSftgrWUWBdD5P1JQ6kDXVj6HzQnPXb55bcPjCGWFmAFFkJ?cluster=devnet) |

The website proof page shows these permanent records plus the latest browser-triggered transaction when the demo stack is running.

## Install

```bash
git clone https://github.com/h4ckm1n/agentspay
cd agentspay
cargo build --release -p agentspay-mcp
claude mcp add agentspay "$PWD/target/release/agentspay-mcp"
```

First run creates:

```text
~/.agentspay/keypair.json
~/.agentspay/agentspay-mcp.db
```

Ask your MCP host to call `agentspay_topup_info`, then fund the returned pubkey with devnet SOL and USDC:

- SOL faucet: <https://faucet.solana.com>
- USDC faucet: <https://faucet.circle.com>

## MCP Tools

| Tool | Purpose |
|---|---|
| `agentspay_balance()` | Current balance, daily spend, cap, and Solana pubkey. |
| `agentspay_pay_url(url, max_amount_usdc)` | Handles 402, signs the payment, retries, and records the ledger entry. |
| `agentspay_set_budget(daily_usd, per_call_usd)` | Sets local spend policy. |
| `agentspay_audit_log(limit)` | Returns recent tool attempts and settlements. |
| `agentspay_topup_info()` | Returns pubkey and faucet instructions. |

## Run The Website Demo

The Docker stack runs the Next.js site, Rust web shim, demo paid endpoint, Redis, and Caddy. The shim spawns the same `agentspay-mcp` binary that users install.

```bash
cp docker/.env.example docker/.env
docker compose -f docker/docker-compose.yml -f docker/docker-compose.local.yml up --build
```

Open <http://localhost:3000>.

Routes:

- `/` - short landing page
- `/demo` - sandbox and devnet demo tabs
- `/docs` - install, tools, env vars, troubleshooting, host configs
- `/proof` - Solscan receipt ledger

For a funded devnet button, seed the Docker wallet volume with `devnet-wallet.json` and `provider-keypair.json`. See [docker/README.md](docker/README.md).

## What's True Today

- The v0.3 wedge is the local MCP wallet, not the older hosted Stripe-like architecture.
- `agentspay-mcp` is the primary product surface.
- Devnet settlement works through direct Solana RPC.
- Sandbox mode works without chain access.
- The frontend demo uses a Rust shim, not business logic in Next.js API routes.
- `services/{auth,gateway,payment,metering}` and `packages/{sdk-js,sdk-python,cli}` are scaffolding/deferred unless a route says otherwise.
- Mainnet, KYC, hosted facilitator, webhooks, multi-tenant dashboard, and production custody are not shipped.

## Development

```bash
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings

pnpm install
pnpm -r typecheck
pnpm --filter frontend build
```

Useful entry points:

| Path | Role |
|---|---|
| `services/mcp/` | Local MCP server, wallet, ledger, budget policy, x402 payment flow. |
| `services/web-shim/` | HTTP bridge for the website demo. |
| `examples/paid-endpoint/` | Demo x402 provider. |
| `apps/frontend/` | Next.js landing page, docs, proof, and demo UI. |
| `docker/` | Website deployment stack and operator runbook. |
| `Plan.md` | Active v3 execution plan. |

## License

MIT.
