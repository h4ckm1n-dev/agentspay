# AgentsPay

AgentsPay is an MCP server that gives an autonomous AI agent a budget-controlled USDC wallet on Solana devnet.

The agent's MCP host (Claude Code, Cursor, Cline, ...) invokes five tools — `agentspay_balance`, `agentspay_pay_url`, `agentspay_set_budget`, `agentspay_audit_log`, `agentspay_topup_info` — over stdio. `agentspay_pay_url` automatically handles the x402 `402 Payment Required` flow: it parses the challenge, applies the per-call and daily budget caps, signs a real Solana USDC `transfer_checked` transaction with the local keypair, retries the request with the signed payload in the `X-PAYMENT` header, persists a ledger + audit row, and returns the resource body. An `AGENTSPAY_NETWORK=sandbox` escape hatch keeps the offline demo path alive for tests.

## Real Solana devnet demo

Prerequisites: a Rust toolchain (the repo's `rust-toolchain.toml` pins the channel) and an MCP host such as the Claude Code CLI.

1. Build both binaries.
   ```bash
   cargo build -p agentspay-mcp -p agentspay-paid-endpoint-demo
   ```
2. Start the MCP server in one terminal. The first run writes a keypair to `~/.agentspay/keypair.json` (mode `0600`) and logs the address on stderr. Copy the `solana_pubkey=...` value from the log line.
   ```bash
   ./target/debug/agentspay-mcp < /dev/null
   ```
   Alternatively, ask the MCP host to call `agentspay_topup_info` — the response includes the pubkey, faucet URL, and step-by-step instructions.
3. Fund the agent address.
   - Visit <https://faucet.solana.com>, request a small amount of SOL (used for rent + signature fees).
   - Visit <https://faucet.circle.com>, pick **Solana Devnet**, paste the same address, request USDC. The Circle faucet requires a manual web captcha — this step cannot be automated.
4. In a second terminal, start the demo provider. Override the recipient address with your own pubkey (or any reachable devnet address) using `AGENTSPAY_DEMO_PAYTO`; otherwise an ephemeral keypair is generated and logged.
   ```bash
   AGENTSPAY_DEMO_PAYTO=<your-recipient-pubkey> ./target/debug/agentspay-paid-endpoint-demo
   ```
5. Wire the MCP server into Claude Code (or your MCP host).
   ```bash
   claude mcp add agentspay $PWD/target/debug/agentspay-mcp
   ```
6. Ask the agent:
   > Buy me an AAPL quote with budget 0.50 USDC from <http://localhost:3001/real-quote/AAPL>.
7. The tool response includes a `transaction` field and an `explorer_url` (Solscan devnet) you can open to verify the on-chain settlement.

A scripted version of step 6 lives at `scripts/devnet-smoke-test.sh`. It does not auto-fund — you still need to top up the pubkey manually before settlement will succeed.

```bash
./scripts/devnet-smoke-test.sh
```

### Routes on the demo provider

| Route | Network | Behavior |
|---|---|---|
| `GET /quote/:symbol` | `sandbox` | Accepts any non-empty `X-PAYMENT` header. No blockchain. Use with `AGENTSPAY_NETWORK=sandbox` for offline regression tests. |
| `GET /real-quote/:symbol` | `solana-devnet` | Requires a base64-encoded, bincode-serialized, signed Solana `Transaction` in the `X-PAYMENT` payload. Submits to devnet RPC, returns the on-chain signature in `X-PAYMENT-RESPONSE`. |
| `GET /health` | — | Liveness probe. |

### Environment variables

| Variable | Default | Purpose |
|---|---|---|
| `AGENTSPAY_NETWORK` | `solana-devnet` | Settlement back-end. One of `solana-devnet`, `solana-mainnet`, `sandbox`. |
| `AGENTSPAY_KEYPAIR_PATH` | `~/.agentspay/keypair.json` | Override path to the agent's Solana keypair (Solana-CLI JSON byte array). |
| `AGENTSPAY_SOLANA_RPC_URL` | `https://api.devnet.solana.com` | RPC endpoint used by both the MCP server and the demo provider. |
| `AGENTSPAY_DEMO_PAYTO` | _(ephemeral)_ | Recipient pubkey for the demo provider's `/real-quote/*` route. |
| `AGENTSPAY_PROVIDER_KEYPAIR` | _(ephemeral)_ | Fallback path to a Solana CLI keypair JSON for the demo provider. Used if `AGENTSPAY_DEMO_PAYTO` is unset. |
| `AGENTSPAY_DATABASE_URL` | `sqlite://~/.agentspay/agentspay-mcp.db?mode=rwc` | SeaORM database URL. |
| `RUST_LOG` | `agentspay_mcp=info` | Tracing filter. |

## Repository

- `services/mcp`: the MCP server binary (`agentspay-mcp`). v0.3 surface.
- `examples/paid-endpoint`: the Axum demo provider (`agentspay-paid-endpoint-demo`) — exposes `/quote/:symbol` (sandbox) and `/real-quote/:symbol` (real Solana devnet) on `127.0.0.1:3001`.
- `services/gateway`, `services/auth`, `services/payment`, `services/metering`: scaffolding for v0.2+. Not built in the demo.
- `packages/proto`: gRPC source of truth (kept for v0.2; no runtime traffic in v0.1).
- `packages/cli`, `packages/sdk-js`, `packages/sdk-python`, `apps/frontend`: deferred.

See `Plan.md` for the current execution plan and `PROJECT_CONTEXT.md` for the running collaboration log.
