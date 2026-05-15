//! Starship-style stderr formatter for `agentspay-mcp`.
//!
//! Output goes to **stderr only** — stdout is reserved for the MCP
//! JSON-RPC framing and any `println!`/stdout write here would
//! immediately corrupt the protocol.
//!
//! Auto-detects:
//!   - ANSI: honors `NO_COLOR`, falls back to plain text when stderr
//!     is not a TTY (e.g. when redirected to a file or piped to
//!     `docker compose logs`).
//!   - Glyphs: requires both ANSI capability and a UTF-8 locale.
//!     Falls back to ASCII tokens (`[net]`, `[key]`, `[db ]`, `[ok ]`,
//!     `t=`, `+`, `x`) otherwise.

use std::io::{IsTerminal, Write};
use std::path::Path;

// ---------------------------------------------------------------------------
// Capability detection
// ---------------------------------------------------------------------------

/// True when we should emit ANSI SGR escapes on stderr.
pub fn supports_ansi() -> bool {
    if std::env::var_os("NO_COLOR").is_some() {
        return false;
    }
    std::io::stderr().is_terminal()
}

/// True when we should emit Nerd-font glyphs. Requires ANSI support
/// (so we already know we're on a TTY-ish stream) **and** a UTF-8
/// locale.
pub fn supports_glyphs() -> bool {
    if !supports_ansi() {
        return false;
    }
    let mut combined = String::new();
    combined.push_str(&std::env::var("LANG").unwrap_or_default());
    combined.push_str(&std::env::var("LC_ALL").unwrap_or_default());
    combined.push_str(&std::env::var("LC_CTYPE").unwrap_or_default());
    let upper = combined.to_uppercase();
    upper.contains("UTF-8") || upper.contains("UTF8")
}

// ---------------------------------------------------------------------------
// Glyph table
// ---------------------------------------------------------------------------

struct Glyphs {
    brand: &'static str,
    network: &'static str,
    wallet: &'static str,
    ledger: &'static str,
    ready: &'static str,
    clock: &'static str,
    ok: &'static str,
    fail: &'static str,
    rule: &'static str,
}

fn glyphs() -> Glyphs {
    if supports_glyphs() {
        Glyphs {
            brand: "\u{f308} agentspay",
            network: "\u{f0c1} ",
            wallet: "\u{f084} ",
            ledger: "\u{f1c0} ",
            ready: "\u{f00c} ",
            clock: "\u{f43d2}",
            ok: "\u{2713}",
            fail: "\u{2717}",
            rule: "\u{2500}",
        }
    } else {
        Glyphs {
            brand: "agentspay",
            network: "[net]",
            wallet: "[key]",
            ledger: "[db ]",
            ready: "[ok ]",
            clock: "t=",
            ok: "+",
            fail: "x",
            rule: "-",
        }
    }
}

// ---------------------------------------------------------------------------
// ANSI helpers
// ---------------------------------------------------------------------------

fn bold(s: &str) -> String {
    if supports_ansi() {
        format!("\x1b[1m{s}\x1b[22m")
    } else {
        s.to_string()
    }
}

fn italic(s: &str) -> String {
    if supports_ansi() {
        format!("\x1b[3m{s}\x1b[23m")
    } else {
        s.to_string()
    }
}

fn fg256(code: u8, s: &str) -> String {
    if supports_ansi() {
        format!("\x1b[38;5;{code}m{s}\x1b[39m")
    } else {
        s.to_string()
    }
}

fn dim(s: &str) -> String {
    fg256(245, s)
}
fn accent(s: &str) -> String {
    fg256(86, s)
}
fn brand_white(s: &str) -> String {
    fg256(231, s)
}
fn sandbox_color(s: &str) -> String {
    fg256(220, s)
}
fn devnet_color(s: &str) -> String {
    fg256(38, s)
}
fn mainnet_color(s: &str) -> String {
    fg256(196, s)
}
fn ok_color(s: &str) -> String {
    fg256(46, s)
}
fn fail_color(s: &str) -> String {
    fg256(203, s)
}

// ---------------------------------------------------------------------------
// Banner
// ---------------------------------------------------------------------------

pub struct Banner<'a> {
    pub version: &'a str,
    pub network: &'a str,
    pub pubkey: &'a str,
    pub keypair_path: &'a Path,
    pub ledger_path: &'a Path,
    pub ledger_tx_count: Option<u64>,
    pub tool_count: usize,
}

pub fn print_banner(b: &Banner<'_>) {
    let g = glyphs();
    let stderr = std::io::stderr();
    let mut out = stderr.lock();

    let rule = dim(&g.rule.repeat(57));
    let title = format!(
        "{} {}",
        bold(&brand_white(g.brand)),
        dim(&format!("v{}", b.version))
    );

    let toggle_hint = if b.network == "solana-devnet" {
        italic(&dim("toggle: AGENTSPAY_NETWORK=sandbox"))
    } else if b.network == "sandbox" {
        italic(&dim("toggle: AGENTSPAY_NETWORK=solana-devnet"))
    } else {
        String::new()
    };

    let pubkey_short = truncate_pubkey(b.pubkey);
    let keypair_detail = dim(&format!(
        "{} ({})",
        b.keypair_path.display(),
        file_perms(b.keypair_path)
    ));

    let ledger_detail = match b.ledger_tx_count {
        Some(n) => dim(&format!("{} ({} entries)", b.ledger_path.display(), n)),
        None => dim(&b.ledger_path.display().to_string()),
    };

    let ready_detail = dim(&format!("{} MCP tools advertised on stdio", b.tool_count));

    let _ = writeln!(out);
    let _ = writeln!(out, "  {title}");
    let _ = writeln!(out, "{rule}");
    let _ = writeln!(
        out,
        "  {} {}   {}",
        accent(g.network),
        brand_white(&bold(b.network)),
        toggle_hint
    );
    let _ = writeln!(
        out,
        "  {} {}   {}",
        accent(g.wallet),
        brand_white(&bold(&pubkey_short)),
        keypair_detail
    );
    let _ = writeln!(
        out,
        "  {} {}          {}",
        accent(g.ledger),
        brand_white(&bold("ledger")),
        ledger_detail
    );
    let _ = writeln!(
        out,
        "  {} {}           {}",
        accent(g.ready),
        brand_white(&bold("ready")),
        ready_detail
    );
    let _ = writeln!(out, "{rule}");
    let _ = writeln!(out);
}

