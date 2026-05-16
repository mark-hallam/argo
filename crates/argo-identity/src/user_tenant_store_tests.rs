//! Integration tests for [`crate::user_tenant_store`].

use super::{link_user_to_tenant, list_tenants_for_user, list_users_in_tenant};
use crate::tenant::Tenant;
use crate::tenant_store::insert_tenant;
use crate::user::UserAccount;
use crate::user_store::insert_user;
use crate::user_tenant::UserTenantRole;
use argo_store::SessionStore;
use argo_types::TenantId;

async fn store() -> SessionStore {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    store
}

async fn seed_tenant(store: &SessionStore, name: &str) -> Tenant {
    let t = Tenant::new(name.into(), format!("billing@{name}.com"));
    insert_tenant(store, &t).await.unwrap();
    t
}

async fn seed_user(store: &SessionStore, email: &str) -> UserAccount {
    let u = UserAccount::new(email.into());
    insert_user(store, &u).await.unwrap();
    u
}

#[tokio::test]
async fn link_creates_row_with_supplied_role() {
    let store = store().await;
    let t = seed_tenant(&store, "acme").await;
    let u = seed_user(&store, "alice@acme.com").await;
    let link = link_user_to_tenant(&store, u.id, t.id, UserTenantRole::Owner)
        .await
        .unwrap();
    assert_eq!(link.user_id, u.id);
    assert_eq!(link.tenant_id, t.id);
    assert_eq!(link.role, UserTenantRole::Owner);
}

#[tokio::test]
async fn duplicate_link_is_rejected() {
    let store = store().await;
    let t = seed_tenant(&store, "acme").await;
    let u = seed_user(&store, "alice@acme.com").await;
    link_user_to_tenant(&store, u.id, t.id, UserTenantRole::Owner)
        .await
        .unwrap();
    let err = link_user_to_tenant(&store, u.id, t.id, UserTenantRole::Admin)
        .await
        .expect_err("composite PK");
    assert!(
        err.to_string().to_lowercase().contains("unique")
            || err.to_string().to_lowercase().contains("primary"),
        "got: {err}"
    );
}

#[tokio::test]
async fn list_tenants_for_user_returns_only_that_users_links() {
    let store = store().await;
    let acme = seed_tenant(&store, "acme").await;
    let bravo = seed_tenant(&store, "bravo").await;
    let alice = seed_user(&store, "alice@acme.com").await;
    let bob = seed_user(&store, "bob@bravo.com").await;
    link_user_to_tenant(&store, alice.id, acme.id, UserTenantRole::Owner)
        .await
        .unwrap();
    link_user_to_tenant(&store, alice.id, bravo.id, UserTenantRole::Guest)
        .await
        .unwrap();
    link_user_to_tenant(&store, bob.id, acme.id, UserTenantRole::Admin)
        .await
        .unwrap();
    let mine = list_tenants_for_user(&store, alice.id).await.unwrap();
    assert_eq!(mine.len(), 2);
    let tenants: std::collections::HashSet<_> = mine.iter().map(|l| l.tenant_id).collect();
    assert!(tenants.contains(&acme.id));
    assert!(tenants.contains(&bravo.id));
}

#[tokio::test]
async fn list_users_in_tenant_returns_each_member_with_role() {
    let store = store().await;
    let acme = seed_tenant(&store, "acme").await;
    let alice = seed_user(&store, "alice@acme.com").await;
    let bob = seed_user(&store, "bob@acme.com").await;
    link_user_to_tenant(&store, alice.id, acme.id, UserTenantRole::Owner)
        .await
        .unwrap();
    link_user_to_tenant(&store, bob.id, acme.id, UserTenantRole::Admin)
        .await
        .unwrap();
    let members = list_users_in_tenant(&store, acme.id).await.unwrap();
    assert_eq!(members.len(), 2);
    let by_user: std::collections::HashMap<_, _> =
        members.iter().map(|l| (l.user_id, l.role)).collect();
    assert_eq!(by_user.get(&alice.id), Some(&UserTenantRole::Owner));
    assert_eq!(by_user.get(&bob.id), Some(&UserTenantRole::Admin));
}

#[tokio::test]
async fn empty_listings_for_unknown_ids() {
    let store = store().await;
    let tenants = list_tenants_for_user(&store, crate::user::UserId::new())
        .await
        .unwrap();
    assert!(tenants.is_empty());
    let users = list_users_in_tenant(&store, TenantId::new()).await.unwrap();
    assert!(users.is_empty());
}
