//! `budget` entity — per-agent spend caps in USD (one row per agent).

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "budget")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false, column_type = "Text")]
    pub id: String,
    #[sea_orm(unique, indexed, column_type = "Text")]
    pub agent_id: String,
    pub daily_usd: f64,
    pub per_call_usd: f64,
    /// Comma-separated base58 pubkey allowlist. NULL/empty = any recipient.
    #[sea_orm(column_type = "Text", nullable)]
    pub allowed_recipients: Option<String>,
    /// Optional mandate expiry — payments are refused after this instant.
    pub not_after: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
