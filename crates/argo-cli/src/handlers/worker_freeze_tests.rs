//! Tests for the worker freeze CLI handler.
#![cfg(test)]

use super::*;
use argo_store::SessionStore;
use argo_types::{ArgoError, Capability};
use std::path::Path;

async fn fixture() -> (SessionStore, TenantId, Principal) {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    let tenant_id = TenantId::new();
    let principal = Principal::Local {
        os_user: "alice@acme".into(),
    };
    (store, tenant_id, principal)
}

#[tokio::test]
async fn freeze_with_valid_capability_succeeds() {
    let (store, tenant_id, principal) = fixture().await;
    let row = freeze_with_role(
        &store,
        tenant_id,
        "W1",
        "investigating prompt injection",
        "owner",
        Path::new("."),
        &principal,
    )
    .await
    .unwrap();

    assert_eq!(row.worker_id, "W1");
    assert_eq!(row.reason, "investigating prompt injection");
    assert_eq!(row.frozen_by, "alice@acme");
    assert!(
        freeze_store::is_frozen(store.pool(), &tenant_id.to_string(), "W1")
            .await
            .unwrap()
            .is_some()
    );
}

#[tokio::test]
async fn freeze_and_unfreeze_without_capability_return_rbac_deny() {
    let (store, tenant_id, principal) = fixture().await;
    let freeze_err = freeze_with_role(
        &store,
        tenant_id,
        "W1",
        "operator request",
        "user",
        Path::new("."),
        &principal,
    )
    .await
    .unwrap_err();
    assert_worker_freeze_denied(freeze_err);

    let unfreeze_err =
        unfreeze_with_role(&store, tenant_id, "W1", "user", Path::new("."), &principal)
            .await
            .unwrap_err();
    assert_worker_freeze_denied(unfreeze_err);
}

#[tokio::test]
async fn double_freeze_updates_reason() {
    let (store, tenant_id, principal) = fixture().await;
    freeze_with_role(
        &store,
        tenant_id,
        "W1",
        "first reason",
        "admin",
        Path::new("."),
        &principal,
    )
    .await
    .unwrap();

    let row = freeze_with_role(
        &store,
        tenant_id,
        "W1",
        "second reason",
        "admin",
        Path::new("."),
        &principal,
    )
    .await
    .unwrap();

    assert_eq!(row.reason, "second reason");
    let active = freeze_store::is_frozen(store.pool(), &tenant_id.to_string(), "W1")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(active.reason, "second reason");
}

#[tokio::test]
async fn unfreeze_removes_active_row() {
    let (store, tenant_id, principal) = fixture().await;
    freeze_with_role(
        &store,
        tenant_id,
        "W1",
        "operator request",
        "owner",
        Path::new("."),
        &principal,
    )
    .await
    .unwrap();

    let changed = unfreeze_with_role(&store, tenant_id, "W1", "owner", Path::new("."), &principal)
        .await
        .unwrap();

    assert!(changed);
    assert!(
        freeze_store::is_frozen(store.pool(), &tenant_id.to_string(), "W1")
            .await
            .unwrap()
            .is_none()
    );
}

#[test]
fn owner_and_admin_default_roles_include_worker_freeze() {
    let roles = argo_capability::default_roles_for_workspace(Path::new("."));
    for role_name in ["owner", "admin"] {
        let role = roles.iter().find(|role| role.name == role_name).unwrap();
        assert!(role.capabilities.covers(&Capability::WorkerFreeze));
    }
}

fn assert_worker_freeze_denied(err: ArgoError) {
    match err {
        ArgoError::RbacDenied { role, cap } => {
            assert_eq!(role, "user");
            assert_eq!(cap, Capability::WorkerFreeze);
        }
        other => panic!("expected worker.freeze RBAC deny, got {other:?}"),
    }
}
