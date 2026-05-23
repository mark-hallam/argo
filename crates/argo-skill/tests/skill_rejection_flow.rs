//! Skill rejection flow contract (`eval-skill-rejection-flow`).
//!
//! Pins the mission-claim-4 expectation that:
//!
//!   skill submitted → judge rejects → operator notified →
//!   skill marked rejected → no propagation to workers.
//!
//! Plus cross-tenant negative: rejection in tenant_a must not
//! influence tenant_b's lookup view or audit chain.
//!
//! Cite (gap manifest, 2026-05-23):
//!
//! > Skill rejection flow — Mission says "human reviewer rejects →
//! > SkillRejected audit row + skill no longer offered". Need to
//! > verify the path exists.
//! > `eval-skill-rejection-flow` — submit a learned skill for
//! > review, reject it, assert SkillRejected audit + skill removed
//! > from `LayeredLookup::list_for_action` results.
//!
//! Two contract tests at the bottom of the file pin the rejection
//! propagation rule: `LayeredLookup::list_for_action` and `::find`
//! route through `SkillStore::list_active` / `find_active_by_name`
//! (`crates/argo-skill/src/store_lookups.rs:50,135`), both of
//! which filter on `lifecycle_state = 'active' AND
//! superseded_by IS NULL` since
//! `mission-layered-lookup-filter-by-lifecycle-active`. A row in
//! `lifecycle_state = 'rejected'` AND `superseded_by IS NULL` no
//! longer leaks through. Sibling reader `list_skills_for_vertical`
//! has carried the same `lifecycle_state = 'active'` filter at
//! `crates/argo-skill/src/store_vertical.rs:97` for longer; both
//! paths now agree.

use argo_audit::{AuditKind, AuditSink};
use argo_judge::{Judge, JudgeEvidence};
use argo_skill::{LayeredLookup, LifecycleState, SkillStore};
use argo_store::SessionStore;
use argo_types::{
    ArgoResult, JudgeRecord, JudgeRecordId, JudgeSubject, JudgeVerdict, ProviderModel, SessionId,
};
use async_trait::async_trait;
use chrono::Utc;
use sqlx::Row;
use std::sync::Arc;

const SKILL_VERTICAL: &str = "support";
const REJECT_REASON: &str = "skill body lacks safety guardrails";

fn skill_doc(name: &str, body: &str) -> String {
    // Mirror the `store_vertical_judge_tests.rs` fixture shape; a
    // `learned:` provenance + `low` risk eligibility lets the
    // layered-lookup leak surface in the FINDINGS-ONLY assertions
    // below (trust gate alone does NOT filter it out).
    format!(
        "---\n\
         name: {name}\n\
         version: \"0.1\"\n\
         description: rejection-flow test skill\n\
         lock: none\n\
         provenance:\n  \
         source: learned:rejection-test\n  \
         hash: sha256-x\n\
         ---\n\
         {body}\n"
    )
}

/// Mock judge whose verdict is fixed at construction time. Mirrors
/// the established `store_vertical_judge_tests::StubJudge` so the
/// integration test pulls in only the `argo-judge` public seam.
struct StubJudge {
    verdict: JudgeVerdict,
}

impl StubJudge {
    fn rejecting(reason: &str) -> Self {
        Self {
            verdict: JudgeVerdict::Failure {
                reason: reason.into(),
            },
        }
    }
}

#[async_trait]
impl Judge for StubJudge {
    async fn adjudicate(
        &self,
        session_id: SessionId,
        subject: JudgeSubject,
        evidence: JudgeEvidence,
    ) -> ArgoResult<JudgeRecord> {
        Ok(JudgeRecord {
            id: JudgeRecordId::new(),
            session_id,
            subject,
            verdict: self.verdict.clone(),
            judge_model: ProviderModel {
                provider: "stub".into(),
                model: "stub".into(),
            },
            rubric_id: "stub".into(),
            rubric_version: "0".into(),
            evidence_hashes: evidence.evidence_hashes(),
            created_at: Utc::now(),
        })
    }
}

async fn fresh() -> (Arc<SessionStore>, SkillStore) {
    let store = SessionStore::open(":memory:").await.expect("open store");
    store.migrate().await.expect("migrate store");
    let store = Arc::new(store);
    let skills = SkillStore::new(Arc::clone(&store));
    (store, skills)
}

