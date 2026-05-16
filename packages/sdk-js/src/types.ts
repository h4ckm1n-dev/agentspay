export type AgentsPayEnvironment = "sandbox" | "live";

export type CurrencyCode = "USDC" | (string & {});

export type JsonPrimitive = string | number | boolean | null;

export type JsonValue =
  | JsonPrimitive
  | { readonly [key: string]: JsonValue }
  | readonly JsonValue[];

export type JsonObject = { readonly [key: string]: JsonValue };

export type FetchLike = (
  input: RequestInfo | URL,
  init?: RequestInit,
) => Promise<Response>;

export interface AgentsPayClientOptions {
  readonly baseUrl?: string | undefined;
  readonly apiKey?: string | undefined;
  readonly environment?: AgentsPayEnvironment | undefined;
  readonly defaultHeaders?: HeadersInit | undefined;
  readonly fetch?: FetchLike | undefined;
  readonly debug?: boolean | undefined;
}

export interface RequestOptions {
  readonly idempotencyKey?: string | undefined;
  readonly headers?: HeadersInit | undefined;
}

export interface HealthResponse {
  readonly status: string;
  readonly service?: string | undefined;
  readonly environment?: string | undefined;
  readonly version?: string | undefined;
  readonly [key: string]: JsonValue | undefined;
}

export interface StatusResponse {
  readonly status: string;
  readonly environment?: string | undefined;
  readonly ledger?: JsonObject | undefined;
  readonly settlement?: JsonObject | undefined;
  readonly [key: string]: JsonValue | undefined;
}

export interface PaymentRequirementInput {
  readonly amount: string;
  readonly currency?: CurrencyCode | undefined;
  readonly endpointId?: string | undefined;
  readonly method?: string | undefined;
  readonly path?: string | undefined;
  readonly url?: string | undefined;
  readonly description?: string | undefined;
  readonly payerAgentId?: string | undefined;
  readonly metadata?: JsonObject | undefined;
  readonly idempotencyKey?: string | undefined;
}

export interface PaymentRequirement {
  readonly id: string;
  readonly amount: string;
  readonly currency: string;
  readonly endpointId?: string | undefined;
  readonly description?: string | undefined;
  readonly expiresAt?: string | undefined;
  readonly paymentUrl?: string | undefined;
  readonly x402?: JsonObject | undefined;
  readonly metadata?: JsonObject | undefined;
  readonly [key: string]: JsonValue | undefined;
}

export interface AuthorizePaymentOptions extends RequestOptions {
  readonly maxAmount?: string | undefined;
  readonly payerAgentId?: string | undefined;
  readonly metadata?: JsonObject | undefined;
}

export interface PaymentAuthorization {
  readonly id: string;
  readonly requirementId?: string | undefined;
  readonly status?: string | undefined;
  readonly paymentSignature?: string | undefined;
  readonly paymentHeader?: string | undefined;
  readonly expiresAt?: string | undefined;
  readonly idempotencyKey?: string | undefined;
  readonly raw?: JsonObject | undefined;
  readonly [key: string]: JsonValue | undefined;
}

export interface VerifyPaymentInput {
  readonly requirement: PaymentRequirement;
  readonly authorization: PaymentAuthorization;
  readonly idempotencyKey?: string | undefined;
  readonly metadata?: JsonObject | undefined;
}

export interface PaymentVerification {
  readonly id?: string | undefined;
  readonly accepted: boolean;
  readonly status?: string | undefined;
  readonly reason?: string | undefined;
  readonly raw?: JsonObject | undefined;
  readonly [key: string]: JsonValue | undefined;
}

export interface SettlePaymentInput {
  readonly requirement?: PaymentRequirement | undefined;
  readonly authorization: PaymentAuthorization;
  readonly verification?: PaymentVerification | undefined;
  readonly idempotencyKey?: string | undefined;
  readonly metadata?: JsonObject | undefined;
}

export interface PaymentSettlement {
  readonly id?: string | undefined;
  readonly status: string;
  readonly transactionId?: string | undefined;
  readonly auditProofId?: string | undefined;
  readonly raw?: JsonObject | undefined;
  readonly [key: string]: JsonValue | undefined;
}

export interface PayAndCallInput {
  readonly url: string;
  readonly method?: string | undefined;
  readonly headers?: HeadersInit | undefined;
  readonly body?: JsonValue | BodyInit | undefined;
  readonly maxAmount?: string | undefined;
  readonly currency?: CurrencyCode | undefined;
  readonly endpointId?: string | undefined;
  readonly description?: string | undefined;
  readonly paymentRequirement?: PaymentRequirement | undefined;
  readonly idempotencyKey?: string | undefined;
  readonly retryOn402?: boolean | undefined;
  readonly settle?: boolean | undefined;
  readonly fetchOptions?: Omit<RequestInit, "body" | "headers" | "method"> | undefined;
}

export interface PayAndCallResult<TData = unknown> {
  readonly response: Response;
  readonly data: TData | null;
  readonly paymentRequired: boolean;
  readonly requirement?: PaymentRequirement | undefined;
  readonly authorization?: PaymentAuthorization | undefined;
  readonly settlement?: PaymentSettlement | undefined;
}
