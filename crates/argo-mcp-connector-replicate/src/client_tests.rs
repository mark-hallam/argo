//! Wiremock-backed unit tests for `ReplicateConnector`.
//!
//! Six scenarios per the brief:
//!  1. `run_prediction` → 201 already `succeeded` returns immediately.
//!  2. `run_prediction` → 201 `starting`, then 2 polls of `processing`,
//!     then `succeeded`.
//!  3. `run_prediction` → terminal `failed` returns typed error w/ message.
//!  4. `get_model` → 200 returns `ModelInfo` with license + version.
//!  5. `cancel_prediction` → 200 returns `Ok(())`.
//!  6. `account` → second call within TTL does NOT hit the API.
//!
//! Plus two extras to lock down the brief's error contract: 401 surfaces
//! the auth marker; 404 on `get_prediction` surfaces typed error, not a
//! panic.

use super::*;
use crate::types::PredictionStatus;
use std::time::Duration;
use wiremock::matchers::{body_partial_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const TEST_TOKEN: &str = "r8_test_token";
const EXPECTED_AUTH: &str = "Token r8_test_token";

fn connector(server: &MockServer) -> ReplicateConnector {
    ReplicateConnector::new(TEST_TOKEN.into())
        .unwrap()
        .with_base_url(server.uri())
        .with_poll_backoff_ms(vec![0, 0, 0, 0, 0])
        .with_poll_tick(Duration::from_millis(1))
}

fn prediction_body(id: &str, status: &str, output: serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "id": id,
        "model": "meta/meta-llama-3-8b-instruct",
        "status": status,
        "output": output,
        "error": null,
        "metrics": {
            "predict_time": 0.42,
            "total_time": 1.21,
            "input_token_count": 6,
            "output_token_count": 1
        },
        "started_at": "2026-05-11T10:00:00Z",
        "completed_at": "2026-05-11T10:00:01Z"
    })
}

#[tokio::test]
async fn prediction_success_immediate() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/predictions"))
        .and(header("authorization", EXPECTED_AUTH))
        .and(body_partial_json(serde_json::json!({
            "model": "meta/meta-llama-3-8b-instruct"
        })))
        .respond_with(
            ResponseTemplate::new(201).set_body_json(prediction_body(
                "pred_immediate",
                "succeeded",
                serde_json::json!(["hi"]),
            )),
        )
        .expect(1)
        .mount(&server)
        .await;

    let rc = connector(&server);
    let result = rc
        .run_prediction(
            "meta/meta-llama-3-8b-instruct",
            serde_json::json!({"prompt": "hello", "max_new_tokens": 1}),
            Duration::from_secs(60),
        )
        .await
        .unwrap();
    assert_eq!(result.id.as_str(), "pred_immediate");
    assert_eq!(result.status, PredictionStatus::Succeeded);
    assert_eq!(
        result.output,
        Some(serde_json::json!(["hi"])),
        "output passthrough"
    );
    let metrics = result.metrics.expect("metrics surfaced");
    assert_eq!(metrics.predict_time, Some(0.42));
    assert_eq!(metrics.input_token_count, Some(6));
}

#[tokio::test]
async fn prediction_success_after_polling_two_ticks() {
    let server = MockServer::start().await;
    // 1. POST → 201 with status=starting.
    Mock::given(method("POST"))
        .and(path("/v1/predictions"))
        .respond_with(
            ResponseTemplate::new(201).set_body_json(prediction_body(
                "pred_poll",
                "starting",
                serde_json::Value::Null,
            )),
        )
        .expect(1)
        .mount(&server)
        .await;
    // 2. First GET → still processing.
    Mock::given(method("GET"))
        .and(path("/v1/predictions/pred_poll"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(prediction_body(
                "pred_poll",
                "processing",
                serde_json::Value::Null,
            )),
        )
        .up_to_n_times(1)
        .expect(1)
        .mount(&server)
        .await;
    // 3. Second GET → terminal succeeded.
    Mock::given(method("GET"))
        .and(path("/v1/predictions/pred_poll"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(prediction_body(
                "pred_poll",
                "succeeded",
                serde_json::json!("done"),
            )),
        )
        .expect(1)
        .mount(&server)
        .await;

    let rc = connector(&server);
    let result = rc
        .run_prediction(
            "meta/meta-llama-3-8b-instruct",
            serde_json::json!({}),
            Duration::from_secs(60),
        )
        .await
        .unwrap();
    assert_eq!(result.status, PredictionStatus::Succeeded);
    assert_eq!(result.output.as_ref().and_then(|v| v.as_str()), Some("done"));
}

#[tokio::test]
async fn prediction_failed_returns_typed_error_payload() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/predictions"))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "id": "pred_fail",
            "model": "meta/meta-llama-3-8b-instruct",
            "status": "failed",
            "output": null,
            "error": "model exploded: OOM",
            "metrics": null,
            "started_at": null,
            "completed_at": null
        })))
        .expect(1)
        .mount(&server)
        .await;

    let rc = connector(&server);
    let result = rc
        .run_prediction(
            "meta/meta-llama-3-8b-instruct",
            serde_json::json!({}),
            Duration::from_secs(60),
        )
        .await
        .unwrap();
    assert_eq!(result.status, PredictionStatus::Failed);
    assert_eq!(result.error.as_deref(), Some("model exploded: OOM"));
    assert!(result.output.is_none());
}

