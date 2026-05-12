//! Tests for `routes_audit_detail` + `routes_audit_screenshot`
//! (Stream J15 — `GET /api/v1/audit/<actionId>` + signed-URL
//! screenshot endpoint).
//!
//! Seeds a `:memory:` store with synthetic `audit_record` rows
//! whose `payload_json` carries the fields the detail composer
//! reads (worker / verb / skill / policy / output / screenshots /
//! external-ref candidates), then drives `detail_response` and
//! `screenshot_response` directly. The bearer-auth gate runs
//! upstream in `routes::dispatch_inner`; these tests target the
//! handlers themselves.

#![allow(clippy::too_many_lines)]

use super::detail_response;
use super::super::routes_audit_screenshot::{screenshot_response, sign_screenshot_url};
use argo_audit::{AuditKind, AuditSink};
use argo_store::SessionStore;
use base64::{engine::general_purpose, Engine as _};
use serde_json::{json, Value};
use std::sync::Arc;

async fn mem_store() -> Arc<SessionStore> {
    let store = SessionStore::open(":memory:").await.expect("open mem db");
    store.migrate().await.expect("migrate");
    Arc::new(store)
}

/// Seed a row through `AuditSink` and read its hex id back so the
/// caller can drive the detail endpoint by id without guessing.
async fn seed_and_id(store: &Arc<SessionStore>, payload: Value) -> String {
    AuditSink::new(Arc::clone(store))
        .write(None, AuditKind::ToolCall, payload, None)
        .await
        .expect("write audit row");
    let id: Vec<u8> = sqlx::query_scalar(
        "SELECT id FROM audit_record WHERE kind = 'tool_call' ORDER BY seq DESC LIMIT 1",
    )
    .fetch_one(store.pool())
    .await
    .expect("read back audit id");
    hex::encode(id)
}

fn get(path: &str) -> String {
    format!("GET {path} HTTP/1.1\r\nHost: x\r\n\r\n")
}

fn parse_body(resp: &(&str, String)) -> Value {
    serde_json::from_str(&resp.1).unwrap_or_else(|_| panic!("invalid JSON: {}", resp.1))
}

fn rich_payload() -> Value {
    let mut p = serde_json::Map::new();
    let strs: &[(&str, &str)] = &[
        ("worker_id", "w-1"),
        ("worker_name", "Triage Worker"),
        ("verb", "issued refund"),
        ("action_type", "refund.create"),
        ("outcome_status", "succeeded"),
        ("approver_id", "u-9"),
        ("approver_name", "Mark Hallam"),
        ("channel", "web"),
        ("data_classification", "internal"),
        ("trigger_kind", "message"),
        ("trigger_summary", "customer asked for refund"),
        ("memory_loaded_snapshot_id", "snap-7"),
        ("memory_loaded_summary", "loaded 4 entries"),
        ("skill_id", "skill-refund"),
        ("skill_version", "v3"),
        ("skill_trust", "review"),
        ("skill_summary", "refund-issuance skill"),
        ("plan_candidates_summary", "two viable paths"),
        ("plan_selected_summary", "chose refund"),
        ("policy_id", "policy-refund-limits"),
        ("policy_outcome", "allow"),
        ("policy_summary", "within $500 cap"),
        ("approval_channel", "web"),
        ("approval_summary", "approved by Mark Hallam"),
        ("executor_kind", "stripe"),
        ("executor_summary", "called Stripe API"),
        ("outcome_summary", "refund landed"),
        ("model_id", "claude-opus-4-7"),
        ("model_version", "1"),
        ("prompt_template_id", "tpl-refund"),
        ("prompt_template_hash", "deadbeef"),
        ("policy_version", "v2"),
        ("policy_hash", "cafefeed"),
        ("signature", "ed25519-sig"),
        ("output_kind", "text"),
        (
            "output_body",
            "refund issued re_3O987654321 — see https://github.com/foo/bar/pull/42 ENG-12",
        ),
    ];
    for (k, v) in strs {
        p.insert((*k).to_string(), Value::String((*v).to_string()));
    }
    p.insert("cost_usd".to_string(), json!(0.12));
    p.insert("predecessor".to_string(), json!({ "id": "prev-1", "verb": "asked for refund" }));
    p.insert("plan_candidates_json".to_string(), json!(["refund", "credit"]));
    p.insert("plan_selected_idx".to_string(), json!(0));
    p.insert("executor_meta".to_string(), json!({ "endpoint": "POST /refunds" }));
    p.insert("downstream_meta".to_string(), json!({ "refund_id": "re_3O123" }));
    Value::Object(p)
}

