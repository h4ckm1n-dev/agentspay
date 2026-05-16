/**
 * AgentsPayClient — TypeScript surface over the agentspay-mcp binary.
 *
 * Each method spawns one MCP subprocess (see ./transport.ts) and returns the
 * wire-faithful response shape (snake_case fields, matching the Rust source
 * of truth in services/mcp/src/main.rs).
 */

import { resolveTransport, callTool, type ResolvedTransportConfig } from "./transport.js";
import { InvalidInputError } from "./errors.js";
import type {
  AgentsPayClientOptions,
  AuditLogInput,
  AuditLogResponse,
  BalanceResponse,
  JsonValue,
  Network,
  PayUrlInput,
  PayUrlResponse,
  SetBudgetInput,
  SetBudgetResponse,
  TopupInfoResponse,
} from "./types.js";

export class AgentsPayClient {
  readonly network: Network;
  readonly binPath: string;

  private readonly config: ResolvedTransportConfig;

  constructor(options: AgentsPayClientOptions = {}) {
    this.config = resolveTransport(options);
    this.network = this.config.network;
    this.binPath = this.config.binPath;
  }

  /**
   * Return the current USDC balance, today's spending, daily/per-call caps,
   * and the agent's signing pubkey.
   */
  async balance(): Promise<BalanceResponse> {
    const result = await callTool(this.config, "agentspay_balance", {});
    return result as unknown as BalanceResponse;
  }

  /**
   * Call an x402-priced URL up to `maxAmountUsdc`. On `solana-devnet` this
   * signs a real SPL USDC transfer. On `sandbox` it sends a placeholder
   * payload that the demo provider accepts.
   *
   * Throws PerCallCapExceededError, BudgetExceededError, X402SettlementError,
   * NetworkMismatchError, or InvalidInputError depending on what went wrong.
   */
  async payUrl(input: PayUrlInput): Promise<PayUrlResponse> {
    if (input.url === "") {
      throw new InvalidInputError("pay_url", "url must not be empty");
    }
    if (input.maxAmountUsdc === "") {
      throw new InvalidInputError(
        "pay_url",
        "maxAmountUsdc must not be empty",
      );
    }
    const args: JsonValue = {
      url: input.url,
      max_amount_usdc: input.maxAmountUsdc,
    };
    const result = await callTool(this.config, "agentspay_pay_url", args);
    return result as unknown as PayUrlResponse;
  }

  /**
   * Update the active per-call and daily USDC spending caps. Subsequent
   * `payUrl` calls are checked against the new caps before settling.
   */
  async setBudget(input: SetBudgetInput): Promise<SetBudgetResponse> {
    if (!(input.dailyUsd > 0)) {
      throw new InvalidInputError(
        "set_budget",
        "dailyUsd must be a positive number",
      );
    }
    if (!(input.perCallUsd > 0)) {
      throw new InvalidInputError(
        "set_budget",
        "perCallUsd must be a positive number",
      );
    }
    const args: JsonValue = {
      daily_usd: input.dailyUsd,
      per_call_usd: input.perCallUsd,
    };
    const result = await callTool(this.config, "agentspay_set_budget", args);
    return result as unknown as SetBudgetResponse;
  }

  /**
   * Return the most recent audit-log entries: tool name, endpoint, amount,
   * status. Default 20, max 100.
   */
  async auditLog(input: AuditLogInput = {}): Promise<AuditLogResponse> {
    const args: JsonValue = input.limit !== undefined ? { limit: input.limit } : {};
    const result = await callTool(this.config, "agentspay_audit_log", args);
    return result as unknown as AuditLogResponse;
  }

  /**
   * Return the agent pubkey and faucet URLs for the configured network.
   * Useful for printing top-up instructions to the user.
   */
  async topupInfo(): Promise<TopupInfoResponse> {
    const result = await callTool(this.config, "agentspay_topup_info", {});
    return result as unknown as TopupInfoResponse;
  }
}
