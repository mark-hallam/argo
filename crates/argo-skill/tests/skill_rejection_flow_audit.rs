//! End-to-end audit pin for mission claim 4's rejection path.
//!
//! Contract: a learned skill can be proposed for review, rejected by
//! the human-review lifecycle seam, recorded with first-class
//! `skill.rejected` audit evidence, and remains absent from runtime
//! `LayeredLookup::list_for_action` results.

use argo_audit::{rows_for_session, AuditKind, AuditSink};
use argo_skill::{LayeredLookup, LifecycleState, SkillStore};
use argo_store::SessionStore;
use argo_types::{SessionId, TenantId};
use std::sync::Arc;

const VERTICAL_ID: &str = "support";
const SKILL_NAME: &str = "rejected_claim4_followup";
const REJECT_NOTE: &str = "operator rejected: unsafe customer follow-up wording";

async fn fixture() -> (Arc<SessionStore>, SkillStore, TenantId, SessionId) {
    let store = SessionStore::open(":memory:").await.expect("open store");
    store.migrate().await.expect("migrate store");
    let (tenant, workspace) = store
        .ensure_workspace("claim4-rejection-audit", "/tmp/claim4-rejection-audit")
        .await
        .expect("workspace");
    let session = store
        .create_session(
            tenant,
            workspace,
            "mock-model",
            "mock-provider",
            Some("claim4 rejection audit"),
            Some(VERTICAL_ID),
        )
        .await
        .expect("session");
    let store = Arc::new(store);
    let skills = SkillStore::new(Arc::clone(&store));
    (store, skills, tenant, session)
}

fn learned_skill_doc() -> String {
    format!(
        "---\n\
         name: {SKILL_NAME}\n\
         version: \"0.1\"\n\
         description: learned skill that must stay invisible after rejection\n\
         lock: none\n\
         provenance:\n  \
           source: learned:claim4-rejection-audit\n  \
           hash: sha256-rejected-claim4\n\
         ---\n\
         follow up with the customer without the required safety checks\n"
    )
}

#[tokio::test]
async fn proposed_skill_rejects_with_audit_and_runtime_exclusion() {
    let (store, skills, tenant, session) = fixture().await;

    let proposed = skills
        .propose_for_review(&learned_skill_doc(), session, VERTICAL_ID)
        .await
        .expect("propose learned skill");
    assert_eq!(proposed.lifecycle_state, LifecycleState::PendingReview);

    let rejected = skills
        .transition_lifecycle(proposed.id, LifecycleState::Rejected)
        .await
        .expect("operator reject transition");
    assert_eq!(rejected, LifecycleState::Rejected);

    let audit = AuditSink::new_with_tenant(Arc::clone(&store), tenant.to_string());
    audit
        .write(
            Some(session),
            AuditKind::SkillRejected,
            serde_json::json!({
                "skill_id": proposed.id.0.to_string(),
                "name": SKILL_NAME,
                "vertical_id": VERTICAL_ID,
                "to": LifecycleState::Rejected.label(),
                "reviewed_by": "claim4-audit-test",
                "note": REJECT_NOTE,
            }),
            None,
        )
        .await
        .expect("write skill.rejected audit");
    audit.flush_pending().await.expect("flush audit");

    let rows = rows_for_session(&store, session)
        .await
        .expect("rows_for_session");
    let rejection = rows
        .iter()
        .find(|row| row.kind == AuditKind::SkillRejected.label())
        .expect("skill.rejected audit row");
    assert_eq!(rejection.payload["skill_id"], proposed.id.0.to_string());
    assert_eq!(rejection.payload["name"], SKILL_NAME);
    assert_eq!(rejection.payload["vertical_id"], VERTICAL_ID);
    assert_eq!(rejection.payload["to"], "rejected");
    assert_eq!(rejection.payload["note"], REJECT_NOTE);
    assert_eq!(rejection.payload["tenant_id"], tenant.to_string());

    let lookup = LayeredLookup::workspace_only(&skills);
    let listed = lookup.list_for_action("low").await.expect("runtime lookup");
    assert!(
        listed
            .iter()
            .all(|row| row.skill.frontmatter.name != SKILL_NAME),
        "rejected learned skill must not be offered to the runtime"
    );
}
