//! Inline tests for [`IdentityStore::create_worker`] / `get_worker` /
//! `list_workers`. Lives in its own file so `store.rs` stays under the
//! 500-LOC ceiling.
//!
//! Extended for Stream J5 with coverage for
//! [`IdentityStore::create_worker_from_draft`]: happy-path commit,
//! validation rejection (short charter, no enabled duty), and
//! approval-policy → `AutoApprovalPolicy.enabled` mapping. The brief
//! cited a `store_tests.rs` file that does not exist on main; existing
//! tests live in this `store_inline_tests.rs` so we extend here.

#![cfg(test)]

use crate::{
    ApprovalPolicyChoice, Duty, HireDraft, HireDutyDraft, IdentityStore, Persona, Role, Worker,
};
use argo_store::SessionStore;
use argo_types::TenantId;

fn sample_draft() -> HireDraft {
    HireDraft {
        suggested_name: "Maya".into(),
        charter: "Own outbound marketplace follow-up end-to-end.".into(),
        suggested_vertical: "marketplace-seller".into(),
        approval_policy: ApprovalPolicyChoice {
            level: "balanced".into(),
        },
        duties: vec![HireDutyDraft {
            cron: "0 9 * * 1-5".into(),
            entry_action: "follow_up".into(),
            description: "Send approved follow-up.".into(),
            default_enabled: true,
        }],
        voice_profile: None,
        tagline: Some("Marketplace seller persona".into()),
    }
}

#[tokio::test]
async fn create_get_list_roundtrip() {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    let tenant = TenantId::new();
    let mut role = Role::new("Own outbound follow-up.");
    role.duties
        .push(Duty::new("0 9 * * 1-5", "follow_up", "Send approved follow-up."));
    let worker = Worker::draft(tenant, "gtm-engineer", Persona::named("Maya", "op"), role);
    let identities = IdentityStore::new(&store);

    identities.create_worker(&worker).await.unwrap();
    let loaded = identities
        .get_worker(tenant, worker.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(loaded.persona.name, "Maya");
    assert_eq!(loaded.role.duties[0].entry_action, "follow_up");
    assert_eq!(identities.list_workers(tenant, false).await.unwrap().len(), 1);
}

#[tokio::test]
async fn create_worker_from_draft_persists_persona_role_duties() {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    let tenant = TenantId::new();
    let identities = IdentityStore::new(&store);
    let worker_id = identities
        .create_worker_from_draft(&sample_draft(), tenant, "human:mark@example.com")
        .await
        .unwrap();
    let loaded = identities.get_worker(tenant, worker_id).await.unwrap().unwrap();
    assert_eq!(loaded.persona.name, "Maya");
    assert_eq!(loaded.persona.builder_attribution, "human:mark@example.com");
    assert_eq!(loaded.vertical_id, "marketplace-seller");
    assert_eq!(loaded.role.duties.len(), 1);
    assert_eq!(loaded.role.duties[0].entry_action, "follow_up");
    assert!(loaded
        .role
        .escalation
        .as_deref()
        .is_some_and(|e| e.contains("balanced")));
    // balanced → auto_approval disabled (Ask-everything default).
    assert!(!loaded.auto_approval.enabled);
}

#[tokio::test]
async fn create_worker_from_draft_aggressive_enables_auto_approval() {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    let tenant = TenantId::new();
    let identities = IdentityStore::new(&store);
    let mut draft = sample_draft();
    draft.approval_policy.level = "aggressive".into();
    let worker_id = identities
        .create_worker_from_draft(&draft, tenant, "human:op")
        .await
        .unwrap();
    let loaded = identities.get_worker(tenant, worker_id).await.unwrap().unwrap();
    assert!(loaded.auto_approval.enabled);
}

#[tokio::test]
async fn create_worker_from_draft_rejects_short_charter() {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    let tenant = TenantId::new();
    let identities = IdentityStore::new(&store);
    let mut draft = sample_draft();
    draft.charter = "too short".into();
    let err = identities
        .create_worker_from_draft(&draft, tenant, "human:op")
        .await
        .unwrap_err();
    assert!(matches!(err, argo_types::ArgoError::Schema(_)), "{err:?}");
}

#[tokio::test]
async fn create_worker_from_draft_requires_one_enabled_duty() {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    let tenant = TenantId::new();
    let identities = IdentityStore::new(&store);
    let mut draft = sample_draft();
    draft.duties[0].default_enabled = false;
    let err = identities
        .create_worker_from_draft(&draft, tenant, "human:op")
        .await
        .unwrap_err();
    assert!(
        matches!(&err, argo_types::ArgoError::Schema(msg) if msg.contains("defaultEnabled")),
        "{err:?}"
    );
}

#[tokio::test]
async fn create_worker_from_draft_rejects_unknown_policy_level() {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    let tenant = TenantId::new();
    let identities = IdentityStore::new(&store);
    let mut draft = sample_draft();
    draft.approval_policy.level = "yolo".into();
    let err = identities
        .create_worker_from_draft(&draft, tenant, "human:op")
        .await
        .unwrap_err();
    assert!(
        matches!(&err, argo_types::ArgoError::Schema(msg) if msg.contains("approvalPolicy")),
        "{err:?}"
    );
}
