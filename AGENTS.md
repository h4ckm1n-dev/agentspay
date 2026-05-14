# AGENTS.md - AgentsPay Development Guide

## Project Vision
**AgentsPay** is financial infrastructure for autonomous AI agents. It provides a Stripe-like platform for the machine-to-machine economy using USDC on Solana.

**Roadmap Reference:** See [Plan.md](./Plan.md) for detailed implementation phases and feature requirements.

## Core Mandates
1. **Security First:** Secure-by-design at every layer. Zero Trust gRPC, HMAC request signing, and strict "Least Privilege" access.
2. **Standard Enterprise Positioning:** Avoid crypto-jargon in UI/Marketing. Use "Balance", "Account", and "Audit Proof".
3. **Stateless Scalability:** All services must be horizontally scalable (K8s-ready). Externalize all state to DB/Redis.
4. **Reliability:** Mandatory idempotency keys, exponential backoff for webhooks, and distributed locking (Redlock).

## Technical Stack
- **Backend:** Rust (Axum Gateway, Tonic gRPC services, SeaORM, Tokio).
- **Frontend:** Next.js, TypeScript, Tailwind CSS, shadcn/ui.
- **Infrastructure:** Docker Compose (multi-stage Rust builds), PostgreSQL, Redis.
- **Communication:** Internal gRPC (via `tonic`), External REST (/v1).

## Repository Structure
- `services/gateway/`: Axum REST API Gateway.
- `services/auth/`: gRPC Identity & API Key service.
- `services/payment/`: gRPC Wallet & Settlement service.
- `services/metering/`: gRPC Usage & Pricing service.
- `packages/proto/`: **Source of Truth** for gRPC definitions.
- `packages/sdk-js/`: TypeScript SDK.
- `packages/sdk-python/`: Python SDK.
- `apps/frontend/`: Next.js Dashboard & Landing page.

## Coding Standards & Style
- **Rust:** 
  - Use `tracing` crate for distributed tracing.
  - Strict error handling with `thiserror` and `anyhow`.
  - Prefer `tonic` for gRPC and `axum` for REST.
- **TypeScript:** 
  - Strict typing (No `any`).
  - Use Functional Components with Hooks.
- **API:**
  - Mandatory `/v1/` versioning.
  - All mutation requests must support `idempotency_key`.

## Common Commands
- **Initialize Workspace:** `pnpm install` & `cargo fetch`
- **Build All:** `docker-compose build`
- **Run Stack:** `docker-compose up`
- **Test Backend:** `cargo test`
- **Test Frontend:** `pnpm --filter frontend test`
- **Lint Rust:** `cargo clippy`
- **Lint Frontend:** `pnpm --filter frontend lint`
