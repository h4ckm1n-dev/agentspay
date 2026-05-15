# AgentsPay

**A budget-controlled USDC wallet for autonomous AI agents.**
One MCP install. Real Solana settlement. Per-call and daily caps enforced before the chain.

> Status — v0.3, devnet. Three independent on-chain settlements have been produced through this stack on Solana devnet (see "Proof" below). Mainnet is gated behind a v0.5 compliance posture review.

---

## Two ways to try it

### 1. The marketing site (Docker — the visitor experience)

A single-page site with a live in-browser sandbox demo and a one-click "trigger a real on-chain transaction" button. The demo subprocess inside the shim container is the *literal* `agentspay-mcp` binary an end user installs — not a re-implementation.

```bash
git clone https://github.com/<your-fork>/agentspay
cd agentspay
cp docker/.env.example docker/.env
# (Optional) seed the devnet wallet for the on-chain demo button.
# Without this step, the sandbox tab still works; only /api/devnet/* is rate-limited
# to "wallet drained".
docker run --rm -v docker_wallet-data:/data -v $HOME/.agentspay:/host alpine \
  sh -c "cp /host/keypair.json /data/devnet-wallet.json \
      && cp /host/provider-keypair.json /data/provider-keypair.json"
docker compose -f docker/docker-compose.yml -f docker/docker-compose.local.yml up -d
open http://localhost:3000
```

The runbook for a real VPS + TLS deployment is in [`docker/README.md`](docker/README.md).

### 2. The MCP binary (Claude Code / Cursor / Cline native)

Skip the website if you just want the agent wallet inside your MCP host.

```bash
cargo build --release -p agentspay-mcp
claude mcp add agentspay $PWD/target/release/agentspay-mcp
# First run writes a keypair to ~/.agentspay/keypair.json (mode 0600).
# Get the address with: ask the agent to call `agentspay_topup_info`.
# Then fund it: https://faucet.solana.com + https://faucet.circle.com (Solana Devnet).
```

Five MCP tools become available to the agent: `agentspay_balance`, `agentspay_pay_url`, `agentspay_set_budget`, `agentspay_audit_log`, `agentspay_topup_info`.

---

## Proof — real on-chain settlements

Each link below is a real, permanent, on-chain transaction signed by `agentspay-mcp` and broadcast to `api.devnet.solana.com` during testing. The signatures cannot be forged — open them in Solscan to verify.

