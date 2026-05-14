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
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn set(&self, tx: LatestTx) {
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
