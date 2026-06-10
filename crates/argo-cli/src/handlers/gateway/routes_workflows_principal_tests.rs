//! Dev-bypass principal-tolerance tests for the workflow routes
//! (`fix-frontier-b-backend-routes`). Split out of
//! `routes_workflows_tests.rs` to keep both files under the 500-LOC
//! ceiling.
//!
//! The Scene-5 workflow-head GET (`/api/v1/verticals/<v>/workflows/<n>`)
//! 401'd when the SPA sent it header-less in recording mode.
//! `resolve_principal` must now derive the same default dev-bypass
//! principal the bearer gate uses when `gateway.auth_bypass` is on (and
//! NOT production), while STILL requiring the header in production. These
//! env-var tests serialize on a local lock + reset the vars they touch
//! (mirrors `routes_auth_tests.rs`).

use super::resolve_principal;
use argo_types::Principal;

const BYPASS_KEY: &str = "ARGO_GATEWAY_AUTH_BYPASS";
const PROFILE_KEY: &str = "ARGO_GATEWAY_PROFILE";
const BIND_KEY: &str = "ARGO_GATEWAY_BIND";

fn reset_auth_env() {
    std::env::remove_var(BYPASS_KEY);
    std::env::remove_var(PROFILE_KEY);
    std::env::remove_var(BIND_KEY);
}

#[test]
fn resolve_principal_honours_present_header_regardless_of_mode() {
    let _g = crate::handlers::gateway::shared_env_mutex().blocking_lock_owned();
    reset_auth_env();
    let req = "GET /api/v1/verticals/v/workflows/n HTTP/1.1\r\n\
               X-Argo-Principal: human:demo@skycot.com\r\nHost: x\r\n\r\n";
    let (principal, role) = resolve_principal(req).expect("present header must resolve");
    match principal {
        Principal::Gateway { adapter, identity } => {
            assert_eq!(adapter, "human");
            assert_eq!(identity, "demo@skycot.com");
        }
        other => panic!("expected Gateway principal; got {other:?}"),
    }
    assert_eq!(role, "guest", "no JWT → least-privilege guest");
    reset_auth_env();
}

#[test]
fn resolve_principal_derives_dev_bypass_when_header_absent_and_bypass_on() {
    let _g = crate::handlers::gateway::shared_env_mutex().blocking_lock_owned();
    reset_auth_env();
    std::env::set_var(BYPASS_KEY, "1");
    let req = "GET /api/v1/verticals/v/workflows/n HTTP/1.1\r\nHost: x\r\n\r\n";
    let (principal, _role) =
        resolve_principal(req).expect("header-less call must resolve under auth_bypass");
    match principal {
        Principal::Gateway { adapter, identity } => {
            assert_eq!(adapter, "system");
            assert_eq!(identity, "dev-bypass");
        }
        other => panic!("expected dev-bypass Gateway principal; got {other:?}"),
    }
    reset_auth_env();
}

#[test]
fn resolve_principal_401s_when_header_absent_and_bypass_off() {
    let _g = crate::handlers::gateway::shared_env_mutex().blocking_lock_owned();
    reset_auth_env(); // bypass NOT set → simulates the production-shaped path
    let req = "GET /api/v1/verticals/v/workflows/n HTTP/1.1\r\nHost: x\r\n\r\n";
    let resp = resolve_principal(req).expect_err("header-less + no bypass must 401");
    assert_eq!(resp.0, "401 Unauthorized");
    reset_auth_env();
}

#[test]
fn resolve_principal_401s_on_malformed_header_even_under_bypass() {
    let _g = crate::handlers::gateway::shared_env_mutex().blocking_lock_owned();
    reset_auth_env();
    std::env::set_var(BYPASS_KEY, "1");
    // Present-but-malformed (no colon) is a client error regardless of mode.
    let req = "GET /api/v1/verticals/v/workflows/n HTTP/1.1\r\n\
               X-Argo-Principal: malformed-no-colon\r\nHost: x\r\n\r\n";
    let resp = resolve_principal(req).expect_err("malformed header must 401");
    assert_eq!(resp.0, "401 Unauthorized");
    reset_auth_env();
}
