//! Tests for `routes_worker_trust_grade` —
//! `GET /api/v1/workers/<id>/trust-grade` (mission-trust-grade-ui).
//!
//! Uses an in-memory SessionStore + the `ARGO_HOME` env mutex pattern
//! shared with `routes_workers_hire_tests`. Two layers of test:
//! the pure assembler (`build_trust_grade_response`) is driven
//! synchronously without a store; the end-to-end path goes through
//! `show_response` with a real store + persisted worker.

use super::{build_trust_grade_response, grade_for_u8, grade_label, grade_tone, show_response};
use argo_identity::{IdentityStore, Persona, Role, Worker, WorkerId};
use argo_skill::TrustGrade;
use argo_store::SessionStore;
use argo_types::TenantId;
use chrono::Utc;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::{Mutex, MutexGuard};

async fn env_lock() -> MutexGuard<'static, ()> {
    static M: Mutex<()> = Mutex::const_new(());
    M.lock().await
}

fn set_test_env(tmp: &std::path::Path) {
    std::env::set_var("ARGO_HOME", tmp.join("home"));
    std::fs::create_dir_all(tmp.join("home")).expect("home dir");
}

async fn mem_store() -> Arc<SessionStore> {
    let store = SessionStore::open(":memory:").await.expect("open mem db");
    store.migrate().await.expect("migrate");
    Arc::new(store)
}

fn show_req(worker_id: &str) -> String {
    format!(
        "GET /api/v1/workers/{worker_id}/trust-grade HTTP/1.1\r\nHost: x\r\n\r\n"
    )
}

fn fresh_worker(tenant: TenantId) -> Worker {
    Worker::draft(
        tenant,
        "gtm-engineer",
        Persona::named("Maya", "op"),
        Role::new("Sell."),
    )
}

#[test]
fn grade_for_u8_maps_inverse_of_enum_order() {
    // u8 0 is fresh / unknown — least trusted; u8 4 is fully
    // trusted. The TrustGrade enum's Ord is inverse to this (lower
    // variant index = more trusted), so the map below is the
    // authoritative bridge between the two.
    assert_eq!(grade_for_u8(0), TrustGrade::LearnedUnreviewed);
    assert_eq!(grade_for_u8(1), TrustGrade::PublishedCommunity);
    assert_eq!(grade_for_u8(2), TrustGrade::Tenant);
    assert_eq!(grade_for_u8(3), TrustGrade::PublishedVerified);
    assert_eq!(grade_for_u8(4), TrustGrade::Builtin);
    // Saturates at Builtin for anything above 4.
    assert_eq!(grade_for_u8(255), TrustGrade::Builtin);
}

#[test]
fn grade_tone_destructive_for_lowest_success_for_highest() {
    assert_eq!(grade_tone(TrustGrade::LearnedUnreviewed), "destructive");
    assert_eq!(grade_tone(TrustGrade::PublishedCommunity), "warning");
    assert_eq!(grade_tone(TrustGrade::Tenant), "warning");
    assert_eq!(grade_tone(TrustGrade::PublishedVerified), "success");
    assert_eq!(grade_tone(TrustGrade::Builtin), "success");
}

#[test]
fn build_response_shape_fresh_worker_at_grade_zero() {
    // A worker created via Worker::draft has trust_grade = 0 (the
    // serde default for older rows). The assembler should emit the
    // bottom-of-ladder badge, a non-empty criteria array, and the
    // next grade (PublishedCommunity).
    let tenant = TenantId::new();
    let worker = fresh_worker(tenant);
    let body = build_trust_grade_response(&worker, Utc::now());

    assert_eq!(body["current_grade"], "learned_unreviewed");
    assert_eq!(body["current_grade_label"], "Learned, unreviewed");
    assert_eq!(body["current_grade_tone"], "destructive");
    assert_eq!(body["current_grade_index"], 0);
    assert_eq!(body["next_grade"], "published_community");
    assert_eq!(body["at_max_grade"], false);
    assert_eq!(body["signals"]["has_been_used"], false);
    let criteria = body["next_grade_criteria"].as_array().expect("array");
    assert!(!criteria.is_empty(), "criteria should not be empty");
    // At least one criterion should be unmet for a fresh worker.
    let any_unmet = criteria.iter().any(|c| c["met"] == false);
    assert!(any_unmet);
}

