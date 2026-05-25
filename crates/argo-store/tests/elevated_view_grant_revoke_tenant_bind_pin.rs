//! Regression pin for tenant-bound elevated-view grant revocation.
//!
//! A caller holding tenant_b context must not revoke tenant_a's grant
//! even when the grant id is known. The storage update must bind both
//! `id` and `tenant_id`.

use argo_store::elevated_view_grant::{ElevatedViewGrantStore, NewElevatedViewGrant};
use argo_store::SessionStore;
use argo_types::TenantId;
use chrono::{Duration, Utc};

fn grant_for(tenant_id: TenantId, principal_id: &str) -> NewElevatedViewGrant {
    let granted_at = Utc::now();
    NewElevatedViewGrant {
        tenant_id,
        principal_id: principal_id.to_string(),
        categories: vec!["email".to_string()],
        reason: "operator needs to inspect customer email".to_string(),
        granted_at,
        expires_at: granted_at + Duration::hours(1),
    }
}

#[tokio::test]
async fn revoke_rejects_cross_tenant_grant() {
    let store = SessionStore::open(":memory:").await.expect("open");
    store.migrate().await.expect("migrate");
    let grants = ElevatedViewGrantStore::new(store.pool());
    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();
    assert_ne!(tenant_a, tenant_b, "test requires distinct tenants");

    let id = grants
        .insert(grant_for(tenant_a, "human:operator-a@example.com"))
        .await
        .expect("insert tenant-a grant");

    let cross_tenant = grants
        .revoke(id, tenant_b, Utc::now())
        .await
        .expect("cross-tenant revoke call");
    assert!(!cross_tenant, "cross-tenant revoke must affect zero rows");
    let row_after_cross = grants
        .get(id)
        .await
        .expect("get after cross-tenant revoke")
        .expect("grant still exists");
    assert!(
        row_after_cross.revoked_at.is_none(),
        "cross-tenant revoke must leave revoked_at NULL"
    );

    let same_tenant = grants
        .revoke(id, tenant_a, Utc::now())
        .await
        .expect("same-tenant revoke call");
    assert!(same_tenant, "same-tenant revoke must still succeed");
    let row_after_same = grants
        .get(id)
        .await
        .expect("get after same-tenant revoke")
        .expect("grant still exists");
    assert!(
        row_after_same.revoked_at.is_some(),
        "same-tenant revoke must stamp revoked_at"
    );
}
