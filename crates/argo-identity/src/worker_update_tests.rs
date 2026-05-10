//! Tests for [`IdentityStore::update_worker`] (Stream
//! `mission-identity-update-worker`, MISSION_BACKLOG #39). Lives in
//! its own file so `store.rs` stays under the 500-LOC ceiling.
//!
//! Fixture style mirrors `store_archive_tests.rs`.
#![cfg(test)]

use chrono::{TimeZone, Utc};

use crate::{IdentityStore, Persona, Role, Worker, WorkerId, WorkerLevel, WorkerUpdate};
use argo_store::SessionStore;
use argo_types::{ArgoError, TenantId};

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
async fn update_vertical_id_preserves_other_fields() {
    let (store, tenant) = fresh_store_and_identity().await;
    let identities = IdentityStore::new(&store);
    let worker = draft_worker(tenant, "gtm-engineer", "Maya");
    identities.create_worker(&worker).await.unwrap();
    identities
        .update_level(tenant, worker.id, WorkerLevel::Junior)
        .await
        .unwrap();

    let updated = identities
        .update_worker(
            tenant,
            worker.id,
            WorkerUpdate::default().with_vertical_id("support"),
        )
        .await
        .unwrap();

    assert_eq!(updated.vertical_id, "support");
    assert_eq!(updated.level, WorkerLevel::Junior, "level preserved");
    assert_eq!(updated.persona.name, "Maya", "persona preserved");
    assert_eq!(updated.trust_grade, 0, "trust_grade preserved");
}

#[tokio::test]
async fn update_trust_grade_only() {
    let (store, tenant) = fresh_store_and_identity().await;
    let identities = IdentityStore::new(&store);
    let worker = draft_worker(tenant, "gtm-engineer", "Maya");
    identities.create_worker(&worker).await.unwrap();

    let updated = identities
        .update_worker(
            tenant,
            worker.id,
            WorkerUpdate::default().with_trust_grade(7),
        )
        .await
        .unwrap();

    assert_eq!(updated.trust_grade, 7);
    assert_eq!(updated.vertical_id, "gtm-engineer", "vertical preserved");
    assert_eq!(updated.level, WorkerLevel::Entry, "level preserved");
}

#[tokio::test]
async fn update_multiple_fields_atomically() {
    let (store, tenant) = fresh_store_and_identity().await;
    let identities = IdentityStore::new(&store);
    let worker = draft_worker(tenant, "gtm-engineer", "Maya");
    identities.create_worker(&worker).await.unwrap();

    let when = Utc.with_ymd_and_hms(2026, 5, 10, 12, 0, 0).unwrap();
    let updated = identities
        .update_worker(
            tenant,
            worker.id,
            WorkerUpdate::default()
                .with_vertical_id("support")
                .with_level(WorkerLevel::Senior)
                .with_trust_grade(5)
                .with_last_used_at(when),
        )
        .await
        .unwrap();

    assert_eq!(updated.vertical_id, "support");
    assert_eq!(updated.level, WorkerLevel::Senior);
    assert_eq!(updated.trust_grade, 5);
    assert_eq!(updated.last_used_at, Some(when));
}

#[tokio::test]
async fn update_unknown_worker_errors_not_found() {
    let (store, tenant) = fresh_store_and_identity().await;
    let identities = IdentityStore::new(&store);
    let ghost = WorkerId::new();

    let err = identities
        .update_worker(
            tenant,
            ghost,
            WorkerUpdate::default().with_trust_grade(3),
        )
        .await
        .expect_err("update on unknown worker must fail");
    assert!(
        matches!(err, ArgoError::NotFound(_)),
        "expected NotFound, got: {err:?}"
    );
}

#[tokio::test]
async fn update_empty_vertical_id_errors_schema() {
    let (store, tenant) = fresh_store_and_identity().await;
    let identities = IdentityStore::new(&store);
    let worker = draft_worker(tenant, "gtm-engineer", "Maya");
    identities.create_worker(&worker).await.unwrap();

    let err = identities
        .update_worker(
            tenant,
            worker.id,
            WorkerUpdate::default().with_vertical_id(""),
        )
        .await
        .expect_err("empty vertical_id must fail validation");
    assert!(
        matches!(err, ArgoError::Schema(_)),
        "expected Schema variant, got: {err:?}"
    );
}

#[tokio::test]
async fn update_no_fields_set_is_noop_returns_current() {
    let (store, tenant) = fresh_store_and_identity().await;
    let identities = IdentityStore::new(&store);
    let worker = draft_worker(tenant, "gtm-engineer", "Maya");
    identities.create_worker(&worker).await.unwrap();

    let updated = identities
        .update_worker(tenant, worker.id, WorkerUpdate::default())
        .await
        .expect("no-op update must succeed");

    assert_eq!(updated.id, worker.id);
    assert_eq!(updated.vertical_id, "gtm-engineer");
    assert_eq!(updated.persona.name, "Maya");
    assert_eq!(updated.level, WorkerLevel::Entry);
    assert_eq!(updated.trust_grade, 0);
    assert!(updated.last_used_at.is_none());
}
