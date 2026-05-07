//! Unit tests for `worker_charter` (mission claim 2 backing).
//! Covers file IO, size enforcement, and IdentityStore round-trip
//! through `update_charter`.

use super::worker_charter::{enforce_charter_size, enforce_kpis_size, read_charter_file, read_kpis_file};
use argo_identity::{
    IdentityStore, Persona, Role, Worker, MAX_CHARTER_BYTES, MAX_KPIS_BYTES,
};
use argo_store::SessionStore;
use argo_types::TenantId;
use serde_json::json;

#[test]
fn read_charter_file_round_trips_markdown() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("charter.md");
    std::fs::write(&path, "# Charter\n\nReply within 24h.\n").unwrap();
    let body = read_charter_file(&path).expect("reads");
    assert!(body.contains("# Charter"));
}

#[test]
fn read_charter_file_errors_on_missing_path() {
    let path = std::path::Path::new("/tmp/skycot-no-such-charter-file.md");
    let err = read_charter_file(path).expect_err("missing path");
    assert!(err.to_string().contains("could not read charter"));
}

#[test]
fn read_charter_file_rejects_oversize_body() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("big.md");
    std::fs::write(&path, "x".repeat(MAX_CHARTER_BYTES + 1)).unwrap();
    let err = read_charter_file(&path).expect_err("too big");
    assert!(err.to_string().contains("charter exceeds"));
}

#[test]
fn read_kpis_file_round_trips_json() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("kpis.json");
    std::fs::write(&path, r#"{"reply_sla_hours": 24}"#).unwrap();
    let value = read_kpis_file(&path).expect("parses");
    assert_eq!(value["reply_sla_hours"], 24);
}

#[test]
fn read_kpis_file_errors_on_invalid_json() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("bad.json");
    std::fs::write(&path, "{ not json").unwrap();
    let err = read_kpis_file(&path).expect_err("bad json");
    assert!(err.to_string().contains("not valid JSON"));
}

#[test]
fn enforce_charter_size_passes_at_ceiling() {
    let body = "x".repeat(MAX_CHARTER_BYTES);
    enforce_charter_size(&body).expect("at ceiling is OK");
}

#[test]
fn enforce_kpis_size_rejects_oversize_blob() {
    // Build a JSON object whose serialised form exceeds the ceiling.
    let blob = "x".repeat(MAX_KPIS_BYTES);
    let value = json!({ "blob": blob });
    let err = enforce_kpis_size(&value).expect_err("too big");
    assert!(err.to_string().contains("kpis exceed"));
}

#[tokio::test]
async fn identity_store_round_trip_preserves_charter_and_kpis() {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    let identities = IdentityStore::new(&store);
    let tenant = TenantId::new();
    let worker = Worker::draft(
        tenant,
        "support",
        Persona::named("Ari", "op"),
        Role::new("Triage."),
    );
    identities.create_worker(&worker).await.unwrap();

    let kpis = json!({"sla_hours": 24, "csat_min": 4.5});
    identities
        .update_charter(
            tenant,
            worker.id,
            Some("# Charter\n\nReply within 24h."),
            Some(&kpis),
        )
        .await
        .expect("update_charter");

    let loaded = identities
        .get_worker(tenant, worker.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        loaded.charter.as_deref(),
        Some("# Charter\n\nReply within 24h.")
    );
    assert_eq!(loaded.kpis, Some(kpis));

    // Clear nulls both columns.
    identities
        .update_charter(tenant, worker.id, None, None)
        .await
        .expect("clear");
    let cleared = identities
        .get_worker(tenant, worker.id)
        .await
        .unwrap()
        .unwrap();
    assert!(cleared.charter.is_none());
    assert!(cleared.kpis.is_none());
}

#[tokio::test]
async fn identity_store_update_charter_errors_on_unknown_worker() {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    let identities = IdentityStore::new(&store);
    let tenant = TenantId::new();
    let worker_id = argo_identity::WorkerId::new();
    let err = identities
        .update_charter(tenant, worker_id, Some("nope"), None)
        .await
        .expect_err("no such worker");
    assert!(err.to_string().contains("not found"));
}
