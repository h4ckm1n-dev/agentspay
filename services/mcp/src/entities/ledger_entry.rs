//! `ledger_entry` entity — one row per settled x402 payment.
//!
//! `payment_id` is unique and acts as the idempotency key for retries.

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "ledger_entry")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false, column_type = "Text")]
    pub id: String,
    #[sea_orm(indexed, column_type = "Text")]
    pub agent_id: String,
    #[sea_orm(column_type = "Text")]
    pub endpoint: String,
    /// Amount as a decimal string. Switch to fixed-point in v0.2.
    #[sea_orm(column_type = "Text")]
    pub amount_usdc: String,
    #[sea_orm(unique, indexed, column_type = "Text")]
    pub payment_id: String,
    #[sea_orm(column_type = "Text")]
    pub transaction_id: String,
    #[sea_orm(column_type = "Text")]
    pub status: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
