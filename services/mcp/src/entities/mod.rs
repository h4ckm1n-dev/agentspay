//! SeaORM entity modules for the AgentsPay ledger.
//!
//! UUIDs are stored as TEXT columns (SQLite has no native UUID type).
//! Timestamps use `DateTimeUtc` (chrono) which SeaORM serialises as
//! ISO-8601 strings under SQLite — chosen because:
//!   * `with-chrono` is already in the workspace,
//!   * round-tripping through `chrono::DateTime<Utc>` avoids the timezone
//!     ambiguity we'd hit storing naive RFC3339 strings ourselves.

pub mod audit_log;
pub mod budget;
pub mod ledger_entry;
pub mod policy;
pub mod prelude;
pub mod wallet;
