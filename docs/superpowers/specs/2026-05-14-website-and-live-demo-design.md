# AgentsPay Website + Live Demo — Design Spec

**Date:** 2026-05-14
**Status:** Draft, pending user review
**Source brainstorm:** `.superpowers/brainstorm/24576-1778768902/`
**Implementation gate:** Do not begin coding until this spec is approved and the writing-plans skill has produced a plan.

---

## 1. Purpose

Ship a single-page marketing website at (e.g.) `agentspay.dev` that promotes the `agentspay-mcp` SDK to indie AI-agent developers. The site must let any visitor:

1. Read what AgentsPay does in under 10 seconds (positioning).
2. Copy a one-line install command for Claude Code / Cursor / Cline.
3. Run a **sandbox** test against an isolated browser-tab session and watch the JSON tool responses live.
4. Trigger a **real Solana devnet** transaction, server-funded and rate-limited, and receive a Solscan URL as proof.
5. Read a comparison of AgentsPay vs the obvious alternatives.
6. Click through to the GitHub repo / future MCP-registry listing.

The site is the proof artefact that turns the existing CLI binary into something a cherry-picked beta-tester can be linked to in a single DM.

---

## 2. Locked decisions (from brainstorm)

| # | Decision | Choice | Reasoning |
|---|---|---|---|
| Q1 | Marketing positioning | **A — Agent-first** ("Give your AI agent a budget-controlled wallet") | Matches Plan.md §5 wedge. Speaks to the named target user (indie MCP dev), not the crypto-curious. |
| Q2 | Visual aesthetic | **A — Dev-dark** (Linear / Vercel / Anthropic console) | Matches the visual world the target user lives in (terminals, Claude Code). Avoids generic SaaS / crypto-glow positioning collisions. |
| Q3 | Live devnet UX | **A — Server-funded one-click** (rate-limited 1×/IP/h) | Zero visitor friction. Drains our faucet allowance, but for the demand-validation window (Plan.md §3, 30 days, target 10-50 installs) this is acceptable cost. |
| Q4 | Page structure | Single-page, 6 sections | Hero · Install · Live demo · How it works · Why · Footer. Approved verbatim. |
| Q4 | Deployment | Docker compose stack on a single VPS (Hetzner CX22 ~5€/mo or Fly.io free tier) | Single-machine is sufficient for the validation-window traffic. Real K8s is over-engineering. |

---

## 3. Architecture

