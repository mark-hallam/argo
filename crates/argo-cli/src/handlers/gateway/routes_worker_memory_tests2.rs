//! Positive tests for the Frontier-B worker-memory wiring —
//! `active_conversations` (backed by `worker_conversation_thread`,
//! migration v87) and `recent_decisions` (derived from the worker's
//! approval audit rows + the canonical `approval_request` outcome).
//!
//! Split into a sibling module because the original
//! `routes_worker_memory_tests.rs` is at the 500-LOC ceiling. Mirrors its
//! env-lock + tempdir-pinned `ARGO_HOME` + cwd-derived workspace tenant so
//! the handler's `ensure_workspace` resolution is deterministic.

#![allow(clippy::too_many_lines)]

use super::list_response;
use argo_audit::{AuditKind, AuditSink};
use argo_identity::{IdentityStore, Persona, Role, Worker};
use argo_store::approvals::{create_request, decide_request, ApprovalDecision, NewApprovalRequest};
use argo_store::conversation_thread::{insert, ConversationThreadInsert};
use argo_store::SessionStore;
use argo_types::TenantId;
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{Mutex, MutexGuard};

async fn env_lock() -> MutexGuard<'static, ()> {
    static M: Mutex<()> = Mutex::const_new(());
    M.lock().await
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root")
        .to_path_buf()
}

fn set_test_env(tmp: &std::path::Path) {
    std::env::set_var("ARGO_HOME", tmp.join("home"));
    std::fs::create_dir_all(tmp.join("home")).expect("home dir");
    std::env::set_var("ARGO_VERTICALS_DIR", workspace_root().join("verticals"));
}

async fn mem_store() -> Arc<SessionStore> {
    let store = SessionStore::open(":memory:").await.expect("open mem db");
    store.migrate().await.expect("migrate");
    Arc::new(store)
}

async fn workspace_tenant(store: &Arc<SessionStore>) -> TenantId {
    let cwd = std::env::current_dir().expect("cwd");
    let name = cwd
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("workspace");
    let root = cwd.to_string_lossy();
    let (tenant, _) = store
        .ensure_workspace(name, &root)
        .await
        .expect("ensure workspace");
    tenant
}

async fn add_worker(store: &Arc<SessionStore>, tenant: TenantId, name: &str) -> Worker {
    let worker = Worker::draft(
        tenant,
        "recruiting",
        Persona::named(name, "human:mark"),
        Role::new("Screen candidates."),
    );
    IdentityStore::new(store)
        .create_worker(&worker)
        .await
        .expect("create worker");
    worker
}

fn get_req(id: &str) -> String {
    format!(
        "GET /api/v1/workers/{id}/memory HTTP/1.1\r\nHost: x\r\n\
         X-Argo-Role: operator\r\nX-Argo-Principal: human:mark\r\n\r\n"
    )
}

#[tokio::test]
async fn active_conversations_round_trip_from_store() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Priya").await;

    insert(
        store.pool(),
        &ConversationThreadInsert {
            id: "ct_1",
            tenant_id: &tenant.to_string(),
            worker_id: worker.id.0.as_bytes().as_slice(),
            counterparty_handle: "Diego Alvarez",
            topic: "Interview scheduling — Thu 10:30",
            last_message_at: chrono::Utc::now().timestamp_micros(),
            audit_href: Some("/audit/action/ct_1"),
        },
    )
    .await
    .expect("seed conversation thread");

    let (status, body) = list_response(&worker.id.to_string(), &get_req(&worker.id.to_string()), Some(&store)).await;
    assert_eq!(status, "200 OK", "body: {body}");
    let v: Value = serde_json::from_str(&body).unwrap();
    let convs = v["active_conversations"].as_array().expect("array");
    assert_eq!(convs.len(), 1, "expected one seeded thread");
    // camelCase item fields — the SPA card reads these directly.
    assert_eq!(convs[0]["conversationId"], "ct_1");
    assert_eq!(convs[0]["counterpartyHandle"], "Diego Alvarez");
    assert_eq!(convs[0]["topic"], "Interview scheduling — Thu 10:30");
    assert_eq!(convs[0]["href"], "/audit/action/ct_1");
    assert!(
        convs[0]["lastMessageAt"].as_str().is_some_and(|s| !s.is_empty()),
        "lastMessageAt should be a non-empty RFC3339 string"
    );
}

#[tokio::test]
async fn active_conversations_are_worker_scoped() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let priya = add_worker(&store, tenant, "Priya").await;
    let jamal = add_worker(&store, tenant, "Jamal").await;

    insert(
        store.pool(),
        &ConversationThreadInsert {
            id: "ct_priya",
            tenant_id: &tenant.to_string(),
            worker_id: priya.id.0.as_bytes().as_slice(),
            counterparty_handle: "Diego",
            topic: "Interview",
            last_message_at: chrono::Utc::now().timestamp_micros(),
            audit_href: None,
        },
    )
    .await
    .expect("seed thread for priya");

    // Jamal must NOT see Priya's thread.
    let (_s, body) =
        list_response(&jamal.id.to_string(), &get_req(&jamal.id.to_string()), Some(&store)).await;
    let v: Value = serde_json::from_str(&body).unwrap();
    assert!(
        v["active_conversations"].as_array().expect("array").is_empty(),
        "Jamal must not see Priya's conversation thread"
    );
}