// ── detail endpoint ─────────────────────────────────────────────

#[tokio::test]
async fn detail_returns_404_on_invalid_hex_id() {
    let store = mem_store().await;
    let resp = detail_response(&get("/api/v1/audit/not-hex"), "not-hex", Some(&store)).await;
    assert_eq!(resp.0, "404 Not Found");
}

#[tokio::test]
async fn detail_returns_404_on_unknown_action_id() {
    let store = mem_store().await;
    let missing = "00112233445566778899aabbccddeeff";
    let resp = detail_response(
        &get(&format!("/api/v1/audit/{missing}")),
        missing,
        Some(&store),
    )
    .await;
    assert_eq!(resp.0, "404 Not Found");
    assert!(resp.1.contains("not_found"));
}

#[tokio::test]
async fn detail_returns_200_with_envelope_for_known_id() {
    let store = mem_store().await;
    let id = seed_and_id(&store, rich_payload()).await;
    let resp = detail_response(&get(&format!("/api/v1/audit/{id}")), &id, Some(&store)).await;
    assert_eq!(resp.0, "200 OK");
    let body = parse_body(&resp);
    let detail = &body["detail"];
    for key in [
        "summary",
        "inputs",
        "decision_chain",
        "provenance",
        "output",
        "external_refs",
    ] {
        assert!(detail.get(key).is_some(), "missing key {key}");
    }
}

#[tokio::test]
async fn detail_summary_carries_worker_verb_cost() {
    let store = mem_store().await;
    let id = seed_and_id(&store, rich_payload()).await;
    let resp = detail_response(&get(&format!("/api/v1/audit/{id}")), &id, Some(&store)).await;
    let body = parse_body(&resp);
    let summary = &body["detail"]["summary"];
    assert_eq!(summary["worker"]["id"], "w-1");
    assert_eq!(summary["worker"]["name"], "Triage Worker");
    assert_eq!(summary["verb"], "issued refund");
    assert_eq!(summary["cost"]["usd"], 0.12);
    assert_eq!(summary["status"], "succeeded");
    assert_eq!(summary["approver"]["id"], "u-9");
    assert_eq!(summary["channel"], "web");
}

#[tokio::test]
async fn detail_chain_has_eight_ordered_steps() {
    let store = mem_store().await;
    let id = seed_and_id(&store, rich_payload()).await;
    let resp = detail_response(&get(&format!("/api/v1/audit/{id}")), &id, Some(&store)).await;
    let body = parse_body(&resp);
    let chain = body["detail"]["decision_chain"].as_array().expect("array");
    assert_eq!(chain.len(), 8);
    for (i, step) in chain.iter().enumerate() {
        assert_eq!(step["n"].as_u64(), Some((i + 1) as u64));
        assert!(step["title"].is_string());
        assert!(step["details"].is_object());
    }
    assert_eq!(chain[1]["details"]["skill_id"], "skill-refund");
    assert_eq!(chain[4]["details"]["policy_id"], "policy-refund-limits");
}

#[tokio::test]
async fn detail_provenance_emits_self_hash_anchor() {
    let store = mem_store().await;
    let id = seed_and_id(&store, rich_payload()).await;
    let row_hash: Vec<u8> = sqlx::query_scalar(
        "SELECT self_hash FROM audit_record WHERE id = ?",
    )
    .bind(hex::decode(&id).unwrap())
    .fetch_one(store.pool())
    .await
    .unwrap();
    let resp = detail_response(&get(&format!("/api/v1/audit/{id}")), &id, Some(&store)).await;
    let body = parse_body(&resp);
    assert_eq!(
        body["detail"]["provenance"]["hash_chain_anchor"],
        hex::encode(&row_hash)
    );
    assert_eq!(body["detail"]["provenance"]["model"]["id"], "claude-opus-4-7");
    assert_eq!(body["detail"]["provenance"]["skill"]["version"], "v3");
}

#[tokio::test]
async fn detail_inputs_carry_predecessor() {
    let store = mem_store().await;
    let id = seed_and_id(&store, rich_payload()).await;
    let resp = detail_response(&get(&format!("/api/v1/audit/{id}")), &id, Some(&store)).await;
    let body = parse_body(&resp);
    let inputs = &body["detail"]["inputs"];
    assert_eq!(inputs["trigger_kind"], "message");
    assert_eq!(inputs["trigger_summary"], "customer asked for refund");
    assert_eq!(inputs["predecessor"]["id"], "prev-1");
}

