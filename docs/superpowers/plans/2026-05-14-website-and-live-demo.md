# AgentsPay Website + Live Demo — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship a single-page marketing website at `agentspay.dev` with an in-browser sandbox demo + a one-click live Solana devnet demo, deployed via Docker.

**Architecture:** Next.js 15 standalone build + a new Rust Axum "shim" service that spawns the real `agentspay-mcp` binary per request (browser → Caddy → shim → spawned mcp → paid-endpoint → Solana devnet RPC). Redis for rate-limit counters. Single VPS deployment via docker compose.

**Tech Stack:** Rust (Axum, tokio, reqwest, redis-rs, sea-orm), Next.js 15 + Tailwind + shadcn/ui, Docker compose, Caddy 2 (TLS).

**Spec:** [`docs/superpowers/specs/2026-05-14-website-and-live-demo-design.md`](../specs/2026-05-14-website-and-live-demo-design.md)

---

## File Structure (locked in advance)

**New crate** — `services/web-shim/`
```
services/web-shim/
  Cargo.toml                       New
  src/
    main.rs                        New — binary entrypoint, axum router, tracing init
    state.rs                       New — AppState struct (redis client, http client, paths, config)
    config.rs                      New — env-var loading + validation
    error.rs                       New — ShimError + IntoResponse
    handlers/
      mod.rs                       New — re-exports
      health.rs                    New — GET /api/health
      sandbox.rs                   New — POST /api/sandbox/session, /call
      devnet.rs                    New — POST /api/devnet/trigger, GET /api/devnet/wallet-status
      stats.rs                     New — GET /api/stats/latest-tx
    session.rs                     New — session lifecycle (tmpdir + redis or in-memory)
    subprocess.rs                  New — spawn agentspay-mcp + pipe JSON-RPC
    ratelimit.rs                   New — sliding-window per-IP counters
    devnet_wallet.rs               New — load wallet, check balance, trigger tx via mcp
    latest_tx.rs                   New — in-memory cache for hero counter
```

**Frontend** — `apps/frontend/` (existing scaffold, fill in)
```
apps/frontend/app/
  page.tsx                         Modify — compose all sections
  layout.tsx                       Modify — meta tags + dev-dark CSS vars
  globals.css                      Modify — dev-dark Tailwind tokens
  api/[...path]/route.ts           New — catch-all proxy to shim
apps/frontend/components/
  sections/
    Hero.tsx                       New
    Install.tsx                    New
    LiveDemo.tsx                   New (composes the 2 tabs)
    SandboxTab.tsx                 New
    DevnetTab.tsx                  New
    HowItWorks.tsx                 New
    Why.tsx                        New
    Footer.tsx                     New
  ui/
    CodeBlock.tsx                  New (copyable command)
    Terminal.tsx                   New (streaming output)
    SolscanLink.tsx                New
    LiveTxBadge.tsx                New (hero counter)
apps/frontend/lib/
  api.ts                           New — typed fetch wrappers
  live-tx.ts                       New — hero counter store
apps/frontend/tailwind.config.ts   Modify — extend with dev-dark palette
```

**Infrastructure** — `docker/` (new top-level)
```
docker/
  README.md                        New — operator runbook
  docker-compose.yml               New — production stack (5 svc)
  docker-compose.local.yml         New — dev override
  Caddyfile                        New — TLS + reverse proxy
  Dockerfile.web                   New — Next.js standalone build
  Dockerfile.shim                  New — Rust builder + 2 binaries (shim + mcp)
  Dockerfile.paid-endpoint         New — Rust builder
  .env.example                     New
  scripts/
    refill-wallet.sh               New
    backup-wallet.sh               New
```

**Workspace** — `Cargo.toml` (root)
```
Modify — add "services/web-shim" to workspace members,
         add `tower-http`, `redis`, `tempfile` workspace dependencies
```

---

## Phase 0 — Workspace foundation (≈ 15 min)

**Goal of phase:** `cargo check --workspace` green with empty `agentspay-web-shim` crate registered.

### Task 0.1: Add workspace dependencies for the shim

**Files:**
- Modify: `Cargo.toml` (workspace root)

- [ ] **Step 1: Edit `Cargo.toml`**

Add to `[workspace.dependencies]` (sorted alphabetically with existing entries):

```toml
redis = { version = "0.27", default-features = false, features = ["tokio-comp", "connection-manager"] }
tempfile = "3"
tower-http = { version = "0.6", features = ["cors", "trace"] }
```

- [ ] **Step 2: Add `services/web-shim` to workspace members**

```toml
[workspace]
members = [
    "examples/paid-endpoint",
    "packages/proto",
    "services/auth",
    "services/gateway",
    "services/mcp",
    "services/metering",
    "services/payment",
    "services/web-shim",
]
```

- [ ] **Step 3: Verify there is no crate yet**

Run: `cargo check --workspace`
Expected: FAIL with `error: failed to load manifest for workspace member \`.../services/web-shim\``.

- [ ] **Step 4: Commit**

```bash
git add Cargo.toml
git commit -m "chore(workspace): register agentspay-web-shim member and shim deps"
```

### Task 0.2: Scaffold empty shim crate

**Files:**
- Create: `services/web-shim/Cargo.toml`
- Create: `services/web-shim/src/main.rs`

- [ ] **Step 1: Create `services/web-shim/Cargo.toml`**

```toml
[package]
name = "agentspay-web-shim"
version.workspace = true
edition.workspace = true
license.workspace = true

[[bin]]
name = "agentspay-web-shim"
path = "src/main.rs"

[dependencies]
anyhow.workspace = true
axum.workspace = true
chrono.workspace = true
redis.workspace = true
reqwest.workspace = true
serde.workspace = true
serde_json.workspace = true
tempfile.workspace = true
thiserror.workspace = true
tokio = { workspace = true, features = ["fs", "macros", "process", "rt-multi-thread", "signal", "sync", "time"] }
tower-http.workspace = true
tracing.workspace = true
tracing-subscriber.workspace = true
url.workspace = true
uuid.workspace = true
```

- [ ] **Step 2: Create `services/web-shim/src/main.rs` (minimal stub)**

```rust
//! agentspay-web-shim — HTTP bridge between the marketing website and the
//! local `agentspay-mcp` binary.
//!
//! Spawns the real MCP server as a subprocess per request, pipes JSON-RPC
//! over stdio, and exposes the responses as JSON over HTTPS. Holds the
//! rate-limited devnet wallet for the one-click "trigger a real TX" demo.

use std::net::SocketAddr;

use axum::{routing::get, Json, Router};
use serde_json::json;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main(flavor = "multi_thread")]
async fn main() -> anyhow::Result<()> {
    init_tracing();

    let app = Router::new().route("/api/health", get(health));
    let addr: SocketAddr = "0.0.0.0:8080".parse()?;
    tracing::info!(%addr, "agentspay-web-shim listening");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn health() -> Json<serde_json::Value> {
    Json(json!({"status":"ok","service":"agentspay-web-shim","version":env!("CARGO_PKG_VERSION")}))
}

fn init_tracing() {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("agentspay_web_shim=info,tower_http=info"));
    tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr))
        .init();
}
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo check --workspace`
Expected: PASS, `Finished dev profile`.

- [ ] **Step 4: Smoke-test the health route**

Run in one terminal: `cargo run -p agentspay-web-shim`
Run in another: `curl -s http://localhost:8080/api/health`
Expected: `{"service":"agentspay-web-shim","status":"ok","version":"0.1.0"}`

Stop the server with Ctrl-C.

- [ ] **Step 5: Commit**

```bash
git add services/web-shim
git commit -m "feat(web-shim): scaffold crate with /api/health endpoint"
```

**Milestone:** Phase 0 complete. Workspace builds clean with the new shim crate.

---

## Phase 1 — Shim sandbox endpoints (≈ 90 min)

**Goal of phase:** Browser can `POST /api/sandbox/session` to get a session_id, then `POST /api/sandbox/call` to invoke any of the 5 MCP tools and receive the parsed JSON response. State isolated per session via tmpdirs. **In-memory session store** (Redis comes in Phase 3).

### Task 1.1: Define error + state + config modules

**Files:**
- Create: `services/web-shim/src/error.rs`
- Create: `services/web-shim/src/state.rs`
- Create: `services/web-shim/src/config.rs`
- Modify: `services/web-shim/src/main.rs`

- [ ] **Step 1: Write `error.rs`**

```rust
//! Typed errors that map to HTTP responses.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum ShimError {
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error("session not found or expired")]
    SessionGone,
    #[error("rate limited (retry in {retry_after_secs}s)")]
    RateLimited { retry_after_secs: u64 },
    #[error("subprocess timed out after {0}s")]
    SubprocessTimeout(u64),
    #[error("subprocess failed: {0}")]
    SubprocessFailed(String),
    #[error("malformed mcp response: {0}")]
    MalformedMcp(String),
    #[error("devnet wallet drained or unhealthy")]
    WalletDrained,
    #[error("internal error: {0}")]
    Internal(String),
}

impl IntoResponse for ShimError {
    fn into_response(self) -> Response {
        let request_id = Uuid::new_v4().to_string();
        let (status, code) = match &self {
            Self::BadRequest(_) => (StatusCode::BAD_REQUEST, "bad_request"),
            Self::SessionGone => (StatusCode::GONE, "session_gone"),
            Self::RateLimited { .. } => (StatusCode::TOO_MANY_REQUESTS, "rate_limited"),
            Self::SubprocessTimeout(_) => (StatusCode::GATEWAY_TIMEOUT, "subprocess_timeout"),
            Self::SubprocessFailed(_) => (StatusCode::BAD_GATEWAY, "subprocess_failed"),
            Self::MalformedMcp(_) => (StatusCode::BAD_GATEWAY, "malformed_mcp"),
            Self::WalletDrained => (StatusCode::SERVICE_UNAVAILABLE, "wallet_drained"),
            Self::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, "internal"),
        };
        tracing::warn!(error = %self, request_id = %request_id, "request failed");
        let mut response = (
            status,
            Json(json!({
                "code": code,
                "message": self.to_string(),
                "request_id": request_id,
            })),
        )
            .into_response();
        if let Self::RateLimited { retry_after_secs } = &self {
            if let Ok(v) = retry_after_secs.to_string().parse() {
                response.headers_mut().insert("retry-after", v);
            }
        }
        response
    }
}
```

- [ ] **Step 2: Write `config.rs`**

```rust
//! Environment-variable configuration for agentspay-web-shim.

use std::{env, net::SocketAddr, path::PathBuf, time::Duration};

#[derive(Debug, Clone)]
pub struct Config {
    pub listen_addr: SocketAddr,
    pub mcp_binary: PathBuf,
    pub paid_endpoint_url: String,
    pub devnet_wallet_path: PathBuf,
    pub devnet_ledger_path: PathBuf,
    pub session_ttl: Duration,
    pub subprocess_timeout: Duration,
    /// `None` → run with in-memory session+ratelimit stores. Phase 3 sets it.
    pub redis_url: Option<String>,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            listen_addr: env::var("AGENTSPAY_SHIM_LISTEN_ADDR")
                .unwrap_or_else(|_| "0.0.0.0:8080".to_string())
                .parse()?,
            mcp_binary: env::var("AGENTSPAY_MCP_BINARY")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("/usr/local/bin/agentspay-mcp")),
            paid_endpoint_url: env::var("AGENTSPAY_PAID_ENDPOINT_URL")
                .unwrap_or_else(|_| "http://localhost:3001".to_string()),
            devnet_wallet_path: env::var("AGENTSPAY_DEVNET_WALLET_PATH")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("/data/devnet-wallet.json")),
            devnet_ledger_path: env::var("AGENTSPAY_DEVNET_LEDGER_PATH")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("/data/devnet-ledger.db")),
            session_ttl: Duration::from_secs(30 * 60),
            subprocess_timeout: Duration::from_secs(10),
            redis_url: env::var("AGENTSPAY_REDIS_URL").ok(),
        })
    }
}
```

- [ ] **Step 3: Write `state.rs`**

```rust
//! Shared `AppState` cloned into every Axum handler.

use std::sync::Arc;

use crate::{config::Config, session::SessionStore};

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub sessions: Arc<SessionStore>,
    pub http: reqwest::Client,
}
```

- [ ] **Step 4: Add module declarations and unused stub to `main.rs`**

Replace the contents of `services/web-shim/src/main.rs` with:

```rust
//! agentspay-web-shim — HTTP bridge between the marketing website and the
//! local `agentspay-mcp` binary.

mod config;
mod error;
mod session;
mod state;

use axum::{routing::get, Json, Router};
use serde_json::json;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use crate::{config::Config, session::SessionStore, state::AppState};

#[tokio::main(flavor = "multi_thread")]
async fn main() -> anyhow::Result<()> {
    init_tracing();

    let config = std::sync::Arc::new(Config::from_env()?);
    let sessions = std::sync::Arc::new(SessionStore::new_in_memory(config.session_ttl));
    let http = reqwest::Client::builder()
        .user_agent("agentspay-web-shim/0.1.0")
        .build()?;
    let state = AppState {
        config: config.clone(),
        sessions,
        http,
    };

    let app = Router::new()
        .route("/api/health", get(health))
        .with_state(state);

    let addr = config.listen_addr;
    tracing::info!(%addr, "agentspay-web-shim listening");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn health() -> Json<serde_json::Value> {
    Json(json!({"status":"ok","service":"agentspay-web-shim","version":env!("CARGO_PKG_VERSION")}))
}

fn init_tracing() {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("agentspay_web_shim=info,tower_http=info"));
    tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr))
        .init();
}
```

