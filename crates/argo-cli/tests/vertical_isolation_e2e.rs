//! End-to-end cross-vertical isolation integration tests
//! (#683 — refined-mission work order #5).
//!
//! Each test asserts ONE work-order-#5 contract by exercising
//! the library APIs (SessionStore, SkillStore, MemoryStore,
//! audit gap detector) directly against an in-memory store.
//! The per-PR unit tests (#678–#682) cover individual layers;
//! this file proves the contracts hold when the layers compose.
//!
//! Coverage of the seven work-order-#5 requirements:
//!
//! | Requirement                                                   | Test |
//! |---|---|
//! | PR-Checker learned skill not visible to Receptionist          | `pr_checker_learned_skill_not_visible_to_receptionist` |
//! | Receptionist learned skill not visible to PR-Checker          | `receptionist_learned_skill_not_visible_to_pr_checker` |
//! | Vertical memory does not leak                                 | `vertical_memory_does_not_leak_across_verticals` |
//! | Shared assets only load when explicitly declared              | `legacy_workspace_global_skill_is_not_silently_shared` |
//! | Learned skill proposal requires review before activation      | `pending_review_skill_is_invisible_until_approved` |
//! | Active learned skill is reused by same vertical               | `active_learned_skill_is_reused_by_same_vertical` |
//! | Denied action cannot become an active skill automatically     | `repeated_denial_records_signal_but_does_not_install_skill` |

use argo_audit::{detect_policy_denials, AuditKind, AuditSink, GapKind};
use argo_memory::{MemoryFile, MemoryStore};
use argo_skill::{LifecycleState, SkillStore};
use argo_store::SessionStore;
use argo_types::{SessionId, TenantId, WorkspaceId};
use std::sync::Arc;

async fn fresh_store() -> Arc<SessionStore> {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    Arc::new(store)
}

async fn fresh_workspace(store: &SessionStore) -> (TenantId, WorkspaceId) {
    let tenant = TenantId::new();
    let ws = WorkspaceId::new();
    sqlx::query(
        "INSERT INTO workspace (id, tenant_id, name, root_path, created_at)
         VALUES (?, ?, 'isolation-e2e', '.', strftime('%s','now'))",
    )
    .bind(ws.0.as_bytes().as_slice())
    .bind(tenant.0.as_bytes().as_slice())
    .execute(store.pool())
    .await
    .unwrap();
    (tenant, ws)
}

async fn create_session_for_vertical(
    store: &SessionStore,
    tenant: TenantId,
    ws: WorkspaceId,
    vertical_id: &str,
) -> SessionId {
    store
        .create_session(tenant, ws, "mock", "mock", Some(vertical_id), Some(vertical_id))
        .await
        .unwrap()
}

fn skill_doc(name: &str, body: &str) -> String {
    format!(
        "---\nname: {name}\nversion: \"0.1\"\ndescription: D\n\
         lock: none\nprovenance:\n  source: e2e-test\n  hash: sha256-x\n---\n{body}"
    )
}

// ============================================================
// Work order #5: PR-Checker learned skill not visible to Receptionist
// ============================================================
#[tokio::test]
async fn pr_checker_learned_skill_not_visible_to_receptionist() {
    let store = fresh_store().await;
    let skills = SkillStore::new(Arc::clone(&store));
    let (tenant, ws) = fresh_workspace(&store).await;

    let pr_session = create_session_for_vertical(&store, tenant, ws, "pr-checker").await;

    // Agent in pr-checker session proposes + human approves.
    let proposed = skills
        .propose_for_review(&skill_doc("pr_helper", "pr-only body"), pr_session, "pr-checker")
        .await
        .unwrap();
    skills
        .transition_lifecycle(proposed.id, LifecycleState::Active)
        .await
        .unwrap();

    // pr-checker sees it.
    let pr_view = skills.list_skills_for_vertical("pr-checker").await.unwrap();
    assert_eq!(pr_view.len(), 1);
    assert_eq!(pr_view[0].frontmatter.name, "pr_helper");

    // receptionist must NOT see it.
    let recept_view = skills.list_skills_for_vertical("receptionist").await.unwrap();
    assert!(
        recept_view.is_empty(),
        "PR-Checker learned skill leaked to Receptionist: {recept_view:?}"
    );
}

