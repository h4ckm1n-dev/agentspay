# AGENTSPAY MASTER PLAN — v3 (MCP Wallet Pivot)

Version: v3.0 (post /office-hours 2026-05-13)
Status: Active execution plan
Objective: Ship `agentspay-mcp`, an MCP server that gives any agent a budget-controlled USDC wallet, in 3-4 weeks. Validate demand thesis in 30 days post-launch or pivot again.

> **Important:** Plan v2 (the Stripe-for-agents architecture) is archived in `Plan-v2-archive.md`. Most of its architectural decisions (ledger-first, idempotency, sandbox/live isolation, policy engine pattern) are **preserved**. The scope is radically narrowed.

---

# 1. The One-Line Pitch

> An MCP server that gives your AI agent a budget-controlled USDC wallet, so it can pay for x402 APIs without you handing it a credit card.

---

# 2. Why This Pivot (the 60-second reasoning)

The original Plan.md v2 was "Stripe for AI agents" — a hosted x402 facilitator + dashboard + multi-SDK + multi-service Rust stack. The /office-hours diagnostic (see `~/.gstack/projects/AgentsPay/h4ckm1n-main-design-20260513-174511.md`) surfaced 5 problems:

1. **Stripe is already shipping this.** MPP, Issuing-for-agents, Agentic Commerce Suite, AWS Bedrock AgentCore. Direct collision with a $1B-ARR competitor.
2. **Market volume is tiny.** ~$28k/day total x402 volume in March 2026. Real demand is forming, not formed.
3. **No demand evidence.** Honest founder assessment: thesis-driven, no users, no LOIs.
4. **Scope = 6 months for a team.** Solo dev in vibe coding cannot ship 4 Rust services + 2 SDKs + CLI + dashboard + facilitator + MCP server in viable time.
5. **MCP angle was undersold.** Plan v2 listed MCP in §15 as one feature among many. It is in fact the wedge.

