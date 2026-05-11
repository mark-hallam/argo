//! Integration smoke for `ReplicateConnector`.
//!
//! Gated by the `ARGO_REPLICATE_TEST_TOKEN` env var. When unset the test
//! prints a skip line and returns success — CI without secrets is a
//! green pass. When set, the test runs one cheap end-to-end prediction
//! against the live Replicate API and asserts `Succeeded` within 60s.
//!
//! The "cheap model" is `meta/meta-llama-3-8b-instruct` with
//! `max_new_tokens=1`, costing fractional cents per invocation.
//! Heavyweight image models (SDXL / Flux) are intentionally avoided —
//! ~$0.05/run is too expensive for routine CI.

use argo_mcp_connector_replicate::{PredictionStatus, ReplicateConnector};
use std::time::Duration;

const SMOKE_TOKEN_ENV: &str = "ARGO_REPLICATE_TEST_TOKEN";
const SMOKE_MODEL: &str = "meta/meta-llama-3-8b-instruct";

#[tokio::test]
async fn end_to_end_cheap_prediction_succeeds() {
    let Ok(token) = std::env::var(SMOKE_TOKEN_ENV) else {
        eprintln!("skip: {SMOKE_TOKEN_ENV} not set");
        return;
    };
    if token.trim().is_empty() {
        eprintln!("skip: {SMOKE_TOKEN_ENV} empty");
        return;
    }

    let rc = ReplicateConnector::new(token)
        .expect("connector should build with a non-empty token");
    let prediction = rc
        .run_prediction(
            SMOKE_MODEL,
            serde_json::json!({
                "prompt": "say hi",
                "max_new_tokens": 1
            }),
            Duration::from_secs(60),
        )
        .await
        .expect("end-to-end prediction should not error");
    assert_eq!(
        prediction.status,
        PredictionStatus::Succeeded,
        "smoke prediction should reach Succeeded; got {:?}, error={:?}",
        prediction.status,
        prediction.error
    );
    assert!(
        prediction.output.is_some(),
        "Succeeded prediction must carry output"
    );
    if let Some(metrics) = &prediction.metrics {
        eprintln!(
            "smoke: predict_time={:?}s total_time={:?}s",
            metrics.predict_time, metrics.total_time
        );
    }
}

#[tokio::test]
async fn account_round_trips_when_credentials_present() {
    let Ok(token) = std::env::var(SMOKE_TOKEN_ENV) else {
        eprintln!("skip: {SMOKE_TOKEN_ENV} not set");
        return;
    };
    if token.trim().is_empty() {
        eprintln!("skip: {SMOKE_TOKEN_ENV} empty");
        return;
    }

    let rc = ReplicateConnector::new(token)
        .expect("connector should build with a non-empty token");
    let info = rc.account().await.expect("account should round-trip");
    assert!(!info.username.is_empty(), "username should be populated");
    assert!(
        info.type_ == "user" || info.type_ == "organization",
        "type_ should be a known account kind, got: {}",
        info.type_
    );
    // Second call inside the 60s TTL — exercise the cache against real
    // latency. We can't observe the cache directly, but it must not
    // error and must return identical data.
    let again = rc.account().await.expect("cached account should round-trip");
    assert_eq!(info, again);
}
