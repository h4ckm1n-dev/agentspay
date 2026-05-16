//! Cross-origin request guard.
//!
//! Browsers send an `Origin` header on cross-origin POST/PUT/DELETE/PATCH
//! (and on CORS preflight). When `AGENTSPAY_ALLOWED_ORIGINS` is set, the
//! shim rejects mutating requests whose Origin is not on the allowlist.
//! GET / HEAD / OPTIONS are unconstrained — read-only endpoints and CORS
//! preflights stay routable.
//!
//! When the env var is unset, the guard is disabled. Sensible default
//! for local dev; production deployments should set it to the deployed
//! frontend origin (e.g. `https://agentspay.dev`).
//!
//! This is *defense in depth* against a CSRF-like scenario where an
//! attacker site embeds `<form action="https://agentspay.dev/api/devnet/trigger" method="POST">`
//! and tricks a visitor's browser into draining the public demo wallet.
//! Per-IP rate-limit also mitigates, but Origin checking is the cleanest
//! cut.

use axum::{
    extract::Request,
    http::{HeaderValue, Method, StatusCode},
    middleware::Next,
    response::Response,
};

const ENV_ALLOWED_ORIGINS: &str = "AGENTSPAY_ALLOWED_ORIGINS";

/// Snapshot of the allowlist resolved at startup. Empty Vec = guard disabled.
#[derive(Clone, Debug)]
pub struct OriginAllowlist(Vec<String>);

impl OriginAllowlist {
    pub fn from_env() -> Self {
        let raw = std::env::var(ENV_ALLOWED_ORIGINS).unwrap_or_default();
        let list: Vec<String> = raw
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        Self(list)
    }

    pub fn is_enabled(&self) -> bool {
        !self.0.is_empty()
    }

    pub fn allows(&self, origin: &str) -> bool {
        self.0.iter().any(|allowed| allowed == origin)
    }
}

/// Tower middleware. Pass the allowlist into Router via
/// `.layer(axum::middleware::from_fn_with_state(allowlist, origin_guard))`.
pub async fn origin_guard(
    axum::extract::State(allowlist): axum::extract::State<OriginAllowlist>,
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    if !allowlist.is_enabled() {
        return Ok(next.run(req).await);
    }

    match *req.method() {
        Method::GET | Method::HEAD | Method::OPTIONS => return Ok(next.run(req).await),
        _ => {}
    }

    let origin = req
        .headers()
        .get(axum::http::header::ORIGIN)
        .and_then(|v: &HeaderValue| v.to_str().ok());

    match origin {
        Some(o) if allowlist.allows(o) => Ok(next.run(req).await),
        _ => Err(StatusCode::FORBIDDEN),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn allow(origins: &[&str]) -> OriginAllowlist {
        OriginAllowlist(origins.iter().map(|s| (*s).to_string()).collect())
    }

    #[test]
    fn empty_allowlist_is_disabled() {
        let a = OriginAllowlist(vec![]);
        assert!(!a.is_enabled());
    }

    #[test]
    fn explicit_match_passes() {
        let a = allow(&["https://agentspay.dev"]);
        assert!(a.allows("https://agentspay.dev"));
        assert!(!a.allows("https://evil.example"));
    }

    #[test]
    fn from_env_parses_comma_separated_list() {
        std::env::set_var(
            "AGENTSPAY_ALLOWED_ORIGINS",
            "https://agentspay.dev, https://staging.agentspay.dev",
        );
        let a = OriginAllowlist::from_env();
        assert!(a.allows("https://agentspay.dev"));
        assert!(a.allows("https://staging.agentspay.dev"));
        assert!(!a.allows("https://attacker.example"));
        std::env::remove_var("AGENTSPAY_ALLOWED_ORIGINS");
    }
}
