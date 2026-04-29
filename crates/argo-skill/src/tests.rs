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