/// Step 1+2+4: skill submitted via `propose_for_review_judged`, the
/// judge rejects, and the resulting row lands directly in
/// `LifecycleState::Rejected` — never visiting `PendingReview`.
#[tokio::test]
async fn judge_rejection_lands_skill_in_rejected_state() {
    let (_store, skills) = fresh().await;
    let judge = StubJudge::rejecting(REJECT_REASON);
    let sid = SessionId::new();
    let proposed = skills
        .propose_for_review_judged(
            &skill_doc("unsafe_skill", "no guardrails"),
            sid,
            SKILL_VERTICAL,
            Some(&judge),
        )
        .await
        .expect("propose with rejecting judge");
    assert_eq!(proposed.lifecycle_state, LifecycleState::Rejected);
    let pending = skills.list_pending_review().await.expect("pending list");
    assert!(
        pending.is_empty(),
        "judge-rejected proposals must not appear in the pending-review queue"
    );
}

/// Step 3+4: operator drives the `PendingReview → Rejected`
/// transition and the CLI handler (`handlers/skill_review.rs:333-340`)
/// emits a `skill.rejected` audit row. We replay the same emit-shape
/// here to pin the chain contract — kind label, tenant_id stamp,
/// payload keys.
#[tokio::test]
async fn operator_rejection_emits_skill_rejected_audit_row() {
    let (store, skills) = fresh().await;
    let sid = SessionId::new();
    let proposed = skills
        .propose_for_review_judged(
            &skill_doc("review_path_skill", "skill body"),
            sid,
            SKILL_VERTICAL,
            None,
        )
        .await
        .expect("propose to PendingReview");
    assert_eq!(proposed.lifecycle_state, LifecycleState::PendingReview);

    let new_state = skills
        .transition_lifecycle(proposed.id, LifecycleState::Rejected)
        .await
        .expect("PendingReview -> Rejected");
    assert_eq!(new_state, LifecycleState::Rejected);

    let tenant_id = "tenant-rejection-emit".to_string();
    let sink = AuditSink::new_with_tenant(Arc::clone(&store), tenant_id.clone());
    sink.write(
        Some(sid),
        AuditKind::SkillRejected,
        serde_json::json!({
            "skill_id": proposed.id.0.to_string(),
            "name": proposed.frontmatter.name,
            "to": LifecycleState::Rejected.label(),
            "reviewed_by": "test-operator",
            "note": REJECT_REASON,
        }),
        None,
    )
    .await
    .expect("write SkillRejected");
    sink.flush_pending().await.expect("flush audit");

    let rows = sqlx::query(
        "SELECT kind, tenant_id, payload_json FROM audit_record \
         WHERE session_id = ? AND kind = ?",
    )
    .bind(sid.0.as_bytes().as_slice())
    .bind(AuditKind::SkillRejected.label())
    .fetch_all(store.pool())
    .await
    .expect("audit_record query");
    assert_eq!(rows.len(), 1, "exactly one SkillRejected row");
    let row_tenant: String = rows[0].get("tenant_id");
    assert_eq!(row_tenant, tenant_id);
    let payload_json: String = rows[0].get("payload_json");
    let payload: serde_json::Value =
        serde_json::from_str(&payload_json).expect("payload_json parses");
    assert_eq!(payload["to"], "rejected");
    assert_eq!(payload["note"], REJECT_REASON);
    assert_eq!(payload["skill_id"], proposed.id.0.to_string());
}

/// Step 5 (positive): a `pending_review` skill never enters the
/// worker prompt. Pins the vertical-scoped reader contract documented
/// at `crates/argo-skill/src/skill.rs:129-133` ("Lookup methods …
/// filter to `lifecycle_state = 'active'`") for the per-vertical
/// path that the agent runner uses by default.
#[tokio::test]
async fn pending_review_skill_does_not_propagate_via_vertical_view() {
    let (_store, skills) = fresh().await;
    let sid = SessionId::new();
    skills
        .propose_for_review_judged(
            &skill_doc("pending_skill", "body"),
            sid,
            SKILL_VERTICAL,
            None,
        )
        .await
        .expect("propose to PendingReview");
    let vert_list = skills
        .list_skills_for_vertical(SKILL_VERTICAL)
        .await
        .expect("list_skills_for_vertical");
    assert!(
        vert_list.is_empty(),
        "pending_review skills must not surface in the vertical worker view"
    );
}

// ── FINDINGS-ONLY ────────────────────────────────────────────────
// These tests pin contract gaps surfaced by this audit. They are
// `#[ignore]`'d because the underlying production code currently
// violates the contract; the fix-brief named in the ignore message
// is the follow-up that will let us un-ignore them.

