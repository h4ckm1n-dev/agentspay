//! SeaORM migrations for the AgentsPay ledger.
//!
//! Used as a library (not via `sea-orm-cli`): the binary calls
//! [`Migrator::up`] at startup to bring the SQLite database to the
//! latest schema before serving any MCP requests.

use sea_orm_migration::prelude::*;

mod m20260513_000001_init;
mod m20260514_000001_solana;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20260513_000001_init::Migration),
            Box::new(m20260514_000001_solana::Migration),
        ]
    }
}
