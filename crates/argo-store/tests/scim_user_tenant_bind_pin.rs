//! Regression pins for `mission-scim-user-update-tenant-pinned`.
//!
//! SCIM user mutation helpers must bind both `id` and `tenant_id` so a
//! caller in tenant_b cannot patch or deactivate a tenant_a row even if
//! it knows the row UUID.

use argo_store::scim_user::{NewScimUser, ScimUserPatch, ScimUserStore};
use argo_store::SessionStore;
use argo_types::TenantId;
use chrono::{TimeZone, Utc};

fn user_for(tenant_id: TenantId, user_name: &str) -> NewScimUser {
    NewScimUser {
        tenant_id,
        external_id: Some("idp-collision".to_string()),
        user_name: user_name.to_string(),
        display_name: Some(user_name.split('@').next().unwrap_or(user_name).to_string()),
        emails: vec![user_name.to_string()],
        active: true,
        created_at: Utc::now(),
    }
}

fn patch_display_name(name: &str) -> ScimUserPatch {
    ScimUserPatch {
        display_name: Some(name.to_string()),
        updated_at: Utc::now(),
        ..ScimUserPatch::default()
    }
}

async fn store() -> SessionStore {
    let store = SessionStore::open(":memory:").await.expect("open");
    store.migrate().await.expect("migrate");
    store
}

#[tokio::test]
async fn patch_user_rejects_cross_tenant_id_at_sql() {
    let store = store().await;
    let scim = ScimUserStore::new(store.pool());
    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();

    let id_a = scim
        .insert(user_for(tenant_a, "alice@example.com"))
        .await
        .expect("insert tenant_a row");
    let id_b = scim
        .insert(user_for(tenant_b, "alice@example.com"))
        .await
        .expect("insert tenant_b row with colliding idp identity");

    let cross_tenant = scim
        .update(tenant_b, id_a, patch_display_name("owned-by-tenant-b"))
        .await
        .expect("cross-tenant patch query");
    assert!(
        !cross_tenant,
        "tenant_b must affect zero rows when patching tenant_a's SCIM user id"
    );

    let row_a = scim
        .get(tenant_a, id_a)
        .await
        .expect("read tenant_a row")
        .expect("tenant_a row still exists");
    assert_eq!(row_a.display_name.as_deref(), Some("alice"));
    assert_eq!(row_a.external_id.as_deref(), Some("idp-collision"));
    assert!(row_a.active);
    assert!(
        scim.get(tenant_b, id_a)
            .await
            .expect("read from tenant_b")
            .is_none(),
        "tenant-bound read must not expose tenant_a's SCIM row"
    );

    let same_tenant = scim
        .update(tenant_b, id_b, patch_display_name("tenant-b-update"))
        .await
        .expect("same-tenant patch query");
    assert!(same_tenant, "same-tenant patch should still update one row");
    let row_b = scim
        .get(tenant_b, id_b)
        .await
        .expect("read tenant_b row")
        .expect("tenant_b row exists");
    assert_eq!(row_b.display_name.as_deref(), Some("tenant-b-update"));
}

#[tokio::test]
async fn deactivate_user_rejects_cross_tenant_id_at_sql() {
    let store = store().await;
    let scim = ScimUserStore::new(store.pool());
    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();
    let id_a = scim
        .insert(user_for(tenant_a, "deprovision@example.com"))
        .await
        .expect("insert tenant_a row");

    let cross_tenant = scim
        .soft_delete(tenant_b, id_a, Utc::now())
        .await
        .expect("cross-tenant deactivate query");
    assert!(
        !cross_tenant,
        "tenant_b must affect zero rows when deactivating tenant_a's SCIM user id"
    );
    assert!(
        scim.tombstone(id_a)
            .await
            .expect("tombstone lookup")
            .is_none(),
        "cross-tenant deactivate must not insert a tombstone"
    );
    let live_after_cross = scim
        .get(tenant_a, id_a)
        .await
        .expect("read tenant_a row")
        .expect("tenant_a row still live");
    assert!(live_after_cross.deleted_at.is_none());
    assert!(live_after_cross.active);

    let deleted_at = Utc.timestamp_opt(1_700_000_001, 0).unwrap();
    let same_tenant = scim
        .soft_delete(tenant_a, id_a, deleted_at)
        .await
        .expect("same-tenant deactivate query");
    assert!(same_tenant, "same-tenant deactivate should still work");
    assert!(
        scim.get(tenant_a, id_a)
            .await
            .expect("live read after deactivate")
            .is_none(),
        "soft-deleted row should not appear in live tenant-bound reads"
    );
    let deleted = scim
        .get_including_deleted(tenant_a, id_a)
        .await
        .expect("including-deleted read")
        .expect("deleted row still stored");
    assert_eq!(deleted.tenant_id, tenant_a);
    assert_eq!(deleted.deleted_at, Some(deleted_at));
    let tombstone = scim
        .tombstone(id_a)
        .await
        .expect("tombstone lookup")
        .expect("same-tenant deactivate writes tombstone");
    assert_eq!(tombstone.tenant_id, tenant_a);
    assert_eq!(tombstone.reason, "scim_delete");
}
