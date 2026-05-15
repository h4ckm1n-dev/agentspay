//! Cache of the most recent successful devnet tx, served to the hero counter
//! at /api/stats/latest-tx.
//!
//! The cache is "best effort" persisted to a SQLite file shared with the
//! agentspay-mcp subprocess. The in-memory `RwLock<Option<LatestTx>>` is the
//! source of truth at runtime; the SQLite row exists so the hero badge survives
//! shim container restarts. DB-write failures are logged but never propagated —
//! a successful devnet trigger must not fail because persistence hiccuped.

use std::path::Path;
use std::sync::Arc;
use std::time::SystemTime;

use chrono::{DateTime, Utc};
use sea_orm::{
    ConnectionTrait, Database, DatabaseConnection, DbBackend, Statement, Value as SeaValue,
};
use serde::Serialize;
use tokio::sync::RwLock;

const SINGLETON_ID: &str = "singleton";

const CREATE_TABLE_SQL: &str = "CREATE TABLE IF NOT EXISTS latest_tx_cache (\
    id TEXT PRIMARY KEY, \
    signature TEXT NOT NULL, \
    amount_usdc TEXT NOT NULL, \
    explorer_url TEXT NOT NULL, \
    recorded_at TEXT NOT NULL\
) STRICT";

const SELECT_SQL: &str = "SELECT signature, amount_usdc, explorer_url, recorded_at \
    FROM latest_tx_cache WHERE id = ?";

const UPSERT_SQL: &str = "INSERT INTO latest_tx_cache \
    (id, signature, amount_usdc, explorer_url, recorded_at) \
    VALUES (?, ?, ?, ?, ?) \
    ON CONFLICT(id) DO UPDATE SET \
    signature = excluded.signature, \
    amount_usdc = excluded.amount_usdc, \
    explorer_url = excluded.explorer_url, \
    recorded_at = excluded.recorded_at";

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
    db: Option<DatabaseConnection>,
}

pub type SharedLatestTx = Arc<LatestTxCache>;

impl LatestTxCache {
    /// Construct an in-memory-only cache. Kept for tests and callers that
    /// don't care about persistence.
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self::default()
    }

    /// Open (or create) the SQLite file at `db_path`, ensure the
    /// `latest_tx_cache` table exists, and hydrate the in-memory cache with
    /// the singleton row (if any).
    pub async fn load(db_path: &Path) -> anyhow::Result<Self> {
        let url = format!("sqlite://{}?mode=rwc", db_path.display());
        let db = Database::connect(&url).await?;

        db.execute(Statement::from_string(
            DbBackend::Sqlite,
            CREATE_TABLE_SQL.to_string(),
        ))
        .await?;

        let initial = match db
            .query_one(Statement::from_sql_and_values(
                DbBackend::Sqlite,
                SELECT_SQL,
                [SeaValue::from(SINGLETON_ID)],
            ))
            .await?
        {
            Some(row) => {
                let signature: String = row.try_get("", "signature")?;
                let amount_usdc: String = row.try_get("", "amount_usdc")?;
                let explorer_url: String = row.try_get("", "explorer_url")?;
                let recorded_at: String = row.try_get("", "recorded_at")?;
                let at = DateTime::parse_from_rfc3339(&recorded_at)
                    .map(|dt| SystemTime::from(dt.with_timezone(&Utc)))
                    .unwrap_or_else(|_| SystemTime::now());
                Some(LatestTx {
                    signature,
                    amount_usdc,
                    explorer_url,
                    at,
                })
            }
            None => None,
        };

        Ok(Self {
            inner: RwLock::new(initial),
            db: Some(db),
        })
    }

    pub async fn set(&self, tx: LatestTx) {
        if let Some(db) = self.db.as_ref() {
            let recorded_at: DateTime<Utc> = tx.at.into();
            let stmt = Statement::from_sql_and_values(
                DbBackend::Sqlite,
                UPSERT_SQL,
                [
                    SeaValue::from(SINGLETON_ID),
                    SeaValue::from(tx.signature.clone()),
                    SeaValue::from(tx.amount_usdc.clone()),
                    SeaValue::from(tx.explorer_url.clone()),
                    SeaValue::from(recorded_at.to_rfc3339()),
                ],
            );
            if let Err(err) = db.execute(stmt).await {
                tracing::warn!(error = %err, "latest_tx_cache: UPSERT failed (in-memory still updated)");
            }
        }
        *self.inner.write().await = Some(tx);
    }

    pub async fn get(&self) -> Option<LatestTxView> {
        let guard = self.inner.read().await;
        let tx = guard.as_ref()?;
        let age = SystemTime::now().duration_since(tx.at).ok()?.as_secs();
        // Hide entries older than 24h.
        if age > 24 * 3600 {
            return None;
        }
        Some(LatestTxView {
            signature: tx.signature.clone(),
            amount_usdc: tx.amount_usdc.clone(),
            explorer_url: tx.explorer_url.clone(),
            age_seconds: age,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn persists_across_new_cache_instance() {
        let tmp = TempDir::new().expect("tempdir");
        let db_path = tmp.path().join("latest-tx.db");

        let signature = "5xFakeDevnetSignatureForTest1111111111111111111111111111111111";
        let amount = "0.10";
        let explorer = "https://solscan.io/tx/abc?cluster=devnet";

        // First instance: write the entry, then drop.
        {
            let cache = LatestTxCache::load(&db_path)
                .await
                .expect("first load should succeed");
            cache
                .set(LatestTx {
                    signature: signature.to_string(),
                    amount_usdc: amount.to_string(),
                    explorer_url: explorer.to_string(),
                    at: SystemTime::now(),
                })
                .await;
        }

        // Second instance pointed at the SAME file: must hydrate the row.
        let cache2 = LatestTxCache::load(&db_path)
            .await
            .expect("second load should succeed");
        let view = cache2.get().await.expect("singleton row should be present");
        assert_eq!(view.signature, signature);
        assert_eq!(view.amount_usdc, amount);
        assert_eq!(view.explorer_url, explorer);
        assert!(
            view.age_seconds < 24 * 3600,
            "freshly-written entry should be inside the 24h window"
        );
    }
}