- [ ] **Step 5: Run `cargo check` — expect "missing session module" failure**

Run: `cargo check -p agentspay-web-shim`
Expected: FAIL with `unresolved module 'session'`. We fix that next.

### Task 1.2: Implement in-memory SessionStore

**Files:**
- Create: `services/web-shim/src/session.rs`

- [ ] **Step 1: Write `session.rs`**

```rust
//! Session lifecycle for the sandbox demo.
//!
//! A session owns an isolated tmpdir that holds the agent's SQLite ledger
//! and Solana keypair. We spawn `agentspay-mcp` against this tmpdir on
//! every `/api/sandbox/call`, so multiple browser tabs never share state.
//!
//! Phase 1 ships with an in-memory implementation (DashMap-style with an
//! `RwLock<HashMap>`). Phase 3 swaps the backing store for Redis without
//! changing this module's public surface.

use std::{
    collections::HashMap,
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};

use tempfile::TempDir;
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Debug)]
pub struct Session {
    pub id: String,
    pub created_at: Instant,
    /// Holds the TempDir so it is deleted when the session expires.
    pub tmpdir: TempDir,
}

impl Session {
    pub fn keypair_path(&self) -> PathBuf {
        self.tmpdir.path().join("keypair.json")
    }
    pub fn db_path(&self) -> PathBuf {
        self.tmpdir.path().join("db.sqlite")
    }
    pub fn db_url(&self) -> String {
        format!("sqlite://{}?mode=rwc", self.db_path().display())
    }
}

pub struct SessionStore {
    inner: RwLock<HashMap<String, Arc<Session>>>,
    ttl: Duration,
}

impl SessionStore {
    pub fn new_in_memory(ttl: Duration) -> Self {
        Self {
            inner: RwLock::new(HashMap::new()),
            ttl,
        }
    }

    pub async fn create(&self) -> anyhow::Result<Arc<Session>> {
        let id = Uuid::new_v4().to_string();
        let tmpdir = TempDir::new()?;
        let session = Arc::new(Session {
            id: id.clone(),
            created_at: Instant::now(),
            tmpdir,
        });
        self.inner.write().await.insert(id, session.clone());
        Ok(session)
    }

    pub async fn get(&self, id: &str) -> Option<Arc<Session>> {
        let mut guard = self.inner.write().await;
        if let Some(s) = guard.get(id).cloned() {
            if s.created_at.elapsed() < self.ttl {
                return Some(s);
            }
            guard.remove(id);
        }
        None
    }

    /// Returns `(active, swept)` counts. Called by a background ticker.
    pub async fn sweep(&self) -> (usize, usize) {
        let mut guard = self.inner.write().await;
        let before = guard.len();
        guard.retain(|_, s| s.created_at.elapsed() < self.ttl);
        let after = guard.len();
        (after, before - after)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn create_then_get_returns_same_session() {
        let store = SessionStore::new_in_memory(Duration::from_secs(60));
        let s = store.create().await.unwrap();
        let id = s.id.clone();
        let fetched = store.get(&id).await.expect("session must be retrievable");
        assert_eq!(fetched.id, id);
    }

    #[tokio::test]
    async fn expired_session_is_swept() {
        let store = SessionStore::new_in_memory(Duration::from_millis(10));
        let s = store.create().await.unwrap();
        tokio::time::sleep(Duration::from_millis(30)).await;
        assert!(store.get(&s.id).await.is_none(), "expired session must be gone");
    }
}
```

- [ ] **Step 2: Run the new tests**

Run: `cargo test -p agentspay-web-shim --lib session`
Expected: 2 passed, 0 failed.

- [ ] **Step 3: Run `cargo check --workspace`**

Run: `cargo check --workspace`
Expected: PASS (warnings allowed).

- [ ] **Step 4: Commit**

```bash
git add services/web-shim
git commit -m "feat(web-shim): add config, error, in-memory SessionStore"
```

### Task 1.3: Implement subprocess JSON-RPC client

**Files:**
- Create: `services/web-shim/src/subprocess.rs`
- Modify: `services/web-shim/src/main.rs` (add `mod subprocess;`)

- [ ] **Step 1: Write `subprocess.rs`**

```rust
//! Spawn `agentspay-mcp`, pipe a JSON-RPC tools/call sequence into stdin,
//! parse the response, return the inner JSON value.
//!
//! The contract with `agentspay-mcp` is the MCP 2025-06-18 stdio transport.
//! We always send three messages in order: initialize, notifications/initialized,
//! tools/call. Then we close stdin and read line-delimited JSON from stdout
//! until we see the response for our tools/call id.

use std::{path::Path, process::Stdio, time::Duration};

use serde_json::{json, Value};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::Command,
    time::timeout,
};

use crate::error::ShimError;

#[derive(Debug, Clone, Copy)]
pub enum NetworkMode {
    Sandbox,
    SolanaDevnet,
}

impl NetworkMode {
    fn as_env(self) -> &'static str {
        match self {
            Self::Sandbox => "sandbox",
            Self::SolanaDevnet => "solana-devnet",
        }
    }
}

pub struct McpCall<'a> {
    pub binary: &'a Path,
    pub network: NetworkMode,
    pub keypair_path: &'a Path,
    pub db_url: &'a str,
    pub tool: &'a str,
    pub args: Value,
    pub timeout: Duration,
}

pub async fn run(call: McpCall<'_>) -> Result<Value, ShimError> {
    let mut cmd = Command::new(call.binary);
    cmd.env("AGENTSPAY_NETWORK", call.network.as_env())
        .env("AGENTSPAY_KEYPAIR_PATH", call.keypair_path)
        .env("AGENTSPAY_DATABASE_URL", call.db_url)
        .env("RUST_LOG", "agentspay_mcp=warn")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd
        .spawn()
        .map_err(|e| ShimError::SubprocessFailed(format!("spawn failed: {e}")))?;

    let mut stdin = child.stdin.take().ok_or_else(|| {
        ShimError::SubprocessFailed("subprocess has no stdin".into())
    })?;
    let stdout = child.stdout.take().ok_or_else(|| {
        ShimError::SubprocessFailed("subprocess has no stdout".into())
    })?;

    let init = json!({
        "jsonrpc":"2.0","id":1,"method":"initialize",
        "params":{
            "protocolVersion":"2025-06-18",
            "capabilities":{},
            "clientInfo":{"name":"agentspay-web-shim","version":env!("CARGO_PKG_VERSION")}
        }
    });
    let initialized = json!({
        "jsonrpc":"2.0","method":"notifications/initialized","params":{}
    });
    let tool_call = json!({
        "jsonrpc":"2.0","id":42,"method":"tools/call",
        "params":{"name":call.tool,"arguments":call.args}
    });

    let payload = format!("{init}\n{initialized}\n{tool_call}\n");
    stdin
        .write_all(payload.as_bytes())
        .await
        .map_err(|e| ShimError::SubprocessFailed(format!("stdin write: {e}")))?;
    drop(stdin);

    let reader = BufReader::new(stdout);
    let mut lines = reader.lines();

    let read_fut = async {
        while let Some(line) = lines.next_line().await.transpose() {
            let line = line.map_err(|e| ShimError::SubprocessFailed(format!("stdout read: {e}")))?;
            if line.trim().is_empty() {
                continue;
            }
            let msg: Value = serde_json::from_str(&line)
                .map_err(|e| ShimError::MalformedMcp(format!("{e}: {line}")))?;
            if msg.get("id").and_then(Value::as_i64) == Some(42) {
                if let Some(err) = msg.get("error") {
                    return Err(ShimError::MalformedMcp(format!("mcp error: {err}")));
                }
                return Ok(msg.get("result").cloned().unwrap_or(Value::Null));
            }
        }
        Err(ShimError::MalformedMcp("subprocess stdout closed without tools/call response".into()))
    };

    let result = timeout(call.timeout, read_fut)
        .await
        .map_err(|_| ShimError::SubprocessTimeout(call.timeout.as_secs()))?;

    // Reap the child; ignore non-zero exit since we already got our answer.
    let _ = child.wait().await;
    result
}
```

- [ ] **Step 2: Wire the module in `main.rs`**

Add `mod subprocess;` next to the other `mod` declarations in `services/web-shim/src/main.rs`.

- [ ] **Step 3: Run cargo check**

Run: `cargo check -p agentspay-web-shim`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add services/web-shim
git commit -m "feat(web-shim): add subprocess JSON-RPC client for agentspay-mcp"
```

### Task 1.4: Implement /api/sandbox/session handler

**Files:**
- Create: `services/web-shim/src/handlers/mod.rs`
- Create: `services/web-shim/src/handlers/health.rs`
- Create: `services/web-shim/src/handlers/sandbox.rs`
- Modify: `services/web-shim/src/main.rs`

- [ ] **Step 1: Write `handlers/mod.rs`**

```rust
pub mod health;
pub mod sandbox;
```

- [ ] **Step 2: Write `handlers/health.rs` (move from main.rs)**

```rust
use axum::Json;
use serde_json::{json, Value};

pub async fn health() -> Json<Value> {
    Json(json!({
        "status":"ok",
        "service":"agentspay-web-shim",
        "version":env!("CARGO_PKG_VERSION")
    }))
}
```

- [ ] **Step 3: Write `handlers/sandbox.rs`**

```rust
//! Sandbox session + tool-call handlers.

use std::time::Duration;

use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    error::ShimError,
    state::AppState,
    subprocess::{self, McpCall, NetworkMode},
};

#[derive(Debug, Serialize)]
pub struct SessionResponse {
    pub session_id: String,
    pub expires_in_secs: u64,
}

pub async fn create_session(
    State(state): State<AppState>,
) -> Result<Json<SessionResponse>, ShimError> {
    let session = state
        .sessions
        .create()
        .await
        .map_err(|e| ShimError::Internal(format!("session create: {e}")))?;
    Ok(Json(SessionResponse {
        session_id: session.id.clone(),
        expires_in_secs: state.config.session_ttl.as_secs(),
    }))
}

#[derive(Debug, Deserialize)]
pub struct CallRequest {
    pub session_id: String,
    pub tool: String,
    #[serde(default)]
    pub args: Value,
}

#[derive(Debug, Serialize)]
pub struct CallResponse {
    pub session_id: String,
    pub tool: String,
    pub result: Value,
    pub latency_ms: u128,
}

const ALLOWED_TOOLS: &[&str] = &[
    "agentspay_balance",
    "agentspay_pay_url",
    "agentspay_set_budget",
    "agentspay_audit_log",
    "agentspay_topup_info",
];

pub async fn call_tool(
    State(state): State<AppState>,
    Json(req): Json<CallRequest>,
) -> Result<Json<CallResponse>, ShimError> {
    if !ALLOWED_TOOLS.contains(&req.tool.as_str()) {
        return Err(ShimError::BadRequest(format!(
            "tool not allowed: {} (allowed: {})",
            req.tool,
            ALLOWED_TOOLS.join(", ")
        )));
    }

    let session = state
        .sessions
        .get(&req.session_id)
        .await
        .ok_or(ShimError::SessionGone)?;

    let start = std::time::Instant::now();
    let result = subprocess::run(McpCall {
        binary: &state.config.mcp_binary,
        network: NetworkMode::Sandbox,
        keypair_path: &session.keypair_path(),
        db_url: &session.db_url(),
        tool: &req.tool,
        args: req.args,
        timeout: Duration::from_secs(10),
    })
    .await?;

    Ok(Json(CallResponse {
        session_id: req.session_id,
        tool: req.tool,
        result,
        latency_ms: start.elapsed().as_millis(),
    }))
}
```

- [ ] **Step 4: Update `main.rs` to mount the handlers**

Replace the router section in `services/web-shim/src/main.rs`:

```rust
mod config;
mod error;
mod handlers;
mod session;
mod state;
mod subprocess;

use axum::{
    routing::{get, post},
    Router,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use crate::{config::Config, handlers::{health, sandbox}, session::SessionStore, state::AppState};

#[tokio::main(flavor = "multi_thread")]
async fn main() -> anyhow::Result<()> {
    init_tracing();

    let config = std::sync::Arc::new(Config::from_env()?);
    let sessions = std::sync::Arc::new(SessionStore::new_in_memory(config.session_ttl));
    let http = reqwest::Client::builder()
        .user_agent("agentspay-web-shim/0.1.0")
        .build()?;
    let state = AppState {
        config: config.clone(),
        sessions: sessions.clone(),
        http,
    };

    let app = Router::new()
        .route("/api/health", get(health::health))
        .route("/api/sandbox/session", post(sandbox::create_session))
        .route("/api/sandbox/call", post(sandbox::call_tool))
        .with_state(state);

    // Background sweep: every minute, drop expired sessions.
    let sweep_sessions = sessions.clone();
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(std::time::Duration::from_secs(60));
        loop {
            ticker.tick().await;
            let (active, swept) = sweep_sessions.sweep().await;
            if swept > 0 {
                tracing::info!(active, swept, "session sweep");
            }
        }
    });

    let addr = config.listen_addr;
    tracing::info!(%addr, mcp_binary = ?config.mcp_binary, "agentspay-web-shim listening");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

