//! Shared `AppState` cloned into every Axum handler.

use std::sync::Arc;

use crate::{config::Config, session::SessionStore};

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub sessions: Arc<SessionStore>,
    pub http: reqwest::Client,
}
