//! `policy` entity — minimal v0.1 policy row.
//!
//! Foreign key is logical only (we point at `wallet.agent_id`); SQLite
//! FKs are deferred to v0.2 once we have a real policy DSL.
//!
//! The struct + enum are part of the SeaORM scaffolding so the
//! `m20260513_000003_policy` migration round-trips, but no v0.3 code
//! path constructs them yet. Real wiring lands when we add the
//! `agentspay_set_policy` tool. `#[allow(dead_code)]` is the SeaORM
//! convention for entity stubs whose schema is shipped ahead of the
//! Rust call sites.

#![allow(dead_code)]

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "policy")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false, column_type = "Text")]
    pub id: String,
    #[sea_orm(unique, indexed, column_type = "Text")]
    pub agent_id: String,
    pub allow_http: bool,
    pub allow_https: bool,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
