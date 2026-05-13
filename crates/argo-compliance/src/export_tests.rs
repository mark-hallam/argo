//! Unit tests for `argo compliance export`.
//!
//! Wired in from `compliance_export.rs` via `#[path = ...] mod tests`
//! so they share the file's private surface.
use super::*;
use argo_store::SessionStore;
use argo_types::{TenantId, WorkspaceId};
use uuid::Uuid;

async fn fresh_store() -> SessionStore {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    store
}

async fn seed_workspace(store: &SessionStore, tenant: TenantId, name: &str) -> WorkspaceId {
    let workspace = WorkspaceId::new();
    sqlx::query(
        "INSERT INTO workspace (id, tenant_id, name, root_path, created_at) \
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(workspace.0.as_bytes().as_slice())
    .bind(tenant.0.as_bytes().as_slice())
    .bind(name)
    .bind(format!("/tmp/{name}"))
    .bind(1_700_000_000_i64)
    .execute(store.pool())
    .await
    .unwrap();
    workspace
}

#[tokio::test]
async fn empty_tenant_yields_shell_with_empty_rows() {
    let store = fresh_store().await;
    let tenant = TenantId::new();
    let envelope = build_envelope(&store, tenant).await.unwrap();
    assert_eq!(envelope["subject"], tenant.to_string());
    assert!(envelope["exported_at"].is_string());
    let tables = envelope["tables"].as_object().unwrap();
    assert!(!tables.is_empty(), "expected table shells even when empty");
    for (_name, entry) in tables {
        let rows = entry["rows"].as_array().unwrap();
        assert!(rows.is_empty(), "tenant has no data, expected empty rows");
        let schema = entry["schema"].as_array().unwrap();
        assert!(!schema.is_empty(), "schema column list is always populated");
    }
}

#[tokio::test]
async fn single_row_tenant_is_present_in_workspace_table() {
    let store = fresh_store().await;
    let tenant = TenantId::new();
    seed_workspace(&store, tenant, "alpha").await;
    let envelope = build_envelope(&store, tenant).await.unwrap();
    let workspace = envelope["tables"]["workspace"].as_object().unwrap();
    let rows = workspace["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["name"], "alpha");
    let raw_blob = rows[0]["tenant_id"].as_str().unwrap();
    assert_eq!(raw_blob, hex::encode(tenant.0.as_bytes()));
}

#[tokio::test]
async fn cross_tenant_isolation_is_enforced() {
    let store = fresh_store().await;
    let alice = TenantId::new();
    let bob = TenantId::new();
    seed_workspace(&store, alice, "alice-ws").await;
    seed_workspace(&store, bob, "bob-ws").await;
    let envelope = build_envelope(&store, alice).await.unwrap();
    let rows = envelope["tables"]["workspace"]["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["name"], "alice-ws");
    let dump = serde_json::to_string(&envelope).unwrap();
    assert!(
        !dump.contains("bob-ws"),
        "bob's workspace must NOT appear in alice's export: {dump}"
    );
}

#[tokio::test]
async fn out_path_writes_pretty_json_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("export.json");
    let envelope = json!({
        "subject": Uuid::nil().to_string(),
        "exported_at": "2026-01-01T00:00:00Z",
        "tables": {},
    });
    write_envelope(&envelope, Some(path.to_str().unwrap())).unwrap();
    let body = std::fs::read_to_string(&path).unwrap();
    let parsed: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(parsed["subject"], Uuid::nil().to_string());
    assert!(
        body.contains('\n'),
        "pretty-printed body should contain newlines"
    );
}

#[tokio::test]
async fn audit_row_is_appended_on_successful_export() {
    let store = fresh_store().await;
    let tenant = TenantId::new();
    seed_workspace(&store, tenant, "audit-check").await;
    let envelope = build_envelope(&store, tenant).await.unwrap();
    append_audit(&store, tenant, &envelope).await.unwrap();
    let kind: String =
        sqlx::query_scalar("SELECT kind FROM audit_record ORDER BY seq DESC LIMIT 1")
            .fetch_one(store.pool())
            .await
            .unwrap();
    assert_eq!(kind, "compliance.export");
    let payload: String =
        sqlx::query_scalar("SELECT payload_json FROM audit_record ORDER BY seq DESC LIMIT 1")
            .fetch_one(store.pool())
            .await
            .unwrap();
    let payload: Value = serde_json::from_str(&payload).unwrap();
    assert_eq!(payload["action"], "compliance.export");
    assert_eq!(payload["target"], tenant.to_string());
    assert!(payload["row_count"].as_u64().unwrap() >= 1);
}

#[tokio::test]
async fn missing_or_malformed_tenant_id_is_rejected() {
    assert!(parse_tenant("").is_err());
    assert!(parse_tenant("   ").is_err());
    assert!(parse_tenant("not-a-uuid").is_err());
    let valid = Uuid::now_v7().to_string();
    assert!(parse_tenant(&valid).is_ok());
}
