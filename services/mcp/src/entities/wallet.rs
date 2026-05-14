//! `wallet` entity — one row per agent's USDC sandbox wallet.

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "wallet")]
pub struct Model {
    /// UUID stored as a TEXT column for SQLite portability.
    #[sea_orm(primary_key, auto_increment = false, column_type = "Text")]
    pub id: String,
    #[sea_orm(unique, indexed, column_type = "Text")]
    pub agent_id: String,
    /// Amount as a decimal string ("100.00"). Switch to fixed-point in v0.2.
    #[sea_orm(column_type = "Text")]
    pub available_usdc: String,
    /// Base58 Solana public key used to sign x402 exact-scheme payloads.
    /// Backfilled at MCP startup once the local keypair is loaded; NULL
    /// only for databases created on v0.1 before the binary has run once
    /// under v0.2.
    #[sea_orm(column_type = "Text", nullable)]
    pub solana_pubkey: Option<String>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
