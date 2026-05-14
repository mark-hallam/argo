//! Classification tests, split out of `routes_retrain_tests.rs` to
//! keep that file under the 500-LOC budget. Loaded as a child mod
//! via `#[path]` from the parent test file. Tests are reachable as
//! `routes_retrain::tests::classify::*`.

use super::{
    add_worker, classify_response, env_lock, json_of, mem_store, req_with, set_test_env,
    workspace_tenant,
};
use argo_types::TenantId;
use serde_json::json;
use uuid::Uuid;

#[tokio::test]
async fn classify_memory_fact_is_default() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().unwrap();
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Maya").await;
    let wid = worker.id.to_string();
    let req = req_with(
        "POST",
        &format!("/api/v1/workers/{wid}/retrain/classify"),
        &json!({ "input": "Customer Acme prefers email contact" }).to_string(),
    );
    let (status, body) = classify_response(&wid, &req, Some(&store)).await;
    let v = json_of(status, &body).await;
    assert_eq!(v["classification"], "memory_fact");
    assert!(v["plain_english"].as_str().unwrap().starts_with("Remember:"));
    assert_eq!(v["structural_change"]["text"], "Customer Acme prefers email contact");
    assert!(v["regression_preview"].is_array());
    assert!(v["regression_preview"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn classify_vocabulary_patch_detected() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().unwrap();
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Maya").await;
    let wid = worker.id.to_string();
    let req = req_with(
        "POST",
        &format!("/api/v1/workers/{wid}/retrain/classify"),
        &json!({ "input": "Use customers instead of users" }).to_string(),
    );
    let (status, body) = classify_response(&wid, &req, Some(&store)).await;
    let v = json_of(status, &body).await;
    assert_eq!(v["classification"], "vocabulary_patch");
    assert_eq!(v["structural_change"]["from"], "users");
    assert_eq!(v["structural_change"]["to"], "customers");
}

#[tokio::test]
async fn classify_action_ban_detected() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().unwrap();
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Maya").await;
    let wid = worker.id.to_string();
    let req = req_with(
        "POST",
        &format!("/api/v1/workers/{wid}/retrain/classify"),
        &json!({ "input": "Stop doing refunds without approval" }).to_string(),
    );
    let (status, body) = classify_response(&wid, &req, Some(&store)).await;
    let v = json_of(status, &body).await;
    assert_eq!(v["classification"], "action_ban");
    assert_eq!(v["structural_change"]["action_type"], "refunds");
}

#[tokio::test]
async fn classify_policy_rule_detected_with_action_type() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().unwrap();
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Maya").await;
    let wid = worker.id.to_string();
    let req = req_with(
        "POST",
        &format!("/api/v1/workers/{wid}/retrain/classify"),
        &json!({ "input": "Always require approval before send_email" }).to_string(),
    );
    let (status, body) = classify_response(&wid, &req, Some(&store)).await;
    let v = json_of(status, &body).await;
    assert_eq!(v["classification"], "policy_rule");
    assert_eq!(v["structural_change"]["action_type"], "send_email");
}

#[tokio::test]
async fn classify_skill_correction_includes_needs_generalization() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().unwrap();
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Maya").await;
    let wid = worker.id.to_string();
    let req = req_with(
        "POST",
        &format!("/api/v1/workers/{wid}/retrain/classify"),
        &json!({ "input": "Next time say it differently" }).to_string(),
    );
    let (status, body) = classify_response(&wid, &req, Some(&store)).await;
    let v = json_of(status, &body).await;
    assert_eq!(v["classification"], "skill_correction");
    assert!(v["needs_generalization"].is_object());
    assert!(v["needs_generalization"]["question"].is_string());
    let opts = v["needs_generalization"]["options"].as_array().unwrap();
    assert_eq!(opts.len(), 2);
}

#[tokio::test]
async fn classify_empty_input_returns_400() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().unwrap();
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Maya").await;
    let wid = worker.id.to_string();
    let req = req_with(
        "POST",
        &format!("/api/v1/workers/{wid}/retrain/classify"),
        &json!({ "input": "   " }).to_string(),
    );
    let (status, _body) = classify_response(&wid, &req, Some(&store)).await;
    assert_eq!(status, "400 Bad Request");
}

#[tokio::test]
async fn classify_unknown_worker_returns_404() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().unwrap();
    set_test_env(tmp.path());
    let store = mem_store().await;
    let _ = workspace_tenant(&store).await;
    let ghost = Uuid::now_v7().to_string();
    let req = req_with(
        "POST",
        &format!("/api/v1/workers/{ghost}/retrain/classify"),
        &json!({ "input": "x" }).to_string(),
    );
    let (status, _) = classify_response(&ghost, &req, Some(&store)).await;
    assert_eq!(status, "404 Not Found");
}

#[tokio::test]
async fn classify_cross_tenant_returns_404() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().unwrap();
    set_test_env(tmp.path());
    let store = mem_store().await;
    let _ = workspace_tenant(&store).await;
    let other = TenantId::new();
    let alien = add_worker(&store, other, "Alien").await;
    let wid = alien.id.to_string();
    let req = req_with(
        "POST",
        &format!("/api/v1/workers/{wid}/retrain/classify"),
        &json!({ "input": "anything" }).to_string(),
    );
    let (status, _) = classify_response(&wid, &req, Some(&store)).await;
    assert_eq!(status, "404 Not Found");
}
