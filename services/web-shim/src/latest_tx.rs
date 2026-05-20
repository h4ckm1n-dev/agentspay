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

const COUNT_LEDGER_SQL: &str = "SELECT COUNT(*) AS total \
    FROM ledger_entry \
    WHERE transaction_id IS NOT NULL AND transaction_id != ''";

const SELECT_LEDGER_SQL: &str = "SELECT agent_id, endpoint, amount_usdc, payment_id, \
    transaction_id, status, created_at \
    FROM ledger_entry \
    WHERE transaction_id IS NOT NULL AND transaction_id != '' \
    ORDER BY created_at DESC \
    LIMIT ? OFFSET ?";

pub const MAX_LEDGER_PAGES: u64 = 20;
const MAX_PAGE_SIZE: u64 = 25;

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

#[derive(Debug, Clone, Serialize)]
pub struct LedgerTxView {
    pub signature: String,
    pub amount_usdc: String,
    pub explorer_url: String,
    pub endpoint: String,
    pub symbol: String,
    pub status: String,
    pub agent_id: String,
    pub payment_id: String,
    pub created_at: String,
    pub age_seconds: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct LedgerTxPage {
    pub page: u64,
    pub page_size: u64,
    pub total: u64,
    pub total_pages: u64,
    pub max_pages: u64,
    pub transactions: Vec<LedgerTxView>,
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

    pub async fn ledger_transactions(
        &self,
        page: u64,
        page_size: u64,
    ) -> anyhow::Result<LedgerTxPage> {
        let page = page.clamp(1, MAX_LEDGER_PAGES);
        let page_size = page_size.clamp(1, MAX_PAGE_SIZE);
        let Some(db) = self.db.as_ref() else {
            return Ok(empty_ledger_page(page, page_size));
        };

        let total = match db
            .query_one(Statement::from_string(
                DbBackend::Sqlite,
                COUNT_LEDGER_SQL.to_string(),
            ))
            .await
        {
            Ok(Some(row)) => {
                let total: i64 = row.try_get("", "total")?;
                total.max(0) as u64
            }
            Ok(None) => 0,
            Err(err) if is_missing_ledger_table(&err) => 0,
            Err(err) => return Err(err.into()),
        };

        if total == 0 {
            return Ok(empty_ledger_page(page, page_size));
        }

        let total_pages = total.div_ceil(page_size).min(MAX_LEDGER_PAGES);
        if page > total_pages {
            return Ok(LedgerTxPage {
                page,
                page_size,
                total,
                total_pages,
                max_pages: MAX_LEDGER_PAGES,
                transactions: Vec::new(),
            });
        }

        let offset = (page - 1) * page_size;
        let rows = match db
            .query_all(Statement::from_sql_and_values(
                DbBackend::Sqlite,
                SELECT_LEDGER_SQL,
                [
                    SeaValue::from(page_size as i64),
                    SeaValue::from(offset as i64),
                ],
            ))
            .await
        {
            Ok(rows) => rows,
            Err(err) if is_missing_ledger_table(&err) => Vec::new(),
            Err(err) => return Err(err.into()),
        };

        let transactions = rows
            .into_iter()
            .map(|row| {
                let endpoint: String = row.try_get("", "endpoint")?;
                let signature: String = row.try_get("", "transaction_id")?;
                let created_at: String = row.try_get("", "created_at")?;
                Ok(LedgerTxView {
                    explorer_url: solscan_url(&signature),
                    symbol: symbol_from_endpoint(&endpoint),
                    signature,
                    endpoint,
                    amount_usdc: row.try_get("", "amount_usdc")?,
                    payment_id: row.try_get("", "payment_id")?,
                    status: row.try_get("", "status")?,
                    agent_id: row.try_get("", "agent_id")?,
                    age_seconds: age_seconds(&created_at),
                    created_at,
                })
            })
            .collect::<Result<Vec<_>, sea_orm::DbErr>>()?;

        Ok(LedgerTxPage {
            page,
            page_size,
            total,
            total_pages,
            max_pages: MAX_LEDGER_PAGES,
            transactions,
        })
    }
}

fn empty_ledger_page(page: u64, page_size: u64) -> LedgerTxPage {
    LedgerTxPage {
        page,
        page_size,
        total: 0,
        total_pages: 0,
        max_pages: MAX_LEDGER_PAGES,
        transactions: Vec::new(),
    }
}

fn is_missing_ledger_table(err: &sea_orm::DbErr) -> bool {
    err.to_string().contains("no such table: ledger_entry")
}

fn solscan_url(signature: &str) -> String {
    format!("https://solscan.io/tx/{signature}?cluster=devnet")
}

fn symbol_from_endpoint(endpoint: &str) -> String {
    endpoint
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .filter(|part| !part.is_empty())
        .map(|part| part.chars().take(12).collect::<String>())
        .unwrap_or_else(|| "PAY".to_string())
}

fn age_seconds(created_at: &str) -> Option<u64> {
    DateTime::parse_from_rfc3339(created_at)
        .ok()
        .and_then(|dt| {
            Utc::now()
                .signed_duration_since(dt.with_timezone(&Utc))
                .to_std()
                .ok()
        })
        .map(|duration| duration.as_secs())
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
