//! Unit tests for `routes_trust_streaks`.
//!
//! Stream `mission-pm-trust-ratchet-ui`. Covers the threshold-source
//! precedence (query param > env var > default) and the
//! query-parsing helper; the streak rollup math itself is tested in
//! `argo-store::trust_streak`, so we don't re-spin a full audit
//! fixture here.

use super::{parse_min, DEFAULT_MIN, ENV_MIN};

/// One serial test covers every env-mutating case. Cargo runs unit
/// tests multi-threaded by default and `std::env::set_var` is a
/// process-wide write; splitting these into separate `#[test]`
/// functions would let them race for the same env var.
#[test]
fn threshold_source_precedence() {
    let saved = std::env::var(ENV_MIN).ok();

    // 1. No param, no env → built-in default.
    std::env::remove_var(ENV_MIN);
    let no_override = "GET /api/v1/workers/abc/trust-streaks HTTP/1.1\r\n\r\n";
    assert_eq!(parse_min(no_override), DEFAULT_MIN);

    // 2. Invalid `min=oops` falls through past the param → default.
    let invalid_param = "GET /api/v1/workers/abc/trust-streaks?min=oops HTTP/1.1\r\n\r\n";
    assert_eq!(parse_min(invalid_param), DEFAULT_MIN);

    // 3. Env-only override.
    std::env::set_var(ENV_MIN, "20");
    assert_eq!(parse_min(no_override), 20);

    // 4. Query param wins over env.
    let with_param = "GET /api/v1/workers/abc/trust-streaks?min=5 HTTP/1.1\r\n\r\n";
    assert_eq!(parse_min(with_param), 5);

    // 5. Restore caller env state.
    match saved {
        Some(v) => std::env::set_var(ENV_MIN, v),
        None => std::env::remove_var(ENV_MIN),
    }
}