fn init_tracing() {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("agentspay_web_shim=info,tower_http=info"));
    tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr))
        .init();
}
```

- [ ] **Step 5: Run cargo check + cargo test**

Run: `cargo check -p agentspay-web-shim && cargo test -p agentspay-web-shim`
Expected: PASS, 2 tests OK.

- [ ] **Step 6: Manual end-to-end smoke test**

Build the mcp binary so the shim has a target to spawn:

```bash
cargo build -p agentspay-mcp -p agentspay-web-shim
```

Start the shim with `AGENTSPAY_MCP_BINARY` pointing at the just-built binary:

```bash
AGENTSPAY_MCP_BINARY=$PWD/target/debug/agentspay-mcp ./target/debug/agentspay-web-shim
```

In a second terminal, create a session and call balance:

```bash
SID=$(curl -s -X POST http://localhost:8080/api/sandbox/session | jq -r .session_id)
curl -s -X POST http://localhost:8080/api/sandbox/call \
  -H 'content-type: application/json' \
  -d "{\"session_id\":\"$SID\",\"tool\":\"agentspay_balance\",\"args\":{}}" | jq
```

Expected: a JSON response with `result.content[0].text` containing the wallet snapshot (a fresh sandbox session shows `available_usdc=100.00`, `budget_remaining_today_usdc=50.00`).

- [ ] **Step 7: Commit**

```bash
git add services/web-shim
git commit -m "feat(web-shim): wire /api/sandbox/session and /api/sandbox/call"
```

**Milestone:** Phase 1 complete. You can create sandbox sessions and call any of the 5 MCP tools via HTTP. The state is in-memory; restart wipes sessions. Phase 3 makes it durable.

---

## Phase 2 — Frontend static skeleton (≈ 75 min)

**Goal of phase:** `pnpm --filter frontend dev` renders the dev-dark home page with Hero, Install, Footer (no live data, no demo tab yet).

### Task 2.1: Set up the dev-dark Tailwind palette

**Files:**
- Modify: `apps/frontend/tailwind.config.ts`
- Modify: `apps/frontend/app/globals.css`

- [ ] **Step 1: Edit `apps/frontend/tailwind.config.ts`**

Replace its contents with:

```ts
import type { Config } from "tailwindcss";

const config: Config = {
  content: [
    "./app/**/*.{ts,tsx}",
    "./components/**/*.{ts,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        // dev-dark palette (Linear/Vercel/Anthropic console feel)
        bg: {
          DEFAULT: "#0a0a0b",
          elev: "#18181b",
          deep:  "#000000",
        },
        border: {
          DEFAULT: "#27272a",
          subtle:  "#1f1f23",
        },
        fg: {
          DEFAULT: "#fafafa",
          muted:   "#a1a1aa",
          dim:     "#71717a",
          faint:   "#52525b",
        },
        accent: {
          DEFAULT: "#10b981", // live/confirmed green
          glow:    "rgba(16,185,129,0.45)",
        },
        terminal: {
          green: "#a1f87f",
        },
      },
      fontFamily: {
        sans: ["Inter", "-apple-system", "BlinkMacSystemFont", "Segoe UI", "sans-serif"],
        mono: ["ui-monospace", "SFMono-Regular", "Menlo", "Monaco", "Consolas", "monospace"],
      },
      letterSpacing: {
        tight: "-0.02em",
      },
    },
  },
  plugins: [],
};

export default config;
```

- [ ] **Step 2: Replace `apps/frontend/app/globals.css`**

```css
@tailwind base;
@tailwind components;
@tailwind utilities;

:root {
  color-scheme: dark;
}

html, body {
  background: #0a0a0b;
  color: #fafafa;
  font-family: Inter, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
}

* { box-sizing: border-box; }

/* small selection accent */
::selection {
  background: rgba(16,185,129,0.25);
  color: #fafafa;
}
```

- [ ] **Step 3: Verify Tailwind compiles**

Run: `pnpm --filter frontend install`
Run: `pnpm --filter frontend lint`
Expected: PASS (no lint errors).

### Task 2.2: Write `<Hero />`

**Files:**
- Create: `apps/frontend/components/sections/Hero.tsx`

- [ ] **Step 1: Write the component**

```tsx
import Link from "next/link";

export function Hero() {
  return (
    <section className="px-6 pt-24 pb-16 max-w-3xl mx-auto">
      <p className="text-xs uppercase tracking-[0.12em] text-fg-muted mb-4">
        AGENTSPAY · v0.3
      </p>
      <h1 className="text-4xl sm:text-5xl font-semibold leading-[1.05] tracking-tight">
        Give your AI agent a<br />budget-controlled wallet.
      </h1>
      <p className="text-fg-muted mt-5 text-base sm:text-lg max-w-2xl">
        One MCP install. Real Solana settlement. Per-call + daily caps
        enforced before the chain — your agent literally cannot drain
        your wallet.
      </p>
      <div className="flex flex-wrap gap-3 mt-8">
        <Link
          href="#install"
          className="bg-white text-black rounded-md px-4 py-2.5 text-sm font-semibold hover:bg-fg transition"
        >
          Install in Claude Code
        </Link>
        <Link
          href="#demo"
          className="border border-border text-fg rounded-md px-4 py-2.5 text-sm font-medium hover:bg-bg-elev transition"
        >
          See live devnet demo →
        </Link>
      </div>
      <div className="mt-10 text-xs font-mono text-accent flex items-center gap-2">
        <span className="w-1.5 h-1.5 rounded-full bg-accent shadow-[0_0_8px_var(--tw-shadow-color)] shadow-accent" />
        Live tx counter loads once the demo wallet is funded — see §3 below.
      </div>
    </section>
  );
}
```

### Task 2.3: Write `<Install />`

**Files:**
- Create: `apps/frontend/components/ui/CodeBlock.tsx`
- Create: `apps/frontend/components/sections/Install.tsx`

- [ ] **Step 1: Write `CodeBlock.tsx`**

```tsx
"use client";

import { useState } from "react";

export function CodeBlock({ value }: { value: string }) {
  const [copied, setCopied] = useState(false);
  return (
    <div className="bg-bg-elev border border-border rounded-md font-mono text-sm text-terminal-green flex items-center justify-between px-4 py-3 group">
      <span className="truncate">$ {value}</span>
      <button
        onClick={() => {
          navigator.clipboard.writeText(value);
          setCopied(true);
          setTimeout(() => setCopied(false), 1200);
        }}
        className="text-xs text-fg-muted opacity-0 group-hover:opacity-100 transition uppercase tracking-wider"
      >
        {copied ? "copied" : "copy"}
      </button>
    </div>
  );
}
```

- [ ] **Step 2: Write `Install.tsx`**

```tsx
import { CodeBlock } from "@/components/ui/CodeBlock";

export function Install() {
  return (
    <section id="install" className="px-6 py-16 max-w-3xl mx-auto border-t border-border-subtle">
      <p className="text-xs uppercase tracking-[0.12em] text-accent mb-4 font-mono">
        §2 · INSTALL
      </p>
      <h2 className="text-2xl sm:text-3xl font-semibold tracking-tight mb-6">
        One command. Five MCP tools. Real on-chain settlement.
      </h2>
      <CodeBlock value="claude mcp add agentspay --from github:user/agentspay" />
      <p className="text-fg-muted text-sm mt-4">
        Cursor / Cline / Zed: see the README — manual config takes 30 seconds.
        Or grab a prebuilt binary from the latest GitHub release.
      </p>
    </section>
  );
}
```

### Task 2.4: Write `<Footer />`

**Files:**
- Create: `apps/frontend/components/sections/Footer.tsx`

- [ ] **Step 1: Write the component**

```tsx
export function Footer() {
  return (
    <footer className="border-t border-border-subtle mt-16 py-10 px-6 max-w-3xl mx-auto text-xs text-fg-dim flex flex-col sm:flex-row justify-between gap-4">
      <span>
        Open source · MIT · Built in <span className="text-fg">Rust</span> + <span className="text-fg">Next.js</span>
      </span>
      <span className="flex gap-4">
        <a href="https://github.com/user/agentspay" className="hover:text-fg transition">GitHub</a>
        <a href="https://x.com/user" className="hover:text-fg transition">X</a>
        <span>MCP registry (soon)</span>
      </span>
    </footer>
  );
}
```

### Task 2.5: Compose the page

**Files:**
- Modify: `apps/frontend/app/page.tsx`
- Modify: `apps/frontend/app/layout.tsx`

- [ ] **Step 1: Replace `page.tsx`**

```tsx
import { Hero } from "@/components/sections/Hero";
import { Install } from "@/components/sections/Install";
import { Footer } from "@/components/sections/Footer";

export default function Home() {
  return (
    <main>
      <Hero />
      <Install />
      <Footer />
    </main>
  );
}
```

- [ ] **Step 2: Update `layout.tsx`**

Replace its contents with:

```tsx
import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "AgentsPay — A budget-controlled USDC wallet for AI agents",
  description:
    "One MCP install. Real Solana settlement. Per-call and daily caps enforced before the chain.",
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en">
      <body>{children}</body>
    </html>
  );
}
```

- [ ] **Step 3: Verify it renders**

Run: `pnpm --filter frontend dev`
Open: http://localhost:3000

Expected: dark page, hero headline visible, "Install in Claude Code" button visible, install section with code block and copy button, footer with GitHub/X links. Ctrl-C the dev server.

- [ ] **Step 4: Commit**

```bash
git add apps/frontend
git commit -m "feat(frontend): static hero + install + footer in dev-dark theme"
```

**Milestone:** Phase 2 complete. The site renders. No demo tab yet, but the bones are visible.

---

## Phase 3 — Sandbox tab wired (≈ 90 min)

**Goal of phase:** Visitor opens the site, the LiveDemo section appears between Install and Footer with a Sandbox tab. Clicking "Run agentspay_balance" shows the real JSON response from the shim subprocess.

### Task 3.1: Next.js API proxy

**Files:**
- Create: `apps/frontend/app/api/[...path]/route.ts`
- Modify: `apps/frontend/next.config.mjs`

- [ ] **Step 1: Edit `next.config.mjs`**

```js
/** @type {import('next').NextConfig} */
const nextConfig = {
  output: "standalone",
  reactStrictMode: true,
};

export default nextConfig;
```

- [ ] **Step 2: Create the catch-all proxy**

```ts
// apps/frontend/app/api/[...path]/route.ts
import { NextRequest } from "next/server";

const SHIM_URL = process.env.AGENTSPAY_SHIM_URL ?? "http://localhost:8080";

async function proxy(req: NextRequest, ctx: { params: Promise<{ path: string[] }> }) {
  const { path } = await ctx.params;
  const targetUrl = `${SHIM_URL}/api/${path.join("/")}${req.nextUrl.search}`;

  const headers = new Headers(req.headers);
  headers.delete("host");
  headers.delete("content-length"); // re-computed by fetch

  const init: RequestInit = {
    method: req.method,
    headers,
    redirect: "manual",
  };
  if (req.method !== "GET" && req.method !== "HEAD") {
    init.body = await req.arrayBuffer();
  }

  const upstream = await fetch(targetUrl, init);
  const respHeaders = new Headers(upstream.headers);
  // strip hop-by-hop
  ["connection", "transfer-encoding", "content-encoding"].forEach((h) => respHeaders.delete(h));
  return new Response(upstream.body, {
    status: upstream.status,
    headers: respHeaders,
  });
}

export const GET = proxy;
export const POST = proxy;
export const PUT = proxy;
export const DELETE = proxy;
export const PATCH = proxy;
```

### Task 3.2: Typed API client

**Files:**
- Create: `apps/frontend/lib/api.ts`

- [ ] **Step 1: Write the typed wrapper**

```ts
export type ToolName =
  | "agentspay_balance"
  | "agentspay_pay_url"
  | "agentspay_set_budget"
  | "agentspay_audit_log"
  | "agentspay_topup_info";

export interface SessionResponse {
  session_id: string;
  expires_in_secs: number;
}

export interface CallResponse {
  session_id: string;
  tool: ToolName;
  result: { content: Array<{ type: "text"; text: string }> };
  latency_ms: number;
}

export interface ShimError {
  code: string;
  message: string;
  request_id: string;
}

async function http<T>(path: string, init: RequestInit): Promise<T> {
  const r = await fetch(path, {
    ...init,
    headers: { "content-type": "application/json", ...(init.headers || {}) },
  });
  if (!r.ok) {
    const body = (await r.json().catch(() => null)) as ShimError | null;
    throw new Error(body?.message ?? `${r.status} ${r.statusText}`);
  }
  return (await r.json()) as T;
}

let cachedSession: { id: string; loadedAt: number } | null = null;
const SESSION_REFRESH_MS = 25 * 60 * 1000;

export async function getSession(): Promise<string> {
  if (cachedSession && Date.now() - cachedSession.loadedAt < SESSION_REFRESH_MS) {
    return cachedSession.id;
  }
  const r = await http<SessionResponse>("/api/sandbox/session", { method: "POST" });
  cachedSession = { id: r.session_id, loadedAt: Date.now() };
  return r.session_id;
}