#[tokio::test]
async fn detail_output_carries_text_body_and_external_refs() {
    let store = mem_store().await;
    let id = seed_and_id(&store, rich_payload()).await;
    let resp = detail_response(&get(&format!("/api/v1/audit/{id}")), &id, Some(&store)).await;
    let body = parse_body(&resp);
    assert_eq!(body["detail"]["output"]["kind"], "text");
    assert!(body["detail"]["output"]["body"].as_str().unwrap().contains("refund issued"));
    let refs = body["detail"]["external_refs"].as_array().expect("array");
    assert!(refs.iter().any(|r| r["system"] == "stripe" && r["id"] == "re_3O987654321"));
    assert!(refs.iter().any(|r| r["system"] == "github"));
    assert!(refs.iter().any(|r| r["system"] == "linear" && r["id"] == "ENG-12"));
}

#[tokio::test]
async fn detail_external_refs_capped_to_thirty() {
    let store = mem_store().await;
    let mut output = String::new();
    for i in 0..50 {
        output.push_str(&format!("re_{i:012x} "));
    }
    let mut payload = rich_payload();
    payload["output_body"] = Value::String(output);
    let id = seed_and_id(&store, payload).await;
    let resp = detail_response(&get(&format!("/api/v1/audit/{id}")), &id, Some(&store)).await;
    let body = parse_body(&resp);
    let refs = body["detail"]["external_refs"].as_array().expect("array");
    assert!(refs.len() <= 30, "got {}", refs.len());
}

#[tokio::test]
async fn detail_redacts_email_in_verb_and_output() {
    let store = mem_store().await;
    let mut payload = rich_payload();
    payload["verb"] = json!("ping sam@acmeco.com about refund");
    payload["output_body"] = json!("notified sam@acmeco.com");
    let id = seed_and_id(&store, payload).await;
    let resp = detail_response(&get(&format!("/api/v1/audit/{id}")), &id, Some(&store)).await;
    let body = parse_body(&resp);
    let verb = body["detail"]["summary"]["verb"].as_str().unwrap();
    let output = body["detail"]["output"]["body"].as_str().unwrap();
    assert!(verb.contains("[EMAIL]"));
    assert!(!verb.contains("sam@acmeco.com"));
    assert!(output.contains("[EMAIL]"));
    assert!(!output.contains("sam@acmeco.com"));
}

#[tokio::test]
async fn detail_screenshots_emit_signed_urls() {
    let store = mem_store().await;
    let png = general_purpose::STANDARD.encode([0x89, 0x50, 0x4e, 0x47]);
    let mut payload = rich_payload();
    payload["screenshots"] = json!([
        { "timestamp": "2026-05-12T18:00:00Z", "description": "step 1", "png_b64": png },
        { "timestamp": "2026-05-12T18:00:05Z", "description": "step 2", "png_b64": png },
    ]);
    let id = seed_and_id(&store, payload).await;
    let resp = detail_response(&get(&format!("/api/v1/audit/{id}")), &id, Some(&store)).await;
    let body = parse_body(&resp);
    let shots = body["detail"]["output"]["screenshots"].as_array().expect("array");
    assert_eq!(shots.len(), 2);
    for (i, shot) in shots.iter().enumerate() {
        let url = shot["url"].as_str().expect("url string");
        assert!(url.contains(&format!("/screenshots/{i}")));
        assert!(url.contains("exp="));
        assert!(url.contains("sig="));
    }
}

#[tokio::test]
async fn detail_view_writes_audit_detail_viewed_row() {
    let store = mem_store().await;
    let id = seed_and_id(&store, rich_payload()).await;
    let before: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_record WHERE payload_json LIKE '%audit.detail.viewed%'",
    )
    .fetch_one(store.pool())
    .await
    .unwrap();
    let _ = detail_response(&get(&format!("/api/v1/audit/{id}")), &id, Some(&store)).await;
    AuditSink::new(Arc::clone(&store))
        .flush_pending()
        .await
        .expect("flush");
    let after: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_record WHERE payload_json LIKE '%audit.detail.viewed%'",
    )
    .fetch_one(store.pool())
    .await
    .unwrap();
    assert_eq!(after, before + 1);
}

