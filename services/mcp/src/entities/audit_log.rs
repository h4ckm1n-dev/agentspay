//! `audit_log` entity — one row per state-mutating tool call.

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "audit_log")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false, column_type = "Text")]
    pub id: String,
    #[sea_orm(indexed, column_type = "Text")]
    pub agent_id: String,
    #[sea_orm(column_type = "Text")]
    pub tool: String,
    #[sea_orm(column_type = "Text", nullable)]
    pub endpoint: Option<String>,
    #[sea_orm(column_type = "Text", nullable)]
    pub amount_usdc: Option<String>,
    #[sea_orm(column_type = "Text")]
    pub status: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
