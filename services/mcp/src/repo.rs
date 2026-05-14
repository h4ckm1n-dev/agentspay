//! Repository facade over the SeaORM `DatabaseConnection`.
//!
//! All persistent state goes through this type. Handlers should never
//! touch [`sea_orm::DatabaseConnection`] directly — it keeps the audit
//! and ledger writes co-located in one module so we can reason about
//! idempotency in one place.

use chrono::{NaiveTime, Utc};
use sea_orm::{
    sea_query::OnConflict, ActiveModelTrait, ColumnTrait, DatabaseConnection, DbErr, EntityTrait,
    PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Set, TransactionTrait,
};
use uuid::Uuid;

use crate::entities::{audit_log, budget, ledger_entry, wallet};

const DEFAULT_AGENT_ID: &str = "default";
const DEFAULT_AVAILABLE_USDC: &str = "100.00";

/// Persistent ledger surface for the MCP handlers.
pub struct LedgerRepo {
    db: DatabaseConnection,
}

impl LedgerRepo {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    #[allow(dead_code)]
    pub fn db(&self) -> &DatabaseConnection {
        &self.db
    }

    /// Ensure the well-known `default` wallet exists, returning the model.
    pub async fn get_or_init_default_wallet(&self) -> Result<wallet::Model, DbErr> {
        if let Some(existing) = wallet::Entity::find()
            .filter(wallet::Column::AgentId.eq(DEFAULT_AGENT_ID))
            .one(&self.db)
            .await?
        {
            return Ok(existing);
        }

        let now = Utc::now();
        let model = wallet::ActiveModel {
            id: Set(Uuid::new_v4().to_string()),
            agent_id: Set(DEFAULT_AGENT_ID.to_string()),
            available_usdc: Set(DEFAULT_AVAILABLE_USDC.to_string()),
            solana_pubkey: Set(None),
            updated_at: Set(now),
        };
        wallet::Entity::insert(model)
            .on_conflict(
                OnConflict::column(wallet::Column::AgentId)
                    .do_nothing()
                    .to_owned(),
            )
            .do_nothing()
            .exec(&self.db)
            .await?;

        wallet::Entity::find()
            .filter(wallet::Column::AgentId.eq(DEFAULT_AGENT_ID))
            .one(&self.db)
            .await?
            .ok_or_else(|| DbErr::Custom("wallet upsert vanished".into()))
    }

    /// Set (or refresh) the `solana_pubkey` column for the given agent.
    /// No-op when the value already matches what's stored.
    pub async fn set_solana_pubkey(
        &self,
        agent_id: &str,
        pubkey_base58: &str,
    ) -> Result<wallet::Model, DbErr> {
        let existing = wallet::Entity::find()
            .filter(wallet::Column::AgentId.eq(agent_id))
            .one(&self.db)
            .await?
            .ok_or_else(|| DbErr::Custom(format!("wallet for agent_id={agent_id} not found")))?;
        if existing.solana_pubkey.as_deref() == Some(pubkey_base58) {
            return Ok(existing);
        }
        let mut active: wallet::ActiveModel = existing.into();
        active.solana_pubkey = Set(Some(pubkey_base58.to_string()));
        active.updated_at = Set(Utc::now());
        active.update(&self.db).await
    }

    pub async fn get_budget(&self, agent_id: &str) -> Result<Option<budget::Model>, DbErr> {
        budget::Entity::find()
            .filter(budget::Column::AgentId.eq(agent_id))
            .one(&self.db)
            .await
    }

    pub async fn upsert_budget(
        &self,
        agent_id: &str,
        daily_usd: f64,
        per_call_usd: f64,
    ) -> Result<budget::Model, DbErr> {
        let now = Utc::now();
        if let Some(existing) = self.get_budget(agent_id).await? {
            let mut active: budget::ActiveModel = existing.into();
            active.daily_usd = Set(daily_usd);
            active.per_call_usd = Set(per_call_usd);
            active.updated_at = Set(now);
            return active.update(&self.db).await;
        }

        let model = budget::ActiveModel {
            id: Set(Uuid::new_v4().to_string()),
            agent_id: Set(agent_id.to_string()),
            daily_usd: Set(daily_usd),
            per_call_usd: Set(per_call_usd),
            updated_at: Set(now),
        };
        model.insert(&self.db).await
    }

