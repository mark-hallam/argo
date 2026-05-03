use super::*;
use argo_store::SessionStore;
use argo_types::Lock;
use std::sync::Arc;

async fn fresh_store() -> SkillStore {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    SkillStore::new(Arc::new(store))
}

fn skill_doc(name: &str, version: &str, lock: &str, body: &str) -> String {
    format!(
        "---\nname: {name}\nversion: \"{version}\"\ndescription: d\n\
         lock: {lock}\nprovenance:\n  source: builtin\n  hash: sha256-x\n---\n{body}"
    )
}

#[tokio::test]
async fn install_then_find_active() {
    let store = fresh_store().await;
    let raw = skill_doc("alpha", "0.1", "none", "alpha body\n");
    let s = store.install(&raw, None).await.unwrap();
    assert_eq!(s.frontmatter.name, "alpha");
    let active = store.find_active_by_name("alpha").await.unwrap();
    assert_eq!(active.unwrap().id, s.id);
}

#[tokio::test]
async fn install_rejects_duplicate_name() {
    let store = fresh_store().await;
    store
        .install(&skill_doc("alpha", "0.1", "none", "v1"), None)
        .await
        .unwrap();
    let err = store
        .install(&skill_doc("alpha", "0.2", "none", "v2"), None)
        .await
        .unwrap_err();
    assert!(matches!(err, argo_types::ArgoError::Schema(_)));
}

#[tokio::test]
async fn supersede_marks_old_inactive() {
    let store = fresh_store().await;
    let v1 = store
        .install(&skill_doc("alpha", "0.1", "none", "v1"), None)
        .await
        .unwrap();
    let v2 = store
        .supersede(
            &skill_doc("alpha", "0.2", "none", "v2"),
            None,
            PrincipalOverride::Agent,
        )
        .await
        .unwrap();
    assert_ne!(v1.id, v2.id);
    let active = store.find_active_by_name("alpha").await.unwrap().unwrap();
    assert_eq!(active.id, v2.id);
    assert_eq!(active.frontmatter.version, "0.2");
}

#[tokio::test]
async fn user_protected_blocks_agent_supersede() {
    let store = fresh_store().await;
    let v1 = store
        .install(&skill_doc("alpha", "0.1", "user-protected", "v1"), None)
        .await
        .unwrap();
    assert_eq!(v1.frontmatter.lock, Lock::None); // frontmatter parsed; lock applied via lock table
                                                 // Apply lock via the store, then attempt agent supersede.
    store
        .set_lock(v1.id, Lock::UserProtected, PrincipalOverride::Force)
        .await
        .unwrap();
    let err = store
        .supersede(
            &skill_doc("alpha", "0.2", "none", "v2"),
            None,
            PrincipalOverride::Agent,
        )
        .await
        .unwrap_err();
    assert!(matches!(err, argo_types::ArgoError::UserProtected { .. }));
    // Verify the original survived.
    let active = store.find_active_by_name("alpha").await.unwrap().unwrap();
    assert_eq!(active.id, v1.id);
}

#[tokio::test]
async fn force_principal_overrides_user_protected() {
    let store = fresh_store().await;
    let v1 = store
        .install(&skill_doc("alpha", "0.1", "none", "v1"), None)
        .await
        .unwrap();
    store
        .set_lock(v1.id, Lock::UserProtected, PrincipalOverride::Force)
        .await
        .unwrap();
    let v2 = store
        .supersede(
            &skill_doc("alpha", "0.2", "none", "v2"),
            None,
            PrincipalOverride::Force,
        )
        .await
        .unwrap();
    assert_ne!(v1.id, v2.id);
}

#[tokio::test]
async fn list_active_returns_only_heads() {
    let store = fresh_store().await;
    store
        .install(&skill_doc("alpha", "0.1", "none", "v1"), None)
        .await
        .unwrap();
    store
        .supersede(
            &skill_doc("alpha", "0.2", "none", "v2"),
            None,
            PrincipalOverride::Agent,
        )
        .await
        .unwrap();
    store
        .install(&skill_doc("beta", "0.1", "none", "b"), None)
        .await
        .unwrap();
    let active = store.list_active().await.unwrap();
    assert_eq!(active.len(), 2);
    let names: Vec<&str> = active.iter().map(|s| s.frontmatter.name.as_str()).collect();
    assert_eq!(names, vec!["alpha", "beta"]);
    let alpha = active
        .iter()
        .find(|s| s.frontmatter.name == "alpha")
        .unwrap();
    assert_eq!(alpha.frontmatter.version, "0.2");
}

#[tokio::test]
async fn agent_cannot_set_or_clear_locks() {
    let store = fresh_store().await;
    let s = store
        .install(&skill_doc("alpha", "0.1", "none", "x"), None)
        .await
        .unwrap();
    let err = store
        .set_lock(s.id, Lock::UserProtected, PrincipalOverride::Agent)
        .await
        .unwrap_err();
    assert!(matches!(err, argo_types::ArgoError::UserProtected { .. }));
}

