# AgentsPay Security Audit — 2026-05-17

Scope: the entire repository at branch `feat/website-and-live-demo`, commit `445e4ba` and earlier. Focus on what an unauthenticated remote attacker (or a malicious x402 seller, or a prompt-injection target) could do to extract funds, exfiltrate data, or DoS the system.

This is a **first-pass audit**, not a third-party pentest. Findings below are concrete bugs caught by reading the code path-by-path. Anything labeled "ACCEPTED RISK" is a known limitation we are not patching today.

## Threat model

| Actor | Capability | Goal |
|---|---|---|
| Malicious x402 seller | Controls the URL the agent is told to pay | Inflate the transfer amount, redirect funds, OOM the agent, force agent to hit internal services |
| Prompt-injected LLM | Controls the URL string passed to `pay_url` | Probe internal services (SSRF), exfiltrate data, drain budget |
| Remote unauthenticated client | Hits the web-shim public API | Drain the public demo wallet, enumerate sessions, bypass rate limits |
| Browser-side attacker | Sends cross-origin requests to the frontend | XSS, CSRF, abuse the API proxy |
| Container escape attempt | Has shell inside one container | Reach root on the host, modify other containers |

Out of scope for this audit: physical access, the host OS, the Solana RPC provider, the Anthropic MCP host. Mainnet usage is out of scope because Plan v3.1 keeps it gated.

## Findings summary

| ID | Severity | Surface | Status |
|---|---|---|---|
| CRIT-1 | Critical | `pay_url` x402 flow | **Fixed** |
| CRIT-2 | Critical | `pay_url` x402 flow | **Fixed** |
| CRIT-3 | Critical | `pay_url` x402 flow | **Fixed** |
| CRIT-4 | Critical | web-shim rate limits | **Fixed** |
| HIGH-1 | High | Docker images | **Fixed** |
| HIGH-2 | High | wallet keypair file | Accepted risk |
| HIGH-3 | High | frontend API proxy | **Fixed** |
| MED-1 | Medium | wallet write TOCTOU | Accepted risk |
| MED-2 | Medium | mainnet USDC mint | Accepted risk |
| LOW-1 | Low | logging | Verified clean |
| LOW-2 | Low | SQL injection | Verified clean (parameterized) |
| LOW-3 | Low | session IDs | Verified clean (UUIDv4) |

---

## CRIT-1 — Decimals mismatch lets a malicious seller inflate the transfer 1000×

**Vector.** A malicious x402 seller declares `extra.decimals=9` (or 12, or anything ≠ 6) in the 402 challenge. The agent computes the fiat cap check using the seller-declared decimals (`maxAmountRequired / 10^decimals`), so a tiny declared amount like `100000` with `decimals=9` reads as `0.0001 USDC` — well under any sane cap. But the actual SPL `transfer_checked` instruction uses the hardcoded `USDC_DECIMALS=6` in `services/mcp/src/solana.rs`, so the agent transfers `100000 / 10^6 = 0.10 USDC`. **1000× overpayment with the agent's own keypair, recipient under attacker control.**

**Fix.** `services/mcp/src/x402.rs::prepare` now validates `requirement.extra.decimals == REQUIRED_USDC_DECIMALS (6)` in real-signing modes and returns `X402Error::DecimalsMismatch` if not. The cap check and the transfer instruction now use the same decimals or the call is rejected.

**Tests.** `x402::tests::decimals_mismatch_is_caught`, `x402::tests::missing_decimals_is_rejected`, `x402::tests::obscenely_large_decimals_safe_via_validator`.

## CRIT-2 — Asset mint not validated; any mint accepted

**Vector.** Same flow as CRIT-1. The seller can set `asset` to any mint pubkey in the 402 challenge — the previous code parsed but never compared it. Combined with CRIT-1 a malicious seller could declare `asset=<some volatile shitcoin>, decimals=9, amount=100000` and the agent would sign a USDC transfer (the signing path always uses USDC regardless of `asset`) of the inflated amount.

**Fix.** `services/mcp/src/x402.rs::prepare` rejects with `X402Error::AssetMismatch` if `requirement.asset` is present and does not equal `crate::solana::USDC_MINT_DEVNET`. Agent will only sign USDC. Multi-asset support is a deliberate non-goal for v0.x.

**Tests.** `x402::tests::asset_mismatch_pattern`.

