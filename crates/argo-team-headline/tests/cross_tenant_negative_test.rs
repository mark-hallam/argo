//! Cross-tenant regression pin for `generate_team_headline`.
//!
//! The negative half alone is not enough: a query that returns no
//! activity for every tenant would also appear safe. This test keeps
//! the round-37 vacuous-pass guard by asserting `leaked == 0` and
//! `owned >= 1` in the same scenario.

use std::sync::Arc;

use argo_audit::{AuditKind, AuditSink};
use argo_identity::WorkerId;
use argo_store::{SessionCreate, SessionStore};
use argo_team_headline::generate_team_headline;
use argo_types::{SessionId, TenantId, WorkspaceId};
use uuid::Uuid;

struct SeededWorker {
    worker: WorkerId,
    session: SessionId,
}

async fn fresh_store() -> Arc<SessionStore> {
    let store = Arc::new(SessionStore::open(":memory:").await.expect("open store"));
    store.migrate().await.expect("migrate store");
    store
}

async fn seed_worker(store: &Arc<SessionStore>, tenant: TenantId) -> SeededWorker {
    let ws = WorkspaceId(Uuid::now_v7());
    sqlx::query(
        "INSERT INTO workspace (id, tenant_id, name, root_path, created_at) \
         VALUES (?, ?, 'headline-test', '.', strftime('%s','now'))",
    )
    .bind(ws.0.as_bytes().as_slice())
    .bind(tenant.0.as_bytes().as_slice())
    .execute(store.pool())
    .await
    .expect("insert workspace");

    let worker_uuid = Uuid::now_v7();
    let worker = WorkerId(worker_uuid);
    let session = store
        .create_session_with_worker(SessionCreate {
            tenant_id: tenant,
            workspace_id: ws,
            model: "test-model",
            provider: "test-provider",
            title: Some("seed"),
            vertical_id: Some("pr-checker"),
            worker_id: Some(argo_store::WorkerId(worker_uuid)),
        })
        .await
        .expect("create session");

    SeededWorker { worker, session }
}

async fn write_tool_calls(store: &Arc<SessionStore>, session: SessionId, count: usize) {
    let sink = AuditSink::new_with_tenant(Arc::clone(store), "team-headline-test".to_string());
    for _ in 0..count {
        sink.write(
            Some(session),
            AuditKind::ToolCall,
            serde_json::json!({ "test": "team-headline-tenant-bind" }),
            None,
        )
        .await
        .expect("write audit row");
    }
}

#[tokio::test]
async fn generate_headline_does_not_count_other_tenant_worker_activity() {
    let store = fresh_store().await;
    let tenant_a = TenantId(Uuid::now_v7());
    let tenant_b = TenantId(Uuid::now_v7());
    let worker_a = seed_worker(&store, tenant_a).await;
    let worker_b = seed_worker(&store, tenant_b).await;

    write_tool_calls(&store, worker_a.session, 2).await;
    write_tool_calls(&store, worker_b.session, 5).await;

    let owned_headline = generate_team_headline(tenant_a, worker_a.worker, "pr-checker", &store)
        .await
        .expect("tenant A headline for tenant A worker");
    let cross_tenant_headline =
        generate_team_headline(tenant_a, worker_b.worker, "pr-checker", &store)
            .await
            .expect("tenant A headline for tenant B worker");

    let owned = owned_headline
        .bullets
        .iter()
        .filter(|bullet| bullet.as_str() == "2 PRs reviewed today")
        .count();
    let leaked = cross_tenant_headline
        .bullets
        .iter()
        .filter(|bullet| bullet.as_str() == "5 PRs reviewed today")
        .count();

    assert_eq!(
        leaked, 0,
        "tenant A query must not count tenant B worker activity; got {:?}",
        cross_tenant_headline.bullets
    );
    assert!(
        owned >= 1,
        "vacuous-pass guard: tenant A's own activity must still be visible; got {:?}",
        owned_headline.bullets
    );
}
