# AGENTSPAY MASTER PLAN

Version: MVP v2
Status: Product and execution plan
Objective: Build the x402-compatible payment and spend-control layer for autonomous AI agents.

---

# 1. Product Thesis

AgentsPay is financial infrastructure for autonomous AI agents.

The product lets agents safely pay for APIs, compute, data, tools, and services with programmable USDC payments while giving developers and organizations clear controls, auditability, and settlement.

The core wedge:

> Make any API payable by agents in minutes, with enterprise-grade budgets and audit proofs.

AgentsPay is not a DeFi product, token project, or wallet app. It is developer infrastructure for agentic commerce.

---

# 2. Market Direction

The agent-payment market is converging around open, HTTP-native payment flows:

- x402 uses HTTP `402 Payment Required` so a server can return payment requirements and a client can retry with signed payment proof.
- AP2 focuses on authorization, authenticity, accountability, and signed user mandates for agent-led purchases.
- ACP focuses on checkout and commerce coordination between agents, buyers, and sellers.
- MCP is becoming the standard way agents discover and call tools.

AgentsPay should interoperate with these standards instead of inventing a closed payment protocol.

Research references:

- x402: https://www.x402.org/
- x402 flow: https://docs.cdp.coinbase.com/x402/core-concepts/how-it-works
- Cloudflare x402 agent payments: https://developers.cloudflare.com/agents/agentic-payments/x402/
- Solana x402: https://solana.com/x402/what-is-x402
- Google AP2: https://cloud.google.com/blog/products/ai-machine-learning/announcing-agents-to-payments-ap2-protocol
- ACP: https://www.agenticcommerce.dev/docs
- MCP authorization: https://modelcontextprotocol.io/specification/2025-06-18/basic/authorization

---

# 3. Positioning

Public positioning:

> The payment and spend-control layer for autonomous AI agents.

Developer-facing positioning:

> Add pay-per-use pricing to any API. Agents receive HTTP 402 payment requirements, pay programmatically, and get access instantly.

Enterprise-facing positioning:

> Let agents transact with budgets, permissions, approvals, and audit proofs.

Preferred UI language:

- Balance
- Account
- Settlement
- Funding Source
- Transaction
- Policy
- Budget
- Audit Proof

Avoid crypto-first language in primary UI:

- gas
- chain ID
- wallet address
- transaction hash
- token program

Expose technical details only in developer logs and advanced views.

---

# 4. MVP Problem

The MVP solves one problem:

> AI agents can autonomously pay for API access without manual accounts, subscriptions, or provider-specific billing setup.

Primary use case:

1. Provider adds AgentsPay middleware or registers an endpoint.
2. Provider defines a price, currency, and policy.
3. Agent requests the paid endpoint.
4. Endpoint returns HTTP 402 payment requirements when payment is missing.
5. Agent signs or delegates payment through AgentsPay.
6. AgentsPay verifies budget, permission, balance, replay safety, and idempotency.
7. Provider receives the API request after valid payment.
8. Usage, settlement, and audit proof are recorded.

The MVP must make this flow work locally in under 5 minutes.

---

# 5. Product Moat

Open standards are not the moat. The moat is the operational layer around them:

- agent spend policies
- budget enforcement
- provider onboarding
- hosted facilitator
- ledger and reconciliation
- beautiful SDK/CLI experience
- dashboard analytics
- webhook reliability
- audit proofs
- risk and compliance controls
- integrations with agent frameworks and MCP

AgentsPay should be compatible with x402, but compete on safety, reliability, and developer experience.

---

# 6. Protocol Strategy

## Core

Implement x402-compatible flows as the primary external payment protocol.

Support:

- HTTP `402 Payment Required`
- payment requirements payloads
- signed payment payload verification
- facilitator-style `/verify`, `/settle`, and `/supported` APIs
- `PAYMENT-REQUIRED`, `PAYMENT-SIGNATURE`, and `PAYMENT-RESPONSE` headers where compatible
- fixed-price `exact` payment scheme for MVP

## AgentsPay Extensions

Add optional AgentsPay policy metadata around x402:

- organization ID
- agent ID
- endpoint ID
- budget ID
- policy decision ID
- idempotency key
- audit proof ID
- sandbox/live environment

Extensions must not break standard x402 clients.

## Future Compatibility

Document but do not implement in MVP:

- AP2 mandate mapping
- ACP checkout mapping
- usage-based `upto` pricing
- batched settlement
- streaming payments
- multi-rail payment negotiation

---

# 7. Blockchain and Settlement Strategy

Use Solana USDC as the first live settlement rail.

Reasons:

- low fees
- fast finality
- strong USDC ecosystem
- good fit for machine-to-machine payments
- growing x402 support

MVP live mode:

- direct Solana USDC settlement
- isolated `SettlementProvider` abstraction
- provider balances tracked in ledger
- sandbox and live fully isolated

MVP sandbox mode:

