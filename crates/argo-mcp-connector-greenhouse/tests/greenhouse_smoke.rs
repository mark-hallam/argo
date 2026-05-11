//! Live Greenhouse Harvest smoke test.
//!
//! Gated by three env vars; if any is missing the test no-ops so the suite
//! is safe on CI without credentials. To enable locally:
//!
//! ```bash
//! export ARGO_GH_TEST_API_KEY=<Harvest API key>
//! export ARGO_GH_TEST_ON_BEHALF_OF=<Greenhouse user id with write access>
//! export ARGO_GH_TEST_CANDIDATE_ID=<existing candidate id in the same account>
//! cargo test -p argo-mcp-connector-greenhouse --test greenhouse_smoke -- --ignored
//! ```
//!
//! The test reads the candidate back, posts an admin-only note, and asserts
//! the call succeeded. Because Greenhouse does not expose a note-delete on
//! the public API we leave the audit trail in place — pick a low-traffic
//! sandbox candidate.

use argo_mcp_connector_greenhouse::{GhNoteVisibility, GreenhouseConnector};

const KEY_ENV: &str = "ARGO_GH_TEST_API_KEY";
const USER_ENV: &str = "ARGO_GH_TEST_ON_BEHALF_OF";
const CANDIDATE_ENV: &str = "ARGO_GH_TEST_CANDIDATE_ID";

fn read_required(name: &str) -> Option<String> {
    match std::env::var(name) {
        Ok(v) if !v.trim().is_empty() => Some(v),
        _ => None,
    }
}

#[tokio::test]
#[ignore = "requires ARGO_GH_TEST_API_KEY/ON_BEHALF_OF/CANDIDATE_ID"]
async fn live_round_trip_candidate_and_note() {
    let api_key = match read_required(KEY_ENV) {
        Some(v) => v,
        None => return,
    };
    let user_id: i64 = match read_required(USER_ENV).and_then(|v| v.parse().ok()) {
        Some(v) => v,
        None => return,
    };
    let candidate_id: i64 = match read_required(CANDIDATE_ENV).and_then(|v| v.parse().ok()) {
        Some(v) => v,
        None => return,
    };

    let gh = GreenhouseConnector::new(api_key).expect("build Greenhouse connector");

    // 1) Read the candidate back. Confirms basic auth + endpoint reachability.
    let candidate = gh
        .get_candidate(candidate_id)
        .await
        .expect("get_candidate succeeds");
    assert_eq!(candidate.id, candidate_id);

    // 2) Post an admin-only note. Confirms write-path auth + On-Behalf-Of.
    let body = format!(
        "argo-smoke {} — automated round-trip, ignore",
        chrono::Utc::now().to_rfc3339()
    );
    gh.add_note(candidate_id, user_id, &body, GhNoteVisibility::Admin)
        .await
        .expect("add_note succeeds");
}