#[tokio::test]
async fn get_model_returns_capability_and_license() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/models/stability-ai/sdxl"))
        .and(header("authorization", EXPECTED_AUTH))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "owner": "stability-ai",
            "name": "sdxl",
            "description": "Stable Diffusion XL",
            "default_example": {"prompt": "a cat"},
            "latest_version": {"id": "v_001abc"},
            "run_count": 9_999_999_u64,
            "cover_image_url": "https://example.test/cover.png",
            "license_url": "https://example.test/license"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let rc = connector(&server);
    let info = rc.get_model("stability-ai", "sdxl").await.unwrap();
    assert_eq!(info.owner, "stability-ai");
    assert_eq!(info.name, "sdxl");
    assert_eq!(info.latest_version_id, "v_001abc");
    assert_eq!(info.run_count, 9_999_999);
    assert_eq!(info.license_url.as_deref(), Some("https://example.test/license"));
    assert!(info.default_example.is_some());
}

#[tokio::test]
async fn cancel_prediction_returns_ok_on_200() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/predictions/pred_cancel/cancel"))
        .and(header("authorization", EXPECTED_AUTH))
        .respond_with(ResponseTemplate::new(200).set_body_string(""))
        .expect(1)
        .mount(&server)
        .await;

    let rc = connector(&server);
    rc.cancel_prediction(&PredictionId::from("pred_cancel".to_string()))
        .await
        .expect("cancel should succeed on 200");
}

#[tokio::test]
async fn account_cached_for_ttl_window() {
    let server = MockServer::start().await;
    // The endpoint is expected EXACTLY ONCE — second call within TTL must
    // resolve from the cache and not hit the server.
    Mock::given(method("GET"))
        .and(path("/v1/account"))
        .and(header("authorization", EXPECTED_AUTH))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "type": "user",
            "username": "ada",
            "name": "Ada Lovelace"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let rc = connector(&server).with_account_cache_ttl(Duration::from_secs(60));
    let first = rc.account().await.unwrap();
    let second = rc.account().await.unwrap();
    assert_eq!(first, second);
    assert_eq!(first.username, "ada");
    assert_eq!(first.type_, "user");
    // Drop server — its `expect(1)` assertion fires on Drop and will
    // panic the test if the second call had hit it.
    drop(server);
}

#[tokio::test]
async fn unauthorized_surfaces_auth_marker_not_panic() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/account"))
        .respond_with(ResponseTemplate::new(401).set_body_string("Unauthorized"))
        .expect(1)
        .mount(&server)
        .await;

    let rc = connector(&server);
    let err = rc.account().await.unwrap_err();
    match err {
        ArgoError::Provider(msg) => {
            assert!(
                msg.contains("auth"),
                "401 should surface auth marker, got: {msg}"
            );
        }
        other => panic!("expected ArgoError::Provider, got {other:?}"),
    }
}

#[tokio::test]
async fn not_found_on_get_prediction_surfaces_typed_error() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/predictions/ghost"))
        .respond_with(ResponseTemplate::new(404).set_body_string("no such prediction"))
        .expect(1)
        .mount(&server)
        .await;

    let rc = connector(&server);
    let err = rc
        .get_prediction(&PredictionId::from("ghost".to_string()))
        .await
        .unwrap_err();
    match err {
        ArgoError::Provider(msg) => {
            assert!(
                msg.contains("not found"),
                "404 should surface not-found marker, got: {msg}"
            );
        }
        other => panic!("expected ArgoError::Provider, got {other:?}"),
    }
}
