//! Store-level tests for `platform_staff` CRUD (DEC-057 Phase 1).

use super::*;
use crate::platform_staff::{PlatformRole, PlatformStaff, PlatformStaffStatus};
use crate::user::UserId;
use argo_store::SessionStore;

async fn store() -> SessionStore {
    let store = SessionStore::open(":memory:").await.expect("open");
    store.migrate().await.expect("migrate");
    store
}

#[tokio::test]
async fn absent_row_is_default_deny() {
    let store = store().await;
    // A user with no grant row resolves to None — the default-deny signal.
    assert!(get_staff(&store, UserId::new()).await.unwrap().is_none());
}

#[tokio::test]
async fn upsert_then_get_round_trips() {
    let store = store().await;
    let uid = UserId::new();
    let grant = PlatformStaff::new(uid, PlatformRole::BillingOps, Some("system".into()));
    upsert_staff(&store, &grant).await.unwrap();

    let got = get_staff(&store, uid).await.unwrap().expect("present");
    assert_eq!(got.user_id, uid);
    assert_eq!(got.role, PlatformRole::BillingOps);
    assert_eq!(got.status, PlatformStaffStatus::Active);
    assert_eq!(got.created_by.as_deref(), Some("system"));
}

#[tokio::test]
async fn upsert_is_idempotent_and_updates_role() {
    let store = store().await;
    let uid = UserId::new();
    upsert_staff(&store, &PlatformStaff::new(uid, PlatformRole::Support, None))
        .await
        .unwrap();
    // Re-grant with a higher role — must update in place, not duplicate.
    upsert_staff(&store, &PlatformStaff::new(uid, PlatformRole::SuperAdmin, None))
        .await
        .unwrap();

    assert_eq!(list_staff(&store).await.unwrap().len(), 1);
    assert_eq!(
        get_staff(&store, uid).await.unwrap().unwrap().role,
        PlatformRole::SuperAdmin
    );
}

#[tokio::test]
async fn disabled_grant_persists_status() {
    let store = store().await;
    let uid = UserId::new();
    let mut grant = PlatformStaff::new(uid, PlatformRole::SuperAdmin, None);
    grant.status = PlatformStaffStatus::Disabled;
    upsert_staff(&store, &grant).await.unwrap();

    let got = get_staff(&store, uid).await.unwrap().unwrap();
    assert!(!got.is_active());
}

#[tokio::test]
async fn remove_staff_revokes() {
    let store = store().await;
    let uid = UserId::new();
    upsert_staff(&store, &PlatformStaff::new(uid, PlatformRole::Support, None))
        .await
        .unwrap();
    assert_eq!(remove_staff(&store, uid).await.unwrap(), 1);
    assert!(get_staff(&store, uid).await.unwrap().is_none());
    // Removing a non-staff user affects zero rows.
    assert_eq!(remove_staff(&store, UserId::new()).await.unwrap(), 0);
}
