//! Authorization-matrix tests for the platform-staff gate (DEC-057 Phase 1).
//!
//! These drive [`authorize_staff`] directly with a resolved `user_id`, so the
//! default-deny + role-floor matrix is pinned without minting a session JWT.

use super::authorize_staff;
use argo_identity::platform_staff::{PlatformRole, PlatformStaff, PlatformStaffStatus, StaffAction};
use argo_identity::platform_staff_store::upsert_staff;
use argo_identity::user::UserId;
use argo_store::SessionStore;
use std::sync::Arc;

async fn store() -> Arc<SessionStore> {
    let store = Arc::new(SessionStore::open(":memory:").await.expect("open"));
    store.migrate().await.expect("migrate");
    store
}

async fn seed(store: &Arc<SessionStore>, role: PlatformRole) -> UserId {
    let uid = UserId::new();
    upsert_staff(store, &PlatformStaff::new(uid, role, None))
        .await
        .expect("seed staff");
    uid
}

#[tokio::test]
async fn no_user_id_is_denied() {
    let store = store().await;
    let deny = authorize_staff(&store, None, StaffAction::ReadAccounts)
        .await
        .expect_err("anonymous must be denied");
    assert!(deny.0.starts_with("403"));
}

#[tokio::test]
async fn non_staff_user_is_denied() {
    let store = store().await;
    // A real, valid user id that simply has no platform_staff row.
    let deny = authorize_staff(&store, Some(UserId::new()), StaffAction::ReadAccounts)
        .await
        .expect_err("non-staff must be denied");
    assert!(deny.0.starts_with("403"));
}

#[tokio::test]
async fn disabled_staff_is_denied() {
    let store = store().await;
    let uid = UserId::new();
    let mut grant = PlatformStaff::new(uid, PlatformRole::SuperAdmin, None);
    grant.status = PlatformStaffStatus::Disabled;
    upsert_staff(&store, &grant).await.unwrap();

    let deny = authorize_staff(&store, Some(uid), StaffAction::ReadAccounts)
        .await
        .expect_err("disabled staff must be denied");
    assert!(deny.0.starts_with("403"));
}

#[tokio::test]
async fn support_can_read_but_not_suspend() {
    let store = store().await;
    let uid = seed(&store, PlatformRole::Support).await;

    assert!(authorize_staff(&store, Some(uid), StaffAction::ReadAccounts)
        .await
        .is_ok());
    let deny = authorize_staff(&store, Some(uid), StaffAction::SuspendTenant)
        .await
        .expect_err("support must not suspend");
    assert!(deny.0.starts_with("403"));
}

#[tokio::test]
async fn billing_ops_and_super_admin_can_suspend() {
    let store = store().await;
    for role in [PlatformRole::BillingOps, PlatformRole::SuperAdmin] {
        let uid = seed(&store, role).await;
        assert!(
            authorize_staff(&store, Some(uid), StaffAction::SuspendTenant)
                .await
                .is_ok(),
            "{role:?} should be able to suspend"
        );
    }
}
