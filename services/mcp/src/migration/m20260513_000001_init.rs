//! Initial migration — creates `wallet`, `budget`, `policy`,
//! `ledger_entry`, `audit_log`.
//!
//! All UUID/ID columns are TEXT (SQLite has no native UUID).
//! Timestamps are TIMESTAMPTZ (chrono `DateTime<Utc>` → ISO-8601).

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Wallet::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Wallet::Id).text().not_null().primary_key())
                    .col(
                        ColumnDef::new(Wallet::AgentId)
                            .text()
                            .not_null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(Wallet::AvailableUsdc).text().not_null())
                    .col(
                        ColumnDef::new(Wallet::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Budget::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Budget::Id).text().not_null().primary_key())
                    .col(
                        ColumnDef::new(Budget::AgentId)
                            .text()
                            .not_null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(Budget::DailyUsd).double().not_null())
                    .col(ColumnDef::new(Budget::PerCallUsd).double().not_null())
                    .col(
                        ColumnDef::new(Budget::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Policy::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Policy::Id).text().not_null().primary_key())
                    .col(
                        ColumnDef::new(Policy::AgentId)
                            .text()
                            .not_null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(Policy::AllowHttp).boolean().not_null())
                    .col(ColumnDef::new(Policy::AllowHttps).boolean().not_null())
                    .col(
                        ColumnDef::new(Policy::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(LedgerEntry::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(LedgerEntry::Id)
                            .text()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(LedgerEntry::AgentId).text().not_null())
                    .col(ColumnDef::new(LedgerEntry::Endpoint).text().not_null())
                    .col(ColumnDef::new(LedgerEntry::AmountUsdc).text().not_null())
                    .col(
                        ColumnDef::new(LedgerEntry::PaymentId)
                            .text()
                            .not_null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(LedgerEntry::TransactionId).text().not_null())
                    .col(ColumnDef::new(LedgerEntry::Status).text().not_null())
                    .col(
                        ColumnDef::new(LedgerEntry::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_ledger_entry_agent_id")
                    .table(LedgerEntry::Table)
                    .col(LedgerEntry::AgentId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(AuditLog::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(AuditLog::Id).text().not_null().primary_key())
                    .col(ColumnDef::new(AuditLog::AgentId).text().not_null())
                    .col(ColumnDef::new(AuditLog::Tool).text().not_null())
                    .col(ColumnDef::new(AuditLog::Endpoint).text().null())
                    .col(ColumnDef::new(AuditLog::AmountUsdc).text().null())
                    .col(ColumnDef::new(AuditLog::Status).text().not_null())
                    .col(
                        ColumnDef::new(AuditLog::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_audit_log_agent_id")
                    .table(AuditLog::Table)
                    .col(AuditLog::AgentId)
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_audit_log_created_at")
                    .table(AuditLog::Table)
                    .col(AuditLog::CreatedAt)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(AuditLog::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(LedgerEntry::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Policy::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Budget::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Wallet::Table).to_owned())
            .await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
enum Wallet {
    Table,
    Id,
    AgentId,
    AvailableUsdc,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum Budget {
    Table,
    Id,
    AgentId,
    DailyUsd,
    PerCallUsd,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum Policy {
    Table,
    Id,
    AgentId,
    AllowHttp,
    AllowHttps,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum LedgerEntry {
    Table,
    Id,
    AgentId,
    Endpoint,
    AmountUsdc,
    PaymentId,
    TransactionId,
    Status,
    CreatedAt,
}

#[derive(DeriveIden)]
enum AuditLog {
    Table,
    Id,
    AgentId,
    Tool,
    Endpoint,
    AmountUsdc,
    Status,
    CreatedAt,
}
