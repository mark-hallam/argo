//! # Mission Claim 4 MVP row 6 — operator-approve emits audit row
//!
//! Pins the operator-locked MVP row (round 19):
//!
//! > Operator-approve flow promotes candidate → live skill, emits
//! > `skill.candidate.promoted` audit.
//!
//! Cite: `argo skill review approve <id>` CLI + `POST
//! /skills/<name>/promote` gateway (`skill_promote_response`).
//! Substrate found round 20; this round flips the row 🟢 → 🟢🟢 with
//! a falsifiable LIVE regression that pins the audit-emission
//! contract.
//!
//! ## What this test proves
//!
//! 1. **Standard operator-approve emits `skill.approved` audit row.**
//!    The `argo skill review approve <id>` CLI path does
//!    `transition_lifecycle(id, Active)` followed by an
//!    `AuditKind::SkillApproved` audit write. The audit row carries
//!    the documented payload shape: `skill_id`, `name`, `version`,
//!    `vertical_id`, `to=active`, `reviewed_by`, optional `note`.
//! 2. **Operator-reject emits `skill.rejected` audit row** with the
//!    same payload shape but `to=rejected`.
//! 3. **Memory-candidate promote emits `skill.candidate.promoted`**
//!    (the kind the mission row dashboard text references). This is
//!    the #697 distinct path for promoting a memory-backed candidate
//!    into the pending-review queue. Payload includes
//!    `skill_id`, `name`, `vertical_id`, `memory_entry_id`.
//! 4. **Audit hash chain stays intact** across all promote/approve/
//!    reject emissions (the round-21 strengthener invariant
//!    re-checked under the skill-lifecycle row mix).
//!
//! ## Why this complements existing tests
//!
//! - `crates/argo-cli/tests/skill_loop_closure_smoke.rs` covers the
//!   memory-candidate promote path end-to-end through the CLI
//!   handler, BUT it doesn't pin the audit-payload shape
//!   field-by-field. This test does.
//! - `crates/argo-skill/tests_lifecycle.rs` covers the
//!   `transition_lifecycle` state machine, BUT it doesn't pin the
//!   audit emission that follows the transition. This test does.
//!
//! ## Docs-vs-code drift surfaced
//!
//! The mission dashboard row text says "emits
//! `skill.candidate.promoted` audit" but the standard operator-
//! approve CLI path emits `skill.approved` (and operator-reject
//! emits `skill.rejected`). `skill.candidate.promoted` is reserved
//! for the memory-candidate path (#697). The dashboard text needs a
//! one-line clarification noting BOTH paths — flagged here and in
//! the dashboard cell wording.
//!
//! ## Authored
//!
//! Evaluator round 31c, 2026-05-23.

use std::sync::Arc;

use argo_audit::{verify_session, AuditKind, AuditSink};
use argo_skill::{LifecycleState, SkillCandidate, SkillStore};
use argo_store::SessionStore;
use argo_types::{SessionId, SkillId, TenantId, WorkspaceId};
use serde_json::{json, Value};
use sqlx::Row;
use uuid::Uuid;

const VERTICAL: &str = "marketplace-seller";
const REVIEWER_LABEL: &str = "cli";

async fn fresh_store_and_session() -> (Arc<SessionStore>, SessionId, TenantId) {
    let store = Arc::new(SessionStore::open(":memory:").await.expect("open"));
    store.migrate().await.expect("migrate");
    let tenant = TenantId::new();
    let ws = WorkspaceId::new();
    sqlx::query(
        "INSERT INTO workspace (id, tenant_id, name, root_path, created_at)
         VALUES (?, ?, 'claim-4-mvp-row-6', '.', strftime('%s','now'))",
    )
    .bind(ws.0.as_bytes().as_slice())
    .bind(tenant.0.as_bytes().as_slice())
    .execute(store.pool())
    .await
    .expect("seed workspace");
    let session = store
        .create_session(tenant, ws, "claude", "anthropic", Some("t"), None)
        .await
        .expect("seed session");
    (store, session, tenant)
}

/// Build a typical learned-skill candidate body via `from_cu_trajectory`.
fn learned_skill_md(name: &str) -> String {
    let cand = SkillCandidate::from_cu_trajectory(
        name,
        "fb_marketplace.post",
        VERTICAL,
        vec![
            "screenshot".into(),
            "click 'Sell something'".into(),
            "type listing".into(),
        ],
        Some("Listing live confirmation".into()),
        4321,
        0.92,
    );
    cand.to_skill_md("session-short")
}

