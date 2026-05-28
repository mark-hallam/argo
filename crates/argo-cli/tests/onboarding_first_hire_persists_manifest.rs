//! First-hire manifest-persistence smoke (F6 P1 closure).
//!
//! The pre-fix first-hire path forwarded a Genesis plan id into the
//! `routes_onboarding::first_hire_response` audit payload, but the
//! plan id never reached the persisted persona row — the `HireDraft`
//! shape dropped it on the floor. The fix wires `HireDraft.genesis_plan_id`
//! through `IdentityStore::create_worker_from_draft` onto
//! `persona.provenance_chain` so the read-side `/workers/{id}/overview`
//! can surface a "Plan provenance" chip.
//!
//! This integration test exercises the persistence layer directly
//! (no HTTP plumbing) so it stays runnable from any worker target,
//! and asserts:
//!
//! - A draft carrying `genesis_plan_id: Some("...")` lands a
//!   `genesis_plan_id=<id>` marker on the persisted persona's
//!   `provenance_chain`.
//! - The reverse helper `genesis_plan_id_from_provenance` round-trips.
//! - A draft with `genesis_plan_id: None` leaves the provenance chain
//!   empty (no spurious marker on fallback / pre-Genesis hires).
//!
//! HTTP-layer coverage for the same plumbing lives at
//! `routes_onboarding_first_hire_genesis_tests::first_hire_invokes_\
//! genesis_and_persists_synthesised_charter`.

#![allow(clippy::expect_used)]

use argo_identity::{
    genesis_plan_id_from_provenance, ApprovalPolicyChoice, HireDraft, HireDutyDraft, IdentityStore,
};
use argo_store::SessionStore;
use argo_types::TenantId;

fn draft_with(plan_id: Option<String>) -> HireDraft {
    HireDraft {
        suggested_name: "Maya".into(),
        charter: "Run weekly outbound campaigns to cold leads and qualify replies.".into(),
        suggested_vertical: "gtm-engineer".into(),
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
        tagline: Some("Marketing intern persona".into()),
        genesis_plan_id: plan_id,
    }
}

#[tokio::test]
async fn first_hire_persists_genesis_plan_id_onto_provenance_chain() {
    let store = SessionStore::open(":memory:").await.expect("mem db");
    store.migrate().await.expect("migrate");
    let tenant = TenantId::new();
    let identities = IdentityStore::new(&store);

    let plan_id = "marketing-intern-maya";
    let worker_id = identities
        .create_worker_from_draft(
            &draft_with(Some(plan_id.into())),
            tenant,
            "human:alice@acme.com",
        )
        .await
        .expect("hire ok");

    let worker = identities
        .get_worker(tenant, worker_id)
        .await
        .expect("load")
        .expect("worker row");

    let surfaced = genesis_plan_id_from_provenance(&worker.persona.provenance_chain);
    assert_eq!(
        surfaced.as_deref(),
        Some(plan_id),
        "genesis_plan_id must round-trip via persona.provenance_chain; chain was {:?}",
        worker.persona.provenance_chain,
    );
}

#[tokio::test]
async fn first_hire_without_plan_id_leaves_provenance_chain_empty() {
    // Fallback (`outcome.is_fallback() == true`) and pre-Genesis hires
    // both pass `genesis_plan_id: None`. The persisted row must not
    // carry a stub `genesis_plan_id=` marker — otherwise the "Plan
    // provenance" chip would render on a worker that has no plan.
    let store = SessionStore::open(":memory:").await.expect("mem db");
    store.migrate().await.expect("migrate");
    let tenant = TenantId::new();
    let identities = IdentityStore::new(&store);

    let worker_id = identities
        .create_worker_from_draft(&draft_with(None), tenant, "human:alice@acme.com")
        .await
        .expect("hire ok");

    let worker = identities
        .get_worker(tenant, worker_id)
        .await
        .expect("load")
        .expect("worker row");

    assert!(
        worker.persona.provenance_chain.is_empty(),
        "no plan id ⇒ no provenance marker; got: {:?}",
        worker.persona.provenance_chain,
    );
    assert!(
        genesis_plan_id_from_provenance(&worker.persona.provenance_chain).is_none(),
        "extractor must return None for unmarked chain",
    );
}

#[tokio::test]
async fn first_hire_ignores_blank_plan_id() {
    // Defence-in-depth: a whitespace-only plan id would otherwise
    // stamp a useless `genesis_plan_id=` marker and surface as an
    // empty chip on the SPA.
    let store = SessionStore::open(":memory:").await.expect("mem db");
    store.migrate().await.expect("migrate");
    let tenant = TenantId::new();
    let identities = IdentityStore::new(&store);

    let worker_id = identities
        .create_worker_from_draft(
            &draft_with(Some("   ".into())),
            tenant,
            "human:alice@acme.com",
        )
        .await
        .expect("hire ok");

    let worker = identities
        .get_worker(tenant, worker_id)
        .await
        .expect("load")
        .expect("worker row");

    assert!(
        worker.persona.provenance_chain.is_empty(),
        "blank plan id must not stamp a marker; got: {:?}",
        worker.persona.provenance_chain,
    );
}
