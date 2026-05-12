//! Tests for `routes_genesis` — `POST /api/v1/genesis/classify`
//! and `POST /api/v1/genesis/draft-worker` (Stream J4).
//!
//! Auth gating is exercised at the dispatch layer in
//! `routes_dispatch_post_tests.rs`; here we directly invoke the
//! handlers so the test surface stays inside one crate's
//! compile graph. `:memory:` SQLite is used for audit-row
//! verification — the same pattern as `routes_request_audit_tests`.

use super::*;
use argo_store::SessionStore;
use std::sync::Arc;

const VALID_PROBLEM: &str =
    "Marketing intern to handle our weekly campaigns and newsletter";

fn classify_req(body: &str) -> String {
    format!(
        "POST /api/v1/genesis/classify HTTP/1.1\r\nHost: x\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    )
}

fn draft_req(body: &str) -> String {
    format!(
        "POST /api/v1/genesis/draft-worker HTTP/1.1\r\nHost: x\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    )
}

async fn mem_store() -> Arc<SessionStore> {
    let store = SessionStore::open(":memory:").await.expect("open mem db");
    store.migrate().await.expect("migrate");
    Arc::new(store)
}

fn valid_answers_value() -> serde_json::Value {
    serde_json::json!({
        "role": "marketing intern",
        "name": "Maya",
        "industry": "B2B SaaS",
        "duties": ["draft weekly newsletter", "post to LinkedIn", "qualify inbound leads"],
        "channels": ["slack-dm", "email"],
        "spend_cap": 50.0,
    })
}

#[tokio::test]
async fn classify_returns_200_and_canonical_question_set() {
    let store = mem_store().await;
    let body = serde_json::json!({ "problem": VALID_PROBLEM }).to_string();
    let (status, payload) = classify_response(&classify_req(&body), Some(&store)).await;
    assert_eq!(status, "200 OK");
    let json: serde_json::Value = serde_json::from_str(&payload).unwrap();
    assert_eq!(json["classification"], "marketing_intern");
    assert!(json["questions"].is_array());
    assert_eq!(json["questions"].as_array().unwrap().len(), 6);
    assert!(json["cost_usd"].is_null());
}

#[tokio::test]
async fn classify_question_shape_matches_frontend_contract() {
    let store = mem_store().await;
    let body = serde_json::json!({ "problem": VALID_PROBLEM }).to_string();
    let (_, payload) = classify_response(&classify_req(&body), Some(&store)).await;
    let json: serde_json::Value = serde_json::from_str(&payload).unwrap();
    let first = &json["questions"][0];
    for field in ["id", "text", "answer_type", "required"] {
        assert!(
            first.get(field).is_some(),
            "missing required field `{field}` in question shape"
        );
    }
    // channels question carries options[]; role/name/etc don't —
    // serde skip_serializing_if drops the field entirely.
    let channels = json["questions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|q| q["id"] == "channels")
        .expect("channels question present");
    assert_eq!(channels["answer_type"], "multi-select");
    assert!(channels["options"].is_array());
}

#[tokio::test]
async fn classify_rejects_empty_problem_with_400_envelope() {
    let store = mem_store().await;
    let body = serde_json::json!({ "problem": "" }).to_string();
    let (status, payload) = classify_response(&classify_req(&body), Some(&store)).await;
    assert_eq!(status, "400 Bad Request");
    let json: serde_json::Value = serde_json::from_str(&payload).unwrap();
    assert_eq!(json["error"]["code"], "bad_request");
    assert!(json["error"]["message"].as_str().unwrap().contains("15"));
}

#[tokio::test]
async fn classify_rejects_short_problem_below_min_length() {
    let store = mem_store().await;
    let body = serde_json::json!({ "problem": "too short" }).to_string();
    let (status, _) = classify_response(&classify_req(&body), Some(&store)).await;
    assert_eq!(status, "400 Bad Request");
}

#[tokio::test]
async fn classify_rejects_malformed_json_body() {
    let store = mem_store().await;
    let req = classify_req("{not valid json");
    let (status, payload) = classify_response(&req, Some(&store)).await;
    assert_eq!(status, "400 Bad Request");
    let json: serde_json::Value = serde_json::from_str(&payload).unwrap();
    assert_eq!(json["error"]["code"], "bad_request");
}

#[tokio::test]
async fn classify_unmatched_problem_returns_unknown_classification() {
    let store = mem_store().await;
    // No archetype keywords — neutral generic prose.
    let body = serde_json::json!({
        "problem": "xyzzy frobnicate the bazquux for the wibble"
    })
    .to_string();
    let (status, payload) = classify_response(&classify_req(&body), Some(&store)).await;
    assert_eq!(status, "200 OK");
    let json: serde_json::Value = serde_json::from_str(&payload).unwrap();
    assert_eq!(json["classification"], "unknown");
    // Question set is still the canonical 6.
    assert_eq!(json["questions"].as_array().unwrap().len(), 6);
}

#[tokio::test]
async fn classify_writes_audit_row_with_genesis_metadata() {
    let store = mem_store().await;
    let body = serde_json::json!({ "problem": VALID_PROBLEM }).to_string();
    let (_, _) = classify_response(&classify_req(&body), Some(&store)).await;

    let rows: Vec<(String, String)> =
        sqlx::query_as("SELECT kind, payload_json FROM audit_record")
            .fetch_all(store.pool())
            .await
            .unwrap();
    assert_eq!(rows.len(), 1, "exactly one audit row per call");
    assert_eq!(rows[0].0, "http.request");
    let payload: serde_json::Value = serde_json::from_str(&rows[0].1).unwrap();
    assert_eq!(payload["endpoint"], "classify");
    assert_eq!(payload["problem_length"], VALID_PROBLEM.len());
    assert_eq!(payload["question_count"], 6);
    assert!(payload["cost_usd"].is_null());
}

#[tokio::test]
async fn draft_worker_returns_200_and_full_worker_draft() {
    let store = mem_store().await;
    let body = serde_json::json!({
        "problem": VALID_PROBLEM,
        "answers": valid_answers_value(),
    })
    .to_string();
    let (status, payload) = draft_worker_response(&draft_req(&body), Some(&store)).await;
    assert_eq!(status, "200 OK");
    let json: serde_json::Value = serde_json::from_str(&payload).unwrap();
    let draft = &json["worker_draft"];
    for field in [
        "suggestedName",
        "persona",
        "charter",
        "duties",
        "exampleReplies",
        "approvalPolicy",
        "suggestedVertical",
    ] {
        assert!(
            draft.get(field).is_some(),
            "missing field `{field}` in worker_draft"
        );
    }
    assert_eq!(draft["suggestedName"], "Maya");
    assert_eq!(draft["persona"], "maya");
    assert_eq!(draft["duties"].as_array().unwrap().len(), 3);
    assert!(json["cost_usd"].is_null());
}

#[tokio::test]
async fn draft_worker_rejects_missing_required_answer() {
    let store = mem_store().await;
    let mut answers = valid_answers_value();
    answers.as_object_mut().unwrap().remove("duties");
    let body = serde_json::json!({
        "problem": VALID_PROBLEM,
        "answers": answers,
    })
    .to_string();
    let (status, payload) = draft_worker_response(&draft_req(&body), Some(&store)).await;
    assert_eq!(status, "400 Bad Request");
    let json: serde_json::Value = serde_json::from_str(&payload).unwrap();
    assert_eq!(json["error"]["code"], "bad_request");
    assert!(json["error"]["message"]
        .as_str()
        .unwrap()
        .contains("duties"));
}

#[tokio::test]
async fn draft_worker_rejects_invalid_spend_cap() {
    let store = mem_store().await;
    let mut answers = valid_answers_value();
    answers["spend_cap"] = serde_json::json!(-1.0);
    let body = serde_json::json!({
        "problem": VALID_PROBLEM,
        "answers": answers,
    })
    .to_string();
    let (status, _) = draft_worker_response(&draft_req(&body), Some(&store)).await;
    assert_eq!(status, "400 Bad Request");
}

#[tokio::test]
async fn draft_worker_accepts_string_form_lists() {
    // Front-end may submit `duties` as a newline-delimited string
    // before E2-d locks the array form; normaliser tolerates both.
    let store = mem_store().await;
    let body = serde_json::json!({
        "problem": VALID_PROBLEM,
        "answers": {
            "role": "marketing intern",
            "name": "Maya",
            "duties": "draft newsletter\npost to LinkedIn\nqualify leads",
            "channels": "slack-dm,email",
            "spend_cap": "50",
        },
    })
    .to_string();
    let (status, payload) = draft_worker_response(&draft_req(&body), Some(&store)).await;
    assert_eq!(status, "200 OK");
    let json: serde_json::Value = serde_json::from_str(&payload).unwrap();
    assert_eq!(json["worker_draft"]["duties"].as_array().unwrap().len(), 3);
}

#[tokio::test]
async fn draft_worker_uses_adjustment_in_charter() {
    let store = mem_store().await;
    let body = serde_json::json!({
        "problem": VALID_PROBLEM,
        "answers": valid_answers_value(),
        "adjustment": "Please avoid emailing leads on weekends.",
    })
    .to_string();
    let (status, payload) = draft_worker_response(&draft_req(&body), Some(&store)).await;
    assert_eq!(status, "200 OK");
    let json: serde_json::Value = serde_json::from_str(&payload).unwrap();
    let charter = json["worker_draft"]["charter"].as_str().unwrap();
    assert!(charter.contains("Adjustments"));
    assert!(charter.contains("weekends"));
}

#[tokio::test]
async fn draft_worker_rejects_missing_answers_object() {
    let store = mem_store().await;
    let body = serde_json::json!({ "problem": VALID_PROBLEM }).to_string();
    let (status, _) = draft_worker_response(&draft_req(&body), Some(&store)).await;
    assert_eq!(status, "400 Bad Request");
}

#[tokio::test]
async fn draft_worker_writes_audit_row_with_genesis_metadata() {
    let store = mem_store().await;
    let body = serde_json::json!({
        "problem": VALID_PROBLEM,
        "answers": valid_answers_value(),
    })
    .to_string();
    let _ = draft_worker_response(&draft_req(&body), Some(&store)).await;
    let payloads: Vec<String> =
        sqlx::query_scalar("SELECT payload_json FROM audit_record WHERE kind = 'http.request'")
            .fetch_all(store.pool())
            .await
            .unwrap();
    assert_eq!(payloads.len(), 1);
    let p: serde_json::Value = serde_json::from_str(&payloads[0]).unwrap();
    assert_eq!(p["endpoint"], "draft-worker");
    assert_eq!(p["problem_length"], VALID_PROBLEM.len());
}

#[test]
fn canonical_questions_lists_six_questions_with_stable_ids() {
    let qs = canonical_questions();
    let ids: Vec<&str> = qs.iter().map(|q| q.id.as_str()).collect();
    assert_eq!(
        ids,
        vec!["role", "name", "industry", "duties", "channels", "spend_cap"],
    );
    // role, name, duties, channels, spend_cap are required; industry is not.
    let required_count = qs.iter().filter(|q| q.required).count();
    assert_eq!(required_count, 5);
}
