//! Week-3 migration: extend the `wallet` table with a `solana_pubkey` column.
//!
//! The column is `TEXT NULL` because the row is created by [`crate::repo`]
//! before the keypair is loaded — the binary backfills it on startup once
//! the [`crate::wallet::AgentWallet`] is available.
//!
//! Additive only: nothing about the existing init migration is touched, so
//! existing v0.1 databases keep working and gain a single nullable column.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Wallet::Table)
                    .add_column_if_not_exists(ColumnDef::new(Wallet::SolanaPubkey).text().null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Wallet::Table)
                    .drop_column(Wallet::SolanaPubkey)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum Wallet {
    Table,
    SolanaPubkey,
}
