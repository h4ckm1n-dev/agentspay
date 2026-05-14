import { AgentsPayApiError, AgentsPayError } from "./errors.js";
import type {
  AgentsPayClientOptions,
  AgentsPayEnvironment,
  AuthorizePaymentOptions,
  FetchLike,
  HealthResponse,
  JsonObject,
  JsonValue,
  PayAndCallInput,
  PayAndCallResult,
  PaymentAuthorization,
  PaymentRequirement,
  PaymentRequirementInput,
  PaymentSettlement,
  PaymentVerification,
  RequestOptions,
  SettlePaymentInput,
  StatusResponse,
  VerifyPaymentInput,
} from "./types.js";

const DEFAULT_BASE_URL = "http://localhost:8080";
const DEFAULT_ENVIRONMENT: AgentsPayEnvironment = "sandbox";

export class AgentsPayClient {
  readonly baseUrl: string;
  readonly environment: AgentsPayEnvironment;

  private readonly apiKey?: string;
  private readonly defaultHeaders?: HeadersInit;
  private readonly fetcher: FetchLike;
  private readonly debug: boolean;

  constructor(options: AgentsPayClientOptions = {}) {
    this.baseUrl = normalizeBaseUrl(options.baseUrl ?? DEFAULT_BASE_URL);
    this.environment = options.environment ?? DEFAULT_ENVIRONMENT;
    this.apiKey = options.apiKey;
    this.defaultHeaders = options.defaultHeaders;
    this.fetcher = options.fetch ?? globalThis.fetch.bind(globalThis);
    this.debug = options.debug ?? false;
  }

  async health(): Promise<HealthResponse> {
    return this.request<HealthResponse>("/v1/health", { method: "GET" });
  }

  async status(): Promise<StatusResponse> {
    return this.request<StatusResponse>("/v1/status", { method: "GET" });
  }

  async createPaymentRequirement(
    input: PaymentRequirementInput,
    options: RequestOptions = {},
  ): Promise<PaymentRequirement> {
    const idempotencyKey =
      options.idempotencyKey ?? input.idempotencyKey ?? createIdempotencyKey();
    const body = buildPaymentRequirementBody(input, idempotencyKey);

    const payload = await this.request<JsonObject>("/v1/payment-requirements", {
      method: "POST",
      body,
      headers: options.headers,
      idempotencyKey,
    });

    return coercePaymentRequirement(payload, {
      amount: input.amount,
      currency: input.currency ?? "USDC",
      endpointId: input.endpointId,
      description: input.description,
    });
  }

  async authorizePayment(
    requirement: PaymentRequirement,
    options: AuthorizePaymentOptions = {},
  ): Promise<PaymentAuthorization> {
    const idempotencyKey = options.idempotencyKey ?? createIdempotencyKey();
    const body: Record<string, JsonValue> = {
      payment_requirement: paymentRequirementToJson(requirement),
      idempotency_key: idempotencyKey,
    };

    if (options.maxAmount !== undefined) {
      body.max_amount = options.maxAmount;
    }
    if (options.payerAgentId !== undefined) {
      body.payer_agent_id = options.payerAgentId;
    }
    if (options.metadata !== undefined) {
      body.metadata = options.metadata;
    }

    const payload = await this.request<JsonObject>("/v1/payments/authorize", {
      method: "POST",
      body,
      headers: options.headers,
      idempotencyKey,
    });

    return coerceAuthorization(payload, requirement.id, idempotencyKey);
  }

  async verifyPayment(
    input: VerifyPaymentInput,
    options: RequestOptions = {},
  ): Promise<PaymentVerification> {
    const idempotencyKey =
      options.idempotencyKey ?? input.idempotencyKey ?? createIdempotencyKey();
    const body: Record<string, JsonValue> = {
      payment_requirement: paymentRequirementToJson(input.requirement),
      authorization: authorizationToJson(input.authorization),
      idempotency_key: idempotencyKey,
    };

    if (input.metadata !== undefined) {
      body.metadata = input.metadata;
    }

    const payload = await this.request<JsonObject>("/v1/payments/verify", {
      method: "POST",
      body,
      headers: options.headers,
      idempotencyKey,
    });

    return coerceVerification(payload);
  }

  async settlePayment(
    input: SettlePaymentInput,
    options: RequestOptions = {},
  ): Promise<PaymentSettlement> {
    const idempotencyKey =
      options.idempotencyKey ?? input.idempotencyKey ?? createIdempotencyKey();
    const body: Record<string, JsonValue> = {
      authorization: authorizationToJson(input.authorization),
      idempotency_key: idempotencyKey,
    };

    if (input.requirement !== undefined) {
      body.payment_requirement = paymentRequirementToJson(input.requirement);
    }
    if (input.verification !== undefined) {
      body.verification = verificationToJson(input.verification);
    }
    if (input.metadata !== undefined) {
      body.metadata = input.metadata;
    }

    const payload = await this.request<JsonObject>("/v1/payments/settle", {
      method: "POST",
      body,
      headers: options.headers,
      idempotencyKey,
    });

    return coerceSettlement(payload);
  }

