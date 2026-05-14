import type { JsonValue } from "./types.js";

export interface AgentsPayErrorOptions {
  readonly code?: string;
  readonly status?: number;
  readonly details?: JsonValue;
  readonly cause?: unknown;
}

export class AgentsPayError extends Error {
  readonly code: string;
  readonly status?: number;
  readonly details?: JsonValue;

  constructor(message: string, options: AgentsPayErrorOptions = {}) {
    super(message, { cause: options.cause });
    this.name = "AgentsPayError";
    this.code = options.code ?? "agentspay_error";
    this.status = options.status;
    this.details = options.details;
  }
}

export class AgentsPayApiError extends AgentsPayError {
  constructor(message: string, options: AgentsPayErrorOptions = {}) {
    super(message, {
      ...options,
      code: options.code ?? "agentspay_api_error",
    });
    this.name = "AgentsPayApiError";
  }
}
