//! Shared `AppState` cloned into every Axum handler.

use std::sync::Arc;

use crate::{config::Config, ratelimit::SharedRateLimit, session::SessionStore};

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub sessions: Arc<SessionStore>,
    /// Reused for Phase 2 outbound calls to the demo `paid-endpoint` provider.
    #[allow(dead_code)]
    pub http: reqwest::Client,
    pub ratelimit: SharedRateLimit,
}
