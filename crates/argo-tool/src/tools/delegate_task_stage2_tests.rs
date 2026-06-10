//! DEC-049 Stage 2 delegate-task tests (atomic fan-out cap + same-tenant
//! target), split from `delegate_task_tests.rs` to keep it under the 500-LOC
//! ceiling. Self-contained: its own `ctx_with` so it shares no state with the
//! sibling test module.
use super::*;
use argo_orchestrator::{OrchestratorStore, Session};
use argo_store::SessionStore;
use argo_types::{Principal, TenantId};
use std::sync::Arc;

fn ctx_with(session_id: argo_orchestrator::SessionId) -> ToolContext {
    let mut c = ToolContext::new(
        Principal::Local {
            os_user: "delegate".into(),
        },
        std::path::PathBuf::from("/tmp"),
    );
    c.session_id = session_id;
    c
}


#[tokio::test]
async fn fan_out_cap_holds_atomically_under_concurrency() {
    // The fan-out cap is enforced inside ONE BEGIN IMMEDIATE tx (count-and-insert),
    // so a burst of CONCURRENT delegations cannot overshoot it (closes the TOCTOU
    // a separate count-then-insert had). File-backed WAL store gives real
    // multi-connection contention — `:memory:` serializes on one conn and would
    // hide the race this test exists to catch.
    let session_id = argo_orchestrator::SessionId::new();
    let path = std::env::temp_dir().join(format!("dec049-fanout-{session_id}.db"));
    let _ = std::fs::remove_file(&path);
    let session_store = Arc::new(SessionStore::open_unencrypted_for_tests(&path).await.unwrap());
    session_store.migrate().await.unwrap();
    let store = Arc::new(OrchestratorStore::new(Arc::clone(&session_store)));
    store.ensure_schema().await.unwrap();
    let mut session = Session::new(TenantId(uuid::Uuid::nil()), "fanout-race");
    session.id = session_id.clone();
    store.insert_session(&session).await.unwrap();

    let mut handles = Vec::new();
    for _ in 0..(MAX_DELEGATION_FANOUT * 3) {
        let store = Arc::clone(&store);
        let sid = session_id.clone();
        handles.push(tokio::spawn(async move {
            let ctx = ctx_with(sid);
            DelegateTask::new(store)
                .dispatch(
                    serde_json::json!({
                        "worker_id": WorkerId::new().0.to_string(),
                        "task": "race",
                    }),
                    &ctx,
                )
                .await
                .unwrap()
        }));
    }
    for h in handles {
        h.await.unwrap();
    }
    let count = store
        .list_handoffs_for_session(TenantId(uuid::Uuid::nil()), &session_id)
        .await
        .unwrap()
        .len();
    assert_eq!(
        count, MAX_DELEGATION_FANOUT,
        "atomic fan-out cap must hold EXACTLY under concurrency — got {count}"
    );
    let _ = std::fs::remove_file(&path);
}

#[tokio::test]
async fn cross_tenant_target_is_refused() {
    // A delegation target that EXISTS under a DIFFERENT tenant must be refused
    // (DEC-049 Stage 2 tenant containment).
    let session_store = Arc::new(SessionStore::open(":memory:").await.unwrap());
    session_store.migrate().await.unwrap();
    let other_tenant = TenantId::new();
    let target = WorkerId::new();
    sqlx::query(
        "INSERT INTO worker (id, tenant_id, vertical_id, persona_id, role_id, \
         memory_scope, auto_approval_json, channels_json, created_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(target.0.to_string())
    .bind(other_tenant.0.to_string())
    .bind("v")
    .bind("p")
    .bind("r")
    .bind("m")
    .bind("{}")
    .bind("[]")
    .bind(0_i64)
    .execute(session_store.pool())
    .await
    .unwrap();

    let store = Arc::new(OrchestratorStore::new(Arc::clone(&session_store)));
    store.ensure_schema().await.unwrap();
    let session_id = argo_orchestrator::SessionId::new();
    let mut session = Session::new(TenantId(uuid::Uuid::nil()), "tenant-target");
    session.id = session_id.clone();
    store.insert_session(&session).await.unwrap();

    let ctx = ctx_with(session_id.clone()); // caller is the nil tenant
    let r = DelegateTask::new(Arc::clone(&store))
        .dispatch(
            serde_json::json!({ "worker_id": target.0.to_string(), "task": "cross-tenant" }),
            &ctx,
        )
        .await
        .unwrap();
    assert!(r.is_error, "cross-tenant target must be refused");
    let count = store
        .list_handoffs_for_session(TenantId(uuid::Uuid::nil()), &session_id)
        .await
        .unwrap()
        .len();
    assert_eq!(count, 0, "refused cross-tenant delegation must not create a handoff");
}

#[tokio::test]
async fn same_tenant_target_is_allowed() {
    // The tenant check must NOT be over-broad: a target worker that exists under
    // the CALLER's OWN tenant is allowed. Pins the same-tenant-allow branch so a
    // future inversion that broke it closed (refuse same-tenant) would fail here.
    let session_store = Arc::new(SessionStore::open(":memory:").await.unwrap());
    session_store.migrate().await.unwrap();
    let caller_tenant = TenantId(uuid::Uuid::nil()); // ctx_with resolves to the nil tenant
    let target = WorkerId::new();
    sqlx::query(
        "INSERT INTO worker (id, tenant_id, vertical_id, persona_id, role_id, \
         memory_scope, auto_approval_json, channels_json, created_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(target.0.to_string())
    .bind(caller_tenant.0.to_string())
    .bind("v")
    .bind("p")
    .bind("r")
    .bind("m")
    .bind("{}")
    .bind("[]")
    .bind(0_i64)
    .execute(session_store.pool())
    .await
    .unwrap();

    let store = Arc::new(OrchestratorStore::new(Arc::clone(&session_store)));
    store.ensure_schema().await.unwrap();
    let session_id = argo_orchestrator::SessionId::new();
    let mut session = Session::new(caller_tenant, "same-tenant-target");
    session.id = session_id.clone();
    store.insert_session(&session).await.unwrap();

    let ctx = ctx_with(session_id.clone());
    let r = DelegateTask::new(Arc::clone(&store))
        .dispatch(
            serde_json::json!({ "worker_id": target.0.to_string(), "task": "same-tenant" }),
            &ctx,
        )
        .await
        .unwrap();
    assert!(!r.is_error, "same-tenant target must be allowed: {:?}", r.content);
    let count = store
        .list_handoffs_for_session(caller_tenant, &session_id)
        .await
        .unwrap()
        .len();
    assert_eq!(count, 1, "same-tenant delegation should create exactly one handoff");
}
