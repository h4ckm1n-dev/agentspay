//! Database bootstrap: resolve the SQLite URL, ensure the parent
//! directory exists, open a SeaORM connection, run all pending
//! migrations.

use std::{env, path::PathBuf, time::Duration};

use anyhow::Context;
use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use sea_orm_migration::MigratorTrait;

use crate::migration::Migrator;

const ENV_VAR: &str = "AGENTSPAY_DATABASE_URL";
const DEFAULT_SUBPATH: &str = ".agentspay/agentspay-mcp.db";

/// Resolve the database URL to use, creating any missing parent directories
/// for SQLite file paths.
pub async fn resolved_url() -> anyhow::Result<String> {
    if let Ok(url) = env::var(ENV_VAR) {
        if !url.trim().is_empty() {
            return Ok(url);
        }
    }

    let home = env::var("HOME").context("$HOME is not set; cannot pick default database path")?;
    let path = PathBuf::from(home).join(DEFAULT_SUBPATH);
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    // SeaORM expects `sqlite://` URLs; we append `?mode=rwc` so the file
    // is created on first open.
    Ok(format!("sqlite://{}?mode=rwc", path.display()))
}

/// Open the connection and run migrations.
pub async fn connect_and_migrate(url: &str) -> anyhow::Result<DatabaseConnection> {
    let mut opts = ConnectOptions::new(url.to_owned());
    opts.max_connections(4)
        .min_connections(1)
        .connect_timeout(Duration::from_secs(5))
        .acquire_timeout(Duration::from_secs(5))
        .idle_timeout(Duration::from_secs(60))
        .sqlx_logging(false);

    let db = Database::connect(opts)
        .await
        .with_context(|| format!("failed to connect to {url}"))?;

    Migrator::up(&db, None)
        .await
        .context("failed to run database migrations")?;

    Ok(db)
}
