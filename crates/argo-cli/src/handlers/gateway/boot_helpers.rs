//! Leaf helpers extracted from `boot.rs` to keep that file under the
//! 500-LOC ceiling: response status-line parsing, the per-phase
//! startup profiler, and the deprecated `check_bind_loopback` test
//! guard. None of these items have callers outside the `boot` module
//! (or its `#[cfg(test)]` sibling), so the public surface of
//! `super::boot` is unchanged — `boot.rs` re-imports
//! `check_bind_loopback` under `#[cfg(test)]` so the existing
//! `super::check_bind_loopback` path in `boot_tests.rs` still resolves.

#[cfg(test)]
use argo_types::{ArgoError, ArgoResult};
#[cfg(test)]
use std::net::IpAddr;
use std::time::{Duration, Instant};

/// Parse the leading numeric status code out of a status-line like
/// `"200 OK"` / `"404 Not Found"`. Returns 0 if the line is malformed
/// so the metric label stays well-typed without panicking on bad input.
pub(super) fn parse_status_line(status_line: &str) -> u16 {
    status_line
        .split_whitespace()
        .next()
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(0)
}

/// Map a numeric status code to a stable string label suitable for
/// `gateway_request_total{status=...}`. The frequently-observed 4xx /
/// 5xx codes get their own labels; everything else collapses to a
/// `Nxx` family bucket so cardinality stays bounded.
pub(super) fn http_status_label(code: u16) -> &'static str {
    match code {
        100..=199 => "1xx",
        200..=299 => "2xx",
        300..=399 => "3xx",
        400 => "400",
        401 => "401",
        402 => "4xx",
        403 => "403",
        404 => "404",
        405..=428 => "4xx",
        429 => "429",
        430..=499 => "4xx",
        500 => "500",
        501 => "5xx",
        502 => "502",
        503 => "503",
        504 => "504",
        505..=599 => "5xx",
        _ => "unknown",
    }
}

/// Per-phase wall-clock recorder used by `--profile-startup`. Each
/// `mark()` call records the elapsed time since the previous boundary
/// under a stable `&'static str` label. `emit()` writes a stable,
/// machine-grep-able summary to stderr (one line per phase + a `total`
/// line). The struct itself stays well under 30 LOC so boot.rs keeps
/// headroom under the 500-LOC ceiling.
pub(super) struct StartupPhases {
    t0: Instant,
    last: Instant,
    items: Vec<(&'static str, Duration)>,
}

impl StartupPhases {
    pub(super) fn new() -> Self {
        let now = Instant::now();
        Self {
            t0: now,
            last: now,
            items: Vec::with_capacity(8),
        }
    }

    pub(super) fn mark(&mut self, label: &'static str) {
        let now = Instant::now();
        self.items.push((label, now.duration_since(self.last)));
        self.last = now;
    }

    pub(super) fn emit(&self) {
        eprintln!("argo gateway: startup phases (ms)");
        for (label, d) in &self.items {
            eprintln!("  phase  {:<20} {:>8.2}", label, d.as_secs_f64() * 1000.0);
        }
        eprintln!(
            "  phase  {:<20} {:>8.2}",
            "total",
            self.last.duration_since(self.t0).as_secs_f64() * 1000.0
        );
    }
}

/// **Deprecated**: superseded by
/// [`super::startup_check::validate_bind_safety`] (which honors the
/// `[gateway.behind_proxy]` contract on top of the
/// `--insecure-bind` flag — see `proxy_guard.rs`). Retained behind
/// `#[cfg(test)]` so the historical `boot_tests` suite continues to
/// exercise the CLI-flag-only path against a default
/// `GatewayBehindProxyConfig`.
#[cfg(test)]
pub(super) fn check_bind_loopback(bind: &str, insecure_bind: bool) -> ArgoResult<()> {
    let is_loopback = bind
        .parse::<IpAddr>()
        .map(|ip| ip.is_loopback())
        .unwrap_or(false);
    if is_loopback {
        return Ok(());
    }
    if insecure_bind {
        eprintln!(
            "\x1b[33margo gateway: binding non-loopback {bind} per --insecure-bind; \
             ensure a TLS-terminating proxy is in front\x1b[0m"
        );
        return Ok(());
    }
    Err(ArgoError::Provider(format!(
        "refusing to bind non-loopback {bind} without --insecure-bind; \
         see docs/skycot/SECURITY.md"
    )))
}
