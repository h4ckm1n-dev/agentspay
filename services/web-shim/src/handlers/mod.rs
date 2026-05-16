use std::net::SocketAddr;

use axum::http::HeaderMap;

pub mod devnet;
pub mod health;
pub mod sandbox;
pub mod stats;

/// Extract the real client IP for rate-limiting purposes.
///
/// In production the shim sits behind Caddy, so `ConnectInfo<SocketAddr>`
/// is always Caddy's container IP — using it for rate-limit keys would
/// merge every visitor into one bucket. Caddy sets `X-Forwarded-For` to
/// the real remote_host, so we prefer that and fall back to the direct
/// peer only for direct-connection deployments (e.g. local dev without
/// a reverse proxy).
///
/// We trust the first hop of `X-Forwarded-For` because the only deployments
/// that put the shim behind a proxy use Caddy, which strips client-provided
/// `X-Forwarded-For` and writes the real `remote_host` itself. If you stand
/// up a different topology you must keep this property — otherwise an
/// attacker forging the header bypasses the per-IP limits.
pub fn client_ip(headers: &HeaderMap, peer: &SocketAddr) -> String {
    if let Some(raw) = headers.get("x-forwarded-for").and_then(|v| v.to_str().ok()) {
        if let Some(first) = raw.split(',').next() {
            let trimmed = first.trim();
            if !trimmed.is_empty() {
                return trimmed.to_string();
            }
        }
    }
    peer.ip().to_string()
}
