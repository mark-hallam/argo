//! Wiremock-backed unit tests for `LeverConnector`.
//!
//! Seven scenarios per the brief:
//!  1. `list_candidates` round-trips a `{data,hasNext,next}` page.
//!  2. `list_candidates` accumulates multiple pages via the `offset` cursor.
//!  3. `list_candidates(since)` passes the `updated_at_start` query param.
//!  4. `get_candidate` returns `Ok(Some(_))` on 200, `Ok(None)` on 404.
//!  5. `add_note` POSTs `{"value": "<note>"}` to the candidate's notes path.
//!  6. `advance_stage` validates against `list_stages` then POSTs `{"stage": "<id>"}`.
//!  7. `advance_stage` with an unknown stage id fails pre-flight with a Schema error.

use super::*;
use wiremock::matchers::{body_json_string, header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn connector(server: &MockServer) -> LeverConnector {
    LeverConnector::new("test-key".into())
        .unwrap()
        .with_base_url(server.uri())
}

#[tokio::test]
async fn list_candidates_round_trips_single_page() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/opportunities"))
        .and(header(
            "authorization",
            // base64("test-key:") = "dGVzdC1rZXk6"
            "Basic dGVzdC1rZXk6",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [
                {
                    "id": "opp-1",
                    "name": "Ada Lovelace",
                    "headline": "Senior Engineer, Acme",
                    "emails": ["ada@example.test"],
                    "phones": ["+15551234567"],
                    "stage": "stage-phone-screen",
                    "postings": ["posting-1", "posting-2"],
                    "updatedAt": 1_715_000_000_000_i64
                },
                {
                    "id": "opp-2",
                    "name": "Grace Hopper",
                    "headline": "",
                    "emails": [],
                    "phones": [],
                    "postings": []
                }
            ],
            "hasNext": false
        })))
        .expect(1)
        .mount(&server)
        .await;

    let lv = connector(&server);
    let candidates = lv.list_candidates(None).await.unwrap();
    assert_eq!(candidates.len(), 2);
    assert_eq!(candidates[0].id, "opp-1");
    assert_eq!(candidates[0].emails, vec!["ada@example.test"]);
    assert_eq!(candidates[0].phones, vec!["+15551234567"]);
    assert_eq!(candidates[0].posting_ids.len(), 2);
    assert_eq!(candidates[0].stage_id.as_deref(), Some("stage-phone-screen"));
    assert!(candidates[1].stage_id.is_none());
}

#[tokio::test]
async fn list_candidates_follows_cursor_across_pages() {
    let server = MockServer::start().await;
    // Page 1: no `offset` query param, hasNext=true with next cursor.
    Mock::given(method("GET"))
        .and(path("/v1/opportunities"))
        .and(wiremock::matchers::query_param_is_missing("offset"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [{"id": "opp-1", "name": "First", "postings": []}],
            "hasNext": true,
            "next": "cursor-page-2"
        })))
        .expect(1)
        .mount(&server)
        .await;
    // Page 2: `offset=cursor-page-2`, hasNext=false.
    Mock::given(method("GET"))
        .and(path("/v1/opportunities"))
        .and(query_param("offset", "cursor-page-2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [{"id": "opp-2", "name": "Second", "postings": []}],
            "hasNext": false
        })))
        .expect(1)
        .mount(&server)
        .await;

    let lv = connector(&server);
    let candidates = lv.list_candidates(None).await.unwrap();
    assert_eq!(candidates.len(), 2);
    assert_eq!(candidates[0].id, "opp-1");
    assert_eq!(candidates[1].id, "opp-2");
}

#[tokio::test]
async fn list_candidates_passes_updated_at_start_query_param() {
    use chrono::TimeZone;
    let server = MockServer::start().await;
    let since = Utc.with_ymd_and_hms(2026, 5, 1, 0, 0, 0).unwrap();
    Mock::given(method("GET"))
        .and(path("/v1/opportunities"))
        .and(query_param("updated_at_start", "1777593600000"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [],
            "hasNext": false
        })))
        .expect(1)
        .mount(&server)
        .await;

    let lv = connector(&server);
    let candidates = lv.list_candidates(Some(since)).await.unwrap();
    assert!(candidates.is_empty());
}

#[tokio::test]
async fn get_candidate_some_on_200_none_on_404() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/opportunities/opp-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": {
                "id": "opp-1",
                "name": "Ada Lovelace",
                "emails": ["ada@example.test"],
                "postings": []
            }
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1/opportunities/opp-missing"))
        .respond_with(ResponseTemplate::new(404).set_body_string(""))
        .expect(1)
        .mount(&server)
        .await;

    let lv = connector(&server);
    let hit = lv.get_candidate("opp-1").await.unwrap();
    assert_eq!(hit.as_ref().map(|c| c.id.as_str()), Some("opp-1"));
    assert_eq!(hit.unwrap().emails, vec!["ada@example.test"]);
    let miss = lv.get_candidate("opp-missing").await.unwrap();
    assert!(miss.is_none());
}

#[tokio::test]
async fn add_note_posts_value_body() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/opportunities/opp-1/notes"))
        .and(header(
            "authorization",
            "Basic dGVzdC1rZXk6",
        ))
        .and(body_json_string(r#"{"value":"strong on-site, recommend offer"}"#))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "data": {"id": "note-9", "value": "strong on-site, recommend offer"}
        })))
        .expect(1)
        .mount(&server)
        .await;

    let lv = connector(&server);
    lv.add_note("opp-1", "strong on-site, recommend offer")
        .await
        .unwrap();
}

#[tokio::test]
async fn advance_stage_validates_then_posts_stage_body() {
    let server = MockServer::start().await;
    // list_stages — single page with two stages.
    Mock::given(method("GET"))
        .and(path("/v1/stages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [
                {"id": "stage-phone-screen", "text": "Phone Screen"},
                {"id": "stage-onsite", "text": "Onsite"}
            ],
            "hasNext": false
        })))
        .expect(1)
        .mount(&server)
        .await;
    // advance_stage — POST /v1/opportunities/opp-1/stage.
    Mock::given(method("POST"))
        .and(path("/v1/opportunities/opp-1/stage"))
        .and(body_json_string(r#"{"stage":"stage-onsite"}"#))
        .respond_with(ResponseTemplate::new(200).set_body_string(""))
        .expect(1)
        .mount(&server)
        .await;

    let lv = connector(&server);
    lv.advance_stage("opp-1", "stage-onsite").await.unwrap();
}

#[tokio::test]
async fn advance_stage_rejects_unknown_stage_id_pre_flight() {
    let server = MockServer::start().await;
    // Only list_stages is mounted. The POST should NEVER be issued because
    // the unknown stage id must fail at pre-flight.
    Mock::given(method("GET"))
        .and(path("/v1/stages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": [{"id": "stage-phone-screen", "text": "Phone Screen"}],
            "hasNext": false
        })))
        .expect(1)
        .mount(&server)
        .await;

    let lv = connector(&server);
    let err = lv
        .advance_stage("opp-1", "stage-nonexistent")
        .await
        .unwrap_err();
    assert!(matches!(err, ArgoError::Schema(_)), "got {err:?}");
}