#[tokio::test]
async fn recent_decisions_derives_decided_approvals_only() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Priya").await;
    let sink = AuditSink::new_with_tenant(Arc::clone(&store), tenant.to_string());

    // Decided approval — should surface as a decision.
    let decided = create_request(
        store.pool(),
        &NewApprovalRequest {
            action_id: uuid::Uuid::now_v7().as_bytes().to_vec(),
            tenant_id: tenant,
            requested_by: "argo-demo-seed",
            deadline_at: (chrono::Utc::now() + chrono::Duration::hours(24)).timestamp_micros(),
            default_decision: argo_store::approvals::DefaultDecision::Deny,
            draft_content: Some("Book Diego interview Thursday."),
        },
    )
    .await
    .expect("create decided approval");
    sink.write(
        None,
        AuditKind::ApprovalAsked,
        serde_json::json!({
            "approval_id": decided,
            "worker_id": worker.id.to_string(),
            "label": "candidate interview slot",
            "summary": "Book Diego interview Thursday.",
            "channel": "slack",
        }),
        None,
    )
    .await
    .expect("write asked audit");
    decide_request(store.pool(), &decided, tenant, "human:mark", ApprovalDecision::Approve)
        .await
        .expect("decide");

    // Pending approval — should NOT surface (it is not a decision).
    let pending = create_request(
        store.pool(),
        &NewApprovalRequest {
            action_id: uuid::Uuid::now_v7().as_bytes().to_vec(),
            tenant_id: tenant,
            requested_by: "argo-demo-seed",
            deadline_at: (chrono::Utc::now() + chrono::Duration::hours(24)).timestamp_micros(),
            default_decision: argo_store::approvals::DefaultDecision::Deny,
            draft_content: Some("Still open."),
        },
    )
    .await
    .expect("create pending approval");
    sink.write(
        None,
        AuditKind::ApprovalAsked,
        serde_json::json!({
            "approval_id": pending,
            "worker_id": worker.id.to_string(),
            "label": "still-open ask",
            "channel": "slack",
        }),
        None,
    )
    .await
    .expect("write pending asked audit");
    sink.flush_pending().await.expect("flush");

    let (status, body) =
        list_response(&worker.id.to_string(), &get_req(&worker.id.to_string()), Some(&store)).await;
    assert_eq!(status, "200 OK", "body: {body}");
    let v: Value = serde_json::from_str(&body).unwrap();
    let decisions = v["recent_decisions"].as_array().expect("array");
    assert_eq!(
        decisions.len(),
        1,
        "only the decided approval is a decision; pending is excluded — got {decisions:?}"
    );
    assert_eq!(decisions[0]["decisionId"], decided);
    assert_eq!(decisions[0]["label"], "candidate interview slot");
    assert_eq!(decisions[0]["outcome"], "approved");
    assert!(
        decisions[0]["href"].as_str().is_some_and(|s| s.starts_with("/audit/action/")),
        "href should be an audit drill-in"
    );
}

#[tokio::test]
async fn recent_decisions_maps_reject_to_refused() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Priya").await;
    let sink = AuditSink::new_with_tenant(Arc::clone(&store), tenant.to_string());

    let rejected = create_request(
        store.pool(),
        &NewApprovalRequest {
            action_id: uuid::Uuid::now_v7().as_bytes().to_vec(),
            tenant_id: tenant,
            requested_by: "argo-demo-seed",
            deadline_at: (chrono::Utc::now() + chrono::Duration::hours(24)).timestamp_micros(),
            default_decision: argo_store::approvals::DefaultDecision::Deny,
            draft_content: Some("Forward candidate email outside org."),
        },
    )
    .await
    .expect("create rejected approval");
    sink.write(
        None,
        AuditKind::ApprovalAsked,
        serde_json::json!({
            "approval_id": rejected,
            "worker_id": worker.id.to_string(),
            "label": "forward email outside org",
            "channel": "slack",
        }),
        None,
    )
    .await
    .expect("write asked audit");
    sink.flush_pending().await.expect("flush");
    decide_request(store.pool(), &rejected, tenant, "human:mark", ApprovalDecision::Reject)
        .await
        .expect("decide reject");

    let (_s, body) =
        list_response(&worker.id.to_string(), &get_req(&worker.id.to_string()), Some(&store)).await;
    let v: Value = serde_json::from_str(&body).unwrap();
    let decisions = v["recent_decisions"].as_array().expect("array");
    assert_eq!(decisions.len(), 1);
    assert_eq!(decisions[0]["outcome"], "refused");
}
