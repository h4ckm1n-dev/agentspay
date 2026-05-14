//! Session lifecycle for the sandbox demo.
//!
//! A session owns an isolated tmpdir that holds the agent's SQLite ledger
//! and Solana keypair. We spawn `agentspay-mcp` against this tmpdir on
//! every `/api/sandbox/call`, so multiple browser tabs never share state.
//!
//! Phase 1 ships with an in-memory implementation. Phase 4 will swap the
//! backing store for Redis without changing this module's public surface.

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
    // Phase 4: when Some, mirror create/get/expire into Redis so multiple
    // shim replicas (or restarts) see the same sessions.
    redis: Option<redis::aio::ConnectionManager>,
}

impl SessionStore {
    pub fn new_in_memory(ttl: Duration) -> Self {
        Self {
            inner: RwLock::new(HashMap::new()),
            ttl,
            redis: None,
        }
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
        assert!(
            store.get(&s.id).await.is_none(),
            "expired session must be gone"
        );
    }
}