```
┌────────────────────────────────────────────────────────────┐
│  Browser (visitor)                                         │
│  · Renders the Next.js single-page site                    │
│  · "Live demo" calls fetch() against /api/* on same origin │
└──────────────────────────────┬─────────────────────────────┘
                               │ HTTPS (TLS via Caddy)
                               ▼
┌────────────────────────────────────────────────────────────┐
│  Caddy reverse proxy (Docker svc: caddy)                   │
│  · :443 → Next.js for `/*` (HTML + JS + CSS)               │
│  · :443 → Shim for `/api/*`                                │
└──────────────┬───────────────────────────────┬─────────────┘
               │                               │
               ▼                               ▼
┌──────────────────────────┐   ┌──────────────────────────────┐
│  Next.js (Docker svc:    │   │  Shim service (Docker svc:   │
│  web)                    │   │  shim) — new Rust crate      │
│  · Static SSR build      │   │  `services/web-shim`         │
│  · Tailwind dev-dark     │   │  · Axum on :8080             │
│  · `/api/*` proxied →    │   │  · /api/sandbox/* endpoints  │
│    shim                  │   │  · /api/devnet/trigger       │
└──────────────────────────┘   │  · Rate limit via Redis      │
                               │  · Per-session ephemeral     │
                               │    tmpdir with isolated      │
                               │    agentspay-mcp subprocess  │
                               └─────┬────────────────┬───────┘
                                     │                │
                                     ▼                ▼
                          ┌──────────────────┐  ┌────────────────────┐
                          │ Spawned          │  │ paid-endpoint      │
                          │ agentspay-mcp    │  │ (Docker svc:       │
                          │ subprocess       │  │ paid-endpoint)     │
                          │ (per request,    │  │ — existing crate   │
                          │ stdio JSON-RPC)  │  │ on :3001 internal  │
                          └──────────────────┘  └────────────────────┘
                                     │
                                     │ HTTPS (only for devnet path)
                                     ▼
                          ┌──────────────────┐
                          │ Solana devnet    │
                          │ RPC              │
                          │ (api.devnet      │
                          │  .solana.com)    │
                          └──────────────────┘
```

The browser **never** talks to MCP directly. It always goes through the shim. This keeps the same `agentspay-mcp` binary that users will install on their machines — the website demoes the **literal product**, not a re-implementation.

---

## 4. Components

### 4.1 `apps/frontend` — Next.js website (existing scaffold, fill in)

**Framework:** Next.js 15 (already in `apps/frontend/package.json`).
**Styling:** Tailwind (already configured) + shadcn-style primitives (already scaffolded under `components/ui/`).
**Output mode:** `output: "standalone"` in `next.config.mjs` so the Docker image is small.

**Pages:**
- `/` — single-page marketing site (the 6 sections from Q4).
- `/api/*` — handled by Next.js's API route layer **only as a thin proxy** to the shim. We do not put business logic in Next API routes. Rationale: keep the rate-limit + subprocess logic in Rust where the agentspay-mcp lives, not split across two languages.

**Section-by-section files:**
```
apps/frontend/app/
  page.tsx                  — composes the 6 sections in order
  layout.tsx                — root layout, meta tags, font (existing)
  globals.css               — Tailwind base + dev-dark CSS vars (existing)
  api/
    [...path]/route.ts      — catch-all proxy to shim (no business logic)
  components/
    sections/
      Hero.tsx              — §1 hero + live-tx counter
      Install.tsx           — §2 copyable command + variants
      LiveDemo.tsx          — §3 wrapper (the 2 tabs)
      SandboxTab.tsx        — calls /api/sandbox/*
      DevnetTab.tsx         — calls /api/devnet/*
      HowItWorks.tsx        — §4 3-card explainer
      Why.tsx               — §5 3-card comparison
      Footer.tsx            — §6 footer
    ui/                     — shadcn primitives (existing: card, button, badge, progress)
    ui/CodeBlock.tsx        — new: dark code block w/ copy button
    ui/Terminal.tsx         — new: terminal-style output viewer with line-by-line stream
    ui/SolscanLink.tsx      — new: link out to solscan.io/tx/{sig}?cluster=devnet w/ "live" badge
  lib/
    api.ts                  — typed fetch wrappers around /api/sandbox/* and /api/devnet/*
    live-tx.ts              — small client store for the "last live tx" hero counter
```

**Live-tx counter (§1 hero):** the shim exposes `GET /api/stats/latest-tx` which returns `{ signature, age_seconds, amount_usdc }` for the most recent successful devnet tx. The hero polls this every 15s, displays "Last live tx: 4pGR…jYau · 23s ago · 0.10 USDC". When no tx in the last 24h, shows nothing instead of stale data.

### 4.2 `services/web-shim` — Rust Axum HTTP bridge (new crate)

A new workspace member at `services/web-shim/`, structurally similar to `services/mcp`.

**Why a Rust shim and not a Next.js API route:** the shim spawns subprocesses, manages per-session tmpdirs, holds the rate-limited devnet wallet, and talks to Redis. Keeping it in Rust lets us reuse `services/mcp/src/wallet.rs` + `solana.rs` + the spawning logic, and keeps memory/state out of a Node-process lifecycle.

**Cargo.toml dependencies** (workspace-inherited):
- `axum`, `tokio` (existing workspace deps)
- `tower`, `tower-http` for CORS + rate-limit middleware
- `reqwest` for talking to paid-endpoint (existing dep)
- `redis = "0.27"` for rate-limit counters (new workspace dep)
- `tempfile = "3"` for per-session isolated tmpdir (new dep, already in mcp dev-deps)
- `serde`, `serde_json`, `tracing`, `thiserror`, `anyhow`, `uuid` (existing)

**HTTP endpoints:**

| Method | Path | Purpose | Rate limit |
|---|---|---|---|
| GET | `/api/health` | Liveness probe | none |
| POST | `/api/sandbox/session` | Create an isolated browser-tab session. Returns `{ session_id, expires_at }`. | 30/min per IP |
| POST | `/api/sandbox/call` | Body: `{ session_id, tool, args }`. Spawns `agentspay-mcp` with isolated `AGENTSPAY_DATABASE_URL` + `AGENTSPAY_KEYPAIR_PATH` pointing into the session tmpdir, runs a JSON-RPC `tools/call`, returns the parsed result. | 60/min per session_id |
| POST | `/api/devnet/trigger` | Triggers one real on-chain tx using the server-controlled funded wallet. Returns `{ signature, explorer_url, amount_usdc, body, latency_ms }`. | **1/hour per IP** (the expensive one) |
| GET | `/api/devnet/wallet-status` | Returns `{ sol_balance, usdc_balance, healthy: bool, message?: string }` so the site can render a "demo wallet status" badge and the operator can monitor without ssh-ing in. Marked `healthy: false` when SOL < 0.05 or USDC < 2. | 60/min per IP |
| GET | `/api/stats/latest-tx` | Returns the most recent successful devnet tx for the hero counter. | 60/min per IP |

**Session lifecycle:**
- `POST /api/sandbox/session` creates `/tmp/agentspay-session-{uuid}/` with subdirs for the SQLite db + keypair.
- Session stored in Redis with 30-min TTL: `session:{uuid} → { tmpdir, created_at }`.
- On `/api/sandbox/call`, spawn `agentspay-mcp` as a subprocess with env vars pointing into the tmpdir. Process exits after the JSON-RPC handshake. **No long-lived processes.**
- A background sweep job in the shim deletes tmpdirs older than 30 min.

**Devnet wallet:**
- One keypair, owned by the shim, stored at `/data/devnet-wallet.json` inside the container (mounted volume so it survives restarts).
- Funded manually by the operator via Circle + Solana faucets.
- `GET /api/devnet/wallet-status` exposes the current SOL + USDC balance so the operator (and a status badge on the site) can see when a refund is needed.
- When balance falls below `$2 USDC` or `0.05 SOL`, return `503 Service Unavailable` with a friendly "devnet faucet drained, will be refilled within 24h — try sandbox tab" message.

**Subprocess strategy:**
- One-shot subprocess per `/api/sandbox/call`. ~50-150ms cold start per call. Acceptable for a demo.
- The shim's Dockerfile (`docker/Dockerfile.shim`) builds **both** `agentspay-web-shim` and `agentspay-mcp` in the same Rust builder stage (`cargo build --release -p agentspay-web-shim -p agentspay-mcp`), then COPYs both binaries into the runtime stage. At runtime the shim invokes `/usr/local/bin/agentspay-mcp` as the subprocess. This guarantees the visitor sees the **literal** binary that's also released to end users — same code, same version, same behavior.

### 4.3 `examples/paid-endpoint` — unchanged

The existing crate is referenced as the "demo x402 provider" by the live demo. In the Docker stack, it's reachable from the shim at `http://paid-endpoint:3001/real-quote/{symbol}`. The shim instructs the spawned agentspay-mcp to call this internal URL.

The visitor never sees `paid-endpoint` directly; it's an internal service.

### 4.4 `docker/` — infrastructure

New top-level directory `docker/` containing:

```
docker/
  README.md                   — operator docs (start / stop / refill wallet / view logs)
  docker-compose.yml          — production stack
  docker-compose.local.yml    — local dev override (mounts source, --build)
  Caddyfile                   — TLS termination + reverse proxy rules
  Dockerfile.web              — multi-stage Next.js standalone build
  Dockerfile.shim             — multi-stage Rust build (cargo-chef for caching)
  Dockerfile.paid-endpoint    — multi-stage Rust build (same recipe as shim)
  .env.example                — DOMAIN, REDIS_URL, RUST_LOG, AGENTSPAY_DEVNET_WALLET_PATH
  scripts/
    refill-wallet.sh          — shows the wallet pubkey + faucet URLs for the operator
    backup-wallet.sh          — cp the wallet keypair to a safe location
```

**docker-compose.yml services:**
- `caddy` — `caddy:2-alpine`, mounts Caddyfile, holds TLS state in a volume, ports 80/443.
- `web` — built from Dockerfile.web, internal only (no exposed port), env: `NEXT_PUBLIC_API_BASE=/api`.
- `shim` — built from Dockerfile.shim, internal only, env: `REDIS_URL`, `AGENTSPAY_PAID_ENDPOINT_URL=http://paid-endpoint:3001`, `AGENTSPAY_DEVNET_WALLET_PATH=/data/devnet-wallet.json`. Mounts `wallet-data` volume to `/data`.
- `paid-endpoint` — built from Dockerfile.paid-endpoint, internal only, env: `AGENTSPAY_PROVIDER_KEYPAIR=/data/provider-keypair.json`. Mounts same `wallet-data` volume.
- `redis` — `redis:7-alpine`, internal only.

**Note:** the existing root `docker-compose.yml` is for the v2 stack (`gateway` + `postgres` + `redis` for the old architecture) and is now stale. We do **not** delete it (Plan.md §6 keeps the scaffolding) but the new `docker/docker-compose.yml` is the one the website ships with.

---

## 5. Data flow — the two interesting paths

### 5.1 Sandbox call (zero blockchain)

```
1. Visitor clicks "Run agentspay_balance()" on the page
2. Browser → POST /api/sandbox/session  (if no session_id in localStorage)
3. Shim creates /tmp/agentspay-session-{uuid}/, stores in Redis 30-min TTL,
   returns { session_id, expires_at }
4. Browser stores session_id in localStorage, calls
   POST /api/sandbox/call { session_id, tool: "agentspay_balance", args: {} }
5. Shim spawns agentspay-mcp subprocess with:
   AGENTSPAY_NETWORK=sandbox
   AGENTSPAY_KEYPAIR_PATH=/tmp/agentspay-session-{uuid}/keypair.json
   AGENTSPAY_DATABASE_URL=sqlite:///tmp/agentspay-session-{uuid}/db.sqlite?mode=rwc
6. Shim pipes initialize + notifications/initialized + tools/call into stdin,
   reads JSON-RPC responses, returns parsed result to browser
7. Subprocess exits. tmpdir survives for the next call.
8. Background sweep deletes tmpdirs > 30 min old
```

### 5.2 Devnet trigger (one real on-chain tx)

```
1. Visitor clicks "Trigger a real TX" on the Devnet tab
2. Browser → POST /api/devnet/trigger
3. Shim checks rate limit in Redis: GET ratelimit:devnet:{ip}
   - If counter >= 1 in last 1h: return 429 with X-RateLimit-Reset
4. Shim checks wallet balance via Solana RPC getBalance + SPL token account:
   - If SOL < 0.05 OR USDC < 2: return 503 "faucet drained"
5. Shim spawns agentspay-mcp subprocess with:
   AGENTSPAY_NETWORK=solana-devnet
   AGENTSPAY_KEYPAIR_PATH=/data/devnet-wallet.json
   AGENTSPAY_DATABASE_URL=sqlite:///data/devnet-ledger.db?mode=rwc  (persistent)
6. Shim pipes the MCP JSON-RPC sequence into the subprocess stdin:
   { initialize } → { notifications/initialized } →
   { tools/call agentspay_pay_url, args: { url: "http://paid-endpoint:3001/real-quote/{symbol}", max_amount_usdc: "0.50" } }
   The symbol is picked round-robin from {AAPL, MSFT, GOOG, NVDA, AMZN} per call so
   the demo doesn't show identical data on every refresh.
7. Subprocess parses 402, builds + signs SPL transfer_checked, base64s,
   sends X-Payment to paid-endpoint
8. paid-endpoint decodes, submits to api.devnet.solana.com, returns 200
   with X-Payment-Response containing the signature
9. Shim parses signature, INCR ratelimit:devnet:{ip} with 1h EXPIRE,
   updates "latest-tx" cache for the hero counter
10. Returns { signature, explorer_url, amount_usdc, body, latency_ms } to browser
11. Browser shows the result + Solscan link + a small "next try in 60 min" countdown
```

---

## 6. Error handling

| Failure | Where caught | UX |
|---|---|---|
| Subprocess crash / timeout (>10s) | Shim, per-call | Returns 502 with the stderr tail. Browser shows red "demo error" box with a retry button. |
| Devnet RPC down / wallet drained | Shim, before spawn | 503 with a friendly explainer. Browser falls back to "switch to sandbox tab". |
| Rate limit hit | Shim middleware | 429 with `Retry-After`. Browser shows countdown and offers sandbox tab. |
| Browser session expired (>30 min idle) | Shim on `/api/sandbox/call` | 410 Gone. Browser silently creates a new session and retries. |
| Redis down | Shim, all endpoints | Degrade to in-memory rate-limit (lossy across restarts). Log to tracing as a soft alert. The site stays functional. |
| paid-endpoint down | Shim, on devnet trigger | 502. Browser shows "demo provider offline, the protocol still works — see this old tx for proof: 4pGR…". |

All error responses carry a request_id (UUID) and a human-readable `message`. The frontend Terminal component shows the request_id so beta-testers can paste it into a GitHub issue.

---

## 7. Testing strategy

### 7.1 `services/web-shim` — Rust tests

- **Unit:** rate-limit logic against an embedded Redis mock (`redis::aio::ConnectionManager` with a fake).
- **Integration:** spin up the shim + a stubbed paid-endpoint in the same test process, hit `/api/sandbox/call` via reqwest, assert the JSON result. Uses `tempfile::TempDir` for isolated session dirs.
- **End-to-end smoke:** an `e2e/` cargo test that spawns the full Docker compose stack and runs a Playwright headless browser through the live demo. Optional, runs only on `--features e2e` flag.

### 7.2 `apps/frontend` — TypeScript tests

- **Unit:** `vitest` on the typed `lib/api.ts` wrappers with `msw` (Mock Service Worker) mocking the shim responses.
- **Component:** `@testing-library/react` snapshots of each section in dev-dark mode.
- **E2E:** **deferred** until shim e2e is green. Playwright tests in `apps/frontend/e2e/` that hit a real `pnpm dev` + shim, verify the hero counter polls, the sandbox tab can call balance, the devnet tab handles rate-limit gracefully.

### 7.3 Manual verification gates (operator playbook)

Before announcing the website to anyone:
1. `docker compose -f docker/docker-compose.yml up -d` — all 5 services healthy via `docker compose ps`.
2. Open the site on a fresh browser profile. Hero loads in < 1s.
3. Click "Run agentspay_balance()" in sandbox tab. Response in < 500ms with realistic-looking JSON.
4. Click "Trigger a real TX" in devnet tab. Solscan link appears within 5s and resolves to a real tx.
5. Click it again. Get a friendly 429 with a 60-min countdown.
6. Open the site from a second IP (phone tether, etc.). Re-trigger devnet. Independent counter, works.
7. Verify the wallet status page shows current SOL + USDC balances correctly.

---

## 8. Out of scope (explicit non-goals for v0.1 of the site)

- Multi-language i18n. English only.
- Dark/light mode toggle. Dark only.
- Email signup / waitlist. The CTA is "install the binary", not "give us your email".
- Authentication or user accounts on the site.
- Persistent visitor state across browser tabs. Each tab is independent.
- Mainnet anywhere on the site. Devnet only, explicitly badged.
- Auto-refilling the devnet wallet (Circle faucet has a captcha; manual operator action).
- Analytics beyond a privacy-respecting hit counter (Plausible or self-hosted Umami later — not v0.1).
- A separate `/docs` page. README on GitHub is the docs for v0.1.
- A blog. Defer.

---

## 9. Implementation order

The order matters because each step unlocks visible progress to test against.

1. **Workspace member:** add `services/web-shim` to `Cargo.toml`. Empty `lib.rs` + bin stub. Verify `cargo check --workspace` green.
2. **Shim — sandbox session + call:** implement `/api/sandbox/session` + `/api/sandbox/call`. No Redis yet — in-memory `HashMap<SessionId, TmpDir>`. Verify by curl-ing the endpoints, observing subprocess spawn in logs.
3. **Frontend — sections 1, 2, 6:** Hero, Install, Footer in dev-dark Tailwind. No live data yet, just static markup. Verify by `pnpm --filter frontend dev` and visual inspection.
4. **Frontend — sandbox tab (§3a):** wire the SandboxTab component to the shim. Tool buttons fire `/api/sandbox/call`. Terminal component streams the responses. Verify end-to-end manual test against `localhost`.
5. **Shim — Redis + rate limits:** introduce Redis service, move session storage and rate-limit counters in. Verify rate-limit behavior with parallel curl.
6. **Shim — devnet trigger:** implement `/api/devnet/trigger` with wallet check + paid-endpoint call. Operator funds the wallet via Circle + Solana faucets. Verify a real on-chain tx end-to-end.
7. **Frontend — devnet tab (§3b):** wire DevnetTab + SolscanLink. Verify the visitor flow.
8. **Frontend — sections 4, 5:** How-it-works + Why cards. Static content.
9. **Frontend — hero live-tx counter:** poll `/api/stats/latest-tx` every 15s, render the badge in §1.
10. **Docker:** write all Dockerfiles + `docker/docker-compose.yml` + Caddyfile. Verify the full stack runs in compose on `localhost`.
11. **Deploy:** Hetzner VPS or Fly.io. TLS via Caddy automatic. Verify DNS + HTTPS.
12. **Smoke gate (§7.3):** run the operator playbook. Fix anything that's not green before sharing the URL.

---

## 10. Open questions for the user before writing-plans

None blocking. The remaining decisions are tactical and will be made inside the writing-plans skill:
- exact Tailwind colour palette tokens for the dev-dark theme (Inter font assumed)
- choice of font for monospace blocks (default to system `ui-monospace`)
- domain name (assume `agentspay.dev` for now, swap later)
- DNS provider / TLS issuer details (Caddy ACME defaults)

---

## 11. References

- `Plan.md` v3 — MCP Wallet pivot
- `PROJECT_CONTEXT.md` — running collaboration log (Week 1-3.1 results)
- `README.md` — current install + demo flow
- `apps/frontend/` — existing Next.js + Tailwind + shadcn scaffold (to be filled in)
- `examples/paid-endpoint/` — the x402-priced API we reference as the demo provider
- `services/mcp/` — the binary we spawn from the shim
- Brainstorm session: `.superpowers/brainstorm/24576-1778768902/`
