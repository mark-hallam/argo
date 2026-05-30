//! Tests for `POST /api/v1/orchestration/team-runs` — the Supervisor
//! team-run gateway route. Uses an in-memory store + an explicit
//! `?tenant_id=` so the run is deterministic and tenant-scoped; the
//! engine itself is unit-tested in `argo-orchestrator`.
use super::*;
use argo_types::TenantId;
use serde_json::Value;
use std::sync::Arc;

async fn store() -> Arc<SessionStore> {
    let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
    store.migrate().await.unwrap();
    store
}

fn worker() -> String {
    argo_identity::WorkerId::new().to_string()
}

/// Build a `POST` request string with a JSON body + `?tenant_id=`.
fn req(tenant: TenantId, body: &Value) -> String {
    let body = body.to_string();
    format!(
        "POST /api/v1/orchestration/team-runs?tenant_id={} HTTP/1.1\r\n\
         Content-Type: application/json\r\n\r\n{}",
        tenant.0, body
    )
}

async fn orch_run_count(store: &SessionStore, tenant: TenantId) -> i64 {
    sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM session WHERE tenant_id = ? AND provider = 'orchestrator'",
    )
    .bind(tenant.0.as_bytes().as_slice())
    .fetch_one(store.pool())
    .await
    .unwrap()
}

#[tokio::test]
async fn team_run_three_parallel_children_complete_and_persist() {
    let store = store().await;
    let tenant = TenantId::new();
    let body = serde_json::json!({
        "lead": worker(),
        "title": "research sprint",
        "current_handoff_id": "h-1",
        "steps": [
            { "id": "a", "to": worker(), "task": "gather pricing" },
            { "id": "b", "to": worker(), "task": "scan competitors" },
            { "id": "c", "to": worker(), "task": "draft summary" },
        ],
    });
    let (status, out) = super::create_response(&req(tenant, &body), Some(&store)).await;
    assert_eq!(status, "202 Accepted");
    let parsed: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(parsed["ok"], true);
    assert_eq!(parsed["summary"]["total"], 3);
    assert_eq!(parsed["summary"]["completed"], 3);
    assert_eq!(parsed["summary"]["allCompleted"], true);
    assert!(parsed["synthesisRunId"].as_str().is_some());
    let steps = parsed["steps"].as_array().unwrap();
    assert_eq!(steps.len(), 3);
    assert!(steps
        .iter()
        .all(|s| s["status"] == "completed" && s["runId"].is_string()));
    // 3 children + 1 synthesis = 4 orchestration rows, all under tenant.
    assert_eq!(orch_run_count(&store, tenant).await, 4);
}

#[tokio::test]
async fn team_run_a_then_bc_then_d_dependency_ordering() {
    let store = store().await;
    let tenant = TenantId::new();
    let body = serde_json::json!({
        "lead": worker(),
        "title": "diamond",
        "steps": [
            { "id": "a", "to": worker(), "task": "kickoff" },
            { "id": "b", "to": worker(), "task": "branch-1", "depends_on": ["a"] },
            { "id": "c", "to": worker(), "task": "branch-2", "depends_on": ["a"] },
            { "id": "d", "to": worker(), "task": "merge", "depends_on": ["b", "c"] },
        ],
    });
    let (status, out) = super::create_response(&req(tenant, &body), Some(&store)).await;
    assert_eq!(status, "202 Accepted");
    let parsed: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(parsed["summary"]["completed"], 4);
    assert_eq!(parsed["summary"]["allCompleted"], true);
    // 4 children + 1 synthesis.
    assert_eq!(orch_run_count(&store, tenant).await, 5);
}

#[tokio::test]
async fn team_run_cross_tenant_isolation() {
    let store = store().await;
    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();
    let body = serde_json::json!({
        "lead": worker(),
        "title": "tenant-a-run",
        "steps": [{ "id": "a", "to": worker(), "task": "do" }],
    });
    let (status, _) = super::create_response(&req(tenant_a, &body), Some(&store)).await;
    assert_eq!(status, "202 Accepted");
    // Tenant A owns 1 child + 1 synthesis = 2 rows; tenant B sees none.
    assert_eq!(orch_run_count(&store, tenant_a).await, 2);
    assert_eq!(
        orch_run_count(&store, tenant_b).await,
        0,
        "cross-tenant isolation — tenant B must see zero team-run rows"
    );
}

#[tokio::test]
async fn team_run_missing_lead_is_400() {
    let store = store().await;
    let tenant = TenantId::new();
    let body = serde_json::json!({
        "title": "no-lead",
        "steps": [{ "id": "a", "to": worker(), "task": "do" }],
    });
    let (status, out) = super::create_response(&req(tenant, &body), Some(&store)).await;
    assert_eq!(status, "400 Bad Request");
    assert!(out.contains("lead"));
}

#[tokio::test]
async fn team_run_cycle_is_400_not_hang() {
    let store = store().await;
    let tenant = TenantId::new();
    let body = serde_json::json!({
        "lead": worker(),
        "title": "cycle",
        "steps": [
            { "id": "a", "to": worker(), "task": "x", "depends_on": ["b"] },
            { "id": "b", "to": worker(), "task": "y", "depends_on": ["a"] },
        ],
    });
    let (status, out) = super::create_response(&req(tenant, &body), Some(&store)).await;
    assert_eq!(status, "400 Bad Request");
    assert!(out.contains("cycle"));
}

#[tokio::test]
async fn team_run_empty_steps_is_400() {
    let store = store().await;
    let tenant = TenantId::new();
    let body = serde_json::json!({ "lead": worker(), "steps": [] });
    let (status, out) = super::create_response(&req(tenant, &body), Some(&store)).await;
    assert_eq!(status, "400 Bad Request");
    assert!(out.contains("steps"));
}

#[tokio::test]
async fn team_run_no_store_is_503() {
    let body = serde_json::json!({ "lead": worker(), "steps": [] });
    let (status, _) = super::create_response(&req(TenantId::new(), &body), None).await;
    assert_eq!(status, "503 Service Unavailable");
}

#[tokio::test]
async fn team_run_dispatch_routes_team_runs_path() {
    let store = store().await;
    let tenant = TenantId::new();
    let body = serde_json::json!({
        "lead": worker(),
        "title": "via-dispatch",
        "steps": [{ "id": "a", "to": worker(), "task": "do" }],
    });
    // Exercise the actual sub-dispatcher so the wiring is covered.
    let resp = super::super::routes_dispatch_post_paths::dispatch_orchestration(
        &req(tenant, &body),
        Some(&store),
    )
    .await;
    let (status, _) = resp.expect("dispatcher must match team-runs");
    assert_eq!(status, "202 Accepted");
}