| Tx | Cost | Symbol | Context |
|---|---|---|---|
| [`4pGRMVgu7j5...`](https://solscan.io/tx/4pGRMVgu7j5itCs7Vf6G9FTQW2Q1B2SjCEKHszLjvF9eVagWvtWq8aJWuYz1JNpBQr4CsbYRXSb9aWAu5hv6jYau?cluster=devnet) | 0.10 USDC | AAPL | First end-to-end real-mode smoke (native CLI) |
| [`3EUyjsdN7Y2...`](https://solscan.io/tx/3EUyjsdN7Y2ZHTUFMaNn3Y3TyMsGcK673Bis5oMw49RgTPGXhUJJiRYyG2JYrkkQypfszJH9FuRBPScTmXh2BFJU?cluster=devnet) | 0.10 USDC | GOOG | Through the web-shim HTTP bridge |
| [`ogEatB8NTZ3...`](https://solscan.io/tx/ogEatB8NTZ3KiLufnWwVjU25jBWygwLNdhNJqKHZPSftgrWUWBdD5P1JQ6kDXVj6HzQnPXb55bcPjCGWFmAFFkJ?cluster=devnet) | 0.10 USDC | GOOG | Through the full Next.js → shim → mcp → paid-endpoint chain |

---

## How it works

```
┌─────────────────────────────────────────────────────────┐
│ MCP host (Claude Code / Cursor / Cline / Anthropic     │
│ Desktop) OR the marketing website at localhost:3000     │
└─────────────────────────┬───────────────────────────────┘
                          │
                          ▼
              MCP stdio JSON-RPC  ──┐
                                    │
              (or HTTPS via the     │
               web-shim's /api/*)   │
                                    ▼
                  ┌─────────────────────────────────┐
                  │ agentspay-mcp (Rust)            │
                  │  · 5 tools                      │
                  │  · per-call + daily budget caps │
                  │    enforced before signature    │
                  │  · SeaORM + SQLite ledger       │
                  │  · solana-sdk SPL transfer     │
                  └─────────────┬───────────────────┘
                                │ HTTPS
                                ▼
                  ┌─────────────────────────────────┐
                  │ x402-priced API (e.g.           │
                  │ examples/paid-endpoint)         │
                  └─────────────┬───────────────────┘
                                │ HTTPS
                                ▼
                  ┌─────────────────────────────────┐
                  │ api.devnet.solana.com           │
                  │  SPL Token program              │
                  │  USDC mint 4zMMC9s...           │
                  └─────────────────────────────────┘
```

The agent's keypair is held locally (mode `0600`). Budget enforcement is a Rust-level invariant — the keypair never touches a transaction whose amount would push the daily total over the configured cap. `Tokio::Mutex` serializes the critical section so two parallel calls cannot both pass against a stale view.

---

## Repository

| Path | Role |
|---|---|
| [`services/mcp/`](services/mcp/) | The `agentspay-mcp` binary an end user installs. Five MCP tools over stdio. SeaORM + SQLite ledger. Real Solana devnet signing. |
| [`services/web-shim/`](services/web-shim/) | HTTP bridge between the marketing website and the local `agentspay-mcp` binary. Six endpoints, rate-limited, Redis-or-in-memory sessions. Spawns the literal mcp binary per request — the website demos the actual product, not a mock. |
| [`apps/frontend/`](apps/frontend/) | Next.js 15 dev-dark marketing site. Hero · Install · Live Demo (sandbox + devnet tabs) · How it works · Why · Footer. |
| [`examples/paid-endpoint/`](examples/paid-endpoint/) | An x402-priced demo provider (Axum). `/quote/:symbol` for sandbox mode, `/real-quote/:symbol` for real Solana devnet settlement. |
| [`docker/`](docker/) | 5-service production stack (caddy · web · shim · paid-endpoint · redis). Multi-stage builds keep the runtime images <330 MB. |
| [`docs/superpowers/`](docs/superpowers/) | The spec + plan that drove the website work. `specs/2026-05-14-website-and-live-demo-design.md`, `plans/2026-05-14-website-and-live-demo.md`. |
| `services/{auth,gateway,payment,metering}/`, `packages/proto/` | Scaffolding for the v0.2+ multi-service architecture. Not built into v0.1. |
| `packages/{cli,sdk-js,sdk-python}/` | Deferred. v0.3+. |
| `Plan-v2-archive.md` | The original "Stripe for agents" architecture, archived for reference. Plan v3 is the active execution plan. |

---

## Key environment variables

| Variable | Default | Used by |
|---|---|---|
| `AGENTSPAY_NETWORK` | `solana-devnet` | mcp · selects `sandbox`/`solana-devnet`/`solana-mainnet` |
| `AGENTSPAY_KEYPAIR_PATH` | `~/.agentspay/keypair.json` | mcp · override the agent's keypair location |
| `AGENTSPAY_DATABASE_URL` | `sqlite://~/.agentspay/agentspay-mcp.db?mode=rwc` | mcp · SeaORM ledger location |
| `AGENTSPAY_SHIM_LISTEN_ADDR` | `0.0.0.0:8080` | shim · HTTP bind address |
| `AGENTSPAY_MCP_BINARY` | `/usr/local/bin/agentspay-mcp` | shim · path to the mcp binary it spawns per request |
| `AGENTSPAY_REDIS_URL` | _(unset → in-memory)_ | shim · Redis URL for cross-process sessions + rate limits |
| `AGENTSPAY_DEVNET_WALLET_PATH` | `/data/devnet-wallet.json` | shim · the rate-limited demo wallet for `/api/devnet/trigger` |
| `AGENTSPAY_DEVNET_LEDGER_PATH` | `/data/devnet-ledger.db` | shim · SQLite path for the durable `LatestTxCache` |
| `AGENTSPAY_PAID_ENDPOINT_URL` | `http://localhost:3001` | shim · the upstream x402 endpoint the demo trigger calls |
| `AGENTSPAY_DEMO_PAYTO` | _(persistent default)_ | paid-endpoint · override the recipient address |
| `AGENTSPAY_PROVIDER_KEYPAIR` | `~/.agentspay/provider-keypair.json` | paid-endpoint · the provider's keypair file |
| `AGENTSPAY_USE_FACILITATOR` | `false` | paid-endpoint · forward verify+settle to `x402.org/facilitator` when `true` |

---

## What's NOT in v0.1 (explicit non-goals)

- Mainnet. Devnet only.
- KYC / compliance workflow. Sandbox + devnet are open; mainnet requires a v0.5 review.
- A custodial flow. Keypairs live on the user's machine (`~/.agentspay/`, mode `0600`).
- TypeScript or Python SDKs. The MCP server *is* the integration surface for v0.1.
- A hosted facilitator API. The CDP / x402.org facilitators handle settlement when you want to use one.
- Webhooks, multi-tenancy, an admin dashboard. v0.3+.

---

## Development

```bash
# Workspace check (all crates)
cargo check --workspace
cargo test --workspace

# Frontend
pnpm install
pnpm --filter frontend dev          # http://localhost:3000
pnpm --filter frontend lint
pnpm --filter frontend typecheck

# Sandbox smoke test (no on-chain spend)
./scripts/devnet-smoke-test.sh      # native CLI flow
docker compose -f docker/docker-compose.yml -f docker/docker-compose.local.yml up --build   # full stack
```

CI runs `cargo check + test + clippy --workspace -- -D warnings` and the frontend `lint + typecheck + build` on every push. Docker images are published to `ghcr.io/<org>/agentspay-{shim,paid-endpoint,web}` on every `v*` tag.

---

## License

Open source, MIT. Built in Rust + Next.js.

See [`Plan.md`](Plan.md) for the active execution plan and [`PROJECT_CONTEXT.md`](PROJECT_CONTEXT.md) for the running collaboration log.