  async payAndCall<TData = unknown>(
    input: PayAndCallInput,
  ): Promise<PayAndCallResult<TData>> {
    const initialRequest = buildPaidRequest(input);
    const initialResponse = await this.fetcher(input.url, initialRequest);

    if (initialResponse.status !== 402 || input.retryOn402 === false) {
      return {
        response: initialResponse,
        data: await parseResponseData<TData>(initialResponse),
        paymentRequired: initialResponse.status === 402,
      };
    }

    const requirementPayload = await readResponseJson(initialResponse.clone());
    const requirement =
      input.paymentRequirement ??
      extractPaymentRequirement(requirementPayload, input, initialResponse);

    this.log("received 402 payment requirement", requirement);

    const authorization = await this.authorizePayment(requirement, {
      maxAmount: input.maxAmount,
      idempotencyKey: input.idempotencyKey,
    });

    const retryHeaders = new Headers(initialRequest.headers);
    applyPaymentHeaders(retryHeaders, authorization);

    const retryResponse = await this.fetcher(input.url, {
      ...initialRequest,
      headers: retryHeaders,
    });

    if (retryResponse.status === 402) {
      throw new AgentsPayError("Payment retry was rejected by the paid endpoint.", {
        code: "payment_retry_rejected",
        status: retryResponse.status,
        details: await readResponseJson(retryResponse.clone()),
      });
    }

    const settlement =
      input.settle === true
        ? await this.settlePayment({
            requirement,
            authorization,
          })
        : undefined;

    return {
      response: retryResponse,
      data: await parseResponseData<TData>(retryResponse),
      paymentRequired: true,
      requirement,
      authorization,
      settlement,
    };
  }

  private async request<TResponse>(
    path: string,
    init: {
      readonly method: "GET" | "POST";
      readonly body?: JsonValue;
      readonly headers?: HeadersInit;
      readonly idempotencyKey?: string;
    },
  ): Promise<TResponse> {
    const headers = this.buildHeaders(init.headers);

    if (init.idempotencyKey !== undefined) {
      headers.set("Idempotency-Key", init.idempotencyKey);
    }

    const requestInit: RequestInit = {
      method: init.method,
      headers,
    };

    if (init.body !== undefined) {
      headers.set("Content-Type", "application/json");
      requestInit.body = JSON.stringify(init.body);
    }

    const url = `${this.baseUrl}${path}`;
    this.log(`${init.method} ${url}`);

    const response = await this.fetcher(url, requestInit);
    const payload = await readResponseJson(response.clone());

    if (!response.ok) {
      throw new AgentsPayApiError(`AgentsPay API request failed: ${response.status}`, {
        status: response.status,
        details: payload,
      });
    }

    return payload as TResponse;
  }

  private buildHeaders(extraHeaders?: HeadersInit): Headers {
    const headers = new Headers(this.defaultHeaders);
    headers.set("Accept", "application/json");
    headers.set("X-AgentsPay-Environment", this.environment);

    if (this.apiKey !== undefined && this.apiKey.length > 0) {
      headers.set("Authorization", `Bearer ${this.apiKey}`);
    }

    if (extraHeaders !== undefined) {
      new Headers(extraHeaders).forEach((value, key) => {
        headers.set(key, value);
      });
    }

    return headers;
  }

  private log(message: string, details?: JsonValue): void {
    if (!this.debug) {
      return;
    }

    if (details === undefined) {
      console.debug(`[agentspay] ${message}`);
      return;
    }

    console.debug(`[agentspay] ${message}`, details);
  }
}

export function createIdempotencyKey(prefix = "ap"): string {
  const randomUUID = globalThis.crypto?.randomUUID;
  if (typeof randomUUID === "function") {
    return `${prefix}_${randomUUID.call(globalThis.crypto)}`;
  }

  const timestamp = Date.now().toString(36);
  const random = Math.random().toString(36).slice(2, 10);
  return `${prefix}_${timestamp}_${random}`;
}

function normalizeBaseUrl(baseUrl: string): string {
  return baseUrl.replace(/\/+$/, "");
}

