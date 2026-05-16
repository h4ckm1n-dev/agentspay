/**
 * examples/balance.ts — the simplest possible AgentsPay call.
 *
 * Run with:
 *   pnpm --filter @agentspay/sdk-js build
 *   node --import tsx packages/sdk-js/examples/balance.ts
 *
 * Prerequisites:
 *   - agentspay-mcp binary on PATH, OR AGENTSPAY_MCP_BIN env var, OR
 *     pass { mcpBinPath } to the client.
 *   - A keypair at ~/.agentspay/keypair.json (auto-created on first run).
 */

import { AgentsPayClient } from "../src/index.js";

const client = new AgentsPayClient({ network: "solana-devnet" });

const balance = await client.balance();

console.log(`Available:        ${balance.available_usdc} USDC`);
console.log(`Today's spending: ${balance.today_spent_usdc} USDC`);
console.log(`Daily cap:        ${balance.daily_cap_usdc} USDC`);
console.log(`Remaining today:  ${balance.budget_remaining_today_usdc} USDC`);
console.log(`Per-call cap:     ${balance.per_call_cap_usdc} USDC`);
console.log(`Network:          ${balance.environment}`);
console.log(`Signer:           ${balance.solana_pubkey}`);