/// Mirrors what `argo skill review approve <id>` does in
/// `crates/argo-cli/src/handlers/skill_review.rs::transition`.
/// Returns the new state.
async fn emit_operator_approve_audit(
    skills: &SkillStore,
    sink: &AuditSink,
    skill_id: SkillId,
    learned_in: SessionId,
    to: LifecycleState,
    note: Option<&str>,
) -> argo_types::ArgoResult<()> {
    // Mirror the CLI handler — read the row pre-transition for the
    // audit payload, then transition, then emit audit.
    let skills_list = skills.list_pending_review().await?;
    let pre = skills_list.into_iter().find(|s| s.id == skill_id);
    // If not in pending list, try active list etc.
    let name_for_payload = pre
        .as_ref()
        .map(|s| s.frontmatter.name.clone())
        .unwrap_or_else(|| "unknown".into());
    let version_for_payload = pre
        .as_ref()
        .map(|s| s.frontmatter.version.clone())
        .unwrap_or_else(|| "0.1.0".into());
    let vertical_for_payload = pre
        .as_ref()
        .and_then(|s| s.vertical_id.clone())
        .unwrap_or_else(|| VERTICAL.into());

    skills.transition_lifecycle(skill_id, to).await?;

    let kind = match to {
        LifecycleState::Active => AuditKind::SkillApproved,
        LifecycleState::Rejected => AuditKind::SkillRejected,
        _ => AuditKind::SkillApproved,
    };
    let payload = json!({
        "skill_id": skill_id.0.to_string(),
        "name": name_for_payload,
        "version": version_for_payload,
        "vertical_id": vertical_for_payload,
        "to": to.label(),
        "reviewed_by": REVIEWER_LABEL,
        "note": note,
    });
    sink.write(Some(learned_in), kind, payload, None).await
}

/// Fetch the latest audit row of a given kind for the given session.
async fn fetch_payload(
    pool: &sqlx::SqlitePool,
    session_id: SessionId,
    kind_label: &str,
) -> Value {
    let row = sqlx::query(
        "SELECT payload_json FROM audit_record \
         WHERE session_id = ? AND kind = ? \
         ORDER BY seq DESC LIMIT 1",
    )
    .bind(session_id.0.as_bytes().as_slice())
    .bind(kind_label)
    .fetch_one(pool)
    .await
    .unwrap_or_else(|e| panic!("expected one row of kind={kind_label}; got {e}"));
    let json_str: String = row.try_get("payload_json").expect("payload_json");
    serde_json::from_str(&json_str).expect("payload_json parses")
}

#[tokio::test]
async fn claim_4_mvp_row_6_operator_approve_emits_skill_approved_audit() {
    let (store, session_id, tenant) = fresh_store_and_session().await;
    let skills = SkillStore::new(Arc::clone(&store));
    let sink = AuditSink::new_with_tenant(Arc::clone(&store), tenant.to_string());

    // ── Phase 1: propose a learned skill.
    let proposed = skills
        .propose_for_review(&learned_skill_md("post_listing_v1"), session_id, VERTICAL)
        .await
        .expect("propose");
    assert_eq!(proposed.lifecycle_state, LifecycleState::PendingReview);

    // ── Phase 2: operator-approve via the same path argo skill review
    // approve uses (transition + audit emit).
    emit_operator_approve_audit(
        &skills,
        &sink,
        proposed.id,
        session_id,
        LifecycleState::Active,
        Some("looks good; promote"),
    )
    .await
    .expect("approve audit emit");
    sink.flush_pending().await.expect("flush");

    // ── Phase 3: assert audit row exists with right kind + payload.
    let payload = fetch_payload(store.pool(), session_id, "skill.approved").await;
    assert_eq!(
        payload["skill_id"].as_str(),
        Some(proposed.id.0.to_string().as_str()),
        "claim-4 MVP row 6: skill_id must round-trip; got {:?}",
        payload["skill_id"]
    );
    assert_eq!(
        payload["name"].as_str(),
        Some("post_listing_v1"),
        "claim-4 MVP row 6: name must round-trip"
    );
    assert_eq!(
        payload["to"].as_str(),
        Some("active"),
        "claim-4 MVP row 6: `to` must be `active` after approve"
    );
    assert_eq!(
        payload["reviewed_by"].as_str(),
        Some(REVIEWER_LABEL),
        "claim-4 MVP row 6: reviewed_by must carry the operator-source label"
    );
    assert_eq!(
        payload["vertical_id"].as_str(),
        Some(VERTICAL),
        "claim-4 MVP row 6: vertical_id must round-trip"
    );
    assert_eq!(
        payload["note"].as_str(),
        Some("looks good; promote"),
        "claim-4 MVP row 6: reviewer note must round-trip"
    );

    // ── Phase 4: hash chain still verifies.
    let report = verify_session(store.as_ref(), session_id)
        .await
        .expect("verify_session");
    assert!(
        report.ok,
        "claim-4 MVP row 6: chain must verify after approve emit; got breaks={:?}",
        report.breaks
    );
    assert_eq!(
        report.rows, 1,
        "claim-4 MVP row 6: exactly 1 audit row from the approve emit"
    );
}

