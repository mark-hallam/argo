//! Unit tests for bearer-token auth on GET routes + the read-only
//! allowlist. Loaded via `#[cfg(test)] #[path = "routes_auth_tests.rs"] mod tests;`
//!
//! Env vars are process-global. Tests serialize on `ENV_LOCK` and reset
//! the two vars they touch at start AND end so order-of-execution does
//! not affect outcomes — same pattern as `routes_dispatch_post_tests.rs`.

use super::{check_bearer_auth, is_public_get};
use std::sync::Mutex;

static ENV_LOCK: Mutex<()> = Mutex::new(());

const TOKEN_KEY: &str = "ARGO_GATEWAY_BEARER_TOKEN";
const BYPASS_KEY: &str = "ARGO_GATEWAY_AUTH_BYPASS";

fn reset_env() {
    std::env::remove_var(TOKEN_KEY);
    std::env::remove_var(BYPASS_KEY);
}

fn get_req(path: &str, auth_header: Option<&str>) -> String {
    let mut s = format!("GET {path} HTTP/1.1\r\n");
    if let Some(value) = auth_header {
        s.push_str(&format!("Authorization: {value}\r\n"));
    }
    s.push_str("Host: localhost\r\n\r\n");
    s
}

/// Mirrors the wiring inside `routes::dispatch_route`. Lets tests
/// exercise allowlist-then-auth as a unit instead of stubbing the
/// dispatcher's full call graph.
fn gate(req: &str) -> Option<(&'static str, String)> {
    if req.starts_with("GET ") && !is_public_get(req) {
        return check_bearer_auth(req);
    }
    None
}

#[test]
fn no_token_configured_passes_protected_and_public() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    reset_env();
    // Public allowlist member.
    assert!(gate(&get_req("/health", None)).is_none());
    // Protected resource — no token configured → still passes (dev default).
    assert!(gate(&get_req("/sessions", None)).is_none());
    reset_env();
}

#[test]
fn token_configured_correct_header_passes_protected() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    reset_env();
    std::env::set_var(TOKEN_KEY, "xyz");
    assert!(gate(&get_req("/sessions", Some("Bearer xyz"))).is_none());
    assert!(gate(&get_req("/audit/recent", Some("Bearer xyz"))).is_none());
    reset_env();
}

#[test]
fn token_configured_wrong_header_returns_401_on_protected() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    reset_env();
    std::env::set_var(TOKEN_KEY, "xyz");
    let wrong = gate(&get_req("/sessions", Some("Bearer nope"))).expect("expected 401");
    assert_eq!(wrong.0, "401 Unauthorized");
    assert!(wrong.1.contains("missing or invalid bearer token"));
    let missing = gate(&get_req("/sessions", None)).expect("expected 401");
    assert_eq!(missing.0, "401 Unauthorized");
    reset_env();
}

#[test]
fn health_allowlist_passes_with_wrong_token() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    reset_env();
    std::env::set_var(TOKEN_KEY, "xyz");
    // /health, /health/live, /health/ready, /health/full all bypass auth.
    assert!(gate(&get_req("/health", Some("Bearer nope"))).is_none());
    assert!(gate(&get_req("/health/live", Some("Bearer nope"))).is_none());
    assert!(gate(&get_req("/health/ready", Some("Bearer nope"))).is_none());
    assert!(gate(&get_req("/health/full", Some("Bearer nope"))).is_none());
    assert!(gate(&get_req("/health", None)).is_none());
    reset_env();
}

#[test]
fn version_and_routes_allowlist_passes_with_wrong_token() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    reset_env();
    std::env::set_var(TOKEN_KEY, "xyz");
    assert!(gate(&get_req("/version", Some("Bearer nope"))).is_none());
    assert!(gate(&get_req("/routes", Some("Bearer nope"))).is_none());
    // Root (`GET /`) is also on the allowlist.
    assert!(gate(&get_req("/", None)).is_none());
    // Query strings on allowlist members don't change the verdict.
    assert!(gate(&get_req("/routes?format=json", None)).is_none());
    reset_env();
}

#[test]
fn bypass_env_lets_protected_through() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    reset_env();
    std::env::set_var(TOKEN_KEY, "xyz");
    std::env::set_var(BYPASS_KEY, "1");
    assert!(gate(&get_req("/sessions", Some("Bearer nope"))).is_none());
    assert!(gate(&get_req("/audit/recent", None)).is_none());
    reset_env();
}

#[test]
fn protected_admin_paths_still_require_auth() {
    // Defends the curated allowlist: /doctor, /db/*, /metrics/json,
    // /audit/*, /workspace/*, /search* are NOT public — they ride
    // through `dispatch_get_admin` but must still hold the token.
    let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    reset_env();
    std::env::set_var(TOKEN_KEY, "xyz");
    for path in [
        "/doctor",
        "/db/stats",
        "/metrics/json",
        "/audit/recent",
        "/workspace/info",
        "/search?q=foo",
    ] {
        let resp =
            gate(&get_req(path, Some("Bearer nope"))).unwrap_or_else(|| panic!("expected 401 for {path}"));
        assert_eq!(resp.0, "401 Unauthorized", "path {path}");
    }
    reset_env();
}
