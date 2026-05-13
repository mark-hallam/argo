use super::{emit_startup_check, hipaa_enabled_from_toml, posture_payload, KIND};
use argo_store::SessionStore;
use serde_json::Value;
use sqlx::Row;

#[test]
fn hipaa_config_defaults_false_when_absent() {
    assert!(!hipaa_enabled_from_toml("[gateway]\nport = 7400\n"));
}

#[test]
fn hipaa_config_reads_security_section() {
    assert!(hipaa_enabled_from_toml(
        "[security.hipaa]\nenabled = true\n"
    ));
}

#[test]
fn posture_payload_lists_required_features() {
    let payload = posture_payload(true);
    let features = payload["features"].as_array().expect("features array");
    assert!(features.len() >= 8);
    assert_eq!(payload["mode"], "claimed");
    assert!(features.iter().any(|f| f["name"] == "breach_notification"));
    assert!(features.iter().any(|f| f["name"] == "baa_provider_review"));
}

#[tokio::test]
async fn startup_check_writes_not_claimed_audit_row() {
    let dir = tempfile::tempdir().unwrap();
    let store = open_store(dir.path()).await;
    emit_startup_check(dir.path(), &store).await.unwrap();
    let row = latest_row(&store).await;
    assert_eq!(row.kind, KIND);
    assert_eq!(row.payload["mode"], "not_claimed");
    assert_eq!(row.parent_hash.len(), 32);
    assert_eq!(row.self_hash.len(), 32);
}

#[tokio::test]
async fn startup_check_writes_claimed_mode_from_config() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("argo.toml"),
        "[security.hipaa]\nenabled = true\n",
    )
    .unwrap();
    let store = open_store(dir.path()).await;
    emit_startup_check(dir.path(), &store).await.unwrap();
    let row = latest_row(&store).await;
    assert_eq!(row.payload["mode"], "claimed");
    assert_eq!(row.payload["hipaa_enabled"], true);
}

async fn open_store(dir: &std::path::Path) -> SessionStore {
    let db = dir.join("argo.sqlite");
    let store = SessionStore::open(&db).await.unwrap();
    store.migrate().await.unwrap();
    store
}

struct AuditRow {
    kind: String,
    payload: Value,
    parent_hash: Vec<u8>,
    self_hash: Vec<u8>,
}

async fn latest_row(store: &SessionStore) -> AuditRow {
    let row = sqlx::query(
        "SELECT kind, payload_json, parent_hash, self_hash \
         FROM audit_record ORDER BY seq DESC LIMIT 1",
    )
    .fetch_one(store.pool())
    .await
    .unwrap();
    let payload_json: String = row.get("payload_json");
    AuditRow {
        kind: row.get("kind"),
        payload: serde_json::from_str(&payload_json).unwrap(),
        parent_hash: row.get("parent_hash"),
        self_hash: row.get("self_hash"),
    }
}