#[tokio::test]
async fn find_active_returns_lock_state() {
    let store = fresh_store().await;
    let s = store
        .install(&skill_doc("alpha", "0.1", "none", "x"), None)
        .await
        .unwrap();
    store
        .set_lock(s.id, Lock::ContentLocked, PrincipalOverride::Force)
        .await
        .unwrap();
    let active = store.find_active_by_name("alpha").await.unwrap().unwrap();
    assert_eq!(active.frontmatter.lock, Lock::ContentLocked);
}

#[tokio::test]
async fn supersede_without_install_is_schema_error() {
    let store = fresh_store().await;
    let err = store
        .supersede(
            &skill_doc("never-installed", "0.1", "none", "x"),
            None,
            PrincipalOverride::Agent,
        )
        .await
        .unwrap_err();
    assert!(matches!(err, argo_types::ArgoError::Schema(_)));
}

#[tokio::test]
async fn promote_flips_active_pointer() {
    let store = fresh_store().await;
    let v1 = store
        .install(&skill_doc("alpha", "0.1", "none", "v1"), None)
        .await
        .unwrap();
    store
        .supersede(
            &skill_doc("alpha", "0.2", "none", "v2"),
            None,
            PrincipalOverride::Force,
        )
        .await
        .unwrap();
    let active_now = store.find_active_by_name("alpha").await.unwrap().unwrap();
    assert_eq!(active_now.frontmatter.version, "0.2");

    // Promote back to v0.1 — flips the chain.
    let promoted = store.promote("alpha", "0.1").await.unwrap();
    assert_eq!(promoted.id, v1.id);
    assert!(promoted.superseded_by.is_none());
    let active_again = store.find_active_by_name("alpha").await.unwrap().unwrap();
    assert_eq!(active_again.id, v1.id);
}

#[tokio::test]
async fn promote_unknown_version_is_schema_error() {
    let store = fresh_store().await;
    store
        .install(&skill_doc("alpha", "0.1", "none", "x"), None)
        .await
        .unwrap();
    let err = store.promote("alpha", "99.0").await.unwrap_err();
    assert!(matches!(err, argo_types::ArgoError::Schema(_)));
}

// ---- vertical scoping (#678) --------------------------------

#[tokio::test]
async fn install_for_vertical_persists_vertical_id_on_skill_row() {
    let store = fresh_store().await;
    let s = store
        .install_for_vertical(
            &skill_doc("greet", "0.1", "none", "hello"),
            None,
            Some("receptionist"),
        )
        .await
        .unwrap();
    assert_eq!(s.vertical_id.as_deref(), Some("receptionist"));

    // The lookup re-reads from the DB and must return the
    // same vertical_id.
    let active = store.find_active_by_name("greet").await.unwrap().unwrap();
    assert_eq!(active.vertical_id.as_deref(), Some("receptionist"));
}

#[tokio::test]
async fn bare_install_keeps_vertical_id_null() {
    // Backward-compat: existing callers (Forge etc.) using
    // `install()` get NULL vertical_id, same as before.
    let store = fresh_store().await;
    let s = store
        .install(&skill_doc("legacy", "0.1", "none", "x"), None)
        .await
        .unwrap();
    assert!(s.vertical_id.is_none());
}

// ---- learned-skill lifecycle (#681) ------------------------

#[tokio::test]
async fn fresh_install_defaults_to_active() {
    let store = fresh_store().await;
    let s = store
        .install(&skill_doc("a", "0.1", "none", "x"), None)
        .await
        .unwrap();
    assert_eq!(s.lifecycle_state, crate::LifecycleState::Active);
}

#[tokio::test]
async fn propose_for_review_starts_in_pending_review() {
    let store = fresh_store().await;
    // Need a real session id for the FK-ish learned_in. The
    // skill table has `learned_in BLOB` without a hard FK so
    // any UUID works for the unit test.
    let sid = argo_types::SessionId::new();
    let s = store
        .propose_for_review(&skill_doc("proposal", "0.1", "none", "x"), sid, "alpha")
        .await
        .unwrap();
    assert_eq!(s.lifecycle_state, crate::LifecycleState::PendingReview);
    assert_eq!(s.vertical_id.as_deref(), Some("alpha"));
    assert_eq!(s.learned_in, Some(sid));
}

#[tokio::test]
async fn pending_review_skill_is_invisible_to_vertical_lookup() {
    let store = fresh_store().await;
    let sid = argo_types::SessionId::new();
    store
        .propose_for_review(&skill_doc("proposal", "0.1", "none", "x"), sid, "alpha")
        .await
        .unwrap();
    // The agent's runtime view (list_skills_for_vertical) must
    // exclude pending_review rows — the whole point of #681.
    let visible = store.list_skills_for_vertical("alpha").await.unwrap();
    assert!(
        visible.is_empty(),
        "pending_review skill must NOT appear in agent's lookup; got {visible:?}"
    );
}

