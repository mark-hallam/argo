//! Tests for [`IdentityStore::archive_worker`] and the
//! `archived_at` filter on [`IdentityStore::list_workers`]
//! (Stream `mvp-identity-archive-worker`). Lives in its own file so
//! `store.rs` stays under the 500-LOC ceiling.
#![cfg(test)]

use crate::{IdentityStore, Persona, Role, Worker, WorkerId};
use argo_store::SessionStore;
use argo_types::TenantId;

async fn fresh_store_and_identity() -> (SessionStore, TenantId) {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    (store, TenantId::new())
}

fn draft_worker(tenant: TenantId, vertical: &str, name: &str) -> Worker {
    Worker::draft(
        tenant,
        vertical,
        Persona::named(name, "op@example.com"),
        Role::new("Own approved follow-up."),
    )
}

#[tokio::test]
async fn archive_worker_sets_timestamp() {
    let (store, tenant) = fresh_store_and_identity().await;
    let identities = IdentityStore::new(&store);
    let worker = draft_worker(tenant, "gtm-engineer", "Maya");
    identities.create_worker(&worker).await.unwrap();

    identities.archive_worker(tenant, worker.id).await.unwrap();

    let archived_at: Option<String> = sqlx::query_scalar(
        "SELECT archived_at FROM worker WHERE id = ? AND tenant_id = ?",
    )
    .bind(worker.id.to_string())
    .bind(tenant.to_string())
    .fetch_one(store.pool())
    .await
    .unwrap();
    assert!(
        archived_at.is_some(),
        "expected archived_at to be set after archive_worker"
    );
}

#[tokio::test]
async fn archive_worker_is_idempotent() {
    let (store, tenant) = fresh_store_and_identity().await;
    let identities = IdentityStore::new(&store);
    let worker = draft_worker(tenant, "gtm-engineer", "Maya");
    identities.create_worker(&worker).await.unwrap();

    identities.archive_worker(tenant, worker.id).await.unwrap();
    let first: String = sqlx::query_scalar(
        "SELECT archived_at FROM worker WHERE id = ? AND tenant_id = ?",
    )
    .bind(worker.id.to_string())
    .bind(tenant.to_string())
    .fetch_one(store.pool())
    .await
    .unwrap();

    // Second call returns Ok without rewriting the timestamp.
    identities
        .archive_worker(tenant, worker.id)
        .await
        .expect("second archive must be a no-op Ok");
    let second: String = sqlx::query_scalar(
        "SELECT archived_at FROM worker WHERE id = ? AND tenant_id = ?",
    )
    .bind(worker.id.to_string())
    .bind(tenant.to_string())
    .fetch_one(store.pool())
    .await
    .unwrap();
    assert_eq!(first, second, "idempotent archive must not rewrite the timestamp");
}

#[tokio::test]
async fn archive_worker_unknown_id_returns_not_found() {
    let (store, tenant) = fresh_store_and_identity().await;
    let identities = IdentityStore::new(&store);
    // No worker created — every WorkerId is "unknown" in this tenant.
    let ghost = WorkerId::new();

    let err = identities
        .archive_worker(tenant, ghost)
        .await
        .expect_err("archiving an unknown worker must fail");
    assert!(
        err.to_string().contains("not found"),
        "expected 'not found' in error, got: {err}"
    );
}

#[tokio::test]
async fn list_workers_excludes_archived_by_default() {
    let (store, tenant) = fresh_store_and_identity().await;
    let identities = IdentityStore::new(&store);
    let live = draft_worker(tenant, "gtm-engineer", "Maya");
    let retired = draft_worker(tenant, "gtm-engineer", "Rhea");
    identities.create_worker(&live).await.unwrap();
    identities.create_worker(&retired).await.unwrap();
    identities.archive_worker(tenant, retired.id).await.unwrap();

    let listed = identities.list_workers(tenant, false).await.unwrap();
    assert_eq!(listed.len(), 1, "default list must exclude archived rows");
    assert_eq!(listed[0].id, live.id);
}

#[tokio::test]
async fn list_workers_include_archived_returns_all() {
    let (store, tenant) = fresh_store_and_identity().await;
    let identities = IdentityStore::new(&store);
    let live = draft_worker(tenant, "gtm-engineer", "Maya");
    let retired = draft_worker(tenant, "gtm-engineer", "Rhea");
    identities.create_worker(&live).await.unwrap();
    identities.create_worker(&retired).await.unwrap();
    identities.archive_worker(tenant, retired.id).await.unwrap();

    let listed = identities.list_workers(tenant, true).await.unwrap();
    assert_eq!(listed.len(), 2, "include_archived=true must return all rows");
    let ids: Vec<WorkerId> = listed.iter().map(|w| w.id).collect();
    assert!(ids.contains(&live.id));
    assert!(ids.contains(&retired.id));
}