function buildPaymentRequirementBody(
  input: PaymentRequirementInput,
  idempotencyKey: string,
): JsonObject {
  const body: Record<string, JsonValue> = {
    amount: input.amount,
    currency: input.currency ?? "USDC",
    idempotency_key: idempotencyKey,
  };

  if (input.endpointId !== undefined) {
    body.endpoint_id = input.endpointId;
  }
  if (input.method !== undefined) {
    body.method = input.method;
  }
  if (input.path !== undefined) {
    body.path = input.path;
  }
  if (input.url !== undefined) {
    body.url = input.url;
  }
  if (input.description !== undefined) {
    body.description = input.description;
  }
  if (input.payerAgentId !== undefined) {
    body.payer_agent_id = input.payerAgentId;
  }
  if (input.metadata !== undefined) {
    body.metadata = input.metadata;
  }

  return body;
}

function buildPaidRequest(input: PayAndCallInput): RequestInit {
  const headers = new Headers(input.headers);
  const requestInit: RequestInit = {
    ...input.fetchOptions,
    method: input.method ?? "POST",
    headers,
  };

  if (input.body !== undefined) {
    if (isJsonBody(input.body)) {
      headers.set("Content-Type", "application/json");
      requestInit.body = JSON.stringify(input.body);
    } else {
      requestInit.body = input.body;
    }
  }

  return requestInit;
}

function isJsonBody(body: JsonValue | BodyInit): body is JsonValue {
  return (
    body === null ||
    typeof body === "string" ||
    typeof body === "number" ||
    typeof body === "boolean" ||
    Array.isArray(body) ||
    isRecord(body)
  );
}

async function parseResponseData<TData>(response: Response): Promise<TData | null> {
  const payload = await readResponseJson(response.clone());
  return payload as TData | null;
}

async function readResponseJson(response: Response): Promise<JsonValue | null> {
  const contentType = response.headers.get("content-type") ?? "";

  if (contentType.includes("application/json")) {
    return (await response.json()) as JsonValue;
  }

  const text = await response.text();
  if (text.length === 0) {
    return null;
  }

  try {
    return JSON.parse(text) as JsonValue;
  } catch {
    return text;
  }
}

function extractPaymentRequirement(
  payload: JsonValue | null,
  input: PayAndCallInput,
  response: Response,
): PaymentRequirement {
  const record = isRecord(payload) ? payload : undefined;
  const direct = selectRecord(record, "payment_requirement", "paymentRequirement");
  const accepts = selectFirstRecord(record, "accepts", "requirements");
  const candidate = direct ?? accepts ?? record;
  const headerRequirementId =
    response.headers.get("PAYMENT-REQUIRED") ??
    response.headers.get("X-AgentsPay-Payment-Requirement-Id") ??
    undefined;

  return coercePaymentRequirement(candidate, {
    id: headerRequirementId,
    amount: input.maxAmount ?? "0",
    currency: input.currency ?? "USDC",
    endpointId: input.endpointId,
    description: input.description,
  });
}

function applyPaymentHeaders(headers: Headers, authorization: PaymentAuthorization): void {
  const paymentSignature =
    authorization.paymentSignature ?? authorization.paymentHeader ?? authorization.id;

  headers.set("PAYMENT-SIGNATURE", paymentSignature);
  headers.set("PAYMENT-RESPONSE", JSON.stringify(authorizationToJson(authorization)));
  headers.set("X-AgentsPay-Authorization", authorization.id);
}

function coercePaymentRequirement(
  value: unknown,
  fallback: Partial<PaymentRequirement>,
): PaymentRequirement {
  const record = isRecord(value) ? value : {};
  const id = readString(record, "id") ?? fallback.id ?? createIdempotencyKey("req");
  const amount = readString(record, "amount") ?? fallback.amount ?? "0";
  const currency = readString(record, "currency") ?? fallback.currency ?? "USDC";

  return {
    id,
    amount,
    currency,
    endpointId: readString(record, "endpoint_id") ?? fallback.endpointId,
    description: readString(record, "description") ?? fallback.description,
    expiresAt: readString(record, "expires_at"),
    paymentUrl: readString(record, "payment_url"),
    x402: readObject(record, "x402"),
    metadata: readObject(record, "metadata"),
  };
}

function coerceAuthorization(
  value: unknown,
  requirementId: string,
  idempotencyKey: string,
): PaymentAuthorization {
  const record = isRecord(value) ? value : {};
  const id = readString(record, "id") ?? createIdempotencyKey("auth");

  return {
    id,
    requirementId: readString(record, "requirement_id") ?? requirementId,
    status: readString(record, "status") ?? "authorized",
    paymentSignature: readString(record, "payment_signature"),
    paymentHeader: readString(record, "payment_header"),
    expiresAt: readString(record, "expires_at"),
    idempotencyKey,
    raw: toJsonObject(record),
  };
}

