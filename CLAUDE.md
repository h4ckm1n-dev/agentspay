# CLAUDE.md — AgentsPay Development Guide

## Project Vision

**AgentsPay** is a local MCP server that gives AI agents (Claude Code, Cursor, Cline, Zed) a budget-controlled USDC wallet for x402-priced APIs. The agent cannot drain the wallet because every payment is checked against a per-call cap and a daily cap before signing.

Status: **v0.3, Solana devnet.** Mainnet is gated behind a v0.5 compliance review.

**Roadmap:** [Plan.md](./Plan.md) is the active v3 plan with §15 v3.1 amendment (website + hosted demo scope).
**Security:** [SECURITY-AUDIT.md](./SECURITY-AUDIT.md) holds the threat model, the 12 findings (4 CRIT + 3 HIGH all fixed), and the adversarial test suite.
**Activity log:** [PROJECT_CONTEXT.md](./PROJECT_CONTEXT.md) is the append-only session log.

## Core Mandates

1. **Wallet cannot be drained.** Per-call cap and daily cap are checked before signing. SSRF guard rejects loopback/RFC1918/IMDS URLs. Asset and decimals validators reject malicious 402 sellers that try to inflate the transfer.
2. **Wire-faithful types.** SDK response shapes use snake_case to mirror what `services/mcp` emits. What you see in `agentspay_audit_log` is what you see in the SDK is what you see in the Rust source.
3. **One subprocess per call.** The SDK and the web-shim both spawn `agentspay-mcp` per request. Cold-start is ~100ms, dwarfed by Solana settlement (~1-2s). No long-lived server, no shared state across calls.
4. **Devnet-only signing today.** Mainnet exists as a `NetworkMode` enum variant but the shipped product targets devnet. The USDC mint is hardcoded to the devnet value.

## Technical Stack

- **MCP server (primary surface):** Rust, `rmcp 0.16`, SeaORM + SQLite, Tokio.
- **TypeScript SDK:** `@agentspay/sdk-js@0.2.0` — wraps the binary via `node:child_process`, exposes 5 typed methods + 9 typed error classes.
- **CLI:** `@agentspay/cli@0.2.0` — 5 subcommands matching the MCP tools, pretty + `--json` output, 256-color ANSI.
- **Web shim:** Rust Axum HTTP bridge that wraps `agentspay-mcp` for the website demo. Not a public REST API.
- **Frontend:** Next.js 15 (App Router), Tailwind CSS, React 19.
- **Demo provider:** `examples/paid-endpoint/` — Rust Axum server that emits x402 challenges.
- **Settlement:** Solana devnet USDC via SPL `transfer_checked` with idempotent recipient-ATA creation.
- **Docker stack:** Caddy + web + shim + paid-endpoint + Redis, all non-root.

## Repository Structure

| Path | Role |
|---|---|
| `services/mcp/` | `agentspay-mcp` binary. The product. |
| `services/web-shim/` | Axum HTTP bridge for the website demo. Not public. |
| `services/{gateway,auth,payment,metering}/` | Plan-v2 scaffolding. Kept for future, unused in v0.3. |
| `packages/sdk-js/` | TypeScript SDK (subprocess transport). |
| `packages/cli/` | `agentspay` command-line tool. |
| `packages/sdk-python/` | Deferred indefinitely per Plan v3.1. |
| `packages/proto/` | gRPC defs, unused in v0.3. |
| `examples/paid-endpoint/` | Demo x402 provider (the "sell" side). |
| `apps/frontend/` | Landing page + `/demo` + `/docs` + `/proof`. |
| `docker/` | 5-service compose stack (caddy / web / shim / paid-endpoint / redis). |
| `Plan.md` | Active execution plan. |
| `SECURITY-AUDIT.md` | Threat model + findings + accepted risks. |

## The 5 MCP Tools

