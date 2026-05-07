//! Round-trip tests for `worker_archive` CLI handler. These drive
//! the high-level flow (Worker → freeze → archive_worker → JSON →
//! Worker via IdentityStore::create_worker) without spawning the
//! `argo` binary; the binary smoke test in
//! `tests/worker_archive_smoke.rs` covers the CLI surface.

use argo_identity::{IdentityStore, Persona, Role, Worker};
use argo_store::{worker_archive, SessionStore};
use argo_types::TenantId;

#[tokio::test]
async fn round_trip_archive_then_rehydrate_via_identity_store() {
    let store = SessionStore::open(":memory:").await.expect("open store");
    store.migrate().await.expect("migrate");
    let identities = IdentityStore::new(&store);
    let tenant = TenantId::new();
    let worker = Worker::draft(
        tenant,
        "support",
        Persona::named("Maya", "operator@example.test"),
        Role::new("Triage tickets."),
    );
    identities.create_worker(&worker).await.expect("create");

    let frozen = serde_json::to_string(&worker).expect("freeze");
    worker_archive::archive_worker(
        store.pool(),
        &worker.id.to_string(),
        &worker.tenant_id.to_string(),
        Some("alice@team"),
        Some("campaign over"),
        &frozen,
    )
    .await
    .expect("archive");

    let live_after_archive =
        identities.get_worker(tenant, worker.id).await.expect("get");
    assert!(
        live_after_archive.is_none(),
        "live worker row removed after archive"
    );

    let thawed_json = worker_archive::rehydrate_worker(
        store.pool(),
        &worker.id.to_string(),
        &worker.tenant_id.to_string(),
    )
    .await
    .expect("rehydrate ok");
    let thawed: Worker = serde_json::from_str(&thawed_json).expect("thaw");
    assert_eq!(thawed.id, worker.id);
    assert_eq!(thawed.persona.name, worker.persona.name);

    identities.create_worker(&thawed).await.expect("re-insert");
    let live = identities
        .get_worker(tenant, worker.id)
        .await
        .expect("get back")
        .expect("present");
    assert_eq!(live.id, worker.id);
    assert_eq!(live.tenant_id, worker.tenant_id);
    assert_eq!(live.vertical_id, worker.vertical_id);
    assert_eq!(live.persona.name, worker.persona.name);
    assert_eq!(live.persona.bio, worker.persona.bio);
    assert_eq!(live.role.charter, worker.role.charter);
    assert_eq!(live.channels, worker.channels);
}

#[tokio::test]
async fn list_archived_returns_metadata_for_one_tenant() {
    let store = SessionStore::open(":memory:").await.expect("open store");
    store.migrate().await.expect("migrate");
    let identities = IdentityStore::new(&store);
    let tenant = TenantId::new();
    let worker = Worker::draft(
        tenant,
        "support",
        Persona::named("Maya", "op@example.test"),
        Role::new("Triage."),
    );
    identities.create_worker(&worker).await.expect("create");
    let frozen = serde_json::to_string(&worker).expect("freeze");
    worker_archive::archive_worker(
        store.pool(),
        &worker.id.to_string(),
        &worker.tenant_id.to_string(),
        None,
        Some("seasonal-pause"),
        &frozen,
    )
    .await
    .expect("archive");

    let rows = worker_archive::list_archived(store.pool(), &tenant.to_string())
        .await
        .expect("list");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, worker.id.to_string());
    assert_eq!(rows[0].reason.as_deref(), Some("seasonal-pause"));
}

#[tokio::test]
async fn rehydrate_fails_if_live_worker_with_same_id_present() {
    let store = SessionStore::open(":memory:").await.expect("open store");
    store.migrate().await.expect("migrate");
    let identities = IdentityStore::new(&store);
    let tenant = TenantId::new();
    let worker = Worker::draft(
        tenant,
        "support",
        Persona::named("Maya", "op@example.test"),
        Role::new("Triage."),
    );
    identities.create_worker(&worker).await.expect("create");
    let frozen = serde_json::to_string(&worker).expect("freeze");

    // Archive then re-create a live worker with the same id BEFORE
    // rehydrating — store-level rehydrate must refuse so we don't
    // double-insert.
    worker_archive::archive_worker(
        store.pool(),
        &worker.id.to_string(),
        &worker.tenant_id.to_string(),
        None,
        None,
        &frozen,
    )
    .await
    .expect("archive");
    identities
        .create_worker(&worker)
        .await
        .expect("recreate live");

    let err = worker_archive::rehydrate_worker(
        store.pool(),
        &worker.id.to_string(),
        &worker.tenant_id.to_string(),
    )
    .await
    .expect_err("rehydrate refused");
    assert!(err.to_string().contains("already exists"));
}