export async function callTool(tool: ToolName, args: object = {}): Promise<CallResponse> {
  const session_id = await getSession();
  try {
    return await http<CallResponse>("/api/sandbox/call", {
      method: "POST",
      body: JSON.stringify({ session_id, tool, args }),
    });
  } catch (e) {
    // session may have expired — refresh once and retry
    cachedSession = null;
    const retrySession = await getSession();
    return http<CallResponse>("/api/sandbox/call", {
      method: "POST",
      body: JSON.stringify({ session_id: retrySession, tool, args }),
    });
  }
}
```

### Task 3.3: Terminal output component

**Files:**
- Create: `apps/frontend/components/ui/Terminal.tsx`

- [ ] **Step 1: Write the component**

```tsx
"use client";

export interface TerminalLine {
  kind: "cmd" | "out" | "err" | "ok";
  text: string;
}

export function Terminal({ lines }: { lines: TerminalLine[] }) {
  return (
    <div className="bg-bg-deep border border-border rounded-md font-mono text-xs leading-relaxed p-4 min-h-[180px] max-h-[360px] overflow-y-auto">
      {lines.length === 0 ? (
        <p className="text-fg-faint">Click a tool above to see its output here.</p>
      ) : (
        lines.map((l, i) => (
          <div
            key={i}
            className={
              l.kind === "cmd" ? "text-fg-muted"
              : l.kind === "out" ? "text-fg"
              : l.kind === "ok"  ? "text-accent"
              : "text-red-400"
            }
          >
            {l.kind === "cmd" ? `$ ${l.text}` : l.text}
          </div>
        ))
      )}
    </div>
  );
}
```

### Task 3.4: Sandbox tab component

**Files:**
- Create: `apps/frontend/components/sections/SandboxTab.tsx`

- [ ] **Step 1: Write the component**

```tsx
"use client";

import { useState } from "react";
import { callTool, ToolName } from "@/lib/api";
import { Terminal, TerminalLine } from "@/components/ui/Terminal";

interface ToolButton {
  tool: ToolName;
  label: string;
  args: () => object;
}

const BUTTONS: ToolButton[] = [
  { tool: "agentspay_balance",     label: "balance()",                              args: () => ({}) },
  { tool: "agentspay_set_budget",  label: "set_budget(daily=25, per_call=1)",       args: () => ({ daily_usd: 25, per_call_usd: 1 }) },
  { tool: "agentspay_audit_log",   label: "audit_log(limit=5)",                     args: () => ({ limit: 5 }) },
  { tool: "agentspay_topup_info",  label: "topup_info()",                           args: () => ({}) },
];

export function SandboxTab() {
  const [lines, setLines] = useState<TerminalLine[]>([]);
  const [busy, setBusy] = useState<ToolName | null>(null);

  async function run(tool: ToolName, args: object, label: string) {
    setBusy(tool);
    setLines((prev) => [...prev, { kind: "cmd", text: label }]);
    try {
      const r = await callTool(tool, args);
      const payload = r.result?.content?.[0]?.text ?? JSON.stringify(r.result);
      setLines((prev) => [
        ...prev,
        { kind: "ok",  text: `✓ ${r.latency_ms}ms · sandbox` },
        { kind: "out", text: prettyJson(payload) },
      ]);
    } catch (e) {
      setLines((prev) => [...prev, { kind: "err", text: `✗ ${(e as Error).message}` }]);
    } finally {
      setBusy(null);
    }
  }

  return (
    <div>
      <p className="text-fg-muted text-sm mb-4">
        Call any of the 4 read/write tools below. Your tab gets an isolated
        SQLite ledger + keypair on the server. State resets after 30 minutes
        of inactivity.
      </p>
      <div className="flex flex-wrap gap-2 mb-4">
        {BUTTONS.map((b) => (
          <button
            key={b.tool}
            disabled={busy !== null}
            onClick={() => run(b.tool, b.args(), b.label)}
            className="bg-bg-elev border border-border rounded-md px-3 py-1.5 text-xs font-mono hover:bg-border-subtle transition disabled:opacity-40"
          >
            {busy === b.tool ? "…" : b.label}
          </button>
        ))}
      </div>
      <Terminal lines={lines} />
    </div>
  );
}

function prettyJson(raw: string): string {
  try {
    return JSON.stringify(JSON.parse(raw), null, 2);
  } catch {
    return raw;
  }
}
```

### Task 3.5: LiveDemo wrapper

**Files:**
- Create: `apps/frontend/components/sections/LiveDemo.tsx`

- [ ] **Step 1: Write the component**

```tsx
"use client";

import { useState } from "react";
import { SandboxTab } from "./SandboxTab";

type TabKey = "sandbox" | "devnet";

export function LiveDemo() {
  const [active, setActive] = useState<TabKey>("sandbox");

  return (
    <section id="demo" className="px-6 py-16 max-w-3xl mx-auto border-t border-border-subtle">
      <p className="text-xs uppercase tracking-[0.12em] text-accent mb-4 font-mono">
        §3 · LIVE DEMO
      </p>
      <h2 className="text-2xl sm:text-3xl font-semibold tracking-tight mb-6">
        Run it in your browser.
      </h2>

      <div className="flex gap-0 border-b border-border mb-6">
        <TabButton active={active === "sandbox"} onClick={() => setActive("sandbox")}>
          Sandbox
        </TabButton>
        <TabButton active={active === "devnet"} onClick={() => setActive("devnet")}>
          Devnet (real on-chain)
        </TabButton>
      </div>

      {active === "sandbox" ? <SandboxTab /> : (
        <p className="text-fg-muted text-sm">Devnet tab arrives in Phase 5.</p>
      )}
    </section>
  );
}

function TabButton({
  active, onClick, children,
}: {
  active: boolean; onClick: () => void; children: React.ReactNode;
}) {
  return (
    <button
      onClick={onClick}
      className={`px-4 py-2 text-sm border-b-2 -mb-px transition ${
        active ? "text-fg border-accent" : "text-fg-dim border-transparent hover:text-fg-muted"
      }`}
    >
      {children}
    </button>
  );
}
```

### Task 3.6: Mount LiveDemo on the page

**Files:**
- Modify: `apps/frontend/app/page.tsx`

- [ ] **Step 1: Edit `page.tsx`**

```tsx
import { Hero } from "@/components/sections/Hero";
import { Install } from "@/components/sections/Install";
import { LiveDemo } from "@/components/sections/LiveDemo";
import { Footer } from "@/components/sections/Footer";

export default function Home() {
  return (
    <main>
      <Hero />
      <Install />
      <LiveDemo />
      <Footer />
    </main>
  );
}
```

### Task 3.7: End-to-end manual verification

- [ ] **Step 1: Start the shim**

```bash
cargo build -p agentspay-mcp -p agentspay-web-shim
AGENTSPAY_MCP_BINARY=$PWD/target/debug/agentspay-mcp ./target/debug/agentspay-web-shim
```

- [ ] **Step 2: Start the frontend in another terminal**

```bash
pnpm --filter frontend dev
```

- [ ] **Step 3: Visit http://localhost:3000**

- Click the "balance()" button. The terminal should show:
  - a `$ balance()` line in muted grey
  - a `✓ ~50-150ms · sandbox` line in green
  - the JSON `{"available_usdc":"100.00","budget_remaining_today_usdc":"50.00",...}` pretty-printed
- Click "set_budget(...)" then "balance()" again — the `budget_remaining_today_usdc` field should now reflect the new cap (since same session reuses the same SQLite db).
- Click "audit_log(limit=5)" — should list the 2 audit rows from the set_budget call.

- [ ] **Step 4: Commit**

```bash
git add apps/frontend
git commit -m "feat(frontend): wire sandbox tab to /api/sandbox via shim subprocess"
```

**Milestone:** Phase 3 complete. The marketing demo of the sandbox flow is fully usable in dev. A visitor could already get the "wow" of seeing tool output in their browser.

---

## Phase 4 — Redis-backed sessions and rate limits (≈ 45 min)

**Goal of phase:** Replace the in-memory SessionStore with a Redis-backed one. Add per-IP and per-session rate-limit middleware. Tested via parallel curl.

### Task 4.1: Add Redis to the session store

**Files:**
- Modify: `services/web-shim/src/session.rs`

- [ ] **Step 1: Extend `SessionStore`**

Replace the `SessionStore` impl block in `services/web-shim/src/session.rs` with:

```rust
pub struct SessionStore {
    inner: RwLock<HashMap<String, Arc<Session>>>,
    ttl: Duration,
    // Phase 4: when Some, mirror create/get/expire into Redis so multiple
    // shim replicas (or restarts) see the same sessions.
    redis: Option<redis::aio::ConnectionManager>,
}

impl SessionStore {
    pub fn new_in_memory(ttl: Duration) -> Self {
        Self { inner: RwLock::new(HashMap::new()), ttl, redis: None }
    }

    pub async fn new_with_redis(ttl: Duration, url: &str) -> anyhow::Result<Self> {
        let client = redis::Client::open(url)?;
        let conn = redis::aio::ConnectionManager::new(client).await?;
        Ok(Self {
            inner: RwLock::new(HashMap::new()),
            ttl,
            redis: Some(conn),
        })
    }

    pub async fn create(&self) -> anyhow::Result<Arc<Session>> {
        let id = Uuid::new_v4().to_string();
        let tmpdir = TempDir::new()?;
        let path = tmpdir.path().to_path_buf();
        let session = Arc::new(Session {
            id: id.clone(),
            created_at: Instant::now(),
            tmpdir,
        });
        self.inner.write().await.insert(id.clone(), session.clone());
        if let Some(mut conn) = self.redis.clone() {
            let key = format!("session:{id}");
            // Store just the tmpdir path. The in-memory map keeps the TempDir
            // RAII guard so it survives until expiry.
            let _: redis::RedisResult<()> = redis::cmd("SET")
                .arg(&key)
                .arg(path.to_string_lossy().to_string())
                .arg("EX")
                .arg(self.ttl.as_secs())
                .query_async(&mut conn)
                .await;
        }
        Ok(session)
    }

    pub async fn get(&self, id: &str) -> Option<Arc<Session>> {
        let mut guard = self.inner.write().await;
        if let Some(s) = guard.get(id).cloned() {
            if s.created_at.elapsed() < self.ttl {
                return Some(s);
            }
            guard.remove(id);
        }
        // Future: when running multi-replica, look up Redis here and re-hydrate
        // from the tmpdir path. v0.1 is single-replica so we only need the
        // in-memory map for the RAII TempDir guard.
        None
    }

    pub async fn sweep(&self) -> (usize, usize) {
        let mut guard = self.inner.write().await;
        let before = guard.len();
        guard.retain(|_, s| s.created_at.elapsed() < self.ttl);
        let after = guard.len();
        (after, before - after)
    }
}
```

- [ ] **Step 2: Run cargo test (existing tests must still pass)**

Run: `cargo test -p agentspay-web-shim --lib session`
Expected: 2 passed.

- [ ] **Step 3: Wire the env var in `main.rs`**

In `services/web-shim/src/main.rs`, replace the `let sessions = ...` line with:

```rust
    let sessions = if let Some(url) = config.redis_url.as_deref() {
        std::sync::Arc::new(SessionStore::new_with_redis(config.session_ttl, url).await?)
    } else {
        std::sync::Arc::new(SessionStore::new_in_memory(config.session_ttl))
    };
```

- [ ] **Step 4: Verify build**

Run: `cargo check -p agentspay-web-shim`
Expected: PASS.

### Task 4.2: Rate-limit middleware

**Files:**
- Create: `services/web-shim/src/ratelimit.rs`
- Modify: `services/web-shim/src/main.rs`

- [ ] **Step 1: Write `ratelimit.rs`**

```rust
//! Sliding-window per-IP rate limiter backed by Redis (or an in-memory
//! fallback when `AGENTSPAY_REDIS_URL` is unset).

use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use tokio::sync::Mutex;

pub struct RateLimit {
    inner: Backend,
}

enum Backend {
    Memory(Mutex<HashMap<String, Vec<Instant>>>),
    Redis(redis::aio::ConnectionManager),
}

impl RateLimit {
    pub fn in_memory() -> Self {
        Self { inner: Backend::Memory(Mutex::new(HashMap::new())) }
    }

    pub async fn redis(url: &str) -> anyhow::Result<Self> {
        let client = redis::Client::open(url)?;
        let conn = redis::aio::ConnectionManager::new(client).await?;
        Ok(Self { inner: Backend::Redis(conn) })
    }

    /// Returns `Ok(())` if the call is allowed, `Err(retry_after_secs)` if
    /// the bucket is full.
    pub async fn check(
        &self,
        bucket: &str,
        max: u32,
        window: Duration,
    ) -> Result<(), u64> {
        match &self.inner {
            Backend::Memory(m) => {
                let now = Instant::now();
                let mut map = m.lock().await;
                let bucket_vec = map.entry(bucket.to_string()).or_default();
                bucket_vec.retain(|t| now.duration_since(*t) < window);
                if bucket_vec.len() as u32 >= max {
                    let oldest = bucket_vec[0];
                    let elapsed = now.duration_since(oldest);
                    let retry = window.saturating_sub(elapsed).as_secs().max(1);
                    return Err(retry);
                }
                bucket_vec.push(now);
                Ok(())
            }
            Backend::Redis(conn) => {
                let mut conn = conn.clone();
                let key = format!("ratelimit:{bucket}");
                let count: u32 = redis::cmd("INCR").arg(&key).query_async(&mut conn).await
                    .map_err(|_| window.as_secs())?;
                if count == 1 {
                    let _: redis::RedisResult<()> = redis::cmd("EXPIRE")
                        .arg(&key).arg(window.as_secs())
                        .query_async(&mut conn).await;
                }
                if count > max {
                    let ttl: i64 = redis::cmd("TTL").arg(&key)
                        .query_async(&mut conn).await.unwrap_or(window.as_secs() as i64);
                    return Err(ttl.max(1) as u64);
                }
                Ok(())
            }
        }
    }
}

