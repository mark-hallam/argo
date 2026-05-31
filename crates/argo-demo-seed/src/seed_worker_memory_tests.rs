//! Tests for the worker-memory seeders (`seed_worker_memory_facts` +
//! `seed_worker_conversation_threads`).
//!
//! Covers: facts land on the exact read path `fetch_active_facts` keys on
//! (tenant + worker + WorkerSelf scope + Memory file); conversation
//! threads land in the v87 store; both are idempotent (re-seed inserts 0);
//! and both are worker-scoped (a worker with no mapped persona gets none).

use super::*;
use argo_identity::{IdentityStore, Persona, Role, Worker};
use std::sync::Arc;

async fn open_store() -> Arc<SessionStore> {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    Arc::new(store)
}

/// Create the canonical hero roster under `tenant`, positional-aligned
/// with `DEMO_SEED_PERSONAS` (the spine the seeders zip against).
async fn seed_roster(store: &Arc<SessionStore>, tenant: TenantId) -> Vec<argo_identity::WorkerId> {
    let identities = IdentityStore::new(store);
    let mut ids = Vec::new();
    for spine in DEMO_SEED_PERSONAS {
        let worker = Worker::draft(
            tenant,
            spine.vertical_id,
            Persona::named(spine.name, "argo-demo-seed"),
            Role::new(spine.tagline),
        );
        ids.push(worker.id);
        identities.create_worker(&worker).await.unwrap();
    }
    ids
}

#[tokio::test]
async fn facts_land_on_the_live_read_predicate() {
    let store = open_store().await;
    let tenant = TenantId::new();
    let ids = seed_roster(&store, tenant).await;

    let inserted = seed_worker_memory_facts(&store, tenant, &ids).await.unwrap();
    assert!(inserted >= 8, "expected >= 1 fact per hero; got {inserted}");

    // Mariah is persona index 0 — assert her Northwind fact is readable on
    // exactly the predicate `fetch_active_facts` uses.
    let mariah = ids[0];
    let bodies: Vec<String> = sqlx::query_scalar(
        "SELECT body FROM memory_entry \
         WHERE tenant_id = ? AND worker_id = ? AND memory_scope = 'worker_self' \
           AND file = ? AND superseded_by IS NULL",
    )
    .bind(tenant.0.as_bytes().as_slice())
    .bind(mariah.0.as_bytes().as_slice())
    .bind(MemoryFile::Memory.filename())
    .fetch_all(store.pool())
    .await
    .unwrap();
    assert!(
        bodies.iter().any(|b| b.contains("Northwind pays NET-30")),
        "Mariah's seeded NET-30 fact must be readable; got {bodies:?}"
    );
}

#[tokio::test]
async fn facts_are_idempotent() {
    let store = open_store().await;
    let tenant = TenantId::new();
    let ids = seed_roster(&store, tenant).await;

    let first = seed_worker_memory_facts(&store, tenant, &ids).await.unwrap();
    assert!(first > 0);
    let second = seed_worker_memory_facts(&store, tenant, &ids).await.unwrap();
    assert_eq!(second, 0, "re-seed must not duplicate facts; got {second}");
}

#[tokio::test]
async fn threads_round_trip_and_are_idempotent() {
    let store = open_store().await;
    let tenant = TenantId::new();
    let ids = seed_roster(&store, tenant).await;

    let first = seed_worker_conversation_threads(&store, tenant, &ids)
        .await
        .unwrap();
    assert!(first >= 4, "expected several seeded threads; got {first}");

    // Priya is persona index 2 — assert she has at least one thread.
    let priya_blob = ids[2].0.as_bytes().to_vec();
    let priya_threads = count_for_worker(store.pool(), &tenant.to_string(), &priya_blob)
        .await
        .unwrap();
    assert!(priya_threads >= 1, "Priya should have a seeded thread");

    let second = seed_worker_conversation_threads(&store, tenant, &ids)
        .await
        .unwrap();
    assert_eq!(second, 0, "re-seed must not duplicate threads; got {second}");
}

#[tokio::test]
async fn empty_worker_ids_is_a_noop() {
    let store = open_store().await;
    let tenant = TenantId::new();
    assert_eq!(seed_worker_memory_facts(&store, tenant, &[]).await.unwrap(), 0);
    assert_eq!(
        seed_worker_conversation_threads(&store, tenant, &[])
            .await
            .unwrap(),
        0
    );
}

#[tokio::test]
async fn facts_are_tenant_scoped() {
    let store = open_store().await;
    let tenant_a = TenantId::new();
    let ids = seed_roster(&store, tenant_a).await;
    seed_worker_memory_facts(&store, tenant_a, &ids).await.unwrap();

    // A different tenant sees zero facts for the same worker blob.
    let tenant_b = TenantId::new();
    let mariah = ids[0];
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM memory_entry WHERE tenant_id = ? AND worker_id = ?",
    )
    .bind(tenant_b.0.as_bytes().as_slice())
    .bind(mariah.0.as_bytes().as_slice())
    .fetch_one(store.pool())
    .await
    .unwrap();
    assert_eq!(count, 0, "tenant B must not see tenant A's seeded facts");
}
