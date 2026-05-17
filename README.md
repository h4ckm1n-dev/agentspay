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

## TypeScript SDK + CLI

`@agentspay/sdk-js` and `@agentspay/cli` (both v0.2.0) wrap the `agentspay-mcp` binary so a Node.js program that is *not* running inside an MCP host can still use the wallet:

```typescript
import { AgentsPayClient } from "@agentspay/sdk-js";
const client = new AgentsPayClient({ network: "solana-devnet" });
const balance = await client.balance();
const result = await client.payUrl({ url: "https://api.example.com/quote", maxAmountUsdc: "0.50" });
```

The CLI exposes the same five tools as terminal commands:

```bash
agentspay balance
agentspay pay-url https://api.example.com/quote --max 0.50
agentspay set-budget --daily 25 --per-call 1
agentspay audit-log --limit 5
agentspay topup-info
```

Both packages live under `packages/` and ship with examples (`packages/sdk-js/examples/`) and pretty + JSON output. See [packages/sdk-js/README.md](packages/sdk-js/README.md).

## Security Model

The agent **cannot drain your wallet** because:

- Every `pay_url` is rate-checked against a **per-call cap** and a **daily cap** before any signing happens.
- A malicious x402 seller cannot inflate the transfer by declaring funky decimals or a different asset — the validators reject `decimals != 6` and `asset != USDC mint` in real-signing modes.
- An LLM-tricked URL targeting `localhost`, RFC1918 ranges, link-local (AWS/GCP metadata at 169.254.169.254), CGNAT, or IPv6 ULA is rejected by the **SSRF guard**. Opt out for local dev with `AGENTSPAY_ALLOW_PRIVATE_HOSTS=1`.
- Response bodies from paid endpoints are **size-capped at 1 MiB** so a malicious seller cannot OOM the agent.
- The keypair lives at `~/.agentspay/keypair.json` mode `0600`; the file is never logged.
- Public demo (web-shim) rate-limits via the **real client IP from `X-Forwarded-For`**, not the direct peer (which would be Caddy in production).
- Browser-side **Origin guard** on mutating endpoints when `AGENTSPAY_ALLOWED_ORIGINS` is set — required for production deployments.
- Containers run as **non-root** (uid 10001 for Rust services, uid 1000 / `node` for the Next.js image).

The full audit, including known-accepted risks and the adversarial test suite, lives in [SECURITY-AUDIT.md](SECURITY-AUDIT.md). Run the regression suite with `cargo test --workspace` (46 Rust tests) plus `pnpm --filter @agentspay/sdk-js test` (10 TypeScript tests).

## What's True Today

- The v0.3 wedge is the local MCP wallet, not the older hosted Stripe-like architecture.
- `agentspay-mcp` is the primary product surface; `@agentspay/sdk-js` and `@agentspay/cli` wrap it for non-MCP-host usage.
- Devnet settlement works through direct Solana RPC.
- Sandbox mode works without chain access.
- The frontend demo uses a Rust shim, not business logic in Next.js API routes.
- `services/{auth,gateway,payment,metering}` and `packages/sdk-python` are scaffolding/deferred unless a route says otherwise.
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
| `examples/paid-endpoint/` | Demo x402 provider (receive side). |
| `apps/frontend/` | Next.js landing page, docs, proof, and demo UI. |
| [`packages/sdk-js/`](packages/sdk-js/README.md) | TypeScript SDK that spawns the MCP binary. |
| [`packages/cli/`](packages/cli/README.md) | `agentspay` command-line tool. |
| [`docker/`](docker/README.md) | Website deployment stack and operator runbook. |
| [`Plan.md`](Plan.md) | Active v3 execution plan (+ §15 v3.1 amendment). |
| [`SECURITY-AUDIT.md`](SECURITY-AUDIT.md) | Threat model, findings, accepted risks. |
| [`CLAUDE.md`](CLAUDE.md) / [`AGENTS.md`](AGENTS.md) | Development guide for AI agents working on this repo. |

## License

MIT.
