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
