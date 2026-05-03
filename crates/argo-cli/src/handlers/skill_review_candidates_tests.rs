//! Unit tests for memory-backed skill-candidate plumbing (#697).
//!
//! Drives the helpers in `skill_review_candidates.rs` directly
//! against an in-memory store + `MemoryStore` so we can verify
//! the contract without spinning up the CLI:
//!
//! - happy-path promote materialises a `pending_review` skill
//!   row + emits the audit event
//! - missing `vertical_id` on the memory entry refuses promote
//! - parsing failure (wrong schema_key) refuses promote
//! - missing `learned_in` refuses promote
//! - the SkillStore-side row carries the right vertical_id +
//!   lifecycle_state after promote (NOT auto-active)
//!
//! End-to-end CLI / cross-vertical isolation tests live in
//! `crates/argo-cli/tests/skill_review_candidates.rs` so they
//! drive the binary surface.
#![cfg(test)]

use super::*;
use argo_memory::{MemoryFile, MemoryStore, SkillCandidate};
use argo_skill::{LifecycleState, SkillStore};
use argo_store::SessionStore;
use argo_types::{TenantId, WorkspaceId};
use std::sync::Arc;

async fn fresh_setup() -> (SkillStore, MemoryStore, argo_types::SessionId, WorkspaceId) {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    let store = Arc::new(store);
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
    let sid = store
        .create_session(tenant, ws, "mock", "mock", Some("promote-test"), Some("gtm-engineer"))
        .await
        .unwrap();
    let skills = SkillStore::new(Arc::clone(&store));
    let memory = MemoryStore::new(Arc::clone(&store));
    (skills, memory, sid, ws)
}

fn candidate_body(skill_name: &str) -> String {
    format!(
        r#"{{"schema_key":"learned_skill_candidates","skill_name":"{skill_name}","problem":"P","reusable_pattern":"R","evidence":"E","expected_reuse":"X","risk":"K"}}"#
    )
}

#[tokio::test]
async fn promote_materialises_pending_review_skill_with_vertical_id() {
    let (skills, memory, sid, ws) = fresh_setup().await;
    // Seed a candidate body into MEMORY.md scoped to gtm-engineer.
    let entry = memory
        .append_for_vertical(
            ws,
            MemoryFile::Memory,
            candidate_body("technical_founder_outreach"),
            Some(sid),
            "gtm-engineer",
        )
        .await
        .unwrap();
    let new_skill =
        promote_candidate(&skills, &entry.id.to_string()).await.unwrap();
    assert_eq!(new_skill.frontmatter.name, "technical_founder_outreach");
    assert_eq!(new_skill.vertical_id.as_deref(), Some("gtm-engineer"));
    assert_eq!(
        new_skill.lifecycle_state,
        LifecycleState::PendingReview,
        "promote MUST NOT auto-activate; new skill stays pending_review"
    );
}

#[tokio::test]
async fn promote_attributes_audit_row_to_originating_session() {
    let (skills, memory, sid, ws) = fresh_setup().await;
    let entry = memory
        .append_for_vertical(
            ws,
            MemoryFile::Memory,
            candidate_body("audit_target"),
            Some(sid),
            "gtm-engineer",
        )
        .await
        .unwrap();
    promote_candidate(&skills, &entry.id.to_string()).await.unwrap();
    // The promote helper writes a SkillCandidatePromoted audit
    // row attributed to `sid`. Verify it landed in the per-
    // session chain.
    let rows = argo_audit::rows_for_session(&skills.store_arc(), sid)
        .await
        .unwrap();
    let promoted = rows
        .iter()
        .find(|r| r.kind == "skill.candidate.promoted")
        .expect("expected a skill.candidate.promoted row in chain");
    assert_eq!(promoted.payload["vertical_id"], "gtm-engineer");
    assert!(promoted.payload["skill_id"].is_string());
    assert_eq!(promoted.payload["memory_entry_id"], entry.id.to_string());
}

#[tokio::test]
async fn promote_refuses_when_memory_entry_has_no_vertical_id() {
    let (skills, memory, sid, ws) = fresh_setup().await;
    // Use the workspace-global append (no vertical_id).
    let entry = memory
        .append(ws, MemoryFile::Memory, candidate_body("orphan"), Some(sid))
        .await
        .unwrap();
    let err = promote_candidate(&skills, &entry.id.to_string()).await.unwrap_err();
    let msg = format!("{err}");
    assert!(
        msg.contains("no vertical_id"),
        "promote on workspace-global memory must refuse with vertical_id message; got: {msg}"
    );
}

#[tokio::test]
async fn promote_refuses_when_body_is_not_a_candidate_row() {
    let (skills, memory, sid, ws) = fresh_setup().await;
    // Body is a JSON object but the schema_key is wrong — it's
    // an ICP row, not a candidate. Promote MUST refuse rather
    // than silently misinterpret the body.
    let entry = memory
        .append_for_vertical(
            ws,
            MemoryFile::Memory,
            r#"{"schema_key":"icp","industry":"saas"}"#.to_string(),
            Some(sid),
            "gtm-engineer",
        )
        .await
        .unwrap();
    let err = promote_candidate(&skills, &entry.id.to_string()).await.unwrap_err();
    assert!(
        format!("{err}").contains("learned_skill_candidates"),
        "wrong schema_key should be flagged: {err}"
    );
}

#[tokio::test]
async fn promote_refuses_when_memory_entry_id_is_invalid_uuid() {
    let (skills, _memory, _sid, _ws) = fresh_setup().await;
    let err = promote_candidate(&skills, "not-a-uuid").await.unwrap_err();
    assert!(
        format!("{err}").contains("invalid memory id"),
        "non-UUID input should be rejected with a clear message: {err}"
    );
}

#[tokio::test]
async fn collect_memory_candidates_returns_empty_on_workspace_with_no_rows() {
    let (skills, _memory, _sid, _ws) = fresh_setup().await;
    // No memory rows have been written; aggregator should
    // return an empty vec without erroring even with no
    // verticals dir.
    let rows = collect_memory_candidates(&skills).await.unwrap();
    assert!(rows.is_empty());
}

#[tokio::test]
async fn build_skill_md_round_trips_through_the_skill_parser() {
    // The synthesised SKILL.md must be parseable by the same
    // parser propose_for_review uses — otherwise promote would
    // fail on every candidate.
    let candidate = SkillCandidate {
        skill_name: "round_trip".into(),
        problem: Some("p".into()),
        reusable_pattern: Some("rp".into()),
        evidence: Some("e".into()),
        expected_reuse: Some("xr".into()),
        risk: Some("k".into()),
    };
    let raw = build_skill_md(&candidate, "gtm-engineer", uuid::Uuid::nil());
    let (front, body) = argo_skill::parse_skill_md(&raw).unwrap();
    assert_eq!(front.name, "round_trip");
    assert_eq!(front.version, "0.1.0");
    assert!(body.contains("round_trip"));
    assert!(body.contains("pending_review"));
}