// ============================================================
// Work order #5: Receptionist learned skill not visible to PR-Checker
// ============================================================
#[tokio::test]
async fn receptionist_learned_skill_not_visible_to_pr_checker() {
    let store = fresh_store().await;
    let skills = SkillStore::new(Arc::clone(&store));
    let (tenant, ws) = fresh_workspace(&store).await;

    let recept_session = create_session_for_vertical(&store, tenant, ws, "receptionist").await;
    let proposed = skills
        .propose_for_review(
            &skill_doc("greeter", "receptionist-only body"),
            recept_session,
            "receptionist",
        )
        .await
        .unwrap();
    skills
        .transition_lifecycle(proposed.id, LifecycleState::Active)
        .await
        .unwrap();

    let recept_view = skills.list_skills_for_vertical("receptionist").await.unwrap();
    assert_eq!(recept_view.len(), 1);

    let pr_view = skills.list_skills_for_vertical("pr-checker").await.unwrap();
    assert!(
        pr_view.is_empty(),
        "Receptionist learned skill leaked to PR-Checker: {pr_view:?}"
    );
}

// ============================================================
// Work order #5: Vertical memory does not leak across verticals
// ============================================================
#[tokio::test]
async fn vertical_memory_does_not_leak_across_verticals() {
    let store = fresh_store().await;
    let memory = MemoryStore::new(Arc::clone(&store));
    let (_tenant, ws) = fresh_workspace(&store).await;

    memory
        .append_for_vertical(
            ws,
            MemoryFile::Memory,
            "pr-checker secret".into(),
            None,
            "pr-checker",
        )
        .await
        .unwrap();
    memory
        .append_for_vertical(
            ws,
            MemoryFile::Memory,
            "receptionist secret".into(),
            None,
            "receptionist",
        )
        .await
        .unwrap();

    let pr_view = memory
        .list_for_vertical(ws, MemoryFile::Memory, "pr-checker")
        .await
        .unwrap();
    assert_eq!(pr_view.len(), 1);
    assert_eq!(pr_view[0].body, "pr-checker secret");

    let recept_view = memory
        .list_for_vertical(ws, MemoryFile::Memory, "receptionist")
        .await
        .unwrap();
    assert_eq!(recept_view.len(), 1);
    assert_eq!(recept_view[0].body, "receptionist secret");

    // Neither sees the other's row.
    assert!(pr_view
        .iter()
        .all(|e| !e.body.contains("receptionist secret")));
    assert!(recept_view
        .iter()
        .all(|e| !e.body.contains("pr-checker secret")));
}

// ============================================================
// Work order #5: Shared assets only load when explicitly declared
//                — proven by verifying legacy NULL-vertical
//                rows are NOT silently shared across verticals.
// ============================================================
#[tokio::test]
async fn legacy_workspace_global_skill_is_not_silently_shared() {
    let store = fresh_store().await;
    let skills = SkillStore::new(Arc::clone(&store));

    // Legacy install (no vertical context).
    skills
        .install(&skill_doc("legacy", "legacy body"), None)
        .await
        .unwrap();

    // pr-checker queries → must NOT see the legacy row.
    let pr_view = skills.list_skills_for_vertical("pr-checker").await.unwrap();
    assert!(
        pr_view.iter().all(|s| s.frontmatter.name != "legacy"),
        "legacy workspace-global skill leaked to pr-checker"
    );

    // receptionist queries → also must NOT see it.
    let recept_view = skills.list_skills_for_vertical("receptionist").await.unwrap();
    assert!(recept_view.iter().all(|s| s.frontmatter.name != "legacy"));
}