## CRIT-3 — Unbounded response body permits memory-exhaustion DoS

**Vector.** Both the initial probe (`prepare`) and the post-payment retry (`complete`) used `reqwest::Response::text()`, which buffers the entire body into RAM. A malicious seller serves a 402 (or a 200 retry response) with a `Content-Length: 10000000000` body and the MCP process OOMs.

**Fix.** New `read_body_bounded` helper in `services/mcp/src/x402.rs` iterates `Response::chunk()` and returns `X402Error::BodyTooLarge` if the cumulative read exceeds `MAX_BODY_BYTES = 1 MiB`. Both `prepare` and `complete` now route through it.

**Tests.** `x402::tests::body_size_cap_rejects_oversized_responses` — a tiny in-process HTTP server emits a 5 MiB body and the validator rejects.

## CRIT-4 — Web-shim rate limit keyed on direct peer IP, defeated by reverse proxy

**Vector.** All web-shim handlers used `ConnectInfo<SocketAddr>` to key the rate limiter. In production the shim sits behind Caddy, so every request appears to come from Caddy's container IP. Result: **all visitors share one bucket**. The `/api/devnet/trigger` endpoint, supposedly limited to 1/hour-per-IP, became 1/hour for the whole world — but worse, a single attacker could exhaust the public demo wallet by hammering the endpoint and getting `WalletDrained` errors after they spent the funds for everyone.

**Fix.** New helper `services/web-shim/src/handlers/mod.rs::client_ip` prefers `X-Forwarded-For` (which Caddy sets to the real `remote_host` and strips the client-provided value, see `docker/Caddyfile`), falling back to the direct peer for non-proxied deployments. All four rate-limited handlers (`create_session`, `wallet_status`, `trigger`, plus `call_tool` which keys on `session_id` already) updated. The frontend API proxy strips any incoming `X-Forwarded-For` / `Forwarded` headers so a client cannot bypass the proxy's add.

**Tests.** Manual review of Caddyfile, see `docker/Caddyfile` — `header_up X-Forwarded-For {remote_host}` overwrites any client value.

## HIGH-1 — Docker images ran as root

**Vector.** None of `Dockerfile.shim`, `Dockerfile.paid-endpoint`, `Dockerfile.web` had a `USER` directive. A successful exploit inside any service would have full root inside the container, simplifying privilege-escalation toward host or sibling-container compromise.

**Fix.** All three Dockerfiles now create a non-root `agentspay` user (uid 10001) and add `USER agentspay`. The Next.js image uses the pre-built `node` user (uid 1000) and `--chown=node:node` on the COPY commands.

## HIGH-2 — Agent can be tricked into paying ANY pubkey within budget

**Vector.** By design of x402, the seller dictates `payTo`. The agent will sign to whatever pubkey appears there, with no allowlist or reputation check. A malicious URL fed to `pay_url` (e.g. via prompt injection) can extract up to `per_call_cap_usd` per call and `daily_cap_usd` per day, sending to an attacker pubkey.

**Status. ACCEPTED RISK.** This is the budget cap's job. The mitigations CRIT-1 + CRIT-2 + the SSRF guard below close every multiplier that was inflating the damage. A future v0.x improvement is a `~/.agentspay/payee-allowlist` / `payee-denylist` config; not in scope for v0.3.

## HIGH-3 — Frontend `/api/[...path]` proxy did not validate path segments

**Vector.** The Next.js dynamic route `app/api/[...path]/route.ts` concatenated user-provided path segments straight into the upstream URL. `/api/../foo` would normalize at fetch time to `${SHIM}/foo`, bypassing the `/api/` scope. Not exploitable today (the shim mounts everything under `/api/`), but a foot-gun the moment anyone adds a non-`/api/` route.

**Fix.** Each segment must match `^[A-Za-z0-9._-]+$`, be ≤ 64 chars, and not equal `.` or `..`. Limit total segments to 8. Reject the request 400 otherwise. Also strip incoming `X-Forwarded-For` so a client can't forge the shim's rate-limit key via the frontend proxy.

## MED-1 — Wallet file write is two-step (write then chmod)

**Vector.** `services/mcp/src/wallet.rs::load_or_create` writes the keypair JSON, then calls `set_owner_only_permissions` to set mode 0600. There's a microsecond TOCTOU window where the file is readable by the default umask. Exploitable only if another local user is monitoring the parent directory at exactly that moment on first-run.

