//! Live Lever Hire smoke test.
//!
//! Gated by two env vars; if either is missing the test no-ops so the
//! suite is safe on CI without credentials. To enable locally:
//!
//! ```bash
//! export ARGO_LEVER_TEST_API_KEY=<Lever API key>
//! export ARGO_LEVER_TEST_OPPORTUNITY_ID=<existing opportunity id in the same account>
//! cargo test -p argo-mcp-connector-lever --test lever_smoke -- --ignored
//! ```
//!
//! The test reads the opportunity back and then enumerates the tenant's
//! posting + stage sets. No writes — Lever does not expose a delete API
//! for notes, so the recommended write smoke is a manual `add_note` run
//! against a sandbox account.

use argo_mcp_connector_lever::LeverConnector;

const KEY_ENV: &str = "ARGO_LEVER_TEST_API_KEY";
const OPP_ENV: &str = "ARGO_LEVER_TEST_OPPORTUNITY_ID";

fn read_required(name: &str) -> Option<String> {
    match std::env::var(name) {
        Ok(v) if !v.trim().is_empty() => Some(v),
        _ => None,
    }
}

#[tokio::test]
#[ignore = "requires ARGO_LEVER_TEST_API_KEY/OPPORTUNITY_ID"]
async fn live_round_trip_opportunity_postings_stages() {
    let api_key = match read_required(KEY_ENV) {
        Some(v) => v,
        None => return,
    };
    let opportunity_id = match read_required(OPP_ENV) {
        Some(v) => v,
        None => return,
    };

    let lv = LeverConnector::new(api_key).expect("build Lever connector");

    // 1) Read the opportunity back. Confirms basic auth + envelope decode.
    let candidate = lv
        .get_candidate(&opportunity_id)
        .await
        .expect("get_candidate succeeds")
        .expect("opportunity id resolves");
    assert_eq!(candidate.id, opportunity_id);

    // 2) List postings. Confirms pagination + posting type decoding.
    let postings = lv.list_postings().await.expect("list_postings succeeds");
    assert!(
        !postings.is_empty(),
        "every Lever tenant has at least one posting"
    );

    // 3) List stages. Confirms stage decoding (used by advance_stage validation).
    let stages = lv.list_stages().await.expect("list_stages succeeds");
    assert!(
        !stages.is_empty(),
        "every Lever tenant has at least one stage"
    );
}