// ============================================================
// Work order #5: Learned skill proposal requires review before activation
// ============================================================
#[tokio::test]
async fn pending_review_skill_is_invisible_until_approved() {
    let store = fresh_store().await;
    let skills = SkillStore::new(Arc::clone(&store));
    let (tenant, ws) = fresh_workspace(&store).await;
    let sid = create_session_for_vertical(&store, tenant, ws, "pr-checker").await;

    let proposed = skills
        .propose_for_review(&skill_doc("draft_skill", "x"), sid, "pr-checker")
        .await
        .unwrap();
    assert_eq!(proposed.lifecycle_state, LifecycleState::PendingReview);

    // Before approval: invisible.
    let before = skills.list_skills_for_vertical("pr-checker").await.unwrap();
    assert!(
        before.is_empty(),
        "pending_review skill must not influence agent behaviour: {before:?}"
    );

    // Approve.
    skills
        .transition_lifecycle(proposed.id, LifecycleState::Active)
        .await
        .unwrap();

    // After approval: visible to pr-checker AND only pr-checker.
    let after = skills.list_skills_for_vertical("pr-checker").await.unwrap();
    assert_eq!(after.len(), 1);
    assert_eq!(after[0].lifecycle_state, LifecycleState::Active);
}

// ============================================================
// Work order #5: Active learned skill is reused by same vertical
//                — across multiple sessions of the same vertical.
// ============================================================
#[tokio::test]
async fn active_learned_skill_is_reused_by_same_vertical() {
    let store = fresh_store().await;
    let skills = SkillStore::new(Arc::clone(&store));
    let (tenant, ws) = fresh_workspace(&store).await;

    // First pr-checker session learns + approves.
    let sess1 = create_session_for_vertical(&store, tenant, ws, "pr-checker").await;
    let proposed = skills
        .propose_for_review(&skill_doc("reuse_me", "reuse body"), sess1, "pr-checker")
        .await
        .unwrap();
    skills
        .transition_lifecycle(proposed.id, LifecycleState::Active)
        .await
        .unwrap();

    // Second pr-checker session — same vertical, different
    // session id. Must see the learned skill.
    let _sess2 = create_session_for_vertical(&store, tenant, ws, "pr-checker").await;
    let view = skills.list_skills_for_vertical("pr-checker").await.unwrap();
    let names: Vec<&str> = view.iter().map(|s| s.frontmatter.name.as_str()).collect();
    assert!(
        names.contains(&"reuse_me"),
        "active learned skill must be reusable by same vertical; got {names:?}"
    );
}

// ============================================================
// Work order #5: Denied/policy-blocked action cannot become
//                an active skill automatically.
// ============================================================
#[tokio::test]
async fn repeated_denial_records_signal_but_does_not_install_skill() {
    let store = fresh_store().await;
    let skills = SkillStore::new(Arc::clone(&store));
    let (tenant, ws) = fresh_workspace(&store).await;
    let sid = create_session_for_vertical(&store, tenant, ws, "pr-checker").await;
    let sink = AuditSink::new(Arc::clone(&store));

    // Simulate the agent repeatedly trying to call a denied tool.
    for _ in 0..3 {
        sink.write(
            Some(sid),
            AuditKind::RbacDeny,
            serde_json::json!({"tool": "delegate_task", "denied_by": "vertical_manifest"}),
        )
        .await
        .unwrap();
    }

    // Detector flags it.
    let gaps = detect_policy_denials(&store, sid, 0, 3).await.unwrap();
    assert_eq!(gaps.len(), 1);
    assert_eq!(gaps[0].kind, GapKind::PolicyDenial);
    assert_eq!(gaps[0].anchor, "delegate_task");

    // CRITICAL: no skill was auto-installed by the detector.
    // The agent's view of available skills is still empty.
    let visible = skills.list_skills_for_vertical("pr-checker").await.unwrap();
    assert!(
        visible.is_empty(),
        "denied action must NOT auto-become an active skill; visible={visible:?}"
    );

    // Even an explicit propose_for_review for the denied tool
    // would land in PendingReview, not Active — proven by #681
    // tests. Lock that the auto-detection path doesn't bypass
    // it: the agent / Forge could call propose_for_review here
    // (a future PR may wire that), but the result MUST still
    // be PendingReview, requiring human approval.
    let proposed = skills
        .propose_for_review(
            &skill_doc("delegate_workaround", "would dispatch X via Y"),
            sid,
            "pr-checker",
        )
        .await
        .unwrap();
    assert_eq!(proposed.lifecycle_state, LifecycleState::PendingReview);
    let still_invisible = skills.list_skills_for_vertical("pr-checker").await.unwrap();
    assert!(
        still_invisible.is_empty(),
        "auto-proposed skill MUST stay invisible until human approval"
    );
}
