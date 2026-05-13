//! Smoke tests for `SessionStore::create_session_with_worker` ×
//! `PromptCacheAffinity` wiring
//! (`mission-prompt-cache-affinity-wire-mints`).
//!
//! Covers:
//! - Same `(tenant, worker)` pair returns the same `SessionId` for
//!   4 consecutive calls within the TTL window.
//! - Default (no affinity attached) path mints 4 distinct ids — the
//!   pre-brief behavior.
//! - Per-tenant isolation: `(tenant_a, worker_x)` and
//!   `(tenant_b, worker_x)` get distinct ids even at identical
//!   wall-clocks.
//! - Disabled-config path (affinity wired but built via
//!   `PromptCacheAffinity::disabled`) behaves like the no-affinity
//!   path — every call mints fresh.

use super::*;
use std::time::Duration;

async fn store_with_workspace() -> (SessionStore, TenantId, WorkspaceId) {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    let (tenant, workspace) = store
        .ensure_workspace("affinity", "/tmp/affinity")
        .await
        .unwrap();
    (store, tenant, workspace)
}

async fn mint(
    store: &SessionStore,
    tenant: TenantId,
    workspace: WorkspaceId,
    worker: Option<WorkerId>,
) -> SessionId {
    store
        .create_session_with_worker(SessionCreate {
            tenant_id: tenant,
            workspace_id: workspace,
            model: "model",
            provider: "provider",
            title: None,
            vertical_id: None,
            worker_id: worker,
        })
        .await
        .unwrap()
}

#[tokio::test]
async fn affinity_replays_same_session_id_within_ttl() {
    let (store, tenant, workspace) = store_with_workspace().await;
    let affinity = Arc::new(PromptCacheAffinity::enabled(Duration::from_secs(270)));
    let store = store.with_affinity(affinity);
    let worker = WorkerId(uuid::Uuid::now_v7());

    let ids: Vec<SessionId> = {
        let mut v = Vec::with_capacity(4);
        for _ in 0..4 {
            v.push(mint(&store, tenant, workspace, Some(worker)).await);
        }
        v
    };

    assert_eq!(ids[0], ids[1]);
    assert_eq!(ids[0], ids[2]);
    assert_eq!(ids[0], ids[3]);

    // ON CONFLICT(id) DO NOTHING means there is exactly one row,
    // not four.
    let rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM session WHERE tenant_id = ?")
        .bind(tenant.0.as_bytes().as_slice())
        .fetch_one(store.pool())
        .await
        .unwrap();
    assert_eq!(rows, 1);
}

#[tokio::test]
async fn no_affinity_mints_distinct_session_ids() {
    let (store, tenant, workspace) = store_with_workspace().await;
    let worker = WorkerId(uuid::Uuid::now_v7());

    let ids: Vec<SessionId> = {
        let mut v = Vec::with_capacity(4);
        for _ in 0..4 {
            v.push(mint(&store, tenant, workspace, Some(worker)).await);
        }
        v
    };

    let unique: std::collections::HashSet<_> = ids.iter().collect();
    assert_eq!(unique.len(), 4, "expected 4 distinct session ids");
}

#[tokio::test]
async fn disabled_affinity_mints_distinct_session_ids() {
    let (store, tenant, workspace) = store_with_workspace().await;
    let store = store.with_affinity(Arc::new(PromptCacheAffinity::disabled()));
    let worker = WorkerId(uuid::Uuid::now_v7());

    let ids: Vec<SessionId> = {
        let mut v = Vec::with_capacity(4);
        for _ in 0..4 {
            v.push(mint(&store, tenant, workspace, Some(worker)).await);
        }
        v
    };

    let unique: std::collections::HashSet<_> = ids.iter().collect();
    assert_eq!(unique.len(), 4, "disabled-affinity must mint fresh ids");
}

#[tokio::test]
async fn affinity_isolates_distinct_tenants_sharing_a_worker_id() {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    let store = store.with_affinity(Arc::new(PromptCacheAffinity::enabled(
        Duration::from_secs(270),
    )));
    let (tenant_a, workspace_a) = store
        .ensure_workspace("alpha", "/tmp/alpha")
        .await
        .unwrap();
    let (tenant_b, workspace_b) = store
        .ensure_workspace("bravo", "/tmp/bravo")
        .await
        .unwrap();
    assert_ne!(
        tenant_a, tenant_b,
        "test precondition: workspaces must mint distinct tenants",
    );
    let worker = WorkerId(uuid::Uuid::now_v7());

    let sid_a = mint(&store, tenant_a, workspace_a, Some(worker)).await;
    let sid_b = mint(&store, tenant_b, workspace_b, Some(worker)).await;
    assert_ne!(
        sid_a, sid_b,
        "cross-tenant share of the same worker_id must not collide",
    );
}

#[tokio::test]
async fn affinity_without_worker_id_still_mints_fresh() {
    let (store, tenant, workspace) = store_with_workspace().await;
    let store = store.with_affinity(Arc::new(PromptCacheAffinity::enabled(
        Duration::from_secs(270),
    )));

    let sid_a = mint(&store, tenant, workspace, None).await;
    let sid_b = mint(&store, tenant, workspace, None).await;
    assert_ne!(
        sid_a, sid_b,
        "missing worker_id bypasses the registry — every call must mint fresh",
    );
}
