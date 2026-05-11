//! Wiremock-backed unit tests for `GreenhouseConnector`.
//!
//! Six scenarios per the brief:
//!  1. `list_candidates` round-trips an array body with email + applications.
//!  2. `advance_application` fetches the application then POSTs `/move` with
//!     `from_stage_id` + `to_stage_id` and the required `On-Behalf-Of` header.
//!  3. `reject_application` POSTs `/reject` with the rejection reason + notes.
//!  4. `add_note` POSTs to `/activity_feed/notes` with the visibility string.
//!  5. The `On-Behalf-Of` header is set on every write request.
//!  6. A 429 with `Retry-After` is honoured exactly once, then the retry wins.

use super::*;
use crate::types::GhApplicationStatus;
use wiremock::matchers::{body_json_string, header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn connector(server: &MockServer) -> GreenhouseConnector {
    GreenhouseConnector::new("test-key".into())
        .unwrap()
        .with_base_url(server.uri())
}

#[tokio::test]
async fn list_candidates_round_trips_array_body() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/candidates"))
        .and(header(
            "authorization",
            // base64("test-key:") = "dGVzdC1rZXk6"
            "Basic dGVzdC1rZXk6",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            {
                "id": 11,
                "first_name": "Ada",
                "last_name": "Lovelace",
                "email_addresses": [
                    {"value": "ada@example.test", "type": "personal"}
                ],
                "applications": [{"id": 101}, {"id": 102}]
            },
            {
                "id": 12,
                "first_name": "Grace",
                "last_name": "Hopper",
                "email_addresses": [],
                "applications": []
            }
        ])))
        .expect(1)
        .mount(&server)
        .await;

    let gh = connector(&server);
    let candidates = gh.list_candidates(None).await.unwrap();
    assert_eq!(candidates.len(), 2);
    assert_eq!(candidates[0].id, 11);
    assert_eq!(candidates[0].email_addresses, vec!["ada@example.test"]);
    assert_eq!(candidates[0].applications, vec![101, 102]);
    assert_eq!(candidates[1].first_name, "Grace");
    assert!(candidates[1].email_addresses.is_empty());
}

#[tokio::test]
async fn advance_application_fetches_current_stage_then_posts_move() {
    let server = MockServer::start().await;
    // 1) GET /v1/applications/55 — read current_stage.id to derive from_stage_id.
    Mock::given(method("GET"))
        .and(path("/v1/applications/55"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": 55,
            "candidate_id": 11,
            "jobs": [{"id": 7}],
            "current_stage": {"id": 200, "name": "Phone Screen"},
            "status": "active",
            "source": null
        })))
        .expect(1)
        .mount(&server)
        .await;
    // 2) POST /v1/applications/55/move with both stage ids + On-Behalf-Of header.
    Mock::given(method("POST"))
        .and(path("/v1/applications/55/move"))
        .and(header("on-behalf-of", "9001"))
        .and(body_json_string(
            r#"{"from_stage_id":200,"to_stage_id":300}"#,
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": 55,
            "candidate_id": 11,
            "jobs": [{"id": 7}],
            "current_stage": {"id": 300, "name": "Onsite"},
            "status": "active",
            "source": {"public_name": "LinkedIn"}
        })))
        .expect(1)
        .mount(&server)
        .await;

    let gh = connector(&server);
    let updated = gh.advance_application(55, 300, 9001).await.unwrap();
    assert_eq!(updated.id, 55);
    assert_eq!(updated.current_stage.id, 300);
    assert_eq!(updated.current_stage.name, "Onsite");
    assert_eq!(updated.status, GhApplicationStatus::Active);
    assert_eq!(updated.source.as_deref(), Some("LinkedIn"));
}

#[tokio::test]
async fn reject_application_posts_reason_and_notes() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/applications/77/reject"))
        .and(header("on-behalf-of", "9001"))
        .and(body_json_string(
            r#"{"rejection_reason_id":42,"notes":"not a culture fit"}"#,
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": 77,
            "candidate_id": 21,
            "jobs": [{"id": 7}],
            "current_stage": {"id": 999, "name": "Rejected"},
            "status": "rejected",
            "source": null
        })))
        .expect(1)
        .mount(&server)
        .await;

    let gh = connector(&server);
    let rejected = gh
        .reject_application(77, 42, 9001, Some("not a culture fit"))
        .await
        .unwrap();
    assert_eq!(rejected.id, 77);
    assert_eq!(rejected.status, GhApplicationStatus::Rejected);
}

#[tokio::test]
async fn add_note_posts_visibility_string() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/candidates/11/activity_feed/notes"))
        .and(header("on-behalf-of", "9001"))
        .and(body_json_string(
            r#"{"user_id":9001,"body":"@user-321 strong screen","visibility":"public"}"#,
        ))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({"id": 5})))
        .expect(1)
        .mount(&server)
        .await;

    let gh = connector(&server);
    gh.add_note(11, 9001, "@user-321 strong screen", GhNoteVisibility::Public)
        .await
        .unwrap();
}

#[tokio::test]
async fn on_behalf_of_zero_fails_pre_flight() {
    // No mounts: any HTTP call would 404 against an empty wiremock and
    // surface a Provider error. We expect a Schema error before that.
    let server = MockServer::start().await;
    let gh = connector(&server);

    let err = gh.advance_application(55, 300, 0).await.unwrap_err();
    assert!(matches!(err, ArgoError::Schema(_)));

    let err = gh.reject_application(77, 42, 0, None).await.unwrap_err();
    assert!(matches!(err, ArgoError::Schema(_)));

    let err = gh
        .add_note(11, 0, "hi", GhNoteVisibility::Public)
        .await
        .unwrap_err();
    assert!(matches!(err, ArgoError::Schema(_)));
}

#[tokio::test]
async fn rate_limited_429_retry_after_is_honoured_once() {
    let server = MockServer::start().await;
    // First call returns 429 with Retry-After: 0 so the unit test stays fast.
    Mock::given(method("GET"))
        .and(path("/v1/candidates"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("Retry-After", "0")
                .set_body_string(""),
        )
        .up_to_n_times(1)
        .expect(1)
        .mount(&server)
        .await;
    // Retry sees a normal 200.
    Mock::given(method("GET"))
        .and(path("/v1/candidates"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            {
                "id": 99,
                "first_name": "After",
                "last_name": "Retry",
                "email_addresses": [],
                "applications": []
            }
        ])))
        .expect(1)
        .mount(&server)
        .await;

    let gh = connector(&server);
    let candidates = gh.list_candidates(None).await.unwrap();
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].id, 99);
    assert_eq!(candidates[0].first_name, "After");
}

#[tokio::test]
async fn list_candidates_passes_updated_after_query_param() {
    use chrono::TimeZone;
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/candidates"))
        .and(query_param("updated_after", "2026-05-01T00:00:00Z"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([])))
        .expect(1)
        .mount(&server)
        .await;

    let gh = connector(&server);
    let since = Utc.with_ymd_and_hms(2026, 5, 1, 0, 0, 0).unwrap();
    let candidates = gh.list_candidates(Some(since)).await.unwrap();
    assert!(candidates.is_empty());
}
