/**
 * Typed error hierarchy for AgentsPay.
 *
 * Match against specific subclasses with `instanceof`, or use the `.code` field
 * for switch statements. Every error carries the underlying `cause` when known.
 */

export type AgentsPayErrorCode =
  | "binary_not_found"
  | "transport_failed"
  | "transport_timeout"
  | "tool_error"
  | "invalid_input"
  | "budget_exceeded"
  | "per_call_cap_exceeded"
  | "network_mismatch"
  | "x402_settlement_failed"
  | "unknown";

export interface AgentsPayErrorOptions {
  readonly cause?: unknown;
  readonly details?: unknown;
}

export class AgentsPayError extends Error {
  readonly code: AgentsPayErrorCode;
  readonly details: unknown | undefined;

  constructor(
    message: string,
    code: AgentsPayErrorCode = "unknown",
    options: AgentsPayErrorOptions = {},
  ) {
    super(message, { cause: options.cause });
    this.name = "AgentsPayError";
    this.code = code;
    this.details = options.details;
  }
}

/** The agentspay-mcp binary could not be located or is not executable. */
export class BinaryNotFoundError extends AgentsPayError {
  constructor(searched: ReadonlyArray<string>, options: AgentsPayErrorOptions = {}) {
    super(
      `agentspay-mcp binary not found. Searched: ${searched.join(", ")}. ` +
        `Install with \`cargo install --path services/mcp\` from the repo, ` +
        `or pass { mcpBinPath } to AgentsPayClient.`,
      "binary_not_found",
      options,
    );
    this.name = "BinaryNotFoundError";
  }
}

/** The MCP subprocess failed to start, crashed, or returned malformed JSON-RPC. */
export class TransportError extends AgentsPayError {
  constructor(message: string, options: AgentsPayErrorOptions = {}) {
    super(message, "transport_failed", options);
    this.name = "TransportError";
  }
}

/** The MCP subprocess did not respond within `callTimeoutMs`. */
export class TransportTimeoutError extends AgentsPayError {
  readonly timeoutMs: number;
  constructor(timeoutMs: number, tool: string, options: AgentsPayErrorOptions = {}) {
    super(
      `agentspay-mcp tool '${tool}' did not respond within ${timeoutMs}ms`,
      "transport_timeout",
      options,
    );
    this.name = "TransportTimeoutError";
    this.timeoutMs = timeoutMs;
  }
}

/**
 * The MCP tool returned a JSON-RPC error. Generic catch-all when a more
 * specific subclass below does not apply.
 */
export class ToolError extends AgentsPayError {
  readonly tool: string;
  constructor(
    tool: string,
    message: string,
    code: AgentsPayErrorCode = "tool_error",
    options: AgentsPayErrorOptions = {},
  ) {
    super(`[${tool}] ${message}`, code, options);
    this.name = "ToolError";
    this.tool = tool;
  }
}

/** A tool argument was rejected by the server (invalid URL, bad decimal, etc.). */
export class InvalidInputError extends ToolError {
  constructor(tool: string, message: string, options: AgentsPayErrorOptions = {}) {
    super(tool, message, "invalid_input", options);
    this.name = "InvalidInputError";
  }
}

/** The endpoint's quoted price exceeds the per-call cap. */
export class PerCallCapExceededError extends ToolError {
  constructor(message: string, options: AgentsPayErrorOptions = {}) {
    super("pay_url", message, "per_call_cap_exceeded", options);
    this.name = "PerCallCapExceededError";
  }
}

/** Settling this call would push today's spend over the daily cap. */
export class BudgetExceededError extends ToolError {
  constructor(message: string, options: AgentsPayErrorOptions = {}) {
    super("pay_url", message, "budget_exceeded", options);
    this.name = "BudgetExceededError";
  }
}

/** The wire `network` does not match what the binary was started with. */
export class NetworkMismatchError extends ToolError {
  constructor(message: string, options: AgentsPayErrorOptions = {}) {
    super("pay_url", message, "network_mismatch", options);
    this.name = "NetworkMismatchError";
  }
}

/** x402 verify/settle failed (facilitator rejected, RPC error, etc.). */
export class X402SettlementError extends ToolError {
  constructor(message: string, options: AgentsPayErrorOptions = {}) {
    super("pay_url", message, "x402_settlement_failed", options);
    this.name = "X402SettlementError";
  }
}

/**
 * Map an rmcp-style error message to the most specific error class.
 * The MCP server emits human-readable messages; we pattern-match a few common
 * ones to produce a typed error the caller can `instanceof` against.
 */
export function classifyToolError(
  tool: string,
  message: string,
  details?: unknown,
): ToolError {
  const lower = message.toLowerCase();
  const options = { details };

  if (lower.includes("per-call cap") || lower.includes("per_call cap")) {
    return new PerCallCapExceededError(message, options);
  }
  if (lower.includes("daily cap") || lower.includes("budget exceeded")) {
    return new BudgetExceededError(message, options);
  }
  if (lower.includes("network mismatch") || lower.includes("network must")) {
    return new NetworkMismatchError(message, options);
  }
  if (
    lower.includes("facilitator") ||
    lower.includes("settle") ||
    lower.includes("verify")
  ) {
    return new X402SettlementError(message, options);
  }
  if (
    lower.includes("not a valid url") ||
    lower.includes("invalid params") ||
    lower.includes("must parse")
  ) {
    return new InvalidInputError(tool, message, options);
  }
  return new ToolError(tool, message, "tool_error", options);
}