#[tokio::test]
async fn detail_returns_503_when_store_missing() {
    let resp = detail_response(&get("/api/v1/audit/abc"), "abc", None).await;
    assert_eq!(resp.0, "503 Service Unavailable");
}

// ── screenshot endpoint ────────────────────────────────────────

fn make_payload_with_one_screenshot() -> (Value, String) {
    let png_bytes: [u8; 8] = [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a];
    let b64 = general_purpose::STANDARD.encode(png_bytes);
    let payload = json!({
        "screenshots": [
            { "timestamp": "2026-05-12T18:00:00Z", "description": "step 1", "png_b64": b64.clone() }
        ],
    });
    (payload, b64)
}

#[tokio::test]
async fn screenshot_returns_200_with_valid_signature() {
    let store = mem_store().await;
    let (payload, b64) = make_payload_with_one_screenshot();
    let id = seed_and_id(&store, payload).await;
    let exp = chrono::Utc::now().timestamp() + 60;
    let url = sign_screenshot_url(&id, 0, exp);
    let qs = url.split_once('?').unwrap().1;
    let resp = screenshot_response(&id, "0", qs, Some(&store)).await;
    assert_eq!(resp.0, "200 OK");
    let body: Value = serde_json::from_str(&resp.1).unwrap();
    assert_eq!(body["kind"], "png");
    assert_eq!(body["b64"], b64);
    assert_eq!(body["size_bytes"], 8);
}

#[tokio::test]
async fn screenshot_401_on_missing_sig() {
    let store = mem_store().await;
    let (payload, _) = make_payload_with_one_screenshot();
    let id = seed_and_id(&store, payload).await;
    let exp = chrono::Utc::now().timestamp() + 60;
    let resp = screenshot_response(&id, "0", &format!("exp={exp}"), Some(&store)).await;
    assert_eq!(resp.0, "401 Unauthorized");
    assert!(resp.1.contains("missing sig"));
}

#[tokio::test]
async fn screenshot_401_on_bad_signature() {
    let store = mem_store().await;
    let (payload, _) = make_payload_with_one_screenshot();
    let id = seed_and_id(&store, payload).await;
    let exp = chrono::Utc::now().timestamp() + 60;
    let qs = format!("exp={exp}&sig=deadbeef");
    let resp = screenshot_response(&id, "0", &qs, Some(&store)).await;
    assert_eq!(resp.0, "401 Unauthorized");
    assert!(resp.1.contains("mismatch"));
}

#[tokio::test]
async fn screenshot_401_on_expired_signature() {
    let store = mem_store().await;
    let (payload, _) = make_payload_with_one_screenshot();
    let id = seed_and_id(&store, payload).await;
    let exp = chrono::Utc::now().timestamp() - 1;
    let url = sign_screenshot_url(&id, 0, exp);
    let qs = url.split_once('?').unwrap().1;
    let resp = screenshot_response(&id, "0", qs, Some(&store)).await;
    assert_eq!(resp.0, "401 Unauthorized");
    assert!(resp.1.contains("expired"));
}

#[tokio::test]
async fn screenshot_404_on_out_of_range_idx() {
    let store = mem_store().await;
    let (payload, _) = make_payload_with_one_screenshot();
    let id = seed_and_id(&store, payload).await;
    let exp = chrono::Utc::now().timestamp() + 60;
    let url = sign_screenshot_url(&id, 5, exp);
    let qs = url.split_once('?').unwrap().1;
    let resp = screenshot_response(&id, "5", qs, Some(&store)).await;
    assert_eq!(resp.0, "404 Not Found");
}

#[tokio::test]
async fn screenshot_400_on_non_numeric_idx() {
    let store = mem_store().await;
    let resp = screenshot_response("abcd", "not-a-number", "", Some(&store)).await;
    assert_eq!(resp.0, "400 Bad Request");
}

#[tokio::test]
async fn screenshot_404_on_unknown_action_id() {
    let store = mem_store().await;
    let missing = "00112233445566778899aabbccddeeff";
    let exp = chrono::Utc::now().timestamp() + 60;
    let url = sign_screenshot_url(missing, 0, exp);
    let qs = url.split_once('?').unwrap().1;
    let resp = screenshot_response(missing, "0", qs, Some(&store)).await;
    assert_eq!(resp.0, "404 Not Found");
}
