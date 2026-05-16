/**
 * Wire-faithful TypeScript surface for the agentspay-mcp binary.
 *
 * Response field names are snake_case to match the JSON the MCP server emits
 * (see services/mcp/src/main.rs structs). This keeps debugging straight:
 * what you see in the SDK matches what you see in `audit_log` output, in
 * MCP server logs, and in the rmcp tool definitions.
 *
 * Input arguments are camelCase (TS convention); the client maps them to
 * snake_case before writing to the JSON-RPC payload.
 */

export type Network = "sandbox" | "solana-devnet" | "solana-mainnet";

export type JsonPrimitive = string | number | boolean | null;
export type JsonValue =
  | JsonPrimitive
  | { readonly [key: string]: JsonValue }
  | readonly JsonValue[];
export type JsonObject = { readonly [key: string]: JsonValue };

// ---------------------------------------------------------------------------
// Client options
// ---------------------------------------------------------------------------

export interface AgentsPayClientOptions {
  /**
   * Absolute path to the agentspay-mcp binary. If omitted, the client looks
   * for `agentspay-mcp` on PATH, then `AGENTSPAY_MCP_BIN`.
   */
  readonly mcpBinPath?: string;

  /**
   * Network the MCP server should target. Sets `AGENTSPAY_NETWORK` on the
   * subprocess. Defaults to `solana-devnet`.
   */
  readonly network?: Network;

  /**
   * Override the keypair path. Sets `AGENTSPAY_KEYPAIR_PATH`.
   * Defaults to `~/.agentspay/keypair.json` (resolved by the binary).
   */
  readonly keypairPath?: string;

  /**
   * Override the SQLite ledger URL. Sets `AGENTSPAY_DATABASE_URL`.
   * Defaults to the binary's own resolution.
   */
  readonly databaseUrl?: string;

  /**
   * Extra environment variables to set on the subprocess.
   */
  readonly env?: Readonly<Record<string, string>>;

  /**
   * Timeout per tool call in milliseconds. Defaults to 30_000 (30s).
   */
  readonly callTimeoutMs?: number;

  /**
   * Emit transport-level debug logs to stderr.
   */
  readonly debug?: boolean;
}

// ---------------------------------------------------------------------------
// Tool: agentspay_balance
// ---------------------------------------------------------------------------

export interface BalanceResponse {
  readonly available_usdc: string;
  readonly budget_remaining_today_usdc: string;
  readonly daily_cap_usdc: string;
  readonly per_call_cap_usdc: string;
  readonly today_spent_usdc: string;
  readonly currency: string;
  readonly environment: Network;
  /** Base58 Solana pubkey the agent will sign x402 payments with. */
  readonly solana_pubkey: string;
}

// ---------------------------------------------------------------------------
// Tool: agentspay_pay_url
// ---------------------------------------------------------------------------

export interface PayUrlInput {
  readonly url: string;
  /** Maximum USDC to authorize for this call, as a decimal string. */
  readonly maxAmountUsdc: string;
}

export interface PayUrlResponse {
  readonly status: string;
  readonly payment_id: string;
  readonly endpoint: string;
  readonly amount_charged_usdc: string;
  readonly ledger_entry_id: string;
  readonly transaction: string;
  /** Upstream response body, as a string. */
  readonly body: string;
  /** `"paid"` when settlement happened, `"none"` for endpoints that served 200 without ever issuing a 402. */
  readonly payment_status: "paid" | "none" | string;
  readonly network: Network;
  /** Solscan URL for the on-chain TX when applicable; empty otherwise. */
  readonly explorer_url: string;
}

// ---------------------------------------------------------------------------
// Tool: agentspay_set_budget
// ---------------------------------------------------------------------------

export interface SetBudgetInput {
  /** Daily spending cap in USD. Must be > 0. */
  readonly dailyUsd: number;
  /** Per-call spending cap in USD. Must be > 0. */
  readonly perCallUsd: number;
}

export interface SetBudgetResponse {
  readonly agent_id: string;
  readonly daily_usd: number;
  readonly per_call_usd: number;
  readonly updated_at_rfc3339: string;
}

// ---------------------------------------------------------------------------
// Tool: agentspay_audit_log
// ---------------------------------------------------------------------------

export interface AuditLogInput {
  /** Number of entries to return. Default 20, max 100. */
  readonly limit?: number;
}

export interface AuditEntry {
  readonly id: string;
  readonly timestamp_rfc3339: string;
  readonly tool: string;
  readonly endpoint?: string;
  readonly amount_usdc?: string;
  readonly status: string;
}

export interface AuditLogResponse {
  readonly entries: ReadonlyArray<AuditEntry>;
  readonly total: number;
  readonly returned: number;
}

// ---------------------------------------------------------------------------
// Tool: agentspay_topup_info
// ---------------------------------------------------------------------------

export interface TopupInfoResponse {
  readonly pubkey: string;
  readonly network: Network;
  readonly faucet_url: string;
  readonly sol_faucet_url: string;
  readonly instructions: string;
}

