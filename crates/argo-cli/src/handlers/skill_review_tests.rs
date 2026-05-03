//! Unit tests for `argo skill review` (#684).
#![cfg(test)]

use super::*;
use argo_skill::SkillStore;
use argo_store::SessionStore;
use argo_types::{TenantId, WorkspaceId};
use std::sync::Arc;

async fn fresh() -> SkillStore {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    SkillStore::new(Arc::new(store))
}

async fn seed_workspace(skills: &SkillStore) -> argo_types::SessionId {
    let store = skills.store_arc();
    let tenant = TenantId::new();
    let ws = WorkspaceId::new();
    sqlx::query(
        "INSERT INTO workspace (id, tenant_id, name, root_path, created_at)
         VALUES (?, ?, 't', '.', strftime('%s','now'))",
    )
    .bind(ws.0.as_bytes().as_slice())
    .bind(tenant.0.as_bytes().as_slice())
    .execute(store.pool())
    .await
    .unwrap();
    store
        .create_session(tenant, ws, "mock", "mock", Some("review-test"), Some("alpha"))
        .await
        .unwrap()
}

fn skill_doc(name: &str) -> String {
    format!(
        "---\nname: {name}\nversion: \"0.1\"\ndescription: D\n\
         lock: none\nprovenance:\n  source: t\n  hash: sha256-x\n---\n{name}-body"
    )
}

#[tokio::test]
async fn list_pending_emits_count_zero_when_queue_empty() {
    let skills = fresh().await;
    // Should not error on an empty queue.
    run(&skills, SkillReviewCmd::List { json: true }).await.unwrap();
    let pending = skills.list_pending_review().await.unwrap();
    assert!(pending.is_empty());
}

#[tokio::test]
async fn list_pending_returns_proposed_skills() {
    let skills = fresh().await;
    let sid = seed_workspace(&skills).await;
    skills
        .propose_for_review(&skill_doc("proposal_a"), sid, "alpha")
        .await
        .unwrap();
    skills
        .propose_for_review(&skill_doc("proposal_b"), sid, "alpha")
        .await
        .unwrap();
    let pending = skills.list_pending_review().await.unwrap();
    assert_eq!(pending.len(), 2);
}

#[tokio::test]
async fn approve_promotes_pending_to_active_and_makes_visible() {
    let skills = fresh().await;
    let sid = seed_workspace(&skills).await;
    let proposed = skills
        .propose_for_review(&skill_doc("approve_target"), sid, "alpha")
        .await
        .unwrap();
    let id_str = proposed.id.0.to_string();
    run(
        &skills,
        SkillReviewCmd::Approve {
            skill_id: id_str.clone(),
            json: true,
        },
    )
    .await
    .unwrap();
    // Pending queue is now empty.
    assert!(skills.list_pending_review().await.unwrap().is_empty());
    // Vertical-scoped lookup now sees the skill.
    let visible = skills.list_skills_for_vertical("alpha").await.unwrap();
    assert_eq!(visible.len(), 1);
    assert_eq!(visible[0].lifecycle_state, argo_skill::LifecycleState::Active);
}

#[tokio::test]
async fn reject_marks_skill_rejected_and_keeps_it_invisible() {
    let skills = fresh().await;
    let sid = seed_workspace(&skills).await;
    let proposed = skills
        .propose_for_review(&skill_doc("reject_target"), sid, "alpha")
        .await
        .unwrap();
    run(
        &skills,
        SkillReviewCmd::Reject {
            skill_id: proposed.id.0.to_string(),
            json: true,
        },
    )
    .await
    .unwrap();
    // Pending queue empty.
    assert!(skills.list_pending_review().await.unwrap().is_empty());
    // Lookup must NOT return the rejected skill.
    let visible = skills.list_skills_for_vertical("alpha").await.unwrap();
    assert!(
        visible.is_empty(),
        "rejected skill must NOT be visible to the agent"
    );
}

#[tokio::test]
async fn approve_invalid_uuid_fails_loudly() {
    let skills = fresh().await;
    let err = run(
        &skills,
        SkillReviewCmd::Approve {
            skill_id: "not-a-uuid".into(),
            json: true,
        },
    )
    .await
    .unwrap_err();
    assert!(format!("{err}").contains("invalid skill id"));
}

#[tokio::test]
async fn approve_unknown_skill_id_fails_loudly() {
    let skills = fresh().await;
    let ghost = uuid::Uuid::now_v7().to_string();
    let err = run(
        &skills,
        SkillReviewCmd::Approve {
            skill_id: ghost,
            json: true,
        },
    )
    .await
    .unwrap_err();
    assert!(format!("{err}").contains("no skill"));
}

#[tokio::test]
async fn approve_emits_skill_approved_audit_row() {
    let skills = fresh().await;
    let sid = seed_workspace(&skills).await;
    let proposed = skills
        .propose_for_review(&skill_doc("audited_approve"), sid, "alpha")
        .await
        .unwrap();
    run(
        &skills,
        SkillReviewCmd::Approve {
            skill_id: proposed.id.0.to_string(),
            json: true,
        },
    )
    .await
    .unwrap();
    let store = skills.store_arc();
    let rows = store.list_audit_for_session(sid).await.unwrap();
    assert!(
        rows.iter().any(|r| r.kind == "skill.approved"),
        "expected skill.approved audit row, got: {:?}",
        rows.iter().map(|r| r.kind.as_str()).collect::<Vec<_>>()
    );
}

#[tokio::test]
async fn reject_emits_skill_rejected_audit_row() {
    let skills = fresh().await;
    let sid = seed_workspace(&skills).await;
    let proposed = skills
        .propose_for_review(&skill_doc("audited_reject"), sid, "alpha")
        .await
        .unwrap();
    run(
        &skills,
        SkillReviewCmd::Reject {
            skill_id: proposed.id.0.to_string(),
            json: true,
        },
    )
    .await
    .unwrap();
    let store = skills.store_arc();
    let rows = store.list_audit_for_session(sid).await.unwrap();
    assert!(rows.iter().any(|r| r.kind == "skill.rejected"));
}
