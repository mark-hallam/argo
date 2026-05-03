//! Tests for `turn::build_system_prompt`. Extracted from
//! `turn.rs` (#679) to keep that file under the 500-LOC
//! budget after vertical-scoped skill lookup landed.
//!
//! Loaded via `#[cfg(test)] #[path = "turn_tests.rs"] mod tests;`
//! at the bottom of `turn.rs`.

use super::*;
use argo_store::SessionStore;

#[tokio::test]
async fn build_system_prompt_returns_none_for_empty_workspace() {
    let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
    store.migrate().await.unwrap();
    let prompt = build_system_prompt(&store, SessionId(uuid::Uuid::nil()), None)
        .await
        .unwrap();
    assert!(prompt.is_none());
}

#[tokio::test]
async fn build_system_prompt_concatenates_active_skills() {
    let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
    store.migrate().await.unwrap();
    let skills = argo_skill::SkillStore::new(Arc::clone(&store));
    let raw = "---\nname: alpha\nversion: \"0.1\"\ndescription: D\n\
               lock: none\nprovenance:\n  source: builtin\n  hash: sha256-x\n---\nbody-of-alpha";
    skills.install(raw, None).await.unwrap();
    let prompt = build_system_prompt(&store, SessionId(uuid::Uuid::nil()), None)
        .await
        .unwrap()
        .unwrap();
    assert!(prompt.contains("## alpha (v0.1)"));
    assert!(prompt.contains("body-of-alpha"));
    assert!(prompt.starts_with("You have access to"));
}

#[tokio::test]
async fn build_system_prompt_honours_per_session_attachments() {
    let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
    store.migrate().await.unwrap();
    let (tenant, ws) = store.ensure_workspace("ws", "/tmp/sa").await.unwrap();
    let sid = store
        .create_session(tenant, ws, "m", "p", Some("t"), None)
        .await
        .unwrap();

    let skills = argo_skill::SkillStore::new(Arc::clone(&store));
    let raw_a = "---\nname: alpha\nversion: \"0.1\"\ndescription: D\n\
                 lock: none\nprovenance:\n  source: x\n  hash: sha256-x\n---\nbody-A";
    let raw_b = "---\nname: beta\nversion: \"0.1\"\ndescription: D\n\
                 lock: none\nprovenance:\n  source: x\n  hash: sha256-x\n---\nbody-B";
    let _alpha = skills.install(raw_a, None).await.unwrap();
    let beta = skills.install(raw_b, None).await.unwrap();

    // Default behaviour: both active skills are in scope.
    let prompt_default = build_system_prompt(&store, sid, None).await.unwrap().unwrap();
    assert!(prompt_default.contains("body-A"));
    assert!(prompt_default.contains("body-B"));

    // Attach only beta — alpha should now drop out.
    skills.attach_to_session(sid, beta.id).await.unwrap();
    let prompt_curated = build_system_prompt(&store, sid, None).await.unwrap().unwrap();
    assert!(prompt_curated.contains("body-B"));
    assert!(!prompt_curated.contains("body-A"));

    // Detach: falls back to all-active.
    let removed = skills.detach_from_session(sid, beta.id).await.unwrap();
    assert_eq!(removed, 1);
    let prompt_after = build_system_prompt(&store, sid, None).await.unwrap().unwrap();
    assert!(prompt_after.contains("body-A"));
    assert!(prompt_after.contains("body-B"));
}

#[tokio::test]
async fn build_system_prompt_for_bound_session_filters_to_vertical_skills() {
    // #679 — when a session is vertical-bound and has no
    // explicit attachments, the prompt fallback uses
    // SkillStore::list_skills_for_vertical, which excludes
    // other verticals' skills + workspace-global skills.
    let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
    store.migrate().await.unwrap();
    let skills = argo_skill::SkillStore::new(Arc::clone(&store));

    // alpha-skill: bundled by vertical "alpha"
    let alpha_doc = "---\nname: alpha-skill\nversion: \"0.1\"\ndescription: D\n\
                     lock: none\nprovenance:\n  source: x\n  hash: sha256-x\n---\nalpha-body";
    skills
        .install_for_vertical(alpha_doc, None, Some("alpha"))
        .await
        .unwrap();
    // beta-skill: bundled by vertical "beta"
    let beta_doc = "---\nname: beta-skill\nversion: \"0.1\"\ndescription: D\n\
                    lock: none\nprovenance:\n  source: x\n  hash: sha256-x\n---\nbeta-body";
    skills
        .install_for_vertical(beta_doc, None, Some("beta"))
        .await
        .unwrap();
    // legacy-skill: workspace-global (no vertical context)
    let legacy_doc = "---\nname: legacy\nversion: \"0.1\"\ndescription: D\n\
                      lock: none\nprovenance:\n  source: x\n  hash: sha256-x\n---\nlegacy-body";
    skills.install(legacy_doc, None).await.unwrap();

    // Create a session bound to vertical "alpha" (no
    // explicit attachments, so the fallback path runs).
    let (tenant, ws) = store.ensure_workspace("ws", "/tmp/v679").await.unwrap();
    let sid = store
        .create_session(tenant, ws, "m", "p", Some("t"), Some("alpha"))
        .await
        .unwrap();

    let prompt = build_system_prompt(&store, sid, Some("alpha"))
        .await
        .unwrap()
        .unwrap();
    assert!(prompt.contains("alpha-body"), "must contain own bundled");
    assert!(
        !prompt.contains("beta-body"),
        "must NOT contain other vertical's skills"
    );
    assert!(
        !prompt.contains("legacy-body"),
        "must NOT contain workspace-global skills (default-deny)"
    );
}

#[tokio::test]
async fn build_system_prompt_for_unbound_session_keeps_legacy_global_lookup() {
    // Backward compat: sessions WITHOUT vertical_id still see
    // the full workspace skill set (legacy behaviour). Locks
    // that #679 doesn't break the unbound path.
    let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
    store.migrate().await.unwrap();
    let skills = argo_skill::SkillStore::new(Arc::clone(&store));
    let raw = "---\nname: any\nversion: \"0.1\"\ndescription: D\n\
               lock: none\nprovenance:\n  source: x\n  hash: sha256-x\n---\nany-body";
    skills
        .install_for_vertical(raw, None, Some("alpha"))
        .await
        .unwrap();

    let (tenant, ws) = store.ensure_workspace("ws", "/tmp/v679u").await.unwrap();
    // session with vertical_id = None
    let sid = store
        .create_session(tenant, ws, "m", "p", Some("t"), None)
        .await
        .unwrap();
    let prompt = build_system_prompt(&store, sid, None)
        .await
        .unwrap()
        .unwrap();
    assert!(
        prompt.contains("any-body"),
        "unbound session must see all workspace skills (legacy)"
    );
}