fn truncate_pubkey(pk: &str) -> String {
    if pk.chars().count() <= 12 {
        pk.to_string()
    } else {
        let prefix: String = pk.chars().take(6).collect();
        let suffix: String = pk
            .chars()
            .rev()
            .take(4)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        format!("{prefix}\u{2026}{suffix}")
    }
}

#[cfg(unix)]
fn file_perms(p: &Path) -> String {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(p)
        .map(|m| format!("{:o}", m.permissions().mode() & 0o777))
        .unwrap_or_else(|_| "?".to_string())
}

#[cfg(not(unix))]
fn file_perms(_: &Path) -> String {
    "?".to_string()
}

// ---------------------------------------------------------------------------
// Per-tool event line
// ---------------------------------------------------------------------------

pub enum ToolOutcome<'a> {
    /// Detail like "paid 0.10 USDC" or "tx 4pGR…jYau · 0.10 USDC".
    /// Use `""` if there is nothing extra to say.
    Ok(&'a str),
    /// Detail like "budget: per_call_cap=0.05 exceeded".
    Err(&'a str),
}

pub fn emit_tool_event(tool: &str, network: &str, latency_ms: u128, outcome: ToolOutcome<'_>) {
    let g = glyphs();
    let now = chrono::Utc::now().format("%H:%M:%S").to_string();

    let net_short = network_short(network);
    let net_padded = format!("{net_short:<7}");
    let net_colored = match network {
        "sandbox" => sandbox_color(&net_padded),
        "solana-devnet" => devnet_color(&net_padded),
        "solana-mainnet" => mainnet_color(&net_padded),
        _ => dim(&net_padded),
    };

    let latency_str = format_latency(latency_ms);
    let latency_padded = format!("{latency_str:>6}");

    let (marker_raw, marker_colored, detail_colored): (&str, String, String) = match outcome {
        ToolOutcome::Ok(s) => (g.ok, ok_color(g.ok), ok_color(s)),
        ToolOutcome::Err(s) => (g.fail, fail_color(g.fail), fail_color(s)),
    };
    // Suppress unused-binding lint when ANSI is off; the marker text is
    // already embedded in `marker_colored` (which falls through to plain
    // when ANSI is disabled).
    let _ = marker_raw;

    let tool_padded = format!("{tool:<13}");
    let line = format!(
        "{}  {}  {}  {} {}  {} {}",
        dim(&now),
        brand_white(&bold(&tool_padded)),
        net_colored,
        accent(g.clock),
        fg256(80, &latency_padded),
        marker_colored,
        detail_colored,
    );

    let stderr = std::io::stderr();
    let mut out = stderr.lock();
    let _ = writeln!(out, "{line}");
}

fn network_short(network: &str) -> &str {
    match network {
        "sandbox" => "sandbox",
        "solana-devnet" => "devnet",
        "solana-mainnet" => "mainnet",
        other => other,
    }
}

fn format_latency(ms: u128) -> String {
    if ms < 1_000 {
        format!("{ms}ms")
    } else {
        // Render as seconds with one decimal, e.g. "2.3s".
        let secs = (ms as f64) / 1_000.0;
        format!("{secs:.1}s")
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_pubkey_long() {
        let pk = "GmBDzsdcPBNpeGchxX2GkZTKYtuCKnj7wyHiYaL9zPEm";
        let out = truncate_pubkey(pk);
        assert!(out.starts_with("GmBDzs"));
        assert!(out.ends_with("9zPEm") || out.ends_with("zPEm"));
        assert!(out.contains('\u{2026}'));
    }

    #[test]
    fn truncate_pubkey_short_is_pass_through() {
        assert_eq!(truncate_pubkey("abc"), "abc");
    }

    #[test]
    fn format_latency_subsecond() {
        assert_eq!(format_latency(0), "0ms");
        assert_eq!(format_latency(16), "16ms");
        assert_eq!(format_latency(999), "999ms");
    }

    #[test]
    fn format_latency_seconds() {
        assert_eq!(format_latency(1_000), "1.0s");
        assert_eq!(format_latency(2_345), "2.3s");
        assert_eq!(format_latency(10_500), "10.5s");
    }

    #[test]
    fn network_short_known() {
        assert_eq!(network_short("sandbox"), "sandbox");
        assert_eq!(network_short("solana-devnet"), "devnet");
        assert_eq!(network_short("solana-mainnet"), "mainnet");
        assert_eq!(network_short("weird"), "weird");
    }
}
