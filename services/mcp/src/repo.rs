//! Repository facade over the SeaORM `DatabaseConnection`.
//!
//! All persistent state goes through this type. Handlers should never
//! touch [`sea_orm::DatabaseConnection`] directly — it keeps the audit
//! and ledger writes co-located in one module so we can reason about
//! idempotency in one place.

use chrono::{DateTime, NaiveTime, Utc};
use sea_orm::{
    sea_query::OnConflict, ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection,
    DbErr, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Set,
    TransactionTrait,
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

    /// Upsert the budget row. `allowed_recipients` / `not_after` use
    /// two-level `Option` semantics: outer `None` = leave the stored value
    /// unchanged, `Some(None)` = clear it, `Some(Some(v))` = set it. This
    /// keeps a plain cap update from silently wiping the mandate fields.
    pub async fn upsert_budget(
        &self,
        agent_id: &str,
        daily_usd: f64,
        per_call_usd: f64,
        allowed_recipients: Option<Option<String>>,
        not_after: Option<Option<DateTime<Utc>>>,
    ) -> Result<budget::Model, DbErr> {
        let now = Utc::now();
        if let Some(existing) = self.get_budget(agent_id).await? {
            let mut active: budget::ActiveModel = existing.into();
            active.daily_usd = Set(daily_usd);
            active.per_call_usd = Set(per_call_usd);
            if let Some(recipients) = allowed_recipients {
                active.allowed_recipients = Set(recipients);
            }
            if let Some(expiry) = not_after {
                active.not_after = Set(expiry);
            }
            active.updated_at = Set(now);
            return active.update(&self.db).await;
        }

        let model = budget::ActiveModel {
            id: Set(Uuid::new_v4().to_string()),
            agent_id: Set(agent_id.to_string()),
            daily_usd: Set(daily_usd),
            per_call_usd: Set(per_call_usd),
            allowed_recipients: Set(allowed_recipients.flatten()),
            not_after: Set(not_after.flatten()),
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

    /// Look up a ledger row by its caller-supplied idempotency key.
    pub async fn find_by_idempotency_key(
        &self,
        key: &str,
    ) -> Result<Option<ledger_entry::Model>, DbErr> {
        ledger_entry::Entity::find()
            .filter(ledger_entry::Column::IdempotencyKey.eq(key))
            .one(&self.db)
            .await
    }

    /// Reserve a payment BEFORE settlement: re-checks the daily cap and
    /// writes a `pending` ledger row in one transaction, so money that is
    /// about to move on-chain is counted against the budget even if the
    /// process dies mid-settle, and a concurrent `pay_url` sees the
    /// reservation in its own cap check.
    ///
    /// `reuse_row_id` recycles a previous `failed` row for the same
    /// idempotency key instead of violating its unique index.
    ///
    /// ponytail: cross-process races are narrowed to the gap between two
    /// SQLite write transactions, not eliminated; move the cap check into a
    /// `BEGIN IMMEDIATE` guard if multi-process throughput ever matters.
    #[allow(clippy::too_many_arguments)]
    pub async fn reserve_payment(
        &self,
        agent_id: &str,
        endpoint: &str,
        amount_usdc: &str,
        payment_id: &str,
        idempotency_key: Option<String>,
        reuse_row_id: Option<String>,
        required_usd: f64,
        daily_cap_usd: f64,
    ) -> Result<ReserveOutcome, DbErr> {
        let txn = self.db.begin().await?;
        let today_spent = today_spending_on(&txn, agent_id).await?;
        if today_spent + required_usd > daily_cap_usd {
            txn.rollback().await?;
            return Ok(ReserveOutcome::OverDailyCap { today_spent });
        }

        let now = Utc::now();
        let model = if let Some(row_id) = reuse_row_id {
            let existing = ledger_entry::Entity::find_by_id(row_id.clone())
                .one(&txn)
                .await?
                .ok_or_else(|| DbErr::Custom(format!("ledger row {row_id} vanished")))?;
            let mut active: ledger_entry::ActiveModel = existing.into();
            active.endpoint = Set(endpoint.to_string());
            active.amount_usdc = Set(amount_usdc.to_string());
            active.payment_id = Set(payment_id.to_string());
            active.transaction_id = Set(String::new());
            active.status = Set("pending".to_string());
            active.created_at = Set(now);
            active.update(&txn).await?
        } else {
            let active = ledger_entry::ActiveModel {
                id: Set(Uuid::new_v4().to_string()),
                agent_id: Set(agent_id.to_string()),
                endpoint: Set(endpoint.to_string()),
                amount_usdc: Set(amount_usdc.to_string()),
                payment_id: Set(payment_id.to_string()),
                transaction_id: Set(String::new()),
                status: Set("pending".to_string()),
                idempotency_key: Set(idempotency_key),
                created_at: Set(now),
            };
            active.insert(&txn).await?
        };
        txn.commit().await?;
        Ok(ReserveOutcome::Reserved(model))
    }

    /// Flip a reserved payment to its settled state and write the audit row
    /// atomically. `status` is the final ledger status (e.g. `"ok status=200"`).
    pub async fn finalize_payment(
        &self,
        ledger_id: &str,
        status: String,
        transaction_id: String,
        audit: audit_log::ActiveModel,
    ) -> Result<(ledger_entry::Model, audit_log::Model), DbErr> {
        let txn = self.db.begin().await?;
        let existing = ledger_entry::Entity::find_by_id(ledger_id.to_string())
            .one(&txn)
            .await?
            .ok_or_else(|| DbErr::Custom(format!("ledger row {ledger_id} vanished")))?;
        let mut active: ledger_entry::ActiveModel = existing.into();
        active.status = Set(status);
        active.transaction_id = Set(transaction_id);
        let ledger_model = active.update(&txn).await?;
        let audit_model = audit.insert(&txn).await?;
        txn.commit().await?;
        Ok((ledger_model, audit_model))
    }

    /// Mark a reserved payment as failed so it stops counting against the
    /// daily budget. Best-effort callers may ignore the result.
    pub async fn mark_payment_failed(&self, ledger_id: &str, detail: &str) -> Result<(), DbErr> {
        let existing = ledger_entry::Entity::find_by_id(ledger_id.to_string())
            .one(&self.db)
            .await?
            .ok_or_else(|| DbErr::Custom(format!("ledger row {ledger_id} vanished")))?;
        let mut active: ledger_entry::ActiveModel = existing.into();
        // Truncate: settle errors can embed full HTTP bodies.
        let detail: String = detail.chars().take(200).collect();
        active.status = Set(format!("failed {detail}"));
        active.update(&self.db).await?;
        Ok(())
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

    /// Sum today's spending. See [`today_spending_on`].
    pub async fn today_spending(&self, agent_id: &str) -> Result<f64, DbErr> {
        today_spending_on(&self.db, agent_id).await
    }
}

/// Sum the `amount_usdc` of today's ledger entries for `agent_id`, on any
/// connection (plain or inside a transaction). `pending` and `ok` rows both
/// count — a reservation is money in flight; only `failed` rows (settlement
/// never happened) are excluded. Returns 0.0 when there are no rows.
///
/// `amount_usdc` is stored as TEXT (decimal string) per the v0.1
/// "no rust_decimal yet" TODO, so we fetch the rows and sum in Rust.
/// Acceptable while daily call counts are small; revisit at v0.2 with
/// proper DECIMAL columns and `SUM(CAST(...))` aggregation.
async fn today_spending_on<C: ConnectionTrait>(conn: &C, agent_id: &str) -> Result<f64, DbErr> {
    let start_of_today = Utc::now().date_naive().and_time(NaiveTime::MIN).and_utc();

    let rows = ledger_entry::Entity::find()
        .filter(ledger_entry::Column::AgentId.eq(agent_id))
        .filter(ledger_entry::Column::CreatedAt.gte(start_of_today))
        .all(conn)
        .await?;

    let total: f64 = rows
        .into_iter()
        .filter(|r| !r.status.starts_with("failed"))
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

/// Outcome of [`LedgerRepo::reserve_payment`].
pub enum ReserveOutcome {
    Reserved(ledger_entry::Model),
    OverDailyCap { today_spent: f64 },
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migration::Migrator;
    use sea_orm::Database;
    use sea_orm_migration::MigratorTrait;

    async fn test_repo() -> LedgerRepo {
        let db = Database::connect("sqlite::memory:").await.expect("connect");
        Migrator::up(&db, None).await.expect("migrate");
        LedgerRepo::new(db)
    }

    fn audit() -> audit_log::ActiveModel {
        new_audit("default", "agentspay_pay_url", None, None, "ok".into())
    }

    #[tokio::test]
    async fn reserve_rejects_over_daily_cap() {
        let repo = test_repo().await;
        match repo
            .reserve_payment(
                "default",
                "https://x",
                "60.00",
                "pay_1",
                None,
                None,
                60.0,
                50.0,
            )
            .await
            .expect("reserve")
        {
            ReserveOutcome::OverDailyCap { today_spent } => assert_eq!(today_spent, 0.0),
            ReserveOutcome::Reserved(_) => panic!("60 > 50 cap must not reserve"),
        }
    }

    #[tokio::test]
    async fn pending_reservation_counts_toward_daily_spend() {
        let repo = test_repo().await;
        let ReserveOutcome::Reserved(row) = repo
            .reserve_payment(
                "default",
                "https://x",
                "10.00",
                "pay_1",
                None,
                None,
                10.0,
                50.0,
            )
            .await
            .expect("reserve")
        else {
            panic!("first reserve must succeed");
        };
        assert_eq!(row.status, "pending");
        assert_eq!(repo.today_spending("default").await.expect("sum"), 10.0);

        // A second call that would tip over the cap sees the pending row.
        match repo
            .reserve_payment(
                "default",
                "https://y",
                "45.00",
                "pay_2",
                None,
                None,
                45.0,
                50.0,
            )
            .await
            .expect("reserve")
        {
            ReserveOutcome::OverDailyCap { today_spent } => assert_eq!(today_spent, 10.0),
            ReserveOutcome::Reserved(_) => panic!("10 + 45 > 50 must not reserve"),
        }

        let (finalized, _) = repo
            .finalize_payment(&row.id, "ok status=200".into(), "sig".into(), audit())
            .await
            .expect("finalize");
        assert_eq!(finalized.status, "ok status=200");
        assert_eq!(finalized.transaction_id, "sig");
        assert_eq!(repo.today_spending("default").await.expect("sum"), 10.0);
    }

    #[tokio::test]
    async fn failed_payment_is_released_from_budget() {
        let repo = test_repo().await;
        let ReserveOutcome::Reserved(row) = repo
            .reserve_payment(
                "default",
                "https://x",
                "10.00",
                "pay_1",
                None,
                None,
                10.0,
                50.0,
            )
            .await
            .expect("reserve")
        else {
            panic!("reserve must succeed");
        };
        repo.mark_payment_failed(&row.id, "rpc timeout")
            .await
            .expect("mark");
        assert_eq!(repo.today_spending("default").await.expect("sum"), 0.0);
    }

    #[tokio::test]
    async fn idempotency_key_lookup_and_failed_row_reuse() {
        let repo = test_repo().await;
        let key = Some("idem-1".to_string());
        let ReserveOutcome::Reserved(row) = repo
            .reserve_payment(
                "default",
                "https://x",
                "5.00",
                "pay_1",
                key.clone(),
                None,
                5.0,
                50.0,
            )
            .await
            .expect("reserve")
        else {
            panic!("reserve must succeed");
        };
        let found = repo
            .find_by_idempotency_key("idem-1")
            .await
            .expect("lookup")
            .expect("row exists");
        assert_eq!(found.id, row.id);

        repo.mark_payment_failed(&row.id, "boom")
            .await
            .expect("mark");
        let ReserveOutcome::Reserved(reused) = repo
            .reserve_payment(
                "default",
                "https://x",
                "5.00",
                "pay_2",
                key,
                Some(row.id.clone()),
                5.0,
                50.0,
            )
            .await
            .expect("re-reserve")
        else {
            panic!("retry after failure must reserve");
        };
        assert_eq!(reused.id, row.id, "failed row is recycled, not duplicated");
        assert_eq!(reused.status, "pending");
        assert_eq!(reused.payment_id, "pay_2");
    }

    #[tokio::test]
    async fn upsert_budget_mandate_semantics() {
        let repo = test_repo().await;
        let expiry = Utc::now() + chrono::Duration::hours(1);
        let b = repo
            .upsert_budget(
                "default",
                20.0,
                5.0,
                Some(Some("pk1,pk2".into())),
                Some(Some(expiry)),
            )
            .await
            .expect("insert");
        assert_eq!(b.allowed_recipients.as_deref(), Some("pk1,pk2"));

        // Plain cap update leaves the mandate untouched.
        let b = repo
            .upsert_budget("default", 30.0, 5.0, None, None)
            .await
            .expect("update");
        assert_eq!(b.daily_usd, 30.0);
        assert_eq!(b.allowed_recipients.as_deref(), Some("pk1,pk2"));
        assert!(b.not_after.is_some());

        // Explicit clear.
        let b = repo
            .upsert_budget("default", 30.0, 5.0, Some(None), Some(None))
            .await
            .expect("clear");
        assert_eq!(b.allowed_recipients, None);
        assert_eq!(b.not_after, None);
    }
}
