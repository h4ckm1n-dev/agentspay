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
  readonly baseUrl?: string;
  readonly apiKey?: string;
  readonly environment?: AgentsPayEnvironment;
  readonly defaultHeaders?: HeadersInit;
  readonly fetch?: FetchLike;
  readonly debug?: boolean;
}

export interface RequestOptions {
  readonly idempotencyKey?: string;
  readonly headers?: HeadersInit;
}

export interface HealthResponse {
  readonly status: string;
  readonly service?: string;
  readonly environment?: string;
  readonly version?: string;
  readonly [key: string]: JsonValue | undefined;
}

export interface StatusResponse {
  readonly status: string;
  readonly environment?: string;
  readonly ledger?: JsonObject;
  readonly settlement?: JsonObject;
  readonly [key: string]: JsonValue | undefined;
}

export interface PaymentRequirementInput {
  readonly amount: string;
  readonly currency?: CurrencyCode;
  readonly endpointId?: string;
  readonly method?: string;
  readonly path?: string;
  readonly url?: string;
  readonly description?: string;
  readonly payerAgentId?: string;
  readonly metadata?: JsonObject;
  readonly idempotencyKey?: string;
}

export interface PaymentRequirement {
  readonly id: string;
  readonly amount: string;
  readonly currency: string;
  readonly endpointId?: string;
  readonly description?: string;
  readonly expiresAt?: string;
  readonly paymentUrl?: string;
  readonly x402?: JsonObject;
  readonly metadata?: JsonObject;
  readonly [key: string]: JsonValue | undefined;
}

export interface AuthorizePaymentOptions extends RequestOptions {
  readonly maxAmount?: string;
  readonly payerAgentId?: string;
  readonly metadata?: JsonObject;
}

export interface PaymentAuthorization {
  readonly id: string;
  readonly requirementId?: string;
  readonly status?: string;
  readonly paymentSignature?: string;
  readonly paymentHeader?: string;
  readonly expiresAt?: string;
  readonly idempotencyKey?: string;
  readonly raw?: JsonObject;
  readonly [key: string]: JsonValue | undefined;
}

export interface VerifyPaymentInput {
  readonly requirement: PaymentRequirement;
  readonly authorization: PaymentAuthorization;
  readonly idempotencyKey?: string;
  readonly metadata?: JsonObject;
}

export interface PaymentVerification {
  readonly id?: string;
  readonly accepted: boolean;
  readonly status?: string;
  readonly reason?: string;
  readonly raw?: JsonObject;
  readonly [key: string]: JsonValue | undefined;
}

export interface SettlePaymentInput {
  readonly requirement?: PaymentRequirement;
  readonly authorization: PaymentAuthorization;
  readonly verification?: PaymentVerification;
  readonly idempotencyKey?: string;
  readonly metadata?: JsonObject;
}

export interface PaymentSettlement {
  readonly id?: string;
  readonly status: string;
  readonly transactionId?: string;
  readonly auditProofId?: string;
  readonly raw?: JsonObject;
  readonly [key: string]: JsonValue | undefined;
}

export interface PayAndCallInput {
  readonly url: string;
  readonly method?: string;
  readonly headers?: HeadersInit;
  readonly body?: JsonValue | BodyInit;
  readonly maxAmount?: string;
  readonly currency?: CurrencyCode;
  readonly endpointId?: string;
  readonly description?: string;
  readonly paymentRequirement?: PaymentRequirement;
  readonly idempotencyKey?: string;
  readonly retryOn402?: boolean;
  readonly settle?: boolean;
  readonly fetchOptions?: Omit<RequestInit, "body" | "headers" | "method">;
}

export interface PayAndCallResult<TData = unknown> {
  readonly response: Response;
  readonly data: TData | null;
  readonly paymentRequired: boolean;
  readonly requirement?: PaymentRequirement;
  readonly authorization?: PaymentAuthorization;
  readonly settlement?: PaymentSettlement;
}