function coerceVerification(value: unknown): PaymentVerification {
  const record = isRecord(value) ? value : {};
  const accepted = readBoolean(record, "accepted") ?? readBoolean(record, "valid") ?? false;

  return {
    id: readString(record, "id"),
    accepted,
    status: readString(record, "status"),
    reason: readString(record, "reason"),
    raw: toJsonObject(record),
  };
}

function coerceSettlement(value: unknown): PaymentSettlement {
  const record = isRecord(value) ? value : {};

  return {
    id: readString(record, "id"),
    status: readString(record, "status") ?? "settled",
    transactionId: readString(record, "transaction_id"),
    auditProofId: readString(record, "audit_proof_id"),
    raw: toJsonObject(record),
  };
}

function paymentRequirementToJson(requirement: PaymentRequirement): JsonObject {
  const body: Record<string, JsonValue> = {
    id: requirement.id,
    amount: requirement.amount,
    currency: requirement.currency,
  };

  if (requirement.endpointId !== undefined) {
    body.endpoint_id = requirement.endpointId;
  }
  if (requirement.description !== undefined) {
    body.description = requirement.description;
  }
  if (requirement.expiresAt !== undefined) {
    body.expires_at = requirement.expiresAt;
  }
  if (requirement.paymentUrl !== undefined) {
    body.payment_url = requirement.paymentUrl;
  }
  if (requirement.x402 !== undefined) {
    body.x402 = requirement.x402;
  }
  if (requirement.metadata !== undefined) {
    body.metadata = requirement.metadata;
  }

  return body;
}

function authorizationToJson(authorization: PaymentAuthorization): JsonObject {
  const body: Record<string, JsonValue> = {
    id: authorization.id,
  };

  if (authorization.requirementId !== undefined) {
    body.requirement_id = authorization.requirementId;
  }
  if (authorization.status !== undefined) {
    body.status = authorization.status;
  }
  if (authorization.paymentSignature !== undefined) {
    body.payment_signature = authorization.paymentSignature;
  }
  if (authorization.paymentHeader !== undefined) {
    body.payment_header = authorization.paymentHeader;
  }
  if (authorization.expiresAt !== undefined) {
    body.expires_at = authorization.expiresAt;
  }
  if (authorization.idempotencyKey !== undefined) {
    body.idempotency_key = authorization.idempotencyKey;
  }
  if (authorization.raw !== undefined) {
    body.raw = authorization.raw;
  }

  return body;
}

function verificationToJson(verification: PaymentVerification): JsonObject {
  const body: Record<string, JsonValue> = {
    accepted: verification.accepted,
  };

  if (verification.id !== undefined) {
    body.id = verification.id;
  }
  if (verification.status !== undefined) {
    body.status = verification.status;
  }
  if (verification.reason !== undefined) {
    body.reason = verification.reason;
  }
  if (verification.raw !== undefined) {
    body.raw = verification.raw;
  }

  return body;
}

function selectRecord(
  record: Record<string, unknown> | undefined,
  ...keys: readonly string[]
): Record<string, unknown> | undefined {
  if (record === undefined) {
    return undefined;
  }

  for (const key of keys) {
    const value = record[key];
    if (isRecord(value)) {
      return value;
    }
  }

  return undefined;
}

function selectFirstRecord(
  record: Record<string, unknown> | undefined,
  ...keys: readonly string[]
): Record<string, unknown> | undefined {
  if (record === undefined) {
    return undefined;
  }

  for (const key of keys) {
    const value = record[key];
    if (Array.isArray(value) && isRecord(value[0])) {
      return value[0];
    }
  }

  return undefined;
}

function readString(record: Record<string, unknown>, key: string): string | undefined {
  const value = record[key];
  return typeof value === "string" ? value : undefined;
}

function readBoolean(record: Record<string, unknown>, key: string): boolean | undefined {
  const value = record[key];
  return typeof value === "boolean" ? value : undefined;
}

function readObject(record: Record<string, unknown>, key: string): JsonObject | undefined {
  const value = record[key];
  return isRecord(value) ? toJsonObject(value) : undefined;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function toJsonObject(record: Record<string, unknown>): JsonObject {
  const result: Record<string, JsonValue> = {};

  for (const [key, value] of Object.entries(record)) {
    if (isJsonValue(value)) {
      result[key] = value;
    }
  }

  return result;
}

function isJsonValue(value: unknown): value is JsonValue {
  if (
    value === null ||
    typeof value === "string" ||
    typeof value === "number" ||
    typeof value === "boolean"
  ) {
    return true;
  }

  if (Array.isArray(value)) {
    return value.every(isJsonValue);
  }

  if (isRecord(value)) {
    return Object.values(value).every(isJsonValue);
  }

  return false;
}