    #[allow(dead_code)]
    pub async fn insert_ledger_entry(
        &self,
        entry: ledger_entry::ActiveModel,
    ) -> Result<ledger_entry::Model, DbErr> {
        entry.insert(&self.db).await
    }

    pub async fn insert_audit_entry(
        &self,
        entry: audit_log::ActiveModel,
    ) -> Result<audit_log::Model, DbErr> {
        entry.insert(&self.db).await
    }

    /// Record a paid `pay_url` call atomically: one ledger row + one audit row.
    pub async fn record_paid_call(
        &self,
        ledger: ledger_entry::ActiveModel,
        audit: audit_log::ActiveModel,
    ) -> Result<(ledger_entry::Model, audit_log::Model), DbErr> {
        let txn = self.db.begin().await?;
        let ledger_model = ledger.insert(&txn).await?;
        let audit_model = audit.insert(&txn).await?;
        txn.commit().await?;
        Ok((ledger_model, audit_model))
    }

    pub async fn recent_audit(&self, limit: u32) -> Result<Vec<audit_log::Model>, DbErr> {
        audit_log::Entity::find()
            .order_by_desc(audit_log::Column::CreatedAt)
            .limit(u64::from(limit))
            .all(&self.db)
            .await
    }

    pub async fn count_audit(&self) -> Result<u64, DbErr> {
        audit_log::Entity::find().count(&self.db).await
    }

    /// Sum the `amount_usdc` of every ledger entry for `agent_id` created
    /// since the start of UTC today. Returns 0.0 when there are no rows.
    ///
    /// `amount_usdc` is stored as TEXT (decimal string) per the v0.1
    /// "no rust_decimal yet" TODO, so we fetch the rows and sum in Rust.
    /// Acceptable while daily call counts are small; revisit at v0.2 with
    /// proper DECIMAL columns and `SUM(CAST(...))` aggregation.
    pub async fn today_spending(&self, agent_id: &str) -> Result<f64, DbErr> {
        let start_of_today = Utc::now().date_naive().and_time(NaiveTime::MIN).and_utc();

        let rows = ledger_entry::Entity::find()
            .filter(ledger_entry::Column::AgentId.eq(agent_id))
            .filter(ledger_entry::Column::CreatedAt.gte(start_of_today))
            .all(&self.db)
            .await?;

        let total: f64 = rows
            .into_iter()
            .filter_map(|r| r.amount_usdc.parse::<f64>().ok())
            .filter(|v| v.is_finite() && *v >= 0.0)
            .sum();

        // Normalize -0.0 → 0.0 and clamp tiny float drift so the
        // user-visible string in balance is "0.00", not "-0.00".
        Ok(if total.is_finite() {
            total.max(0.0)
        } else {
            0.0
        })
    }
}

/// Convenience: build an active-model helper for a new audit entry.
pub fn new_audit(
    agent_id: &str,
    tool: &str,
    endpoint: Option<String>,
    amount_usdc: Option<String>,
    status: String,
) -> audit_log::ActiveModel {
    audit_log::ActiveModel {
        id: Set(Uuid::new_v4().to_string()),
        agent_id: Set(agent_id.to_string()),
        tool: Set(tool.to_string()),
        endpoint: Set(endpoint),
        amount_usdc: Set(amount_usdc),
        status: Set(status),
        created_at: Set(Utc::now()),
    }
}

/// Convenience: build an active-model helper for a new ledger entry.
pub fn new_ledger_entry(
    agent_id: &str,
    endpoint: String,
    amount_usdc: String,
    payment_id: String,
    transaction_id: String,
    status: String,
) -> ledger_entry::ActiveModel {
    ledger_entry::ActiveModel {
        id: Set(Uuid::new_v4().to_string()),
        agent_id: Set(agent_id.to_string()),
        endpoint: Set(endpoint),
        amount_usdc: Set(amount_usdc),
        payment_id: Set(payment_id),
        transaction_id: Set(transaction_id),
        status: Set(status),
        created_at: Set(Utc::now()),
    }
}
