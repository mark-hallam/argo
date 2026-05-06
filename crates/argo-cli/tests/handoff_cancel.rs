mod common;

use argo_identity::WorkerId;
use argo_orchestrator::{EscalationStep, Handoff, OrchestratorStore, SessionId};
use argo_store::SessionStore;
use argo_types::TenantId;
use common::argo_bin;
use std::{process::Command, sync::Arc};

#[tokio::test]
async fn handoff_cancel_marks_cancelled_and_writes_audit() {
    let home = tempfile::tempdir().unwrap();
    let tenant = TenantId::new();
    let tenant_s = tenant.to_string();
    let start = Command::new(argo_bin()).env("ARGO_HOME", home.path()).args(["session", "start", "--tenant", &tenant_s, "--title", "handoff cancel", "--json"]).output().unwrap();
    assert!(start.status.success(), "{}", String::from_utf8_lossy(&start.stderr));
    let session: serde_json::Value = serde_json::from_slice(&start.stdout).unwrap();
    let sid = session["id"].as_str().unwrap().to_string();
    let store = Arc::new(SessionStore::open(home.path().join("sessions.db")).await.unwrap());
    store.migrate().await.unwrap();
    let repo = OrchestratorStore::new(Arc::clone(&store));
    repo.ensure_schema().await.unwrap();
    let handoff = Handoff::new(SessionId::from_string(sid), WorkerId::new(), EscalationStep::Worker(WorkerId::new()), "finish", serde_json::json!({"case":"demo"}));
    let hid = handoff.id.to_string();
    repo.insert_handoff(tenant, &handoff).await.unwrap();
    let out = Command::new(argo_bin()).env("ARGO_HOME", home.path()).args(["handoff", "cancel", &hid, "--tenant", &tenant_s, "--note", "operator stopped", "--json"]).output().unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let body: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(body["status"], "cancelled");
    assert_eq!(body["payload"]["cancel_note"], "operator stopped");
    let rows: Vec<String> = sqlx::query_scalar("SELECT payload_json FROM audit_record WHERE kind = 'handoff_cancelled'").fetch_all(store.pool()).await.unwrap();
    assert!(rows.iter().any(|p| p.contains(&hid) && p.contains("operator stopped")), "audit rows: {rows:?}");
}
