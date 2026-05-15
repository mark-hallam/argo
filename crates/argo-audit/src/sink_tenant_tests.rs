use crate::{AuditKind, AuditSink, ANONYMOUS_AUDIT_TENANT_ID, SYSTEM_AUDIT_TENANT_ID};
use argo_store::SessionStore;
use std::sync::Arc;

#[test]
fn sentinel_tenant_ids_are_stable() {
    assert_eq!(SYSTEM_AUDIT_TENANT_ID, "system");
    assert_eq!(ANONYMOUS_AUDIT_TENANT_ID, "anonymous");
}

#[tokio::test]
async fn system_and_anonymous_sinks_stamp_expected_tenant() {
    let system = write_and_fetch_tenant(SYSTEM_AUDIT_TENANT_ID).await;
    let anonymous = write_and_fetch_tenant(ANONYMOUS_AUDIT_TENANT_ID).await;

    assert_eq!(system, SYSTEM_AUDIT_TENANT_ID);
    assert_eq!(anonymous, ANONYMOUS_AUDIT_TENANT_ID);
}

async fn write_and_fetch_tenant(tenant_id: &str) -> String {
    let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
    store.migrate().await.unwrap();
    let sink = AuditSink::new_with_tenant(Arc::clone(&store), tenant_id.to_string());
    sink.write(None, AuditKind::HttpRequest, serde_json::json!({}), None)
        .await
        .unwrap();
    sink.flush_pending().await.unwrap();

    let (column_tenant, payload): (String, String) =
        sqlx::query_as("SELECT tenant_id, payload_json FROM audit_record LIMIT 1")
            .fetch_one(store.pool())
            .await
            .unwrap();
    let payload: serde_json::Value = serde_json::from_str(&payload).unwrap();
    assert_eq!(payload["tenant_id"].as_str(), Some(tenant_id));
    column_tenant
}