#[test]
fn build_response_at_max_grade_omits_next_and_empties_criteria() {
    // Saturating the trust_grade above the ladder caps at Builtin.
    // When already at the max rung, next_grade is null,
    // at_max_grade is true, and the criteria array is empty.
    let tenant = TenantId::new();
    let mut worker = fresh_worker(tenant);
    worker.trust_grade = 4;
    let body = build_trust_grade_response(&worker, Utc::now());

    assert_eq!(body["current_grade"], "builtin");
    assert_eq!(body["current_grade_label"], "Builtin");
    assert_eq!(body["current_grade_tone"], "success");
    assert!(body["next_grade"].is_null());
    assert_eq!(body["at_max_grade"], true);
    let criteria = body["next_grade_criteria"].as_array().expect("array");
    assert!(criteria.is_empty(), "no next grade → no criteria");
}

#[test]
fn build_response_marks_recent_activity_criterion_met() {
    let tenant = TenantId::new();
    let mut worker = fresh_worker(tenant);
    worker.last_used_at = Some(Utc::now());
    let body = build_trust_grade_response(&worker, Utc::now());

    assert_eq!(body["signals"]["has_been_used"], true);
    let criteria = body["next_grade_criteria"].as_array().expect("array");
    let recent = criteria
        .iter()
        .find(|c| {
            c["name"]
                .as_str()
                .is_some_and(|n| n.contains("active in the last 30 days"))
        })
        .expect("recent-activity criterion present");
    assert_eq!(recent["met"], true);
    assert_eq!(recent["current_value"], "yes");
}

#[test]
fn label_strings_are_human_friendly_for_every_grade() {
    assert_eq!(grade_label(TrustGrade::Builtin), "Builtin");
    assert_eq!(grade_label(TrustGrade::PublishedVerified), "Published verified");
    assert_eq!(grade_label(TrustGrade::Tenant), "Tenant");
    assert_eq!(grade_label(TrustGrade::PublishedCommunity), "Published community");
    assert_eq!(grade_label(TrustGrade::LearnedUnreviewed), "Learned, unreviewed");
}

#[tokio::test]
async fn show_response_returns_200_for_existing_worker() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;

    // Ensure the workspace tenant exists, then drop a worker on it.
    // We bind to the same cwd-derived tenant the handler will use.
    let cwd = std::env::current_dir().unwrap();
    let name = cwd.file_name().and_then(|s| s.to_str()).unwrap_or("workspace");
    let root = cwd.to_string_lossy();
    let (tenant, _ws) = store.ensure_workspace(name, &root).await.unwrap();

    let identities = IdentityStore::new(&store);
    let worker = fresh_worker(tenant);
    identities.create_worker(&worker).await.unwrap();
    identities
        .set_worker_trust_grade(tenant, worker.id, 2)
        .await
        .unwrap();

    let (status, body) =
        show_response(&worker.id.to_string(), &show_req(&worker.id.to_string()), Some(&store))
            .await;
    assert_eq!(status, "200 OK", "body: {body}");
    let v: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["worker_id"], worker.id.to_string());
    assert_eq!(v["current_grade"], "tenant");
    assert_eq!(v["current_grade_index"], 2);
}

#[tokio::test]
async fn show_response_404_for_unknown_worker() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let bogus = WorkerId::new().to_string();
    let (status, body) = show_response(&bogus, &show_req(&bogus), Some(&store)).await;
    assert_eq!(status, "404 Not Found", "body: {body}");
    let v: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["error"]["code"], "not_found");
}

#[tokio::test]
async fn show_response_400_for_invalid_worker_id() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let bad = "not-a-uuid";
    let (status, body) = show_response(bad, &show_req(bad), Some(&store)).await;
    assert_eq!(status, "400 Bad Request", "body: {body}");
    let v: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["error"]["code"], "bad_request");
}

#[tokio::test]
async fn show_response_503_when_store_absent() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let bogus = WorkerId::new().to_string();
    let (status, _body) = show_response(&bogus, &show_req(&bogus), None).await;
    assert_eq!(status, "503 Service Unavailable");
}