**Status. ACCEPTED RISK.** Parent dir is `~/.agentspay/` which is created with default umask (typically `drwxr-xr-x` for `$HOME`'s subdir), so a non-owner user could `cat` the file in that window. Strict fix is `OpenOptions::new().mode(0o600).create_new(true).open(...)` (unix-only); deferred to v0.x. The keypair test `wallet::tests::created_keypair_is_mode_0600` verifies the steady-state mode.

## MED-2 — Mainnet USDC mint is hardcoded to the devnet value

**Vector.** `services/mcp/src/solana.rs::usdc_mint_devnet` is used unconditionally even when `AGENTSPAY_NETWORK=solana-mainnet`. A mainnet transfer using the devnet USDC mint would fail (the mint doesn't exist on mainnet), so this is a **liveness** bug not a security one. But if someone hooks a custom mint in the future and forgets to route it, it could become a security bug.

**Status. ACCEPTED RISK.** Plan v3.1 gates mainnet behind a compliance review; v0.x is devnet only. The hardcoded mint is correct for the only mode that actually signs.

## LOW-1 — No secret material logged

**Verified.** `services/mcp/src/wallet.rs` does not derive `Debug` for `AgentWallet`; the keypair bytes never appear in any `tracing::*` event or `format!`. The pretty stderr banner shows only the public pubkey. The audit log captures URLs and amounts but never request/response bodies.

## LOW-2 — SQL injection

**Verified.** All ledger access goes through SeaORM entities (parameterized). The two raw-SQL sites in `services/web-shim/src/latest_tx.rs` use `Statement::from_string` only for the constant `CREATE TABLE IF NOT EXISTS` DDL and `Statement::from_sql_and_values` (parameterized) for INSERT/UPSERT/SELECT.

## LOW-3 — Session ID generation

**Verified.** `services/web-shim/src/session.rs` uses `Uuid::new_v4()`. 122 bits of entropy — not enumerable.

---

## Test coverage added by this audit

- **Rust adversarial unit tests** (`services/mcp/src/main.rs::tests`, `services/mcp/src/x402.rs::tests`):
  - 7 SSRF tests covering loopback, RFC1918, link-local (AWS/GCP metadata), CGNAT, IPv4 zero, IPv6 link-local + ULA, public-address allow path, env-opt-out.
  - 6 x402 adversarial tests covering decimals mismatch, asset mismatch, network mismatch, amount-parse edge cases, missing decimals, oversized decimals, and one tokio integration test that spins up a local HTTP server emitting an over-sized body to validate `read_body_bounded`.
- Pre-existing tests still green (24 mcp + 4 web-shim + 10 sdk-js).

Total: 43 Rust unit tests + 10 TypeScript unit tests in CI.

## What this audit did NOT cover (deferred)

- **Browser fuzzing of the public API.** A targeted scan with Burp or zaproxy against the deployed shim. Sub-bullet: CORS preflight behavior under various Origin headers.
- **Time-of-check vs time-of-use on the keypair file** beyond the steady-state mode test.
- **Cross-container privilege boundaries** under a hostile compose deployment (e.g., what if redis is compromised, can it reach the shim's `/data` volume? Answer: no by default, redis runs in its own container with a separate volume, but worth a follow-up validation in production).
- **Solana RPC failure modes.** What happens if the RPC sends an attacker-controlled response to `get_latest_blockhash`? The blockhash is consumed by `Transaction::new_signed_with_payer` and signed; the signer can't be tricked into anything beyond using a wrong/stale blockhash, which causes the tx to fail. Worth a code review but not in this pass.
- **CSRF on the web-shim**. The shim has no auth/cookies, so traditional CSRF doesn't apply. But a CSRF-like attack where attacker.com calls `/api/devnet/trigger` from a victim's browser is now limited by the per-victim-IP rate-limit (after CRIT-4 fix). Still worth tightening with an Origin/Referer check; deferred.

## How to maintain this audit

1. Every change that touches `services/mcp/src/x402.rs`, `services/mcp/src/main.rs::agentspay_pay_url`, or `services/web-shim/src/handlers/` must keep the existing adversarial tests passing.
2. Run `cargo test --workspace` before pushing.
3. When adding a new MCP tool that takes URL or amount input, write the adversarial counterpart test first.
4. Re-run this audit before any mainnet-mode work (Plan v0.5).