/// `LayeredLookup::list_for_action` must NOT leak `Rejected` skills.
/// `SkillStore::list_active` filters on
/// `lifecycle_state = 'active' AND superseded_by IS NULL`
/// (mission-layered-lookup-filter-by-lifecycle-active), so a
/// rejected row (lifecycle_state = 'rejected' AND superseded_by
/// IS NULL) no longer surfaces from the layered worker view.
#[tokio::test]
async fn rejected_skill_does_not_propagate_via_layered_lookup_list() {
    let (_store, skills) = fresh().await;
    let sid = SessionId::new();
    let proposed = skills
        .propose_for_review_judged(
            &skill_doc("leaky_rejected_list", "body"),
            sid,
            SKILL_VERTICAL,
            None,
        )
        .await
        .expect("propose");
    skills
        .transition_lifecycle(proposed.id, LifecycleState::Rejected)
        .await
        .expect("reject");

    let lookup = LayeredLookup::workspace_only(&skills);
    let listed = lookup.list_for_action("low").await.expect("layered list");
    assert!(
        !listed
            .iter()
            .any(|l| l.skill.frontmatter.name == "leaky_rejected_list"),
        "rejected skill must not surface via LayeredLookup::list_for_action"
    );
}

/// Same contract on the `::find` codepath. Pinned independently
/// because the two methods use distinct queries
/// (`find_active_by_name` vs `list_active`); the lifecycle filter
/// landed on both via mission-layered-lookup-filter-by-lifecycle-
/// active.
#[tokio::test]
async fn rejected_skill_does_not_propagate_via_layered_lookup_find() {
    let (_store, skills) = fresh().await;
    let sid = SessionId::new();
    let proposed = skills
        .propose_for_review_judged(
            &skill_doc("leaky_rejected_find", "body"),
            sid,
            SKILL_VERTICAL,
            None,
        )
        .await
        .expect("propose");
    skills
        .transition_lifecycle(proposed.id, LifecycleState::Rejected)
        .await
        .expect("reject");

    let lookup = LayeredLookup::workspace_only(&skills);
    let found = lookup.find("leaky_rejected_find").await.expect("find");
    assert!(
        found.is_none(),
        "rejected skill must not surface via LayeredLookup::find"
    );
}

/// Cross-tenant negative: two independent stores model the workspace
/// boundary. Tenant A submits + rejects "shared_name" and writes the
/// audit row; tenant B's lookup must see no skill, no audit row, and
/// must be free to install its own "shared_name" without clashing.
///
/// `SkillStore` today is per-workspace-store (not tenant-keyed at
/// SQL bind level), so the cross-tenant fence here is structural —
/// each workspace has its own SQLite. The assertion pins that
/// structural property end-to-end alongside the audit-tenant denorm.
#[tokio::test]
async fn cross_tenant_negative_rejection_isolated_per_workspace() {
    let (store_a, skills_a) = fresh().await;
    let (store_b, skills_b) = fresh().await;
    let sid = SessionId::new();
    let proposed = skills_a
        .propose_for_review_judged(
            &skill_doc("shared_name", "tenant_a body"),
            sid,
            SKILL_VERTICAL,
            None,
        )
        .await
        .expect("propose tenant_a");
    skills_a
        .transition_lifecycle(proposed.id, LifecycleState::Rejected)
        .await
        .expect("reject tenant_a");

    let sink_a = AuditSink::new_with_tenant(Arc::clone(&store_a), "tenant-a".into());
    sink_a
        .write(
            Some(sid),
            AuditKind::SkillRejected,
            serde_json::json!({
                "skill_id": proposed.id.0.to_string(),
                "to": LifecycleState::Rejected.label(),
            }),
            None,
        )
        .await
        .expect("write tenant_a audit");
    sink_a.flush_pending().await.expect("flush a");

    let b_lookup = LayeredLookup::workspace_only(&skills_b);
    assert!(
        b_lookup
            .find("shared_name")
            .await
            .expect("find b")
            .is_none(),
        "tenant_b must not see tenant_a's skill"
    );
    let b_list = b_lookup
        .list_for_action("low")
        .await
        .expect("list b");
    assert!(b_list.is_empty(), "tenant_b layered list must be empty");

    let count_b: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM audit_record WHERE kind = ?")
        .bind(AuditKind::SkillRejected.label())
        .fetch_one(store_b.pool())
        .await
        .expect("count b");
    assert_eq!(count_b, 0, "tenant_b audit_record must hold zero SkillRejected rows");

    // Tenant B can install its own version of the same skill name —
    // workspace boundary holds even after the cross-tenant audit
    // write above.
    let b_proposed = skills_b
        .propose_for_review_judged(
            &skill_doc("shared_name", "tenant_b body"),
            sid,
            SKILL_VERTICAL,
            None,
        )
        .await
        .expect("tenant_b can propose the same name");
    assert_eq!(b_proposed.lifecycle_state, LifecycleState::PendingReview);
}