pub type SharedRateLimit = Arc<RateLimit>;

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn allows_under_cap_then_rejects_over() {
        let rl = RateLimit::in_memory();
        for _ in 0..3 {
            rl.check("ip:1.2.3.4", 3, Duration::from_secs(60)).await.unwrap();
        }
        let err = rl.check("ip:1.2.3.4", 3, Duration::from_secs(60)).await.unwrap_err();
        assert!(err > 0 && err <= 60);
    }
}
```

- [ ] **Step 2: Wire it in `state.rs`**

```rust
//! Shared `AppState` cloned into every Axum handler.

use std::sync::Arc;

use crate::{config::Config, ratelimit::SharedRateLimit, session::SessionStore};

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub sessions: Arc<SessionStore>,
    pub http: reqwest::Client,
    pub ratelimit: SharedRateLimit,
}
```

- [ ] **Step 3: Wire it in `main.rs`**

Add `mod ratelimit;` near the other mod declarations. Replace the AppState construction in `main.rs`:

```rust
    let ratelimit = if let Some(url) = config.redis_url.as_deref() {
        std::sync::Arc::new(crate::ratelimit::RateLimit::redis(url).await?)
    } else {
        std::sync::Arc::new(crate::ratelimit::RateLimit::in_memory())
    };
    let state = AppState {
        config: config.clone(),
        sessions: sessions.clone(),
        http,
        ratelimit,
    };
```

### Task 4.3: Apply rate limits to the sandbox handlers

**Files:**
- Modify: `services/web-shim/src/handlers/sandbox.rs`

- [ ] **Step 1: Add the client-IP extractor + rate-limit checks**

Replace the contents of `services/web-shim/src/handlers/sandbox.rs` with:

```rust
//! Sandbox session + tool-call handlers.

use std::time::Duration;

use axum::{
    extract::{ConnectInfo, State},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::net::SocketAddr;

use crate::{
    error::ShimError,
    state::AppState,
    subprocess::{self, McpCall, NetworkMode},
};

const SESSION_RL_MAX: u32 = 30;
const SESSION_RL_WINDOW: Duration = Duration::from_secs(60);
const CALL_RL_MAX: u32 = 60;
const CALL_RL_WINDOW: Duration = Duration::from_secs(60);

#[derive(Debug, Serialize)]
pub struct SessionResponse {
    pub session_id: String,
    pub expires_in_secs: u64,
}

pub async fn create_session(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<AppState>,
) -> Result<Json<SessionResponse>, ShimError> {
    state
        .ratelimit
        .check(&format!("sandbox-session:{}", addr.ip()), SESSION_RL_MAX, SESSION_RL_WINDOW)
        .await
        .map_err(|retry_after_secs| ShimError::RateLimited { retry_after_secs })?;
    let session = state
        .sessions
        .create()
        .await
        .map_err(|e| ShimError::Internal(format!("session create: {e}")))?;
    Ok(Json(SessionResponse {
        session_id: session.id.clone(),
        expires_in_secs: state.config.session_ttl.as_secs(),
    }))
}

#[derive(Debug, Deserialize)]
pub struct CallRequest {
    pub session_id: String,
    pub tool: String,
    #[serde(default)]
    pub args: Value,
}

#[derive(Debug, Serialize)]
pub struct CallResponse {
    pub session_id: String,
    pub tool: String,
    pub result: Value,
    pub latency_ms: u128,
}

const ALLOWED_TOOLS: &[&str] = &[
    "agentspay_balance",
    "agentspay_pay_url",
    "agentspay_set_budget",
    "agentspay_audit_log",
    "agentspay_topup_info",
];

pub async fn call_tool(
    State(state): State<AppState>,
    Json(req): Json<CallRequest>,
) -> Result<Json<CallResponse>, ShimError> {
    if !ALLOWED_TOOLS.contains(&req.tool.as_str()) {
        return Err(ShimError::BadRequest(format!(
            "tool not allowed: {} (allowed: {})",
            req.tool,
            ALLOWED_TOOLS.join(", ")
        )));
    }

    state
        .ratelimit
        .check(&format!("sandbox-call:{}", req.session_id), CALL_RL_MAX, CALL_RL_WINDOW)
        .await
        .map_err(|retry_after_secs| ShimError::RateLimited { retry_after_secs })?;

    let session = state
        .sessions
        .get(&req.session_id)
        .await
        .ok_or(ShimError::SessionGone)?;

    let start = std::time::Instant::now();
    let result = subprocess::run(McpCall {
        binary: &state.config.mcp_binary,
        network: NetworkMode::Sandbox,
        keypair_path: &session.keypair_path(),
        db_url: &session.db_url(),
        tool: &req.tool,
        args: req.args,
        timeout: Duration::from_secs(10),
    })
    .await?;

    Ok(Json(CallResponse {
        session_id: req.session_id,
        tool: req.tool,
        result,
        latency_ms: start.elapsed().as_millis(),
    }))
}
```

- [ ] **Step 2: Add `ConnectInfo` to the listener in `main.rs`**

Replace the last few lines of `main()` with:

```rust
    let addr = config.listen_addr;
    tracing::info!(%addr, mcp_binary = ?config.mcp_binary, "agentspay-web-shim listening");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>()).await?;
    Ok(())
```

Add the import: `use std::net::SocketAddr;` near the top of `main.rs`.

- [ ] **Step 3: Run cargo test**

Run: `cargo test -p agentspay-web-shim`
Expected: 3 passed.

- [ ] **Step 4: Manual rate-limit smoke**

Start shim + run 31 parallel session-creates from the same IP:

```bash
AGENTSPAY_MCP_BINARY=$PWD/target/debug/agentspay-mcp ./target/debug/agentspay-web-shim &
SHIM_PID=$!
sleep 1
for i in $(seq 1 31); do
  curl -s -o /dev/null -w "%{http_code} " -X POST http://localhost:8080/api/sandbox/session
done
echo
kill $SHIM_PID
```

Expected: 30× `200`, 1× `429`.

- [ ] **Step 5: Commit**

```bash
git add services/web-shim
git commit -m "feat(web-shim): redis-backed sessions and per-IP rate limits"
```

**Milestone:** Phase 4 complete. Shim is production-grade for the sandbox flow.

---

## Phase 5 — Devnet wallet + trigger endpoint (≈ 60 min)

**Goal of phase:** `POST /api/devnet/trigger` produces a real on-chain Solana devnet tx (signed by the shim's funded wallet) and returns the Solscan URL. Rate limit: 1 per IP per hour.

### Task 5.1: Wallet status endpoint

**Files:**
- Create: `services/web-shim/src/devnet_wallet.rs`
- Create: `services/web-shim/src/handlers/devnet.rs`

- [ ] **Step 1: Write `devnet_wallet.rs`**

```rust
//! Live status check of the shim's devnet wallet (SOL + USDC balances)
//! via the public Solana devnet RPC.

use std::path::Path;

use serde::Serialize;

const RPC_URL: &str = "https://api.devnet.solana.com";
const USDC_DEVNET_MINT: &str = "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU";
const MIN_SOL: f64 = 0.05;
const MIN_USDC: f64 = 2.0;

#[derive(Debug, Clone, Serialize)]
pub struct WalletStatus {
    pub pubkey: String,
    pub sol_balance: f64,
    pub usdc_balance: f64,
    pub healthy: bool,
    pub message: Option<String>,
}

pub async fn read_pubkey(path: &Path) -> anyhow::Result<String> {
    let raw = tokio::fs::read_to_string(path).await?;
    let bytes: Vec<u8> = serde_json::from_str(&raw)?;
    if bytes.len() != 64 {
        anyhow::bail!("keypair file must contain 64 bytes, got {}", bytes.len());
    }
    // Solana ed25519 keypair JSON = secret(32) || public(32). Encode last 32 as base58.
    Ok(bs58::encode(&bytes[32..]).into_string())
}

pub async fn fetch_status(
    http: &reqwest::Client,
    pubkey: &str,
) -> anyhow::Result<WalletStatus> {
    let sol_lamports = rpc_call_u64(http, "getBalance", serde_json::json!([pubkey])).await?;
    let sol_balance = sol_lamports as f64 / 1_000_000_000.0;

    let usdc_balance = fetch_usdc_balance(http, pubkey).await.unwrap_or(0.0);

    let mut warns = Vec::new();
    if sol_balance < MIN_SOL { warns.push(format!("SOL {sol_balance:.4} < {MIN_SOL}")); }
    if usdc_balance < MIN_USDC { warns.push(format!("USDC {usdc_balance:.2} < {MIN_USDC}")); }
    let healthy = warns.is_empty();
    let message = if healthy { None } else { Some(warns.join("; ")) };

    Ok(WalletStatus {
        pubkey: pubkey.to_string(),
        sol_balance,
        usdc_balance,
        healthy,
        message,
    })
}

async fn rpc_call_u64(
    http: &reqwest::Client,
    method: &str,
    params: serde_json::Value,
) -> anyhow::Result<u64> {
    let resp: serde_json::Value = http
        .post(RPC_URL)
        .json(&serde_json::json!({
            "jsonrpc":"2.0","id":1,"method":method,"params":params
        }))
        .send()
        .await?
        .json()
        .await?;
    resp.get("result")
        .and_then(|r| r.get("value"))
        .and_then(|v| v.as_u64())
        .ok_or_else(|| anyhow::anyhow!("rpc {method} returned {resp}"))
}

async fn fetch_usdc_balance(
    http: &reqwest::Client,
    pubkey: &str,
) -> anyhow::Result<f64> {
    let resp: serde_json::Value = http
        .post(RPC_URL)
        .json(&serde_json::json!({
            "jsonrpc":"2.0","id":1,"method":"getTokenAccountsByOwner",
            "params":[pubkey,{"mint":USDC_DEVNET_MINT},{"encoding":"jsonParsed"}]
        }))
        .send()
        .await?
        .json()
        .await?;
    let accounts = resp.pointer("/result/value").and_then(|v| v.as_array());
    let total: f64 = accounts.map(|a| a.iter()).into_iter().flatten()
        .filter_map(|acc| {
            acc.pointer("/account/data/parsed/info/tokenAmount/uiAmountString")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse::<f64>().ok())
        })
        .sum();
    Ok(total)
}
```

- [ ] **Step 2: Write the handlers in `handlers/devnet.rs`**

```rust
//! Devnet handlers: wallet status + one-click on-chain trigger.

use std::{sync::Arc, time::Duration};

use axum::{extract::{ConnectInfo, State}, Json};
use serde::Serialize;
use serde_json::{json, Value};
use std::net::SocketAddr;

use crate::{
    devnet_wallet::{self, WalletStatus},
    error::ShimError,
    latest_tx::LatestTx,
    state::AppState,
    subprocess::{self, McpCall, NetworkMode},
};

const TRIGGER_RL_MAX: u32 = 1;
const TRIGGER_RL_WINDOW: Duration = Duration::from_secs(3600);
const STATUS_RL_MAX: u32 = 60;
const STATUS_RL_WINDOW: Duration = Duration::from_secs(60);
const SYMBOLS: &[&str] = &["AAPL", "MSFT", "GOOG", "NVDA", "AMZN"];

pub async fn wallet_status(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<AppState>,
) -> Result<Json<WalletStatus>, ShimError> {
    state.ratelimit.check(
        &format!("devnet-status:{}", addr.ip()), STATUS_RL_MAX, STATUS_RL_WINDOW,
    ).await.map_err(|retry_after_secs| ShimError::RateLimited { retry_after_secs })?;

    let pubkey = devnet_wallet::read_pubkey(&state.config.devnet_wallet_path)
        .await
        .map_err(|e| ShimError::Internal(format!("read wallet: {e}")))?;
    let status = devnet_wallet::fetch_status(&state.http, &pubkey)
        .await
        .map_err(|e| ShimError::Internal(format!("fetch status: {e}")))?;
    Ok(Json(status))
}

#[derive(Debug, Serialize)]
pub struct TriggerResponse {
    pub signature: String,
    pub explorer_url: String,
    pub symbol: String,
    pub amount_charged_usdc: String,
    pub body: Value,
    pub latency_ms: u128,
}