- no blockchain interaction
- internal ledger
- fake balances
- simulated payment signatures
- deterministic demo data

Important architecture rule:

> Ledger first, rail second.

All payment events must be represented in the database before and after settlement so the system can later support batching, net settlement, additional rails, and reconciliation.

Do not implement in MVP:

- custom Solana programs
- escrow
- Token-2022
- confidential transfers
- ZK systems
- treasury management
- streaming payments

---

# 8. Architecture

Follow the repository guide in `AGENTS.md`.

External API:

- Axum REST gateway
- `/v1` versioned public API
- OpenAPI documentation

Internal services:

- Tonic gRPC services
- `packages/proto` is the source of truth
- service boundaries are explicit even in Docker Compose

Services:

- `services/gateway`: public REST API and x402 HTTP surface
- `services/auth`: identity, organizations, API keys, agent keys
- `services/payment`: ledger, wallet, authorization, settlement
- `services/metering`: usage events, endpoint pricing, quotas
- `apps/frontend`: dashboard and landing page
- `packages/proto`: gRPC definitions
- `packages/sdk-js`: TypeScript SDK
- `packages/sdk-python`: Python SDK

Infrastructure:

- Docker Compose for local and first VPS deployment
- PostgreSQL for durable state
- Redis for queues, locks, rate limits, and webhook retries
- no Kubernetes in MVP deployment
- code remains horizontally scalable and K8s-ready

---

# 9. Backend Standards

Rust stack:

- Axum
- Tonic
- Tokio
- SeaORM
- Tower
- Tracing
- thiserror
- anyhow

Design rule:

```txt
REST route -> application service -> repository/provider -> database or external rail
```

Business logic must not live in HTTP handlers.

Mandatory backend capabilities:

- structured tracing
- request IDs
- idempotency keys on all mutation requests
- raw-body webhook signature verification
- replay protection
- scoped API keys
- per-agent rate limits
- encrypted secrets
- audit logs
- sandbox/live isolation

---

# 10. Core Domain Model

Primary entities:

- User
- Organization
- ApiKey
- Agent
- Provider
- PaidEndpoint
- PaymentRequirement
- PaymentAuthorization
- PolicyDecision
- Wallet
- WalletBalance
- LedgerAccount
- LedgerEntry
- PaymentIntent
- Transaction
- SettlementBatch
- UsageEvent
- WebhookEndpoint
- WebhookDelivery
- AuditLog

Minimum ledger states:

- pending
- authorized
- captured
- settled
- failed
- refunded
- reversed

Minimum payment authorization fields:

- authorization ID
- organization ID
- agent ID
- endpoint ID
- amount
- currency
- environment
- idempotency key
- nonce
- expires at
- signature or sandbox proof
- policy decision ID

---

# 11. API Surface

Public REST API must be `/v1`.

MVP endpoints:

- `GET /v1/health`
- `GET /v1/status`
- `POST /v1/agents`
- `GET /v1/agents`
- `POST /v1/api-keys`
- `POST /v1/endpoints`
- `GET /v1/endpoints`
- `POST /v1/payment-requirements`
- `POST /v1/payments/authorize`
- `POST /v1/payments/verify`
- `POST /v1/payments/settle`
- `GET /v1/transactions`
- `GET /v1/balances`
- `POST /v1/webhooks/endpoints`

x402-compatible surface:

- `GET /.well-known/agentspay`
- `POST /x402/verify`
- `POST /x402/settle`
- `GET /x402/supported`

All mutations require an idempotency key.

Idempotency behavior:

- store first result by organization, endpoint, method, path, and idempotency key
- return the original result on retry
- reject key reuse with different request fingerprint
- retain keys for at least 24 hours in MVP

---

# 12. Security and Policy

Security is a first-order product feature.

MVP controls:

- HMAC request signing for SDK-to-API calls
- timestamp and nonce replay protection
- scoped API keys
- per-agent budgets
- per-endpoint allowlists
- max amount per request
- max amount per day
- sandbox/live key separation
- secret encryption at rest
- audit log on every payment decision
- raw payload webhook signing
- exponential backoff webhook delivery
- Redlock for settlement and wallet balance critical sections

Policy engine MVP:

```txt
request context
-> authenticate principal
-> load agent, endpoint, organization, environment
-> check balance
-> check budget
-> check endpoint permission
-> check rate limits
-> emit PolicyDecision
-> authorize or deny
```

---

# 13. SDK Strategy

The SDKs are the product.

SDKs must support two layers:

High-level:

```ts
await client.payAndCall({
  url: "https://api.example.com/premium",
  maxAmount: "0.002",
  currency: "USDC",
  body: { prompt: "Summarize this" }
});
```

Low-level:

```ts
const requirement = await client.createPaymentRequirement(...);
const authorization = await client.authorizePayment(requirement);
const result = await client.callWithPayment(url, authorization);
```

Provider middleware:

```ts
app.post(
  "/premium",
  agentsPay.protect({
    price: "0.002",
    currency: "USDC",
    description: "Premium summarization endpoint"
  }),
  handler
);
```