#[tokio::test]
async fn transition_pending_review_to_active_makes_skill_visible() {
    let store = fresh_store().await;
    let sid = argo_types::SessionId::new();
    let proposal = store
        .propose_for_review(
            &skill_doc("approve-me", "0.1", "none", "x"),
            sid,
            "alpha",
        )
        .await
        .unwrap();

    // Simulate human approval.
    let new_state = store
        .transition_lifecycle(proposal.id, crate::LifecycleState::Active)
        .await
        .unwrap();
    assert_eq!(new_state, crate::LifecycleState::Active);

    // Now visible to alpha-bound sessions.
    let visible = store.list_skills_for_vertical("alpha").await.unwrap();
    assert_eq!(visible.len(), 1);
    assert_eq!(visible[0].lifecycle_state, crate::LifecycleState::Active);
}

#[tokio::test]
async fn rejected_skill_never_becomes_visible() {
    let store = fresh_store().await;
    let sid = argo_types::SessionId::new();
    let proposal = store
        .propose_for_review(
            &skill_doc("rejected-skill", "0.1", "none", "x"),
            sid,
            "alpha",
        )
        .await
        .unwrap();
    store
        .transition_lifecycle(proposal.id, crate::LifecycleState::Rejected)
        .await
        .unwrap();
    let visible = store.list_skills_for_vertical("alpha").await.unwrap();
    assert!(visible.is_empty(), "rejected skill must NOT be visible");
}

#[tokio::test]
async fn illegal_transitions_are_rejected() {
    let store = fresh_store().await;
    let sid = argo_types::SessionId::new();
    let proposal = store
        .propose_for_review(
            &skill_doc("illegal-test", "0.1", "none", "x"),
            sid,
            "alpha",
        )
        .await
        .unwrap();
    // PendingReview → Archived is not a legal transition.
    let err = store
        .transition_lifecycle(proposal.id, crate::LifecycleState::Archived)
        .await
        .unwrap_err();
    let msg = format!("{err}");
    assert!(
        msg.contains("illegal lifecycle transition"),
        "got: {msg}"
    );
    // Active → PendingReview is also illegal — first promote
    // legitimately, then try to roll back to PendingReview.
    let pending2 = store
        .propose_for_review(
            &skill_doc("active-skill", "0.1", "none", "x"),
            sid,
            "alpha",
        )
        .await
        .unwrap();
    store
        .transition_lifecycle(pending2.id, crate::LifecycleState::Active)
        .await
        .unwrap();
    let err = store
        .transition_lifecycle(pending2.id, crate::LifecycleState::PendingReview)
        .await
        .unwrap_err();
    assert!(format!("{err}").contains("illegal"));
}

#[tokio::test]
async fn active_to_archived_is_legal_and_archived_is_invisible() {
    let store = fresh_store().await;
    let sid = argo_types::SessionId::new();
    let proposal = store
        .propose_for_review(
            &skill_doc("rollback-me", "0.1", "none", "x"),
            sid,
            "alpha",
        )
        .await
        .unwrap();
    store
        .transition_lifecycle(proposal.id, crate::LifecycleState::Active)
        .await
        .unwrap();
    // Now roll back.
    let new_state = store
        .transition_lifecycle(proposal.id, crate::LifecycleState::Archived)
        .await
        .unwrap();
    assert_eq!(new_state, crate::LifecycleState::Archived);
    let visible = store.list_skills_for_vertical("alpha").await.unwrap();
    assert!(visible.is_empty(), "archived skill must NOT be visible");
}

#[tokio::test]
async fn list_skills_bundled_by_vertical_filters_by_vertical_id() {
    let store = fresh_store().await;
    // Two verticals install distinct skills.
    store
        .install_for_vertical(
            &skill_doc("intent", "0.1", "none", "x"),
            None,
            Some("receptionist"),
        )
        .await
        .unwrap();
    store
        .install_for_vertical(
            &skill_doc("invariants", "0.1", "none", "y"),
            None,
            Some("pr-checker"),
        )
        .await
        .unwrap();
    // Plus a legacy skill with no vertical.
    store
        .install(&skill_doc("legacy", "0.1", "none", "z"), None)
        .await
        .unwrap();

    let recept = store
        .list_skills_bundled_by_vertical("receptionist")
        .await
        .unwrap();
    assert_eq!(recept.len(), 1);
    assert_eq!(recept[0].frontmatter.name, "intent");

    let prc = store
        .list_skills_bundled_by_vertical("pr-checker")
        .await
        .unwrap();
    assert_eq!(prc.len(), 1);
    assert_eq!(prc[0].frontmatter.name, "invariants");

    // No cross-contamination — pr-checker can't see receptionist's,
    // and vice versa.
    assert!(recept.iter().all(|s| s.frontmatter.name != "invariants"));
    assert!(prc.iter().all(|s| s.frontmatter.name != "intent"));

    // Legacy skill with NULL vertical_id is in NEITHER list.
    assert!(recept.iter().all(|s| s.frontmatter.name != "legacy"));
    assert!(prc.iter().all(|s| s.frontmatter.name != "legacy"));

    // Asking for an unknown vertical returns an empty list.
    let ghost = store
        .list_skills_bundled_by_vertical("ghost")
        .await
        .unwrap();
    assert!(ghost.is_empty());
}
