//! Shared `AppState` cloned into every Axum handler.

use std::sync::Arc;

use crate::{
    config::Config, latest_tx::SharedLatestTx, ratelimit::SharedRateLimit, session::SessionStore,
};

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub sessions: Arc<SessionStore>,
    pub http: reqwest::Client,
    pub ratelimit: SharedRateLimit,
    pub latest_tx: SharedLatestTx,
}