| Tool | Returns |
|---|---|
| `agentspay_balance()` | available_usdc, today_spent, daily_cap, per_call_cap, pubkey, network |
| `agentspay_pay_url(url, max_amount_usdc)` | Handles 402, signs the payment, retries, persists ledger entry |
| `agentspay_set_budget(daily_usd, per_call_usd)` | Updates active policy row |
| `agentspay_audit_log(limit)` | Recent ledger rows including rejected attempts |
| `agentspay_topup_info()` | Pubkey + faucet URLs + instructions |

## Coding Standards

- **Rust:** `tracing` for spans, `thiserror` for typed errors, `anyhow` for top-level. No `unwrap()` in handlers — always return `McpError`. SeaORM transactions for any ledger write. Type-level isolation between sandbox/devnet/mainnet via `NetworkMode` enum.
- **TypeScript:** `strict` + `exactOptionalPropertyTypes` + `noUncheckedIndexedAccess`. No `any`. Errors as typed subclasses (`BudgetExceededError`, `PerCallCapExceededError`, `X402SettlementError`, etc.) so callers can `instanceof`-match.
- **MCP wire format:** snake_case responses, camelCase input args (SDK maps inputs to the wire shape).

## Common Commands

```bash
# Build everything
cargo build --release --workspace
pnpm install

# Test
cargo test --workspace                          # 46 Rust unit + integration tests
pnpm --filter @agentspay/sdk-js test            # 10 SDK error classifier tests
pnpm --filter frontend typecheck                # Next.js strict typecheck

# Lint
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
pnpm --filter frontend lint

# CLI smoke against the built binary (sandbox)
node packages/cli/dist/index.js balance \
  --network sandbox \
  --bin ./target/release/agentspay-mcp

# Local website demo (Docker)
docker compose -f docker/docker-compose.yml \
               -f docker/docker-compose.local.yml \
               up --build
# then open http://localhost:3000

# Frontend dev server
pnpm --filter frontend dev
```

## Key Environment Variables

| Variable | Default | Purpose |
|---|---|---|
| `AGENTSPAY_NETWORK` | `solana-devnet` | `sandbox` / `solana-devnet` / `solana-mainnet` |
| `AGENTSPAY_KEYPAIR_PATH` | `~/.agentspay/keypair.json` | Solana keypair (mode 0600 enforced on first-run) |
| `AGENTSPAY_DATABASE_URL` | `sqlite://~/.agentspay/agentspay-mcp.db` | SeaORM ledger DB |
| `AGENTSPAY_ALLOW_PRIVATE_HOSTS` | unset | Set to `1` to bypass SSRF guard. Local dev only — never in production. |
| `AGENTSPAY_ALLOWED_ORIGINS` | unset | Comma-separated origin allowlist for web-shim Origin guard. Required for production. |
| `AGENTSPAY_SOLANA_RPC_URL` | `https://api.devnet.solana.com` | Solana RPC endpoint |
| `AGENTSPAY_MCP_BIN` (SDK) | PATH lookup | Override binary location for the SDK |

## Security Model Cheat Sheet

- **Per-call + daily caps** — checked before any signing happens.
- **SSRF guard on `pay_url`** — blocks loopback, RFC1918, link-local (incl. AWS/GCP IMDS 169.254.169.254), CGNAT 100.64/10, IPv4 0/8, IPv6 ULA and link-local. Audited when triggered.
- **Asset + decimals validators** — `requirement.asset` must equal `USDC_MINT_DEVNET`; `extra.decimals` must equal 6. Rejects malicious sellers that try to inflate the transfer.
- **1 MiB body cap** on every paid-endpoint response (probe + retry). Prevents OOM via attacker-controlled body.
- **Keypair file mode 0600**, never logged.
- **Web-shim rate-limit** keys on `X-Forwarded-For` (Caddy sets and strips). Per real client IP, not per Caddy IP.
- **Origin guard middleware** on mutating endpoints when `AGENTSPAY_ALLOWED_ORIGINS` is set.
- **Containers run non-root** (uid 10001 for Rust, uid 1000 / `node` for Next.js).
- **CI runs** `cargo test --workspace` + `pnpm --filter @agentspay/sdk-js test` + frontend typecheck/lint/build on every push.

For the full audit including accepted risks, see [SECURITY-AUDIT.md](./SECURITY-AUDIT.md).
