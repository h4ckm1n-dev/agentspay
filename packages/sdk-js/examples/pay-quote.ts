/**
 * examples/pay-quote.ts — pay a real x402-priced URL and capture the body.
 *
 * Pairs with the demo provider at examples/paid-endpoint/. Bring it up first:
 *
 *   cargo run -p agentspay-paid-endpoint-demo
 *
 * Then run this example:
 *
 *   pnpm --filter @agentspay/sdk-js build
 *   node --import tsx packages/sdk-js/examples/pay-quote.ts
 *
 * Sandbox mode (no real settlement) is the default here so this example can
 * run without a funded keypair. Flip the network to "solana-devnet" once
 * you've topped up the agent with the URLs returned by agentspay_topup_info.
 */

import {
  AgentsPayClient,
  BudgetExceededError,
  PerCallCapExceededError,
  X402SettlementError,
} from "../src/index.js";

const client = new AgentsPayClient({ network: "sandbox" });

try {
  const result = await client.payUrl({
    url: "http://localhost:3001/quote/AAPL",
    maxAmountUsdc: "0.50",
  });

  console.log("Status:        ", result.status);
  console.log("Charged:       ", result.amount_charged_usdc, "USDC");
  console.log("Payment status:", result.payment_status);
  console.log("Body:          ", result.body);
  if (result.transaction !== "") {
    console.log("Tx signature:  ", result.transaction);
    console.log("Solscan:       ", result.explorer_url);
  }
} catch (err) {
  if (err instanceof PerCallCapExceededError) {
    console.error("Endpoint quoted more than the per-call cap.");
  } else if (err instanceof BudgetExceededError) {
    console.error("Today's spending would exceed the daily cap.");
  } else if (err instanceof X402SettlementError) {
    console.error("Solana RPC or x402 facilitator rejected the payload.");
    console.error(err.message);
  } else {
    throw err;
  }
  process.exit(1);
}