SDK requirements:

- strict typing
- no `any` in TypeScript
- structured errors
- automatic idempotency keys
- retry with exponential backoff
- x402 402/retry loop support
- sandbox-first defaults
- useful debug logs

---

# 14. CLI Strategy

The CLI is onboarding, testing, and marketing.

Commands:

- `agentspay login`
- `agentspay init`
- `agentspay demo`
- `agentspay dev`
- `agentspay sandbox topup`
- `agentspay balance`
- `agentspay transactions`
- `agentspay endpoints create`
- `agentspay endpoints list`
- `agentspay doctor`
- `agentspay status`

`agentspay demo` must:

1. initialize sandbox
2. start a demo provider
3. start a demo agent
4. request a paid endpoint
5. receive HTTP 402
6. authorize payment
7. retry with payment proof
8. return provider response
9. show dashboard URL

Goal:

> From clone to first agent payment in under 5 minutes.

---

# 15. MCP Strategy

MCP must be first-class.

Build:

- `agentspay-mcp` server
- local stdio mode for developer demos
- remote HTTP mode for hosted AgentsPay
- OAuth-ready design for remote MCP

MCP tools:

- `agentspay_balance`
- `agentspay_list_endpoints`
- `agentspay_authorize_payment`
- `agentspay_pay_and_call`
- `agentspay_audit_proof`

MCP resource examples:

- organization payment policy
- agent budgets
- recent transactions
- endpoint catalog

---

# 16. Dashboard

Dashboard MVP:

- overview
- sandbox/live toggle
- balances
- agents
- API keys
- paid endpoints
- pricing configuration
- payment logs
- transaction history
- policy decisions
- webhook deliveries
- audit proofs

Design:

- premium
- dense but readable
- enterprise trust
- no crypto-first jargon
- no decorative finance gimmicks

---

# 17. Open Source Strategy

Open:

- SDKs
- CLI
- examples
- provider middleware
- protocol adapters
- docs

Closed:

- hosted infrastructure
- risk systems
- settlement orchestration
- analytics
- compliance workflows
- managed facilitator

Open standards compatibility increases trust and adoption. Hosted reliability, controls, and analytics create the business.

---

# 18. Compliance Strategy

Live mode must be gated until legal/compliance posture is explicit.

Risks to review:

- custodial wallet operation
- money transmission
- KYC/KYT requirements
- sanctions screening
- stablecoin transfer controls
- tax/reporting obligations
- provider payout model

MVP compliance posture:

- sandbox open to all developers
- live mode invite-only
- no public custody claims until reviewed
- clear terms for test mode vs live mode
- audit-friendly transaction records from day one

---

# 19. Implementation Phases

## Phase 0: Repo Foundation

- workspace scaffolding
- Docker Compose
- Rust service crates
- proto package
- frontend app
- SDK package skeletons
- shared formatting and lint commands

## Phase 1: Local Sandbox Payment

- agent creation
- endpoint registration
- sandbox balance
- payment requirement creation
- payment authorization
- x402-style verify/settle
- transaction ledger
- CLI demo

## Phase 2: Dashboard

- overview
- endpoint management
- agent management
- balances
- transactions
- policy decisions
- webhook delivery logs

## Phase 3: SDK and Middleware

- TypeScript client
- Python client
- Express middleware
- FastAPI middleware
- x402 402/retry flow
- examples

## Phase 4: Live Solana Alpha

- Solana USDC provider
- live/sandbox isolation
- funding addresses
- settlement records
- reconciliation jobs
- invite-only live mode

## Phase 5: Hosted Facilitator

- `/x402/verify`
- `/x402/settle`
- `/x402/supported`
- provider onboarding
- reporting
- reliability hardening

---

# 20. Success Metrics

Developer metrics:

- time to first sandbox payment under 5 minutes
- SDK install to paid request under 20 lines of code
- demo works without external credentials
- clean errors with suggested fixes

Business metrics:

- registered developers
- active endpoints
- successful payments
- provider revenue
- transaction volume
- SDK installs
- GitHub stars
- integrations shipped

Reliability metrics:

- payment authorization latency
- verify/settle success rate
- webhook delivery success rate
- duplicate-payment prevention
- ledger reconciliation accuracy

---

# 21. Non-Goals for MVP

Do not build:

- custom Solana program
- decentralized marketplace
- custom token
- DeFi yield
- escrow
- streaming payments
- multi-region deployment
- Kubernetes deployment
- complex fraud ML
- consumer shopping checkout
- full AP2/ACP implementation

Document these paths, but keep MVP narrow.

---

# 22. Final Objective

The MVP should make a developer think:

> I can make my API payable by agents today, safely, without building billing infrastructure.

The product should feel like:

- developer infrastructure
- enterprise-grade financial controls
- agent-native payments
- standard-compatible protocol infrastructure
- a credible venture-scale platform