pub async fn trigger(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<AppState>,
) -> Result<Json<TriggerResponse>, ShimError> {
    state.ratelimit.check(
        &format!("devnet-trigger:{}", addr.ip()), TRIGGER_RL_MAX, TRIGGER_RL_WINDOW,
    ).await.map_err(|retry_after_secs| ShimError::RateLimited { retry_after_secs })?;

    let pubkey = devnet_wallet::read_pubkey(&state.config.devnet_wallet_path)
        .await
        .map_err(|e| ShimError::Internal(format!("read wallet: {e}")))?;
    let status = devnet_wallet::fetch_status(&state.http, &pubkey)
        .await
        .map_err(|e| ShimError::Internal(format!("fetch status: {e}")))?;
    if !status.healthy {
        tracing::warn!(message = ?status.message, "devnet wallet drained");
        return Err(ShimError::WalletDrained);
    }

    let symbol = pick_symbol();
    let url = format!("{}/real-quote/{}", state.config.paid_endpoint_url, symbol);
    let db_url = format!("sqlite://{}?mode=rwc", state.config.devnet_ledger_path.display());

    let start = std::time::Instant::now();
    let mcp_result = subprocess::run(McpCall {
        binary: &state.config.mcp_binary,
        network: NetworkMode::SolanaDevnet,
        keypair_path: &state.config.devnet_wallet_path,
        db_url: &db_url,
        tool: "agentspay_pay_url",
        args: json!({"url": url, "max_amount_usdc": "0.50"}),
        timeout: Duration::from_secs(20),
    }).await?;

    let payload_text = mcp_result.pointer("/content/0/text").and_then(Value::as_str)
        .ok_or_else(|| ShimError::MalformedMcp("missing content[0].text".into()))?;
    let payload: Value = serde_json::from_str(payload_text)
        .map_err(|e| ShimError::MalformedMcp(format!("inner json: {e}")))?;

    let signature = payload.get("transaction").and_then(Value::as_str)
        .ok_or_else(|| ShimError::MalformedMcp("missing transaction".into()))?
        .to_string();
    let explorer_url = payload.get("explorer_url").and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| format!("https://solscan.io/tx/{signature}?cluster=devnet"));
    let amount_charged_usdc = payload.get("amount_charged_usdc").and_then(Value::as_str)
        .unwrap_or("0.10").to_string();
    let body_str = payload.get("body").and_then(Value::as_str).unwrap_or("{}");
    let body: Value = serde_json::from_str(body_str).unwrap_or(Value::Null);

    let latency_ms = start.elapsed().as_millis();

    // Update the hero counter cache.
    state.latest_tx.set(LatestTx {
        signature: signature.clone(),
        amount_usdc: amount_charged_usdc.clone(),
        explorer_url: explorer_url.clone(),
        at: std::time::SystemTime::now(),
    }).await;

    Ok(Json(TriggerResponse {
        signature, explorer_url, symbol: symbol.into(),
        amount_charged_usdc, body, latency_ms,
    }))
}

fn pick_symbol() -> &'static str {
    let i = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default().as_secs() as usize) % SYMBOLS.len();
    SYMBOLS[i]
}

// Convenience constructor for callers that want to share an Arc<AppState>.
#[allow(dead_code)]
pub fn _share(state: AppState) -> Arc<AppState> { Arc::new(state) }
```

### Task 5.2: Latest-tx cache for the hero counter

**Files:**
- Create: `services/web-shim/src/latest_tx.rs`
- Create: `services/web-shim/src/handlers/stats.rs`
- Modify: `services/web-shim/src/state.rs`

- [ ] **Step 1: Write `latest_tx.rs`**

```rust
//! In-memory cache of the most recent successful devnet tx, served to the
//! hero counter at /api/stats/latest-tx.

use std::sync::Arc;
use std::time::SystemTime;

use serde::Serialize;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize)]
pub struct LatestTx {
    pub signature: String,
    pub amount_usdc: String,
    pub explorer_url: String,
    #[serde(skip)]
    pub at: SystemTime,
}

#[derive(Debug, Serialize)]
pub struct LatestTxView {
    pub signature: String,
    pub amount_usdc: String,
    pub explorer_url: String,
    pub age_seconds: u64,
}

#[derive(Default)]
pub struct LatestTxCache {
    inner: RwLock<Option<LatestTx>>,
}

pub type SharedLatestTx = Arc<LatestTxCache>;

impl LatestTxCache {
    pub fn new() -> Self { Self::default() }
    pub async fn set(&self, tx: LatestTx) { *self.inner.write().await = Some(tx); }
    pub async fn get(&self) -> Option<LatestTxView> {
        let guard = self.inner.read().await;
        let tx = guard.as_ref()?;
        let age = SystemTime::now().duration_since(tx.at).ok()?.as_secs();
        // Hide entries older than 24h.
        if age > 24 * 3600 { return None; }
        Some(LatestTxView {
            signature: tx.signature.clone(),
            amount_usdc: tx.amount_usdc.clone(),
            explorer_url: tx.explorer_url.clone(),
            age_seconds: age,
        })
    }
}
```

- [ ] **Step 2: Write `handlers/stats.rs`**

```rust
use axum::{extract::State, Json};
use serde_json::{json, Value};

use crate::state::AppState;

pub async fn latest_tx(State(state): State<AppState>) -> Json<Value> {
    match state.latest_tx.get().await {
        Some(v) => Json(serde_json::to_value(v).unwrap_or(Value::Null)),
        None => Json(json!({"signature":null,"age_seconds":null})),
    }
}
```

- [ ] **Step 3: Extend `state.rs`**

```rust
//! Shared `AppState` cloned into every Axum handler.

use std::sync::Arc;

use crate::{
    config::Config,
    latest_tx::SharedLatestTx,
    ratelimit::SharedRateLimit,
    session::SessionStore,
};

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub sessions: Arc<SessionStore>,
    pub http: reqwest::Client,
    pub ratelimit: SharedRateLimit,
    pub latest_tx: SharedLatestTx,
}
```

### Task 5.3: Wire new modules + routes in `main.rs`

**Files:**
- Modify: `services/web-shim/src/main.rs`
- Modify: `services/web-shim/src/handlers/mod.rs`

- [ ] **Step 1: Add modules in `main.rs` and `handlers/mod.rs`**

`services/web-shim/src/handlers/mod.rs`:

```rust
pub mod devnet;
pub mod health;
pub mod sandbox;
pub mod stats;
```

`services/web-shim/src/main.rs`: add `mod devnet_wallet; mod latest_tx;` near the other `mod` declarations, then update the router:

```rust
    let app = Router::new()
        .route("/api/health", get(health::health))
        .route("/api/sandbox/session", post(sandbox::create_session))
        .route("/api/sandbox/call", post(sandbox::call_tool))
        .route("/api/devnet/trigger", post(devnet::trigger))
        .route("/api/devnet/wallet-status", get(devnet::wallet_status))
        .route("/api/stats/latest-tx", get(stats::latest_tx))
        .with_state(state);
```

And inject the latest_tx cache into AppState:

```rust
    let latest_tx = std::sync::Arc::new(crate::latest_tx::LatestTxCache::new());
    let state = AppState {
        config: config.clone(),
        sessions: sessions.clone(),
        http,
        ratelimit,
        latest_tx,
    };
```

- [ ] **Step 2: Run cargo check + cargo test**

Run: `cargo check -p agentspay-web-shim && cargo test -p agentspay-web-shim`
Expected: PASS, 3 tests OK.

- [ ] **Step 3: Manual devnet smoke test**

Use the **existing funded** keypair from earlier work (`~/.agentspay/keypair.json`):

```bash
AGENTSPAY_MCP_BINARY=$PWD/target/debug/agentspay-mcp \
AGENTSPAY_DEVNET_WALLET_PATH=$HOME/.agentspay/keypair.json \
AGENTSPAY_DEVNET_LEDGER_PATH=$HOME/.agentspay/agentspay-mcp.db \
AGENTSPAY_PAID_ENDPOINT_URL=http://localhost:3001 \
./target/debug/agentspay-web-shim &
SHIM_PID=$!

# Start the paid endpoint in parallel (with a known persistent provider).
./target/debug/agentspay-paid-endpoint-demo &
PE_PID=$!
sleep 2

curl -s http://localhost:8080/api/devnet/wallet-status | jq
curl -s -X POST http://localhost:8080/api/devnet/trigger | jq
curl -s http://localhost:8080/api/stats/latest-tx | jq

# Second trigger should 429
curl -s -i -X POST http://localhost:8080/api/devnet/trigger | head -5