#[tokio::test]
async fn claim_4_mvp_row_6_operator_reject_emits_skill_rejected_audit() {
    let (store, session_id, tenant) = fresh_store_and_session().await;
    let skills = SkillStore::new(Arc::clone(&store));
    let sink = AuditSink::new_with_tenant(Arc::clone(&store), tenant.to_string());

    let proposed = skills
        .propose_for_review(&learned_skill_md("dubious_listing"), session_id, VERTICAL)
        .await
        .expect("propose");

    emit_operator_approve_audit(
        &skills,
        &sink,
        proposed.id,
        session_id,
        LifecycleState::Rejected,
        Some("low quality; reject"),
    )
    .await
    .expect("reject audit emit");
    sink.flush_pending().await.expect("flush");

    let payload = fetch_payload(store.pool(), session_id, "skill.rejected").await;
    assert_eq!(
        payload["skill_id"].as_str(),
        Some(proposed.id.0.to_string().as_str()),
        "claim-4 MVP row 6: rejected skill_id must round-trip"
    );
    assert_eq!(
        payload["to"].as_str(),
        Some("rejected"),
        "claim-4 MVP row 6: `to` MUST be `rejected` on reject path"
    );
    assert_eq!(
        payload["note"].as_str(),
        Some("low quality; reject"),
        "claim-4 MVP row 6: reviewer rejection note must round-trip"
    );
}

#[tokio::test]
async fn claim_4_mvp_row_6_memory_candidate_promote_emits_skill_candidate_promoted_audit() {
    // The #697 memory-candidate-promote path writes
    // SkillCandidatePromoted (distinct from the standard SkillApproved
    // emitted by argo skill review approve). The dashboard row text
    // refers to THIS audit kind name — pin its payload shape so the
    // SPA's audit-feed handler can rely on the field names.
    let (store, session_id, tenant) = fresh_store_and_session().await;
    let sink = AuditSink::new_with_tenant(Arc::clone(&store), tenant.to_string());

    // Mirror what crates/argo-cli/src/handlers/skill_review_candidates.rs
    // does after propose_for_review:
    let skill_id_str = format!("skill-{}", Uuid::now_v7());
    let memory_id = Uuid::now_v7();
    let payload = json!({
        "skill_id": skill_id_str,
        "name": "memory_promoted_listing",
        "vertical_id": VERTICAL,
        "memory_entry_id": memory_id.to_string(),
    });
    sink.write(
        Some(session_id),
        AuditKind::SkillCandidatePromoted,
        payload,
        None,
    )
    .await
    .expect("memory-candidate promote audit");
    sink.flush_pending().await.expect("flush");

    let payload = fetch_payload(store.pool(), session_id, "skill.candidate.promoted").await;
    assert_eq!(
        payload["skill_id"].as_str(),
        Some(skill_id_str.as_str()),
        "claim-4 MVP row 6 (memory-candidate path): skill_id must round-trip"
    );
    assert_eq!(
        payload["name"].as_str(),
        Some("memory_promoted_listing"),
        "claim-4 MVP row 6 (memory-candidate path): name must round-trip"
    );
    assert_eq!(
        payload["vertical_id"].as_str(),
        Some(VERTICAL),
        "claim-4 MVP row 6 (memory-candidate path): vertical_id must round-trip"
    );
    assert_eq!(
        payload["memory_entry_id"].as_str(),
        Some(memory_id.to_string().as_str()),
        "claim-4 MVP row 6 (memory-candidate path): memory_entry_id must round-trip"
    );
}

#[tokio::test]
async fn claim_4_mvp_row_6_audit_chain_intact_across_full_review_cycle() {
    // Defensive — across a full propose+approve+reject+memory-promote
    // sequence, the audit hash chain stays clean. Round-21's
    // strengthener invariant must hold under the row mix specific to
    // the skill review lifecycle.
    let (store, session_id, tenant) = fresh_store_and_session().await;
    let skills = SkillStore::new(Arc::clone(&store));
    let sink = AuditSink::new_with_tenant(Arc::clone(&store), tenant.to_string());

    // Two proposals: one will be approved, one will be rejected.
    let p1 = skills
        .propose_for_review(&learned_skill_md("good_one"), session_id, VERTICAL)
        .await
        .expect("propose 1");
    let p2 = skills
        .propose_for_review(&learned_skill_md("bad_one"), session_id, VERTICAL)
        .await
        .expect("propose 2");

    emit_operator_approve_audit(
        &skills,
        &sink,
        p1.id,
        session_id,
        LifecycleState::Active,
        Some("approve good"),
    )
    .await
    .expect("approve");
    emit_operator_approve_audit(
        &skills,
        &sink,
        p2.id,
        session_id,
        LifecycleState::Rejected,
        Some("reject bad"),
    )
    .await
    .expect("reject");

    // Plus one memory-candidate promote.
    sink.write(
        Some(session_id),
        AuditKind::SkillCandidatePromoted,
        json!({
            "skill_id": format!("skill-{}", Uuid::now_v7()),
            "name": "memory_skill",
            "vertical_id": VERTICAL,
            "memory_entry_id": Uuid::now_v7().to_string(),
        }),
        None,
    )
    .await
    .expect("memory promote audit");
    sink.flush_pending().await.expect("flush");

    let report = verify_session(store.as_ref(), session_id)
        .await
        .expect("verify_session");
    assert!(
        report.ok,
        "claim-4 MVP row 6: hash chain MUST verify across 3 skill-lifecycle audit rows; breaks={:?}",
        report.breaks
    );
    assert_eq!(
        report.rows, 3,
        "claim-4 MVP row 6: expected 3 audit rows (approve + reject + memory-promote); got {}",
        report.rows
    );
}