The pivot: **drop everything except the MCP server.** Use Coinbase CDP facilitator underneath (don't rebuild). Distribute via the Anthropic MCP registry (free distribution to the target user). Validate demand in 30 days.

---

# 3. The 30-Day Demand Validation

Explicit kill criteria. If on day 30 post-launch we have:
- < 10 MCP registry installs **AND**
- 0 real (non-test) paid x402 calls through AgentsPay

The thesis is falsified. We do not iterate on this product. We re-run /office-hours with the data and pivot to Approach B (Control Plane) or D (Build endpoints).

If we hit ≥10 installs **and** ≥3 real paid calls: validated. Proceed to v0.2.

---

# 4. The Product (v0.1)

A single binary: `agentspay-mcp`.

Installed via:
```bash
# Once Anthropic MCP registry listing is live:
claude mcp add agentspay  # or equivalent for Cursor/Cline

# Or manually:
brew install agentspay/tap/agentspay   # or download from GitHub Releases
agentspay init
```

Exposes 4 MCP tools to the host agent:

| Tool | Purpose |
|---|---|
| `agentspay_balance()` | Returns current USDC balance + remaining daily budget |
| `agentspay_pay_url(url, max_amount)` | HTTP GET/POST that handles 402 → signs → retries → returns body |
| `agentspay_set_budget(daily_usd, per_call_usd)` | Set per-agent spending policy |
| `agentspay_audit_log(limit)` | Recent transactions: provider, amount, endpoint, status |

That's the entire surface area for v0.1.

---

# 5. Target User & Wedge

**User:** Solo dev or 2-5 person team building an agent on Claude Code / Cursor / Cline / Zed / Anthropic Desktop. Already pays for LLM tokens. Wants their agent to autonomously pay for x402-priced APIs without a credit card or a hardcoded keypair.

**Wedge:** "Install one MCP server → your agent has a budgeted wallet → it can pay for anything that speaks x402."

**Anti-wedge** (explicitly not chasing):
- Enterprises with compliance teams (Stripe wins).
- Crypto-native devs who already write Solana code by hand.
- Consumer-facing checkout (Crossmint / Stripe Link wins).
- Mass-market agent platforms (LangChain / CrewAI already integrate with Stripe).

---

# 6. Architecture (preserved from Plan v2, scoped down)

```
┌──────────────────────────────────────────┐
│ Claude Code / Cursor / Cline (MCP host)  │
└──────────────┬───────────────────────────┘
               │  MCP stdio (JSON-RPC)
               ▼
┌──────────────────────────────────────────┐
│  agentspay-mcp (single Rust binary)      │
│  - rmcp 0.16 server                      │
│  - 4 tool handlers                       │
│  - policy engine (load → check → emit)   │
│  - ledger (SeaORM + SQLite for v0.1)     │
│  - tracing (structured logs)             │
└──────────────┬───────────────────────────┘
               │  HTTPS
               ▼
┌──────────────────────────────────────────┐
│  Coinbase CDP Facilitator (rented)       │
│  - /x402/verify, /x402/settle             │
│  - Solana USDC, free tier 1k tx/month    │
└──────────────────────────────────────────┘
```

**No** separate auth service, metering service, payment service, gateway. **No** dashboard. **No** TS/Python SDK. **No** hosted backend (v0.1 is local-only stdio).

Workspace layout (delta from current):
```
services/
  mcp/              ← NEW. Primary v0.1 crate. rmcp server.
  gateway/          ← Keep scaffolding, do not implement v0.1.
  auth/             ← Keep scaffolding, do not implement v0.1.
  payment/          ← Implement ledger module only. SeaORM models.
  metering/         ← Keep scaffolding, do not implement v0.1.
packages/
  proto/            ← Keep, but no gRPC traffic in v0.1.
  cli/              ← Implement `agentspay init` and `agentspay topup`. Rust.
  sdk-js/           ← Defer to v0.3.
  sdk-python/       ← Defer indefinitely.
apps/
  frontend/         ← Defer to v0.4.
examples/
  paid-endpoint/    ← NEW. Demo Axum server returning x402 on one route.
```

---

# 7. Stack

| Layer | Choice | Reason |
|---|---|---|
| MCP server | `rmcp 0.16` (Rust) | Official Anthropic SDK, Tokio-native, ergonomic `#[tool]` macros |
| Async runtime | Tokio | rmcp dependency |
| HTTP client | `reqwest` | x402 calls to providers + CDP facilitator |
| Ledger | SeaORM + SQLite | Single-file DB for v0.1, zero ops |
| Tracing | `tracing` + `tracing-subscriber` | Preserved from Plan v2 |
| Errors | `thiserror` + `anyhow` | Preserved from Plan v2 |
| x402 settlement | Coinbase CDP facilitator (Solana) | Free 1k tx/month, no infra to rebuild |
| Chain | Solana USDC | 400ms finality, $0.00025 fee |
| CI/CD | GitHub Actions | Build binaries for macOS/Linux/Windows on tag push |

**Killed:** Tonic / gRPC (no internal services for v0.1), Axum gateway (v0.2), Docker Compose (no need), PostgreSQL (SQLite for v0.1), Next.js dashboard (v0.4), Python SDK, K8s readiness.

---

# 8. Three-Week Build Plan

### Week 1 — Core MCP Server (working stubs)

- [ ] Add `rmcp = "0.16"` and `tokio` features to workspace.
- [ ] Create `services/mcp/` Rust crate.
- [ ] Implement 4 `#[tool]` handlers backed by in-memory `HashMap<AgentId, Wallet>`.
- [ ] Wire stdio transport.
- [ ] Local test: `claude --mcp /path/to/agentspay-mcp` lists tools and returns mock data.
- [ ] Acceptance: I can ask Claude Code "what's my balance?" and get a deterministic fake answer.

### Week 2 — Real Ledger + Sandbox

- [ ] SeaORM models: `Wallet`, `LedgerEntry`, `Budget`, `Policy`, `AuditLog`.
- [ ] SQLite migrations.
- [ ] Sandbox `pay_url` flow: receives 402 → simulates signature → calls demo provider → ledger entry.
- [ ] `examples/paid-endpoint/` — Axum service with one route returning 402.
- [ ] Acceptance: `agentspay_pay_url("http://localhost:3001/quote", "0.01")` returns the quote and the ledger shows the transaction.

### Week 3 — CDP Facilitator + Ship

- [ ] Integrate Coinbase CDP `/x402/verify` and `/x402/settle` (Solana devnet).
- [ ] `agentspay init` CLI: provision local keypair, set sandbox/devnet, init SQLite.
- [ ] `agentspay topup` CLI: faucet for devnet USDC.
- [ ] GitHub Actions release pipeline: tag `v0.1.0` → binaries on GitHub Releases.
- [ ] README with 60-second GIF demo: Claude buying a stock quote.
- [ ] Submit to Anthropic MCP registry.
- [ ] Show HN post + Anthropic Discord soft launch.
- [ ] Acceptance: end-to-end devnet payment from Claude Code → CDP → demo paid endpoint, single command.

### Week 4 — Triage & Decide

- [ ] Monitor MCP registry install count daily.
- [ ] Respond to GitHub issues within 24h.
- [ ] Track real (non-test) paid x402 calls.
- [ ] Day 30: run /office-hours again with the data. Validate, iterate, or pivot.

---

# 9. The Pre-Code Assignment (do this in the next 7 days)

Before writing any rmcp code, DM 3 specific people building agents on Claude Code / Cursor / Cline. Ask them verbatim:

> "I'm scoping an MCP server that gives your agent a budget-controlled USDC wallet so it can pay for x402 APIs without giving it a credit card. Would you actually install this? What would it have to do for you to keep it installed past day 2?"

- 0/3 positive → thesis dead before any code is written. Pivot now, not in 30 days.
- 1/3 positive → scope down further or pick a different wedge.
- 2-3/3 positive → build it.

**This is mandatory.** Skipping costs $0 in code. Skipping costs 3 weeks if the answer was 0/3.

---

# 10. Coding Standards (preserved from Plan v2)

- Rust: `tracing` for spans, `thiserror` for typed errors, `anyhow` for top-level.
- Idempotency: every state-mutating MCP tool call writes a row to `audit_log` before the side effect.
- Sandbox vs live: type-level isolation (`Network::Devnet` vs `Network::Mainnet`), no boolean flags.
- No `unwrap()` in handlers — always return `McpError`.
- Cargo workspace stays as-is; new code lives in `services/mcp/`.

---

# 11. Non-Goals for v0.1

Do not build:
- Auth / API key service (the MCP host is the auth boundary in v0.1).
- gRPC anything.
- REST `/v1` API surface.
- Dashboard UI.
- Webhook delivery system.
- Multi-org / multi-tenant.
- TypeScript or Python SDK.
- Hosted facilitator endpoint.
- Compliance / KYC workflow.
- Live mainnet mode.

Document them. Build none of them in v0.1.

---

# 12. Open Decisions (decide during week 1)

1. **Custody:** self-custodial (user holds keypair, signs locally) for v0.1. Defer custodial flow to v0.3.
2. **Pricing:** $0 sandbox forever; pricing for mainnet decided at week 3 ship if v0.1 validates.
3. **Naming:** "AgentsPay" survives the pivot for now. Revisit if MCP-wallet thesis confirms in week 4.
4. **Devnet vs mainnet for v0.1 launch:** devnet only. No mainnet money until v0.2 (compliance posture review).

---

# 13. Success Metrics (30-day kill switch)

| Metric | Falsify (kill) | Survive (iterate) | Win (double down) |
|---|---|---|---|
| MCP registry installs | < 10 | 10-50 | 50+ |
| Real paid x402 calls | 0 | 1-5 | 5+ |
| GitHub stars | < 20 | 20-100 | 100+ |
| Weekly active users (week 4) | 0 | 1-3 | 4+ |
| Inbound feature requests | 0 | 1-5 | 5+ |

Decision rule: if any "Falsify" cell is hit, run /office-hours again before writing more code.

---

# 14. References

- /office-hours design doc: `~/.gstack/projects/AgentsPay/h4ckm1n-main-design-20260513-174511.md`
- Archived v2 plan: `Plan-v2-archive.md`
- rmcp official: https://github.com/modelcontextprotocol/rust-sdk
- Coinbase CDP facilitator (Solana): https://docs.cdp.coinbase.com/x402/welcome
- x402 spec: https://www.x402.org/
- MCP spec: https://modelcontextprotocol.io/specification/2025-06-18
