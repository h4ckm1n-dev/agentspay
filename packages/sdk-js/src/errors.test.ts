/**
 * Unit tests for the error classifier. Run with `node --test` after `pnpm build`.
 *
 * The wire fixtures below are the REAL messages and `data` payloads emitted by
 * services/mcp/src/main.rs — keep them in sync when the server wording changes.
 * Transport + subprocess behavior is exercised by the integration smoke test in
 * scripts/smoke-test.ts (requires a built agentspay-mcp binary on PATH).
 */

import { strict as assert } from "node:assert";
import { describe, it } from "node:test";

import {
  AgentsPayError,
  BinaryNotFoundError,
  BudgetExceededError,
  DuplicatePaymentError,
  InvalidInputError,
  NetworkMismatchError,
  PerCallCapExceededError,
  PolicyRejectedError,
  ToolError,
  X402SettlementError,
  classifyToolError,
} from "./errors.js";

describe("classifyToolError — structured reason (primary path)", () => {
  it("per_call_budget → PerCallCapExceededError", () => {
    const err = classifyToolError(
      "pay_url",
      "endpoint price exceeds configured per-call budget (required=0.50 per_call_cap=0.10). If this payment is intended, ask the user to raise the cap via agentspay_set_budget.",
      { required_usdc: "0.50", per_call_cap_usd: 0.1, reason: "per_call_budget" },
    );
    assert.ok(err instanceof PerCallCapExceededError);
    assert.equal(err.code, "per_call_cap_exceeded");
  });

  it("above_call_cap → PerCallCapExceededError", () => {
    const err = classifyToolError(
      "pay_url",
      "endpoint requires more than max_amount_usdc (required=0.50, cap=0.10)",
      { required_usdc: "0.50", max_amount_usdc: "0.10", reason: "above_call_cap" },
    );
    assert.ok(err instanceof PerCallCapExceededError);
  });

  it("daily_budget → BudgetExceededError", () => {
    const err = classifyToolError(
      "pay_url",
      "this call would exceed today's remaining budget (required=10.00 today_spent=45.00 daily_cap=50.00). The budget resets at UTC midnight; ask the user to raise it via agentspay_set_budget if this payment is intended.",
      { required_usdc: "10.00", today_spent_usd: 45, daily_cap_usd: 50, reason: "daily_budget" },
    );
    assert.ok(err instanceof BudgetExceededError);
    assert.equal(err.code, "budget_exceeded");
  });

  it("recipient_not_allowed → PolicyRejectedError", () => {
    const err = classifyToolError(
      "pay_url",
      "recipient 9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin is not in the configured allowlist — if this seller is legitimate, ask the user to add it via agentspay_set_budget allowed_recipients",
      { pay_to: "9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin", reason: "recipient_not_allowed" },
    );
    assert.ok(err instanceof PolicyRejectedError);
    assert.equal(err.code, "policy_rejected");
  });

  it("mandate_expired → PolicyRejectedError", () => {
    const err = classifyToolError(
      "pay_url",
      "the spending mandate expired at 2026-07-01T00:00:00+00:00 — ask the user to refresh it via agentspay_set_budget",
      { not_after_rfc3339: "2026-07-01T00:00:00+00:00", reason: "mandate_expired" },
    );
    assert.ok(err instanceof PolicyRejectedError);
  });

  it("duplicate_payment → DuplicatePaymentError", () => {
    const err = classifyToolError(
      "pay_url",
      "a payment with this idempotency_key already settled — not paying again; the original result is referenced below",
      { reason: "duplicate_payment", payment_id: "pay_abc", transaction: "sig", status: "ok status=200" },
    );
    assert.ok(err instanceof DuplicatePaymentError);
    assert.equal(err.code, "duplicate_payment");
  });

  it("payment_in_flight → DuplicatePaymentError", () => {
    const err = classifyToolError(
      "pay_url",
      "a payment with this idempotency_key is still settling — wait and check agentspay_audit_log before retrying",
      { reason: "payment_in_flight" },
    );
    assert.ok(err instanceof DuplicatePaymentError);
  });

  it("settlement_failed → X402SettlementError", () => {
    const err = classifyToolError(
      "pay_url",
      "x402 payment flow failed: server returned unexpected status 502 on retry: upstream down. If the wallet lacks USDC (or SOL for fees), call agentspay_topup_info for funding instructions.",
      { url: "https://api.example.com/quote", reason: "settlement_failed" },
    );
    assert.ok(err instanceof X402SettlementError);
    assert.equal(err.code, "x402_settlement_failed");
  });

  it("ssrf_blocked → InvalidInputError", () => {
    const err = classifyToolError(
      "pay_url",
      "url rejected: internal or non-public host is blocked (host resolves to a non-public IP address)",
      { field: "url", host: "169.254.169.254", reason: "ssrf_blocked", detail: "host resolves to a non-public IP address" },
    );
    assert.ok(err instanceof InvalidInputError);
    assert.equal(err.code, "invalid_input");
  });
});

describe("classifyToolError — prose fallback (no data.reason)", () => {
  it("real per-call wire message classifies without reason", () => {
    const err = classifyToolError(
      "pay_url",
      "endpoint price exceeds configured per-call budget (required=0.50 per_call_cap=0.10)",
    );
    assert.ok(err instanceof PerCallCapExceededError);
  });

  it("real daily wire message classifies without reason", () => {
    const err = classifyToolError(
      "pay_url",
      "this call would exceed today's remaining budget (required=10.00 today_spent=45.00 daily_cap=50.00)",
    );
    assert.ok(err instanceof BudgetExceededError);
  });

  it("real network-mismatch wire message classifies without reason", () => {
    const err = classifyToolError(
      "pay_url",
      'server requirement network="solana-mainnet" does not match the locally configured network="solana-devnet"',
    );
    assert.ok(err instanceof NetworkMismatchError);
  });

  it("real settle-failure wire message classifies without reason", () => {
    const err = classifyToolError(
      "pay_url",
      "x402 payment flow failed: server returned unexpected status 502 on retry: upstream down",
    );
    assert.ok(err instanceof X402SettlementError);
  });

  it("maps url-parse messages to InvalidInputError", () => {
    const err = classifyToolError("pay_url", "url is not a valid URL: empty host");
    assert.ok(err instanceof InvalidInputError);
    assert.equal(err.code, "invalid_input");
  });

  it("falls back to generic ToolError for unknown messages", () => {
    const err = classifyToolError("balance", "some unexpected failure");
    assert.ok(err instanceof ToolError);
    assert.equal(err.code, "tool_error");
    assert.equal(err.tool, "balance");
  });

  it("preserves details on the error", () => {
    const details = { field: "url", value: "" };
    const err = classifyToolError("pay_url", "url is not a valid URL", details);
    assert.deepEqual(err.details, details);
  });
});

describe("error hierarchy", () => {
  it("BinaryNotFoundError inherits from AgentsPayError", () => {
    const err = new BinaryNotFoundError(["/path/to/missing"]);
    assert.ok(err instanceof AgentsPayError);
    assert.equal(err.code, "binary_not_found");
    assert.match(err.message, /not found/);
  });

  it("ToolError prefixes the tool name", () => {
    const err = new ToolError("balance", "db read failed");
    assert.match(err.message, /^\[balance\]/);
  });

  it("subclass names are preserved through subclassing", () => {
    const err = new BudgetExceededError("over cap");
    assert.equal(err.name, "BudgetExceededError");
    assert.ok(err instanceof ToolError);
    assert.ok(err instanceof AgentsPayError);
    assert.ok(err instanceof Error);
  });
});
