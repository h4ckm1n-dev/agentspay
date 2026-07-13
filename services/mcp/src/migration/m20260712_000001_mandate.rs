//! Launch-hardening migration: spend-mandate columns + idempotent payments.
//!
//! * `budget.allowed_recipients` — comma-separated base58 pubkey allowlist.
//!   NULL/empty = any recipient (backwards compatible). When set, `pay_url`
//!   refuses to sign for a `payTo` outside the list.
//! * `budget.not_after` — optional mandate expiry; payments are refused
//!   after this instant until the budget is refreshed.
//! * `ledger_entry.idempotency_key` — optional caller-supplied key with a
//!   unique index so a retried `pay_url` cannot settle twice.
//!
//! Additive only: existing databases gain nullable columns and keep working.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Budget::Table)
                    .add_column_if_not_exists(
                        ColumnDef::new(Budget::AllowedRecipients).text().null(),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Budget::Table)
                    .add_column_if_not_exists(
                        ColumnDef::new(Budget::NotAfter)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(LedgerEntry::Table)
                    .add_column_if_not_exists(
                        ColumnDef::new(LedgerEntry::IdempotencyKey).text().null(),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_ledger_entry_idempotency_key")
                    .table(LedgerEntry::Table)
                    .col(LedgerEntry::IdempotencyKey)
                    .unique()
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("idx_ledger_entry_idempotency_key")
                    .table(LedgerEntry::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(LedgerEntry::Table)
                    .drop_column(LedgerEntry::IdempotencyKey)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Budget::Table)
                    .drop_column(Budget::NotAfter)
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Budget::Table)
                    .drop_column(Budget::AllowedRecipients)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum Budget {
    Table,
    AllowedRecipients,
    NotAfter,
}

#[derive(DeriveIden)]
enum LedgerEntry {
    Table,
    IdempotencyKey,
}
