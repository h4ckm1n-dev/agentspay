# @agentspay/sdk-js

TypeScript SDK for [AgentsPay](https://agentspay.dev) — a local MCP server that gives AI agents a budget-controlled USDC wallet.

This SDK is the way to use AgentsPay from a TypeScript program that is **not** running inside an MCP host (Claude Code, Cursor, Cline). It spawns the `agentspay-mcp` binary as a subprocess and exposes the five MCP tools as typed methods.

## Install

```bash
pnpm add @agentspay/sdk-js
# or: npm install @agentspay/sdk-js
```

You also need the `agentspay-mcp` binary on PATH. Build it from this repo:

```bash
cargo build --release -p agentspay-mcp
cp target/release/agentspay-mcp /usr/local/bin/
```

Or set `AGENTSPAY_MCP_BIN` (env var) or pass `{ mcpBinPath }` to the client.

## Quick start

```typescript
import { AgentsPayClient } from "@agentspay/sdk-js";

const client = new AgentsPayClient({ network: "solana-devnet" });

// What's in the wallet?
const balance = await client.balance();
console.log(`${balance.available_usdc} USDC, ${balance.budget_remaining_today_usdc} remaining today`);

// Set spending caps.
await client.setBudget({ dailyUsd: 25, perCallUsd: 1 });

// Pay a real Solana devnet endpoint.
const result = await client.payUrl({
  url: "http://localhost:3001/real-quote/AAPL",
  maxAmountUsdc: "0.50",
});
console.log(result.body);                  // the upstream response
console.log(result.transaction);           // Solana signature
console.log(result.explorer_url);          // Solscan link
```

## Tools

The client exposes one method per MCP tool. Response fields are snake_case to match the wire format (see the [docs](https://agentspay.dev/docs)).

| Method | MCP tool | Returns |
|---|---|---|
| `balance()` | `agentspay_balance` | Current balance, budget, today's spend, pubkey |
| `payUrl({ url, maxAmountUsdc })` | `agentspay_pay_url` | Settlement + upstream body |
| `setBudget({ dailyUsd, perCallUsd })` | `agentspay_set_budget` | Updated policy row |
| `auditLog({ limit? })` | `agentspay_audit_log` | Recent ledger rows |
| `topupInfo()` | `agentspay_topup_info` | Pubkey + faucet URLs |

## Errors

Match on specific subclasses with `instanceof`:

```typescript
import {
  AgentsPayClient,
  BudgetExceededError,
  PerCallCapExceededError,
  X402SettlementError,
} from "@agentspay/sdk-js";

try {
  await client.payUrl({ url, maxAmountUsdc: "0.50" });
} catch (err) {
  if (err instanceof PerCallCapExceededError) {
    // The endpoint asks for more than the per-call cap.
  } else if (err instanceof BudgetExceededError) {
    // Today's spend + this call would exceed the daily cap.
  } else if (err instanceof X402SettlementError) {
    // The Solana RPC or x402 facilitator rejected the payload.
  } else {
    throw err;
  }
}
```

All errors carry a `.code` (typed string union) and inherit from `AgentsPayError`.

## Configuration

```typescript
new AgentsPayClient({
  mcpBinPath: "/usr/local/bin/agentspay-mcp",  // override binary location
  network: "solana-devnet",                     // or "sandbox" or "solana-mainnet"
  keypairPath: "/path/to/keypair.json",        // override AGENTSPAY_KEYPAIR_PATH
  databaseUrl: "sqlite:///path/to/ledger.db",  // override AGENTSPAY_DATABASE_URL
  env: { RUST_LOG: "info" },                   // extra env vars on subprocess
  callTimeoutMs: 60_000,                        // default 30_000
  debug: true,                                  // inherit subprocess stderr
});
```

## Notes

- Node.js 18+. One subprocess per tool call; cold-start cost ~100–150ms. For `pay_url` on devnet, total latency is dominated by the on-chain settlement (~1–2s), so the spawn cost is invisible.
- The SDK does not implement payment retries or the x402 protocol itself — that all lives in the `agentspay-mcp` binary. The SDK is a thin typed wrapper.
- Browser support: **not yet**. The transport requires `node:child_process`. For browser-side usage, run the [web-shim](https://github.com/h4ckm1n-dev/agentspay/tree/main/services/web-shim) and call it via HTTP.

## License

MIT
