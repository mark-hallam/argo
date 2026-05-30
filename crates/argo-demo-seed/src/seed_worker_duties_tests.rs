//! Tests for `seed_worker_duties` — proves the canonical demo seed
//! lands real `role_duty` rows for Mariah (and the other scripted
//! heroes) so the Responsibilities / Schedules card renders instead of
//! the empty state, and proves the cron specs parse.

use super::{seed_worker_duties, MARIAH_DUTIES};
use crate::personas::DEMO_SEED_PERSONAS;
use argo_identity::{IdentityStore, Persona, Role, Worker};
use argo_store::SessionStore;
use argo_types::TenantId;
use std::sync::Arc;

async fn store_with_workers() -> (Arc<SessionStore>, TenantId, Vec<argo_identity::WorkerId>) {
    let store = Arc::new(SessionStore::open(":memory:").await.expect("open"));
    store.migrate().await.expect("migrate");
    let tenant = TenantId::new();
    let identities = IdentityStore::new(&store);
    identities.ensure_schema().await.expect("identity schema");
    let mut ids = Vec::new();
    for p in DEMO_SEED_PERSONAS {
        let worker = Worker::draft(
            tenant,
            p.vertical_id,
            Persona::named(p.name, "argo-demo-seed"),
            Role::new("seed"),
        );
        ids.push(worker.id);
        identities.create_worker(&worker).await.expect("create");
    }
    (store, tenant, ids)
}

#[tokio::test]
async fn seeds_mariahs_four_duties() {
    let (store, tenant, ids) = store_with_workers().await;
    let inserted = seed_worker_duties(&store, tenant, DEMO_SEED_PERSONAS, &ids)
        .await
        .expect("seed duties");
    // Mariah (4) + Priya (2) + Jamal (2) + Nori (2) + Peta (2) = 12.
    assert_eq!(inserted, 12, "expected 12 total seeded duties");

    let mariah_id = ids[0]; // DEMO_SEED_PERSONAS[0] == Mariah
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM role_duty WHERE tenant_id = ? AND worker_id = ?")
            .bind(tenant.0.as_bytes().as_slice())
            .bind(mariah_id.to_string())
            .fetch_one(store.pool())
            .await
            .expect("count");
    assert_eq!(count, 4, "Mariah must have exactly four duties");
}

#[tokio::test]
async fn mariah_duty_descriptions_match_runbook() {
    let (store, tenant, ids) = store_with_workers().await;
    seed_worker_duties(&store, tenant, DEMO_SEED_PERSONAS, &ids)
        .await
        .expect("seed duties");
    let descriptions: Vec<String> = sqlx::query_scalar(
        "SELECT description FROM role_duty WHERE tenant_id = ? AND worker_id = ? ORDER BY created_at, id",
    )
    .bind(tenant.0.as_bytes().as_slice())
    .bind(ids[0].to_string())
    .fetch_all(store.pool())
    .await
    .expect("descriptions");
    assert!(descriptions.contains(&"List new inventory with photos + tags".to_string()));
    assert!(descriptions.contains(&"Re-price stale listings against competitor data".to_string()));
    assert!(descriptions.contains(&"Respond to buyer questions on active listings".to_string()));
    assert!(descriptions.contains(&"Flag items dropping below floor price".to_string()));
}

#[test]
fn every_mariah_cron_is_six_field() {
    // The gateway's `next_fire_for` + the duty scheduler parse these
    // with the `cron` crate (asserted parseable in argo-cli's
    // `routes_worker_overview` tests, which exercise the live reader).
    // Here we pin the structural contract: every demo cron is the
    // 6-field form (`sec min hour dom mon dow`) the reader expects, so a
    // mis-shaped spec can't silently drop `next_fire_at`.
    for d in MARIAH_DUTIES {
        let fields = d.cron.split_whitespace().count();
        assert_eq!(
            fields, 6,
            "cron '{}' for '{}' must be 6-field, got {fields}",
            d.cron, d.entry_action
        );
    }
}

#[tokio::test]
async fn seed_is_idempotent_per_worker() {
    let (store, tenant, ids) = store_with_workers().await;
    let first = seed_worker_duties(&store, tenant, DEMO_SEED_PERSONAS, &ids)
        .await
        .expect("first seed");
    let second = seed_worker_duties(&store, tenant, DEMO_SEED_PERSONAS, &ids)
        .await
        .expect("second seed");
    assert!(first > 0, "first run seeds duties");
    assert_eq!(second, 0, "re-seed must not duplicate any duty");

    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM role_duty WHERE tenant_id = ?")
        .bind(tenant.0.as_bytes().as_slice())
        .fetch_one(store.pool())
        .await
        .expect("total");
    assert_eq!(total, first as i64, "row count stable after re-seed");
}