kill $SHIM_PID $PE_PID
```

Expected:
- wallet-status returns `healthy: true` with non-zero SOL + USDC.
- first trigger returns a `signature` + `explorer_url`. Open the Solscan link in your browser — real on-chain tx.
- latest-tx now returns the same signature with `age_seconds < 60`.
- second trigger returns HTTP 429 with a `retry-after: 3599` header.

- [ ] **Step 4: Commit**

```bash
git add services/web-shim
git commit -m "feat(web-shim): devnet trigger + wallet status + latest-tx cache"
```

**Milestone:** Phase 5 complete. The shim can drive a real on-chain devnet tx from a single HTTP POST.

---

## Phase 6 — Devnet tab + hero counter in the browser (≈ 60 min)

**Goal of phase:** The Devnet tab in the website wires the `/api/devnet/*` endpoints. The hero shows a live counter of the most recent on-chain tx.

### Task 6.1: SolscanLink + LiveTxBadge UI primitives

**Files:**
- Create: `apps/frontend/components/ui/SolscanLink.tsx`
- Create: `apps/frontend/components/ui/LiveTxBadge.tsx`
- Create: `apps/frontend/lib/live-tx.ts`

- [ ] **Step 1: Write `SolscanLink.tsx`**

```tsx
export function SolscanLink({ signature }: { signature: string }) {
  const short = `${signature.slice(0, 4)}…${signature.slice(-4)}`;
  return (
    <a
      href={`https://solscan.io/tx/${signature}?cluster=devnet`}
      target="_blank"
      rel="noreferrer"
      className="inline-flex items-center gap-2 font-mono text-xs text-accent hover:underline"
    >
      <span className="w-1.5 h-1.5 rounded-full bg-accent" />
      <span>solscan {short}</span>
    </a>
  );
}
```

- [ ] **Step 2: Write `live-tx.ts`**

```ts
export interface LatestTxView {
  signature: string | null;
  amount_usdc?: string;
  explorer_url?: string;
  age_seconds?: number | null;
}

export async function fetchLatestTx(): Promise<LatestTxView> {
  const r = await fetch("/api/stats/latest-tx", { cache: "no-store" });
  if (!r.ok) return { signature: null };
  return (await r.json()) as LatestTxView;
}
```

- [ ] **Step 3: Write `LiveTxBadge.tsx`**

```tsx
"use client";

import { useEffect, useState } from "react";
import { fetchLatestTx, LatestTxView } from "@/lib/live-tx";
import { SolscanLink } from "./SolscanLink";

const POLL_MS = 15_000;

export function LiveTxBadge() {
  const [tx, setTx] = useState<LatestTxView | null>(null);

  useEffect(() => {
    let stop = false;
    async function tick() {
      const r = await fetchLatestTx();
      if (!stop) setTx(r);
    }
    void tick();
    const id = setInterval(tick, POLL_MS);
    return () => {
      stop = true;
      clearInterval(id);
    };
  }, []);

  if (!tx?.signature) {
    return (
      <div className="text-xs font-mono text-fg-faint">
        No live tx in the last 24h — be the first ↓
      </div>
    );
  }
  const age = tx.age_seconds ?? 0;
  const ageStr = age < 60 ? `${age}s ago` : age < 3600 ? `${Math.floor(age / 60)}m ago` : `${Math.floor(age / 3600)}h ago`;
  return (
    <div className="text-xs font-mono text-accent flex items-center gap-3">
      <SolscanLink signature={tx.signature} />
      <span>· {ageStr}</span>
      {tx.amount_usdc && <span>· {tx.amount_usdc} USDC</span>}
    </div>
  );
}
```

### Task 6.2: DevnetTab component

**Files:**
- Create: `apps/frontend/components/sections/DevnetTab.tsx`
- Modify: `apps/frontend/components/sections/LiveDemo.tsx`
- Modify: `apps/frontend/lib/api.ts`

- [ ] **Step 1: Extend `lib/api.ts` with devnet helpers**

Append to `apps/frontend/lib/api.ts`:

```ts
export interface DevnetWalletStatus {
  pubkey: string;
  sol_balance: number;
  usdc_balance: number;
  healthy: boolean;
  message: string | null;
}

export interface DevnetTriggerResponse {
  signature: string;
  explorer_url: string;
  symbol: string;
  amount_charged_usdc: string;
  body: unknown;
  latency_ms: number;
}

export async function fetchWalletStatus(): Promise<DevnetWalletStatus> {
  return http<DevnetWalletStatus>("/api/devnet/wallet-status", { method: "GET" });
}

export async function triggerDevnet(): Promise<DevnetTriggerResponse> {
  return http<DevnetTriggerResponse>("/api/devnet/trigger", { method: "POST" });
}
```

- [ ] **Step 2: Write `DevnetTab.tsx`**

```tsx
"use client";

import { useEffect, useState } from "react";
import { fetchWalletStatus, triggerDevnet, DevnetWalletStatus, DevnetTriggerResponse } from "@/lib/api";
import { Terminal, TerminalLine } from "@/components/ui/Terminal";
import { SolscanLink } from "@/components/ui/SolscanLink";

export function DevnetTab() {
  const [status, setStatus] = useState<DevnetWalletStatus | null>(null);
  const [busy, setBusy] = useState(false);
  const [lines, setLines] = useState<TerminalLine[]>([]);
  const [result, setResult] = useState<DevnetTriggerResponse | null>(null);

  useEffect(() => {
    fetchWalletStatus().then(setStatus).catch(() => setStatus(null));
  }, []);

  async function trigger() {
    setBusy(true);
    setResult(null);
    setLines((prev) => [...prev, { kind: "cmd", text: "agentspay_pay_url(real-quote/...)" }]);
    try {
      const r = await triggerDevnet();
      setResult(r);
      setLines((prev) => [
        ...prev,
        { kind: "ok",  text: `✓ ${r.latency_ms}ms · solana-devnet · ${r.amount_charged_usdc} USDC` },
        { kind: "out", text: typeof r.body === "string" ? r.body : JSON.stringify(r.body, null, 2) },
      ]);
    } catch (e) {
      setLines((prev) => [...prev, { kind: "err", text: `✗ ${(e as Error).message}` }]);
    } finally {
      setBusy(false);
    }
  }

  return (
    <div>
      <p className="text-fg-muted text-sm mb-4">
        One click triggers a <strong>real on-chain transaction</strong> on Solana devnet,
        signed by a server-controlled funded wallet. Rate limited to 1 per IP per hour
        to keep the faucet from draining.
      </p>

      {status && (
        <div className="text-xs text-fg-dim mb-3 font-mono">
          demo wallet: {status.pubkey.slice(0, 4)}…{status.pubkey.slice(-4)} ·
          {" "}{status.sol_balance.toFixed(3)} SOL ·
          {" "}{status.usdc_balance.toFixed(2)} USDC ·
          {" "}<span className={status.healthy ? "text-accent" : "text-red-400"}>
            {status.healthy ? "healthy" : status.message ?? "drained"}
          </span>
        </div>
      )}

      <button
        disabled={busy || (status && !status.healthy) || false}
        onClick={trigger}
        className="bg-accent text-black rounded-md px-4 py-2 text-sm font-semibold hover:opacity-90 transition disabled:opacity-40 mb-4"
      >
        {busy ? "Signing + broadcasting…" : "Trigger a real on-chain tx"}
      </button>

      {result && (
        <div className="mb-4">
          <SolscanLink signature={result.signature} />
        </div>
      )}

      <Terminal lines={lines} />
    </div>
  );
}
```

- [ ] **Step 3: Mount DevnetTab in `LiveDemo.tsx`**

In `apps/frontend/components/sections/LiveDemo.tsx`, replace the placeholder:

```tsx
{active === "sandbox" ? <SandboxTab /> : <DevnetTab />}
```

Add the import: `import { DevnetTab } from "./DevnetTab";`.

### Task 6.3: Mount LiveTxBadge in the hero

**Files:**
- Modify: `apps/frontend/components/sections/Hero.tsx`

- [ ] **Step 1: Replace the static counter line with the live component**

Replace the last `<div>` of the hero with:

```tsx
      <div className="mt-10">
        <LiveTxBadge />
      </div>
```

Add the import: `import { LiveTxBadge } from "@/components/ui/LiveTxBadge";`.

### Task 6.4: End-to-end browser verification

- [ ] **Step 1: Start all three processes**

```bash
# t1
cargo build -p agentspay-mcp -p agentspay-web-shim -p agentspay-paid-endpoint-demo
./target/debug/agentspay-paid-endpoint-demo &
PE_PID=$!

# t2
AGENTSPAY_MCP_BINARY=$PWD/target/debug/agentspay-mcp \
AGENTSPAY_DEVNET_WALLET_PATH=$HOME/.agentspay/keypair.json \
AGENTSPAY_DEVNET_LEDGER_PATH=$HOME/.agentspay/agentspay-mcp.db \
AGENTSPAY_PAID_ENDPOINT_URL=http://localhost:3001 \
./target/debug/agentspay-web-shim &
SHIM_PID=$!

# t3
pnpm --filter frontend dev
```

- [ ] **Step 2: Walk the visitor flow at http://localhost:3000**

- Hero shows "No live tx in the last 24h — be the first ↓" (since we never triggered yet) OR a real Solscan link if you already have one cached.
- Click "See live devnet demo →" — scrolls to §3.
- Switch to the **Devnet** tab. The wallet status line appears.
- Click "Trigger a real on-chain tx". A new Solscan link appears within ~3 seconds, the terminal shows the AAPL/MSFT/... quote body.
- Refresh the page. The hero now shows the live tx badge with `~5s ago`.
- Click trigger again. The button shows a red "rate limited" message inside the terminal.

- [ ] **Step 3: Commit**

```bash
git add apps/frontend
git commit -m "feat(frontend): devnet tab + hero live-tx counter"
```

**Milestone:** Phase 6 complete. The full magic of the demo works in your dev environment.

---

## Phase 7 — How it works + Why sections (≈ 30 min)

**Goal of phase:** §4 and §5 of the page are filled in. Static content. Polish pass on hero/install copy.

### Task 7.1: HowItWorks component

**Files:**
- Create: `apps/frontend/components/sections/HowItWorks.tsx`

- [ ] **Step 1: Write the component**

```tsx
const CARDS = [
  {
    n: "1",
    title: "MCP host calls a tool",
    body: "Claude Code, Cursor, or Cline invokes one of the 5 tools over MCP stdio JSON-RPC. Your agent talks to the local binary, never to a hosted service it doesn't control.",
  },
  {
    n: "2",
    title: "Budget check before signature",
    body: "Per-call cap and rolling-daily cap are enforced before any keypair touches the transaction. A tokio Mutex serializes the critical section so two parallel calls can't both pass against a stale view.",
  },
  {
    n: "3",
    title: "On-chain settlement",
    body: "SPL transfer_checked signed locally, base64-encoded, sent in the X-Payment header, settled by the upstream x402 server through Solana devnet RPC. The signature comes back in X-Payment-Response.",
  },
];

export function HowItWorks() {
  return (
    <section className="px-6 py-16 max-w-3xl mx-auto border-t border-border-subtle">
      <p className="text-xs uppercase tracking-[0.12em] text-accent mb-4 font-mono">
        §4 · HOW IT WORKS
      </p>
      <h2 className="text-2xl sm:text-3xl font-semibold tracking-tight mb-6">
        Three steps between a tool call and an on-chain receipt.
      </h2>
      <div className="grid sm:grid-cols-3 gap-4">
        {CARDS.map((c) => (
          <div key={c.n} className="bg-bg-elev border border-border rounded-md p-4">
            <div className="text-xs font-mono text-accent mb-2">{c.n}</div>
            <h3 className="font-semibold text-fg text-sm mb-2">{c.title}</h3>
            <p className="text-fg-muted text-xs leading-relaxed">{c.body}</p>
          </div>
        ))}
      </div>
    </section>
  );
}
```

### Task 7.2: Why component

**Files:**
- Create: `apps/frontend/components/sections/Why.tsx`

- [ ] **Step 1: Write the component**

```tsx
const CARDS = [
  {
    title: "vs hardcoded API key",
    body: "Caps + audit trail. Your agent literally cannot drain your OpenAI bill — pay_url refuses any call that would push the day's spend above your budget.",
  },
  {
    title: "vs Stripe MPP",
    body: "Self-custodial. Open source. Solana, not card rails. The whole stack runs on your laptop — no SaaS dependency, no merchant onboarding.",
  },
  {
    title: "vs Coinbase CDP direct",
    body: "Five MCP tools, not forty REST endpoints. No API key. Drops into Claude Code with one command. The CDP facilitator is still an option under the hood when you want it.",
  },
];

export function Why() {
  return (
    <section className="px-6 py-16 max-w-3xl mx-auto border-t border-border-subtle">
      <p className="text-xs uppercase tracking-[0.12em] text-accent mb-4 font-mono">
        §5 · WHY
      </p>
      <h2 className="text-2xl sm:text-3xl font-semibold tracking-tight mb-6">
        Built for indie devs in Claude Code, not enterprise procurement.
      </h2>
      <div className="grid sm:grid-cols-3 gap-4">
        {CARDS.map((c) => (
          <div key={c.title} className="bg-bg-elev border border-border rounded-md p-4">
            <h3 className="font-semibold text-fg text-sm mb-2">{c.title}</h3>
            <p className="text-fg-muted text-xs leading-relaxed">{c.body}</p>
          </div>
        ))}
      </div>
    </section>
  );
}
```

### Task 7.3: Compose all sections

**Files:**
- Modify: `apps/frontend/app/page.tsx`

- [ ] **Step 1: Edit the page**

```tsx
import { Hero } from "@/components/sections/Hero";
import { Install } from "@/components/sections/Install";
import { LiveDemo } from "@/components/sections/LiveDemo";
import { HowItWorks } from "@/components/sections/HowItWorks";
import { Why } from "@/components/sections/Why";
import { Footer } from "@/components/sections/Footer";

export default function Home() {
  return (
    <main>
      <Hero />
      <Install />
      <LiveDemo />
      <HowItWorks />
      <Why />
      <Footer />
    </main>
  );
}
```

- [ ] **Step 2: Visual check**

Run: `pnpm --filter frontend dev`
Open: http://localhost:3000

Walk the full page top-to-bottom. Adjust copy/spacing only if something is clearly broken — this is not the polish phase.

- [ ] **Step 3: Commit**

```bash
git add apps/frontend
git commit -m "feat(frontend): how-it-works + why sections"
```

**Milestone:** Phase 7 complete. The site is feature-complete in dev mode.

---

## Phase 8 — Docker stack (≈ 90 min)

**Goal of phase:** `docker compose -f docker/docker-compose.yml up -d` brings up all 5 services. The site is reachable at `http://localhost` with Caddy TLS in production mode (or plain HTTP in local override).

### Task 8.1: Dockerfile for the shim (builds 2 binaries)

**Files:**
- Create: `docker/Dockerfile.shim`

- [ ] **Step 1: Write the Dockerfile**

```dockerfile
# syntax=docker/dockerfile:1.7
FROM rust:1.83-slim AS builder
WORKDIR /build
RUN apt-get update && apt-get install -y --no-install-recommends \
      pkg-config libssl-dev ca-certificates protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

# Pre-copy manifests to enable layer caching.
COPY Cargo.toml Cargo.lock ./
COPY rust-toolchain.toml ./
COPY packages packages
COPY services services
COPY examples examples

# Build only the two binaries the shim runtime needs.
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/build/target \
    cargo build --release -p agentspay-web-shim -p agentspay-mcp \
 && cp target/release/agentspay-web-shim /usr/local/bin/agentspay-web-shim \
 && cp target/release/agentspay-mcp /usr/local/bin/agentspay-mcp

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends \
      ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/bin/agentspay-web-shim /usr/local/bin/agentspay-web-shim
COPY --from=builder /usr/local/bin/agentspay-mcp /usr/local/bin/agentspay-mcp
ENV AGENTSPAY_MCP_BINARY=/usr/local/bin/agentspay-mcp \
    AGENTSPAY_SHIM_LISTEN_ADDR=0.0.0.0:8080 \
    RUST_LOG=agentspay_web_shim=info,tower_http=info
EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/agentspay-web-shim"]
```

### Task 8.2: Dockerfile for the paid-endpoint

**Files:**
- Create: `docker/Dockerfile.paid-endpoint`

- [ ] **Step 1: Write the Dockerfile**

```dockerfile
# syntax=docker/dockerfile:1.7
FROM rust:1.83-slim AS builder
WORKDIR /build
RUN apt-get update && apt-get install -y --no-install-recommends \
      pkg-config libssl-dev ca-certificates protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*
COPY Cargo.toml Cargo.lock ./
COPY rust-toolchain.toml ./
COPY packages packages
COPY services services
COPY examples examples
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/build/target \
    cargo build --release -p agentspay-paid-endpoint-demo \
 && cp target/release/agentspay-paid-endpoint-demo /usr/local/bin/agentspay-paid-endpoint-demo

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends \
      ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/bin/agentspay-paid-endpoint-demo /usr/local/bin/agentspay-paid-endpoint-demo
ENV RUST_LOG=agentspay_paid_endpoint_demo=info
EXPOSE 3001
ENTRYPOINT ["/usr/local/bin/agentspay-paid-endpoint-demo"]
```

### Task 8.3: Dockerfile for the web

**Files:**
- Create: `docker/Dockerfile.web`

- [ ] **Step 1: Write the Dockerfile**

```dockerfile
# syntax=docker/dockerfile:1.7
FROM node:22-slim AS deps
WORKDIR /app
RUN corepack enable
COPY package.json pnpm-workspace.yaml pnpm-lock.yaml* .npmrc ./
COPY apps/frontend/package.json apps/frontend/package.json
RUN pnpm install --frozen-lockfile --filter frontend

FROM node:22-slim AS builder
WORKDIR /app
RUN corepack enable
COPY --from=deps /app/node_modules node_modules
COPY --from=deps /app/apps/frontend/node_modules apps/frontend/node_modules
COPY package.json pnpm-workspace.yaml pnpm-lock.yaml* .npmrc ./
COPY apps/frontend apps/frontend
ENV NEXT_TELEMETRY_DISABLED=1
RUN pnpm --filter frontend build

FROM node:22-slim
WORKDIR /app
ENV NODE_ENV=production
ENV NEXT_TELEMETRY_DISABLED=1
ENV PORT=3000 HOSTNAME=0.0.0.0
COPY --from=builder /app/apps/frontend/.next/standalone ./
COPY --from=builder /app/apps/frontend/.next/static ./apps/frontend/.next/static
COPY --from=builder /app/apps/frontend/public ./apps/frontend/public
EXPOSE 3000
CMD ["node", "apps/frontend/server.js"]
```

### Task 8.4: Caddyfile + compose stack

**Files:**
- Create: `docker/Caddyfile`
- Create: `docker/docker-compose.yml`
- Create: `docker/docker-compose.local.yml`
- Create: `docker/.env.example`

- [ ] **Step 1: Write `Caddyfile`**

```caddy
{
  email {$ACME_EMAIL}
}

{$DOMAIN} {
  encode zstd gzip

  @api path /api/*
  reverse_proxy @api shim:8080 {
    header_up X-Forwarded-For {remote_host}
  }

  reverse_proxy web:3000
}
```

- [ ] **Step 2: Write `docker-compose.yml`**

```yaml
services:
  caddy:
    image: caddy:2-alpine
    restart: unless-stopped
    ports:
      - "80:80"
      - "443:443"
    environment:
      DOMAIN: ${DOMAIN}
      ACME_EMAIL: ${ACME_EMAIL}
    volumes:
      - ./Caddyfile:/etc/caddy/Caddyfile:ro
      - caddy-data:/data
      - caddy-config:/config
    depends_on:
      - web
      - shim

  web:
    build:
      context: ..
      dockerfile: docker/Dockerfile.web
    restart: unless-stopped
    environment:
      NEXT_PUBLIC_API_BASE: /api
      AGENTSPAY_SHIM_URL: http://shim:8080

  shim:
    build:
      context: ..
      dockerfile: docker/Dockerfile.shim
    restart: unless-stopped
    environment:
      AGENTSPAY_REDIS_URL: redis://redis:6379
      AGENTSPAY_PAID_ENDPOINT_URL: http://paid-endpoint:3001
      AGENTSPAY_DEVNET_WALLET_PATH: /data/devnet-wallet.json
      AGENTSPAY_DEVNET_LEDGER_PATH: /data/devnet-ledger.db
    volumes:
      - wallet-data:/data
    depends_on:
      - redis
      - paid-endpoint

  paid-endpoint:
    build:
      context: ..
      dockerfile: docker/Dockerfile.paid-endpoint
    restart: unless-stopped
    environment:
      AGENTSPAY_PROVIDER_KEYPAIR: /data/provider-keypair.json
    volumes:
      - wallet-data:/data

  redis:
    image: redis:7-alpine
    restart: unless-stopped
    volumes:
      - redis-data:/data

volumes:
  caddy-data:
  caddy-config:
  wallet-data:
  redis-data:
```

- [ ] **Step 3: Write `docker-compose.local.yml` (no Caddy, expose web/shim directly)**

```yaml
services:
  caddy:
    profiles: [_disabled]

  web:
    ports:
      - "3000:3000"

  shim:
    ports:
      - "8080:8080"

  paid-endpoint:
    ports:
      - "3001:3001"
```

- [ ] **Step 4: Write `.env.example`**

```env
DOMAIN=agentspay.dev
ACME_EMAIL=you@example.com
```

### Task 8.5: Operator scripts + README

**Files:**
- Create: `docker/scripts/refill-wallet.sh`
- Create: `docker/scripts/backup-wallet.sh`
- Create: `docker/README.md`

- [ ] **Step 1: Write `refill-wallet.sh`**

```bash
#!/usr/bin/env bash
# Show the demo wallet's pubkey and the faucet URLs the operator must visit.
set -euo pipefail

WALLET=${1:-/var/lib/docker/volumes/agentspay_wallet-data/_data/devnet-wallet.json}
if [ ! -f "$WALLET" ]; then
  echo "wallet file not found at $WALLET" >&2
  exit 1
fi

PUBKEY=$(python3 -c "
import json, base58, sys
bytes_=json.load(open('$WALLET'))
print(base58.b58encode(bytes(bytes_[32:])).decode())
")
echo "pubkey: $PUBKEY"
echo "fund SOL : https://faucet.solana.com (paste the pubkey)"
echo "fund USDC: https://faucet.circle.com (select Solana Devnet)"
```

- [ ] **Step 2: Write `backup-wallet.sh`**

```bash
#!/usr/bin/env bash
set -euo pipefail
SRC=${1:-/var/lib/docker/volumes/agentspay_wallet-data/_data/devnet-wallet.json}
DEST=${2:-$HOME/devnet-wallet-$(date +%Y%m%d-%H%M%S).json}
cp "$SRC" "$DEST"
chmod 0600 "$DEST"
echo "backed up to $DEST"
```

- [ ] **Step 3: Write `README.md`**

```markdown
# AgentsPay Docker stack

## First-time setup

```bash
cp .env.example .env
# edit DOMAIN and ACME_EMAIL

# Generate a fresh devnet wallet inside the volume:
docker volume create agentspay_wallet-data
docker run --rm -v agentspay_wallet-data:/data rust:1.83-slim bash -lc \
  "cargo install --git https://github.com/solana-labs/solana --bin solana-keygen --quiet 2>/dev/null; \
   solana-keygen new --no-bip39-passphrase --silent -o /data/devnet-wallet.json"

# Fund it (manual)
./scripts/refill-wallet.sh

# Bring everything up
docker compose -f docker-compose.yml up -d --build

# Tail logs
docker compose logs -f
```

## Local dev (no TLS, no domain)

```bash
docker compose -f docker-compose.yml -f docker-compose.local.yml up --build
open http://localhost:3000
```

## Useful commands

- check wallet status: `curl https://$DOMAIN/api/devnet/wallet-status`
- restart shim only: `docker compose restart shim`
- backup wallet: `./scripts/backup-wallet.sh`
```

### Task 8.6: Local compose smoke test

- [ ] **Step 1: Build + run the local compose stack**

```bash
cd docker
cp .env.example .env
docker compose -f docker-compose.yml -f docker-compose.local.yml up --build -d
docker compose ps
```

Expected: 4 services up (caddy is disabled by the local profile), web/shim/paid-endpoint/redis all healthy.

- [ ] **Step 2: Visit http://localhost:3000**

Repeat the visitor flow from Phase 6 step 2. Everything must work identically. If the devnet trigger 503s with "wallet drained", that's fine — funding the volume's wallet is a separate operator step documented in `docker/README.md`.

- [ ] **Step 3: Tear it down**

```bash
docker compose -f docker-compose.yml -f docker-compose.local.yml down
```

- [ ] **Step 4: Commit**

```bash
git add docker
git commit -m "feat(docker): production stack (web + shim + paid-endpoint + redis + caddy)"
```

**Milestone:** Phase 8 complete. The whole stack is containerized and runs from `docker compose up`.

---

## Phase 9 — Deploy + operator smoke gate (≈ 60 min, operator action)

**Goal of phase:** The site is reachable at the chosen domain with TLS. Smoke playbook is documented and exercised.

### Task 9.1: Pick a host + DNS

- [ ] **Step 1:** Provision a $5/mo VPS (Hetzner CX22 or similar). Install Docker + Docker Compose plugin.
- [ ] **Step 2:** Point your domain's `A` record at the VPS IPv4.
- [ ] **Step 3:** SSH in, `git clone` this repo, `cd docker`, `cp .env.example .env`, fill in `DOMAIN` and `ACME_EMAIL`.
- [ ] **Step 4:** Generate the devnet wallet inside the `wallet-data` volume per `docker/README.md`. Run `./scripts/refill-wallet.sh` and visit both faucets.
- [ ] **Step 5:** `docker compose -f docker-compose.yml up -d --build`. Caddy will request a Let's Encrypt cert on first request. Tail `docker compose logs caddy` until you see `certificate obtained successfully`.

### Task 9.2: Operator smoke playbook

- [ ] **Step 1:** `curl -s https://$DOMAIN/api/health | jq` — expect `{"status":"ok"}`.
- [ ] **Step 2:** `curl -s https://$DOMAIN/api/devnet/wallet-status | jq` — expect `healthy: true`, with SOL ≥ 0.05 and USDC ≥ 2.
- [ ] **Step 3:** Open the site in a fresh browser profile. Hero loads in < 1s. Live tx badge shows either a real Solscan link or the "be the first" placeholder.
- [ ] **Step 4:** Sandbox tab — click each of the 4 tool buttons. All return realistic JSON within ~500ms.
- [ ] **Step 5:** Devnet tab — click "Trigger a real on-chain tx". Solscan link appears within ~5s. Open it. It must resolve to a real, confirmed transaction.
- [ ] **Step 6:** Click trigger again — must show a 429 rate-limit error.
- [ ] **Step 7:** Tether your phone to a different network, repeat step 5. Independent IP gets its own bucket; the trigger works.

- [ ] **Step 8:** Commit the README link in the project's main README so the smoke gate is discoverable next time:

```bash
# Append to README.md
echo "" >> README.md
echo "## Live demo site" >> README.md
echo "Production deployment lives at https://$DOMAIN. See \`docker/README.md\` for operator runbook." >> README.md
git add README.md
git commit -m "docs: link to live demo site in README"
```

**Milestone:** Phase 9 complete. The site is live and you can DM the URL to your 3 cherry-picked devs.

---

## Self-review

### Spec coverage

I walked every spec section against the plan:

- **Spec §1 Purpose:** covered by Phases 2-7 (the 6 page sections + 5 visitor outcomes).
- **Spec §2 Locked decisions:** all four embedded in the plan (dev-dark colors in Task 2.1, agent-first hero copy in Task 2.2, server-funded trigger in Task 5.1, single-page composition in Task 7.3).
- **Spec §3 Architecture:** Phases 0-1 (shim), Phase 8 (Docker), Phase 9 (Caddy + TLS).
- **Spec §4.1 frontend file list:** every file listed appears in Phases 2-7 with a Task.
- **Spec §4.2 shim endpoints:** 5 endpoints, all implemented:
  - `/api/health` → Task 1.4
  - `/api/sandbox/session` → Task 1.4 + Task 4.3 (rate limit)
  - `/api/sandbox/call` → Task 1.4 + Task 4.3
  - `/api/devnet/trigger` → Task 5.1
  - `/api/devnet/wallet-status` → Task 5.1
  - `/api/stats/latest-tx` → Task 5.2
- **Spec §4.4 Docker:** all 5 services in Phase 8 (Tasks 8.1-8.4). Operator scripts in 8.5.
- **Spec §5 data flows:** sandbox call covered by Tasks 1.4 + 3.4-3.7 (e2e check); devnet trigger covered by Tasks 5.1 + 6.2-6.4.
- **Spec §6 error handling:** ShimError variants in Task 1.1 cover every failure listed in the spec's error table. The devnet drained case is enforced in Task 5.1. Frontend retry-on-410 in Task 3.2 (api.ts).
- **Spec §7 testing strategy:** unit tests in Task 1.2 (session) + Task 4.2 (ratelimit). Manual smoke gates in every phase. The optional Playwright e2e is deferred per spec §7.2; not added to plan.
- **Spec §8 out of scope:** not added to plan. ✓
- **Spec §9 implementation order:** plan phases 0-9 map 1:1 to spec steps 1-12 (a couple are merged: spec step 9 = part of plan Phase 6; spec step 8 = plan Phase 7).

No gaps.

### Placeholder scan

I grep'd the plan for the red-flag phrases: `TBD`, `TODO`, `fill in`, `appropriate error handling`, `similar to Task`, `add validation`. None found except inside actual code comments (e.g., `TODO(week-2)` in legacy quoted code — that's pre-existing repo state, not a plan-level placeholder).

Every code block is complete and runnable.

### Type consistency

Cross-checked the names used across tasks:
- `Config` (Task 1.1) → used in `state.rs` Task 1.1 → AppState construction in main.rs (Task 1.4 + 4.2 + 5.3). Field names match: `mcp_binary`, `paid_endpoint_url`, `devnet_wallet_path`, `devnet_ledger_path`, `session_ttl`, `redis_url`.
- `SessionStore` constructors: `new_in_memory(ttl)` (Task 1.2) and `new_with_redis(ttl, url)` (Task 4.1) — both consistent with main.rs construction in Task 4.1 step 3.
- `McpCall` struct fields (Task 1.3) match the usage sites in Tasks 1.4 (`sandbox::call_tool`) and 5.1 (`devnet::trigger`).
- `AppState` evolves additively across phases: starts in Task 1.1 with `{config, sessions, http}`, gains `ratelimit` in Task 4.2 step 2, gains `latest_tx` in Task 5.2 step 3. Every later task uses the latest shape.
- Frontend `ToolName` union (Task 3.2) matches the `ALLOWED_TOOLS` slice in the shim (Tasks 1.4 + 4.3). Both list the same 5 tools.
- `DevnetTriggerResponse` (Task 6.2 api.ts) matches `TriggerResponse` (Task 5.1 handler) field-for-field.

No type drift.

---

## Execution Handoff

**Plan complete and saved to `docs/superpowers/plans/2026-05-14-website-and-live-demo.md`. Two execution options:**

**1. Subagent-Driven (recommended)** — I dispatch a fresh subagent per task, review between tasks, fast iteration. Best for the Rust shim + frontend wiring tasks where the exact code matters.

**2. Inline Execution** — Execute tasks in this session using executing-plans, batch execution with checkpoints. Better if you want to watch each step live and intervene.

**Which approach?**
