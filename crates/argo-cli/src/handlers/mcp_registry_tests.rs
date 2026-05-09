use super::*;
use argo_store::queries::mcp_registry::NewMcpServer;

async fn store() -> SessionStore {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    store
}

async fn add_row(registry: &McpRegistryStore<'_>, tenant: &str, server: &str) -> McpServerRow {
    registry
        .add(
            NewMcpServer {
                tenant_id: tenant,
                server_id: server,
                kind: "http",
                base_url: "http://127.0.0.1:9",
                secrets_ref: Some("cred/mcp"),
                last_seen: Some(42),
                last_error: None,
            },
            Utc::now(),
        )
        .await
        .unwrap()
}

#[tokio::test]
async fn add_inserts_row_and_records_registered_at() {
    let store = store().await;
    let registry = McpRegistryStore::new(store.pool());
    let row = add_row(&registry, "tenant-a", "jira").await;
    assert_eq!(row.tenant_id, "tenant-a");
    assert_eq!(row.server_id, "jira");
    assert_eq!(row.kind, "http");
    assert!(row.registered_at > 0);
    assert_eq!(row.last_seen, Some(42));
}

#[test]
fn add_rejects_invalid_url() {
    let err = validate_url("not a url").unwrap_err().to_string();
    assert!(err.contains("invalid --url"));
}

#[tokio::test]
async fn list_returns_rows_for_the_right_tenant_only() {
    let store = store().await;
    let registry = McpRegistryStore::new(store.pool());
    add_row(&registry, "tenant-a", "jira").await;
    add_row(&registry, "tenant-b", "jira").await;
    let rows = registry.list("tenant-a").await.unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].tenant_id, "tenant-a");
}

#[tokio::test]
async fn list_markdown_shape_matches_markdown_family() {
    let store = store().await;
    let registry = McpRegistryStore::new(store.pool());
    add_row(&registry, "tenant-a", "jira").await;
    let rows = registry.list("tenant-a").await.unwrap();
    let md = render_markdown(&rows);
    assert!(md.starts_with("| server_id | kind | base_url | status | last_seen |"));
    assert!(md.contains("| `jira` | `http` | http://127.0.0.1:9 | ok | 42 |"));
}

#[tokio::test]
async fn remove_deletes_refuses_active_and_force_overrides() {
    let store = store().await;
    let registry = McpRegistryStore::new(store.pool());
    add_row(&registry, "tenant-a", "jira").await;
    let err = remove_with_active_count(&registry, "tenant-a", "jira", false, 1)
        .await
        .unwrap_err()
        .to_string();
    assert!(err.contains("has 1 active action"));
    let removed = remove_with_active_count(&registry, "tenant-a", "jira", true, 1)
        .await
        .unwrap();
    assert_eq!(removed.server_id, "jira");
    assert!(registry.get("tenant-a", "jira").await.unwrap().is_none());
}

#[tokio::test]
async fn remove_missing_returns_not_found() {
    let store = store().await;
    let registry = McpRegistryStore::new(store.pool());
    let err = remove_with_active_count(&registry, "tenant-a", "missing", false, 0)
        .await
        .unwrap_err()
        .to_string();
    assert!(err.contains("not found"));
}

#[tokio::test]
async fn tenant_isolation_makes_cross_tenant_remove_not_found() {
    let store = store().await;
    let registry = McpRegistryStore::new(store.pool());
    add_row(&registry, "tenant-a", "jira").await;
    let err = remove_with_active_count(&registry, "tenant-b", "jira", false, 0)
        .await
        .unwrap_err()
        .to_string();
    assert!(err.contains("tenant-b"));
    assert!(registry.get("tenant-a", "jira").await.unwrap().is_some());
}

#[tokio::test]
async fn duplicate_server_id_returns_conflict_error() {
    let store = store().await;
    let registry = McpRegistryStore::new(store.pool());
    add_row(&registry, "tenant-a", "jira").await;
    let err = registry
        .add(
            NewMcpServer {
                tenant_id: "tenant-a",
                server_id: "jira",
                kind: "http",
                base_url: "http://127.0.0.1:9",
                secrets_ref: None,
                last_seen: None,
                last_error: None,
            },
            Utc::now(),
        )
        .await
        .unwrap_err()
        .to_string();
    assert!(err.contains("conflict"));
}
