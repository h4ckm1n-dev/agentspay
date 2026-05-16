/**
 * Unit tests for the error classifier. Run with `node --test` after `pnpm build`.
 *
 * These cover the pure logic in errors.ts. Transport + subprocess behavior is
 * exercised by the integration smoke test in scripts/smoke-test.ts (requires
 * a built agentspay-mcp binary on PATH).
 */

import { strict as assert } from "node:assert";
import { describe, it } from "node:test";

import {
  AgentsPayError,
  BinaryNotFoundError,
  BudgetExceededError,
  InvalidInputError,
  NetworkMismatchError,
  PerCallCapExceededError,
  ToolError,
  X402SettlementError,
  classifyToolError,
} from "./errors.js";

describe("classifyToolError", () => {
  it("maps per-call cap messages to PerCallCapExceededError", () => {
    const err = classifyToolError("pay_url", "endpoint price 0.50 exceeds per-call cap 0.10");
    assert.ok(err instanceof PerCallCapExceededError);
    assert.equal(err.code, "per_call_cap_exceeded");
    assert.equal(err.tool, "pay_url");
  });

  it("maps daily-cap messages to BudgetExceededError", () => {
    const err = classifyToolError("pay_url", "daily cap exceeded by this call");
    assert.ok(err instanceof BudgetExceededError);
    assert.equal(err.code, "budget_exceeded");
  });

  it("maps facilitator errors to X402SettlementError", () => {
    const err = classifyToolError("pay_url", "facilitator /verify returned 400");
    assert.ok(err instanceof X402SettlementError);
    assert.equal(err.code, "x402_settlement_failed");
  });

  it("maps network mismatch messages to NetworkMismatchError", () => {
    const err = classifyToolError("pay_url", "network mismatch: payload says devnet, server is mainnet");
    assert.ok(err instanceof NetworkMismatchError);
    assert.equal(err.code, "network_mismatch");
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
