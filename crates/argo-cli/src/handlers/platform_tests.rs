//! Tests for the `argo platform staff` handlers (DEC-057 Phase 1).

use super::*;
use argo_identity::platform_staff::PlatformRole;
use argo_identity::platform_staff_store::get_staff;
use argo_identity::user::UserAccount;
use argo_identity::user_store::insert_user;

async fn mem() -> SessionStore {
    let store = SessionStore::open(":memory:").await.expect("open");
    store.migrate().await.expect("migrate");
    store
}

#[tokio::test]
async fn add_requires_existing_user() {
    let store = mem().await;
    let err = add(&store, "nobody@example.com", "support", true)
        .await
        .expect_err("must error when no such user");
    assert!(matches!(err, ArgoError::NotFound(_)));
}

#[tokio::test]
async fn add_bad_role_is_rejected() {
    let store = mem().await;
    let u = UserAccount::new("ceo@skycot.com".into());
    insert_user(&store, &u).await.unwrap();
    // 'owner' is a tenant role, not a platform-staff role.
    assert!(add(&store, "ceo@skycot.com", "owner", true).await.is_err());
}

#[tokio::test]
async fn add_grants_then_remove_revokes() {
    let store = mem().await;
    let u = UserAccount::new("ceo@skycot.com".into());
    insert_user(&store, &u).await.unwrap();

    add(&store, "ceo@skycot.com", "super_admin", true)
        .await
        .unwrap();
    let got = get_staff(&store, u.id).await.unwrap().expect("staff row");
    assert_eq!(got.role, PlatformRole::SuperAdmin);

    remove(&store, "ceo@skycot.com", true).await.unwrap();
    assert!(get_staff(&store, u.id).await.unwrap().is_none());
}
