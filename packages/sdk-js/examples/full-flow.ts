/**
 * examples/full-flow.ts — set a budget, pay a quote, inspect the audit log.
 *
 * Demonstrates the four most common tools in sequence. Useful as a quick
 * sanity check that the binary, keypair, and ledger are wired correctly.
 *
 * Run with:
 *   cargo run -p agentspay-paid-endpoint-demo   # in another shell
 *   node --import tsx packages/sdk-js/examples/full-flow.ts
 */

import { AgentsPayClient } from "../src/index.js";

const client = new AgentsPayClient({ network: "sandbox" });

console.log("[1] Setting budget…");
const budget = await client.setBudget({ dailyUsd: 5, perCallUsd: 1 });
console.log(`    daily=${budget.daily_usd} per_call=${budget.per_call_usd}`);

console.log("[2] Reading initial balance…");
const before = await client.balance();
console.log(`    spent today: ${before.today_spent_usdc} USDC`);

console.log("[3] Paying a sandbox quote…");
const pay = await client.payUrl({
  url: "http://localhost:3001/quote/AAPL",
  maxAmountUsdc: "0.50",
});
console.log(`    charged: ${pay.amount_charged_usdc} USDC, status=${pay.payment_status}`);

console.log("[4] Reading audit log…");
const log = await client.auditLog({ limit: 5 });
for (const entry of log.entries) {
  const amount = entry.amount_usdc ?? "-";
  console.log(`    ${entry.timestamp_rfc3339}  ${entry.tool.padEnd(14)}  ${amount.padEnd(8)}  ${entry.status}`);
}

console.log("[5] Reading topup info…");
const topup = await client.topupInfo();
console.log(`    pubkey:   ${topup.pubkey}`);
console.log(`    network:  ${topup.network}`);
