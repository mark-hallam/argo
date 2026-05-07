//! Unit tests for bearer-token auth on POST dispatch.
//! Loaded via `#[cfg(test)] #[path = "routes_dispatch_post_tests.rs"] mod tests;`
//!
//! Env vars are process-global. Tests serialize on `ENV_LOCK` and reset
//! the two vars they touch at start AND end so order-of-execution does
//! not affect outcomes.

use super::super::routes_auth::check_bearer_auth;
use std::sync::Mutex;

static ENV_LOCK: Mutex<()> = Mutex::new(());

const TOKEN_KEY: &str = "ARGO_GATEWAY_BEARER_TOKEN";
const BYPASS_KEY: &str = "ARGO_GATEWAY_AUTH_BYPASS";

fn reset_env() {
    std::env::remove_var(TOKEN_KEY);
    std::env::remove_var(BYPASS_KEY);
}

fn req(auth_header: Option<&str>) -> String {
    let mut s = String::from("POST /sessions HTTP/1.1\r\n");
    if let Some(value) = auth_header {
        s.push_str(&format!("Authorization: {value}\r\n"));
    }
    s.push_str("Host: localhost\r\n\r\n");
    s
}

#[test]
fn no_token_configured_passes() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    reset_env();
    assert!(check_bearer_auth(&req(None)).is_none());
    assert!(check_bearer_auth(&req(Some("Bearer anything"))).is_none());
    reset_env();
}

#[test]
fn token_configured_correct_header_passes() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    reset_env();
    std::env::set_var(TOKEN_KEY, "xyz");
    assert!(check_bearer_auth(&req(Some("Bearer xyz"))).is_none());
    reset_env();
}

#[test]
fn token_configured_wrong_header_returns_401() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    reset_env();
    std::env::set_var(TOKEN_KEY, "xyz");
    let wrong = check_bearer_auth(&req(Some("Bearer nope"))).expect("expected 401");
    assert_eq!(wrong.0, "401 Unauthorized");
    assert!(wrong.1.contains("missing or invalid bearer token"));
    let missing = check_bearer_auth(&req(None)).expect("expected 401");
    assert_eq!(missing.0, "401 Unauthorized");
    reset_env();
}

#[test]
fn bypass_env_overrides_wrong_token() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    reset_env();
    std::env::set_var(TOKEN_KEY, "xyz");
    std::env::set_var(BYPASS_KEY, "1");
    assert!(check_bearer_auth(&req(Some("Bearer nope"))).is_none());
    assert!(check_bearer_auth(&req(None)).is_none());
    reset_env();
}
