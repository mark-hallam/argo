//! Integration tests for [`crate::tenant_provision`].

use super::{provision_for_user, ProvisionRequest};
use crate::tenant_store::list_tenants;
use crate::user::UserAccount;
use crate::user_store::insert_user;
use crate::user_tenant_store::list_tenants_for_user;
use argo_store::SessionStore;

async fn store() -> SessionStore {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    store
}

async fn seed_user(store: &SessionStore, email: &str) -> UserAccount {
    let u = UserAccount::new(email.into());
    insert_user(store, &u).await.unwrap();
    u
}

#[tokio::test]
async fn first_call_creates_tenant_workspace_and_owner_link() {
    let store = store().await;
    let user = seed_user(&store, "alice@acme.com").await;
    let outcome = provision_for_user(
        &store,
        ProvisionRequest {
            user_id: user.id,
            user_email: &user.email,
            workspace_name: "Acme Workspace",
            company_name: Some("Acme Co"),
        },
    )
    .await
    .unwrap();
    assert!(outcome.created);

    let tenants = list_tenants(&store).await.unwrap();
    assert_eq!(tenants.len(), 1);
    assert_eq!(tenants[0].id, outcome.tenant_id);
    assert_eq!(tenants[0].name, "Acme Co");
    assert_eq!(tenants[0].billing_email, "alice@acme.com");

    let links = list_tenants_for_user(&store, user.id).await.unwrap();
    assert_eq!(links.len(), 1);
    assert_eq!(links[0].tenant_id, outcome.tenant_id);
    assert_eq!(
        links[0].role,
        crate::user_tenant::UserTenantRole::Owner
    );
}

#[tokio::test]
async fn second_call_for_same_user_is_idempotent() {
    let store = store().await;
    let user = seed_user(&store, "alice@acme.com").await;
    let first = provision_for_user(
        &store,
        ProvisionRequest {
            user_id: user.id,
            user_email: &user.email,
            workspace_name: "Acme",
            company_name: None,
        },
    )
    .await
    .unwrap();
    let second = provision_for_user(
        &store,
        ProvisionRequest {
            user_id: user.id,
            user_email: &user.email,
            workspace_name: "Acme",
            company_name: None,
        },
    )
    .await
    .unwrap();
    assert_eq!(first.tenant_id, second.tenant_id);
    assert_eq!(first.workspace_id, second.workspace_id);
    assert!(!second.created);
    assert_eq!(list_tenants(&store).await.unwrap().len(), 1);
}

#[tokio::test]
async fn company_name_falls_back_to_workspace_name() {
    let store = store().await;
    let user = seed_user(&store, "bob@bravo.com").await;
    let outcome = provision_for_user(
        &store,
        ProvisionRequest {
            user_id: user.id,
            user_email: &user.email,
            workspace_name: "Bravo Workspace",
            company_name: None,
        },
    )
    .await
    .unwrap();
    let tenants = list_tenants(&store).await.unwrap();
    assert_eq!(tenants.len(), 1);
    assert_eq!(tenants[0].id, outcome.tenant_id);
    assert_eq!(tenants[0].name, "Bravo Workspace");
}

#[tokio::test]
async fn empty_workspace_name_is_rejected() {
    let store = store().await;
    let user = seed_user(&store, "alice@acme.com").await;
    let err = provision_for_user(
        &store,
        ProvisionRequest {
            user_id: user.id,
            user_email: &user.email,
            workspace_name: "   ",
            company_name: None,
        },
    )
    .await
    .unwrap_err();
    assert!(err.to_string().to_lowercase().contains("workspace_name"));
}

#[tokio::test]
async fn two_users_get_isolated_tenants() {
    let store = store().await;
    let alice = seed_user(&store, "alice@acme.com").await;
    let bob = seed_user(&store, "bob@bravo.com").await;
    let a = provision_for_user(
        &store,
        ProvisionRequest {
            user_id: alice.id,
            user_email: &alice.email,
            workspace_name: "Acme",
            company_name: None,
        },
    )
    .await
    .unwrap();
    let b = provision_for_user(
        &store,
        ProvisionRequest {
            user_id: bob.id,
            user_email: &bob.email,
            workspace_name: "Bravo",
            company_name: None,
        },
    )
    .await
    .unwrap();
    assert_ne!(a.tenant_id, b.tenant_id);
    assert_ne!(a.workspace_id, b.workspace_id);
    assert_eq!(list_tenants(&store).await.unwrap().len(), 2);
}
