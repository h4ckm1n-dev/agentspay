//! SeaORM-convention prelude: re-export each generated `Entity` so call
//! sites can `use crate::entities::prelude::*;` and refer to `Wallet`,
//! `Budget`, etc. directly. Today's `repo.rs` qualifies them explicitly,
//! so allow unused-imports here to keep the SeaORM-generated layout
//! intact without polluting `-D warnings` builds.

#![allow(unused_imports)]

pub use super::audit_log::Entity as AuditLog;
pub use super::budget::Entity as Budget;
pub use super::ledger_entry::Entity as LedgerEntry;
pub use super::policy::Entity as Policy;
pub use super::wallet::Entity as Wallet;
