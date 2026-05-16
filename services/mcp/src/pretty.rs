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
// Domain-token colors (must match the frontend syntax-highlighter palette)
// ---------------------------------------------------------------------------
//
// signature (#c084fc) → 141   light purple
// pubkey    (#7dd3fc) → 117   light sky
// usdc      (#34d399) →  79   mint green
// timestamp (#fcd34d) → 222   amber
// uuid      (#94a3b8) → 145   cool grey
// url       (#10b981) →  38   accent green, underlined
// kv-key                245   dim grey

fn color_signature(s: &str) -> String {
    fg256(141, s)
}
fn color_pubkey(s: &str) -> String {
    fg256(117, s)
}
fn color_usdc(s: &str) -> String {
    fg256(79, s)
}
fn color_timestamp(s: &str) -> String {
    fg256(222, s)
}
fn color_uuid(s: &str) -> String {
    fg256(145, s)
}
fn color_url(s: &str) -> String {
    if supports_ansi() {
        format!("\x1b[4m{}\x1b[24m", fg256(38, s))
    } else {
        s.to_string()
    }
}
fn color_kv_key(s: &str) -> String {
    fg256(245, s)
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
        ToolOutcome::Ok(s) => (g.ok, ok_color(g.ok), render_detail(s, ok_color)),
        ToolOutcome::Err(s) => (g.fail, fail_color(g.fail), render_detail(s, fail_color)),
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
// Domain-aware detail tokenizer
// ---------------------------------------------------------------------------
//
// The detail strings produced by `main.rs` mix several token kinds:
//   "tx 4pGR…jYau · 0.10 USDC"
//   "daily=25.00 per_call=1.00"
//   "budget: per_call_cap=0.05 exceeded"
//   "12/256 entries"
//
// `tokenize_detail` walks the string left-to-right, greedy, recognizing:
//   - URLs              ("http(s)://…" up to whitespace)
//   - RFC3339 prefixes  (YYYY-MM-DDThh:mm:ss…)
//   - UUIDs             (8-4-4-4-12 hex)
//   - Truncated sigs    ([base58]+…[base58]+, U+2026 separator)
//   - Long base58 runs  → signature (86..=88) or pubkey (32..=44)
//   - key=value         (with USDC-ish key heuristic for value class)
//   - Bare \d+\.\d+ followed by " USDC" literal → usdc
//
// All other spans fall through as `Plain` and inherit the outcome's
// base color (green for ok, red for err).

#[derive(Debug, PartialEq, Eq)]
pub enum SegmentKind {
    Plain,
    Signature,
    Pubkey,
    Usdc,
    Timestamp,
    Uuid,
    Url,
}

#[derive(Debug, PartialEq, Eq)]
pub enum DetailSegment<'a> {
    Plain(&'a str),
    Signature(&'a str),
    Pubkey(&'a str),
    Usdc(&'a str),
    Timestamp(&'a str),
    Uuid(&'a str),
    Url(&'a str),
    Kv {
        key: &'a str,
        sep: char,
        value: &'a str,
        value_kind: SegmentKind,
    },
}

/// Base58 alphabet check (Bitcoin / Solana): `[1-9A-HJ-NP-Za-km-z]`.
/// Excludes `0`, `O`, `I`, `l`.
fn is_base58_char(c: char) -> bool {
    matches!(c, '1'..='9'
        | 'A'..='H'
        | 'J'..='N'
        | 'P'..='Z'
        | 'a'..='k'
        | 'm'..='z')
}

/// Hex digit (lowercase) for UUID matching.
fn is_hex_lower(c: char) -> bool {
    c.is_ascii_digit() || matches!(c, 'a'..='f')
}

/// Key tokens (case-insensitive) whose value should be colored as USDC.
const USDC_KEYS: &[&str] = &[
    "usdc",
    "usd",
    "amount",
    "balance",
    "price",
    "cap",
    "spent",
    "value",
    "cost",
    "daily",
    "per_call",
    "per_call_cap",
    "max_amount",
];

fn key_implies_usdc(key: &str) -> bool {
    let lower = key.to_ascii_lowercase();
    USDC_KEYS
        .iter()
        .any(|needle| lower == *needle || lower.ends_with(needle))
}

/// True if `s` looks like an RFC3339-prefixed timestamp at offset 0:
/// "YYYY-MM-DDTHH:MM:SS". Returns the byte length of the matched prefix,
/// extended through any contiguous trailing offset/fraction characters
/// (digits, `.`, `+`, `-`, `:`, `Z`).
fn match_rfc3339_prefix(s: &str) -> Option<usize> {
    let b = s.as_bytes();
    if b.len() < 19 {
        return None;
    }
    let digit = |i: usize| b[i].is_ascii_digit();
    let want = |i: usize, c: u8| b[i] == c;
    if !(digit(0) && digit(1) && digit(2) && digit(3) && want(4, b'-'))
        || !(digit(5) && digit(6) && want(7, b'-'))
        || !(digit(8) && digit(9) && want(10, b'T'))
        || !(digit(11) && digit(12) && want(13, b':'))
        || !(digit(14) && digit(15) && want(16, b':'))
        || !(digit(17) && digit(18))
    {
        return None;
    }
    // Extend through any trailing offset / fractional-second characters.
    let mut end = 19;
    while end < b.len() {
        let c = b[end];
        let is_trailing =
            c.is_ascii_digit() || c == b'.' || c == b'+' || c == b'-' || c == b':' || c == b'Z';
        if !is_trailing {
            break;
        }
        end += 1;
    }
    Some(end)
}

/// True if `s` starts with a UUID at offset 0 (lowercase hex,
/// 8-4-4-4-12). Returns the matched byte length (36) on success.
fn match_uuid_prefix(s: &str) -> Option<usize> {
    let b = s.as_bytes();
    if b.len() < 36 {
        return None;
    }
    let groups = [(0, 8), (9, 13), (14, 18), (19, 23), (24, 36)];
    let dashes = [8, 13, 18, 23];
    for &d in &dashes {
        if b[d] != b'-' {
            return None;
        }
    }
    for (start, end) in groups {
        for &byte in &b[start..end] {
            let c = byte as char;
            if !is_hex_lower(c) {
                return None;
            }
        }
    }
    Some(36)
}

/// Match a bare unsigned decimal `\d+\.\d+` at offset 0. Returns the
/// matched byte length. Only ASCII digits + a single `.` are considered.
fn match_decimal_prefix(s: &str) -> Option<usize> {
    let b = s.as_bytes();
    let mut i = 0;
    while i < b.len() && b[i].is_ascii_digit() {
        i += 1;
    }
    if i == 0 || i >= b.len() || b[i] != b'.' {
        return None;
    }
    let dot = i;
    i += 1;
    let frac_start = i;
    while i < b.len() && b[i].is_ascii_digit() {
        i += 1;
    }
    if i == frac_start {
        return None;
    }
    let _ = dot;
    Some(i)
}

/// Match a URL prefix at offset 0 (`http://` or `https://`). The URL
/// runs until the next whitespace character or end of string. Returns
/// the matched byte length.
fn match_url_prefix(s: &str) -> Option<usize> {
    let lowered_head = &s.as_bytes()[..s.len().min(8)];
    let starts_http = lowered_head.starts_with(b"http://") || lowered_head.starts_with(b"https://");
    if !starts_http {
        return None;
    }
    let end = s
        .char_indices()
        .find_map(|(i, c)| if c.is_whitespace() { Some(i) } else { None })
        .unwrap_or(s.len());
    Some(end)
}

/// Match a maximal base58 run starting at offset 0. Returns matched
/// byte length (== char count since all base58 chars are ASCII).
fn match_base58_run(s: &str) -> usize {
    s.chars().take_while(|&c| is_base58_char(c)).count()
}

/// Match a truncated signature pattern at offset 0:
/// `[base58]+…[base58]+`. Returns matched byte length on success.
fn match_truncated_sig_prefix(s: &str) -> Option<usize> {
    let head_len = match_base58_run(s);
    if head_len < 2 {
        return None;
    }
    let rest = &s[head_len..];
    let mut iter = rest.char_indices();
    let (ell_idx, ell_char) = iter.next()?;
    if ell_char != '\u{2026}' {
        return None;
    }
    let ell_end = ell_idx + ell_char.len_utf8();
    let tail = &rest[ell_end..];
    let tail_len = match_base58_run(tail);
    if tail_len < 2 {
        return None;
    }
    Some(head_len + ell_end + tail_len)
}

/// Match a `key=value` (or `key:value`) span at offset 0 where the key
/// is an identifier (`[A-Za-z_][A-Za-z0-9_]*`). Returns
/// `(key_len, sep, value_len)`, where `value_len` covers contiguous
/// non-whitespace bytes after the separator.
fn match_kv_prefix(s: &str) -> Option<(usize, char, usize)> {
    let b = s.as_bytes();
    if b.is_empty() {
        return None;
    }
    let first = b[0];
    if !(first.is_ascii_alphabetic() || first == b'_') {
        return None;
    }
    let mut i = 1;
    while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_') {
        i += 1;
    }
    if i == b.len() {
        return None;
    }
    let sep = b[i];
    if sep != b'=' {
        // Conservative: do not treat plain `:` as kv to avoid clobbering
        // colon-prefixed prose like "budget:".
        return None;
    }
    let key_len = i;
    let value_start = i + 1;
    // Value runs until whitespace.
    let mut j = value_start;
    while j < b.len() && !(b[j] as char).is_whitespace() {
        j += 1;
    }
    let value_len = j - value_start;
    if value_len == 0 {
        return None;
    }
    Some((key_len, sep as char, value_len))
}

/// Classify a kv value into a `SegmentKind` based on its shape + the
/// USDC-key heuristic. Used to color the right-hand side of `daily=25.00`.
fn classify_kv_value(key: &str, value: &str) -> SegmentKind {
    if let Some(end) = match_url_prefix(value) {
        if end == value.len() {
            return SegmentKind::Url;
        }
    }
    if let Some(end) = match_uuid_prefix(value) {
        if end == value.len() {
            return SegmentKind::Uuid;
        }
    }
    if let Some(end) = match_rfc3339_prefix(value) {
        if end == value.len() {
            return SegmentKind::Timestamp;
        }
    }
    if let Some(end) = match_truncated_sig_prefix(value) {
        if end == value.len() {
            return SegmentKind::Signature;
        }
    }
    let b58_len = match_base58_run(value);
    if b58_len == value.len() {
        if (86..=88).contains(&b58_len) {
            return SegmentKind::Signature;
        }
        if (32..=44).contains(&b58_len) {
            return SegmentKind::Pubkey;
        }
    }
    if key_implies_usdc(key) {
        if let Some(end) = match_decimal_prefix(value) {
            if end == value.len() {
                return SegmentKind::Usdc;
            }
        }
    }
    // A pure decimal under a usdc-implying key also flows above; numeric
    // values without decimal point inherit Plain.
    SegmentKind::Plain
}

/// Tokenize a detail string into typed spans, greedy left-to-right.
pub fn tokenize_detail(input: &str) -> Vec<DetailSegment<'_>> {
    let mut out: Vec<DetailSegment<'_>> = Vec::new();
    if input.is_empty() {
        return out;
    }
    // `plain_start` tracks the byte offset where the current accreting
    // plain-text run began. We never set it to `None` — we always reset
    // it to the *new* `i` immediately after flushing, so a single
    // `usize` is sufficient.
    let mut plain_start: usize = 0;
    let mut i: usize = 0;
    let bytes = input.as_bytes();

    // Flush helper is inlined via macro to keep the borrow-checker
    // happy: a closure capturing `input` would extend its lifetime to
    // `'static`, but a `macro_rules!` substitution stays bound to the
    // function-local borrow.
    macro_rules! flush_plain {
        ($upto:expr) => {{
            let upto: usize = $upto;
            if upto > plain_start {
                out.push(DetailSegment::Plain(&input[plain_start..upto]));
            }
        }};
    }

    while i < bytes.len() {
        // Only attempt token matches at "word boundaries": start of
        // string, or after whitespace / punctuation. This avoids
        // grabbing a base58 run that starts mid-word.
        let at_boundary = i == 0
            || {
                let prev = bytes[i - 1] as char;
                prev.is_whitespace() || matches!(prev, '(' | '[' | '{' | ',' | ';' | ':' | '·')
            }
            || {
                // U+00B7 MIDDLE DOT is multi-byte (0xC2 0xB7) — handle
                // it explicitly by scanning the previous char via
                // string slice.
                let head = &input[..i];
                head.ends_with('\u{00b7}')
            };

        if at_boundary {
            // 1. URL
            if let Some(end) = match_url_prefix(&input[i..]) {
                flush_plain!(i);
                out.push(DetailSegment::Url(&input[i..i + end]));
                i += end;
                plain_start = i;
                continue;
            }
            // 2. RFC3339 timestamp
            if let Some(end) = match_rfc3339_prefix(&input[i..]) {
                flush_plain!(i);
                out.push(DetailSegment::Timestamp(&input[i..i + end]));
                i += end;
                plain_start = i;
                continue;
            }
            // 3. UUID
            if let Some(end) = match_uuid_prefix(&input[i..]) {
                flush_plain!(i);
                out.push(DetailSegment::Uuid(&input[i..i + end]));
                i += end;
                plain_start = i;
                continue;
            }
            // 4. key=value
            if let Some((klen, sep, vlen)) = match_kv_prefix(&input[i..]) {
                let key = &input[i..i + klen];
                let value = &input[i + klen + 1..i + klen + 1 + vlen];
                let value_kind = classify_kv_value(key, value);
                flush_plain!(i);
                out.push(DetailSegment::Kv {
                    key,
                    sep,
                    value,
                    value_kind,
                });
                i += klen + 1 + vlen;
                plain_start = i;
                continue;
            }
            // 5. Truncated signature (4pGR…jYau)
            if let Some(end) = match_truncated_sig_prefix(&input[i..]) {
                flush_plain!(i);
                out.push(DetailSegment::Signature(&input[i..i + end]));
                i += end;
                plain_start = i;
                continue;
            }
            // 6. Long base58 run → signature / pubkey. We must NOT
            // gobble the next word, so peek at the char after the run:
            // if it is alphanumeric (continuation of a non-base58 word),
            // skip the classification.
            let run = match_base58_run(&input[i..]);
            if run > 0 {
                let after = input[i + run..].chars().next();
                let bounded = after
                    .map(|c| !c.is_ascii_alphanumeric() && c != '_')
                    .unwrap_or(true);
                if bounded {
                    if (86..=88).contains(&run) {
                        flush_plain!(i);
                        out.push(DetailSegment::Signature(&input[i..i + run]));
                        i += run;
                        plain_start = i;
                        continue;
                    }
                    if (32..=44).contains(&run) {
                        flush_plain!(i);
                        out.push(DetailSegment::Pubkey(&input[i..i + run]));
                        i += run;
                        plain_start = i;
                        continue;
                    }
                }
            }
            // 7. Bare decimal followed by " USDC" → usdc.
            if let Some(end) = match_decimal_prefix(&input[i..]) {
                let tail = &input[i + end..];
                if tail.starts_with(" USDC") || tail.starts_with(" usdc") {
                    flush_plain!(i);
                    out.push(DetailSegment::Usdc(&input[i..i + end]));
                    i += end;
                    plain_start = i;
                    continue;
                }
            }
        }

        // Default: advance one UTF-8 char into the running plain span.
        // `plain_start` is already pointing at the start of the run
        // (either 0 or the byte just past the most recently emitted
        // typed segment), so we don't need to update it here.
        let step = match input[i..].chars().next() {
            Some(c) => c.len_utf8(),
            None => 1,
        };
        i += step;
    }
    flush_plain!(bytes.len());
    out
}

/// Render a tokenized detail string with per-segment ANSI colors.
/// Plain spans inherit the outcome's `base_color` (green for ok, red
/// for err). When ANSI is unavailable, every helper returns its input
/// unchanged, so the output collapses to plain ASCII.
pub fn render_detail(input: &str, base_color: fn(&str) -> String) -> String {
    if input.is_empty() {
        return String::new();
    }
    let segs = tokenize_detail(input);
    let mut out = String::with_capacity(input.len() * 2);
    for seg in segs {
        match seg {
            DetailSegment::Plain(t) => out.push_str(&base_color(t)),
            DetailSegment::Signature(t) => out.push_str(&color_signature(t)),
            DetailSegment::Pubkey(t) => out.push_str(&color_pubkey(t)),
            DetailSegment::Usdc(t) => out.push_str(&color_usdc(t)),
            DetailSegment::Timestamp(t) => out.push_str(&color_timestamp(t)),
            DetailSegment::Uuid(t) => out.push_str(&color_uuid(t)),
            DetailSegment::Url(t) => out.push_str(&color_url(t)),
            DetailSegment::Kv {
                key,
                sep,
                value,
                value_kind,
            } => {
                out.push_str(&color_kv_key(key));
                out.push_str(&color_kv_key(&sep.to_string()));
                match value_kind {
                    SegmentKind::Signature => out.push_str(&color_signature(value)),
                    SegmentKind::Pubkey => out.push_str(&color_pubkey(value)),
                    SegmentKind::Usdc => out.push_str(&color_usdc(value)),
                    SegmentKind::Timestamp => out.push_str(&color_timestamp(value)),
                    SegmentKind::Uuid => out.push_str(&color_uuid(value)),
                    SegmentKind::Url => out.push_str(&color_url(value)),
                    SegmentKind::Plain => out.push_str(&base_color(value)),
                }
            }
        }
    }
    out
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

    // -----------------------------------------------------------------
    // Detail tokenizer
    // -----------------------------------------------------------------

    #[test]
    fn tokenize_detail_empty_input() {
        assert!(tokenize_detail("").is_empty());
    }

    #[test]
    fn tokenize_detail_plain_passthrough() {
        let segs = tokenize_detail("no payment required");
        assert_eq!(segs.len(), 1);
        assert!(matches!(
            segs[0],
            DetailSegment::Plain("no payment required")
        ));
    }

    #[test]
    fn tokenize_detail_recognizes_signature_and_usdc() {
        // Real 88-char base58 signature lifted from PROJECT_CONTEXT.md.
        let input = "tx 4pGRMVgu7j5itCs7Vf6G9FTQW2Q1B2SjCEKHszLjvF9eVagWvtWq8aJWuYz1JNpBQr4CsbYRXSb9aWAu5hv6jYau \u{00b7} 0.10 USDC";
        let segs = tokenize_detail(input);
        let mut saw_sig = false;
        let mut saw_usdc = false;
        let mut saw_usdc_word_plain = false;
        for seg in &segs {
            match seg {
                DetailSegment::Signature(t) => {
                    assert_eq!(t.len(), 88);
                    saw_sig = true;
                }
                DetailSegment::Usdc(t) => {
                    assert_eq!(*t, "0.10");
                    saw_usdc = true;
                }
                DetailSegment::Plain(t) if t.contains("USDC") => {
                    saw_usdc_word_plain = true;
                }
                _ => {}
            }
        }
        assert!(saw_sig, "expected signature segment, got {segs:?}");
        assert!(saw_usdc, "expected usdc segment, got {segs:?}");
        assert!(
            saw_usdc_word_plain,
            "expected `USDC` word as plain, got {segs:?}"
        );
    }

    #[test]
    fn tokenize_detail_recognizes_truncated_sig() {
        let input = "tx 4pGR\u{2026}jYau \u{00b7} 0.10 USDC";
        let segs = tokenize_detail(input);
        let trunc = segs
            .iter()
            .find_map(|s| match s {
                DetailSegment::Signature(t) if t.contains('\u{2026}') => Some(*t),
                _ => None,
            })
            .expect("expected a truncated-signature segment");
        assert_eq!(trunc, "4pGR\u{2026}jYau");
    }

    #[test]
    fn tokenize_detail_recognizes_kv_usdc() {
        let input = "daily=25.00 per_call=1.00";
        let segs = tokenize_detail(input);
        let kvs: Vec<_> = segs
            .iter()
            .filter_map(|s| match s {
                DetailSegment::Kv {
                    key,
                    value,
                    value_kind,
                    ..
                } => Some((*key, *value, value_kind)),
                _ => None,
            })
            .collect();
        assert_eq!(kvs.len(), 2, "expected 2 kv segments, got {segs:?}");
        assert_eq!(kvs[0].0, "daily");
        assert_eq!(kvs[0].1, "25.00");
        assert_eq!(*kvs[0].2, SegmentKind::Usdc);
        assert_eq!(kvs[1].0, "per_call");
        assert_eq!(kvs[1].1, "1.00");
        assert_eq!(*kvs[1].2, SegmentKind::Usdc);
    }

    #[test]
    fn tokenize_detail_recognizes_url_underline() {
        let input = "see https://solscan.io/tx/abc?cluster=devnet for proof";
        let segs = tokenize_detail(input);
        let url = segs
            .iter()
            .find_map(|s| match s {
                DetailSegment::Url(t) => Some(*t),
                _ => None,
            })
            .expect("expected url segment");
        assert!(url.starts_with("https://"));
        assert!(!url.contains(' '));
    }

    #[test]
    fn tokenize_detail_recognizes_uuid() {
        let input = "id=550e8400-e29b-41d4-a716-446655440000";
        let segs = tokenize_detail(input);
        let kv = segs
            .iter()
            .find_map(|s| match s {
                DetailSegment::Kv {
                    key,
                    value,
                    value_kind,
                    ..
                } => Some((*key, *value, value_kind)),
                _ => None,
            })
            .expect("expected kv segment");
        assert_eq!(kv.0, "id");
        assert_eq!(*kv.2, SegmentKind::Uuid);
    }

    #[test]
    fn tokenize_detail_does_not_panic_on_unicode_soup() {
        let input = "\u{00b7}\u{2026}\u{1f680} mixed key=val 1.5 USDC \u{2014} done";
        let _ = tokenize_detail(input);
    }

    #[test]
    fn render_detail_empty_is_empty() {
        assert_eq!(render_detail("", ok_color), "");
    }

    #[test]
    fn render_detail_plain_string_no_panic() {
        // Even when NO_COLOR makes color helpers no-ops, plain input
        // should round-trip through render_detail unchanged.
        let out = render_detail("no payment required", ok_color);
        // The output should *contain* the original text (possibly with
        // ANSI escapes wrapping it).
        let stripped: String = out
            .chars()
            .filter(|c| !c.is_control() && *c != '[' || c.is_alphanumeric() || c.is_whitespace())
            .collect();
        assert!(stripped.contains("no payment required") || out.contains("no payment required"));
    }
}
