//! Acceptance test for the demo-seed cohort surface after the
//! no-theater curation (DEC-008/DEC-016 + 2026-05-29 operator call):
//! the demo tenant seeds exactly the SIX real hero teammates — the
//! former "dept-filler" rows (fake roster entries seeded only to pad
//! `/cohorts` tabs) were removed as theater. So the seed must produce
//! six cohorts, one per real department the tenant staffs, with the
//! `pitch-agent` cohort present (Portia, the Track-P Pitch Agent, is
//! now a first-class hero teammate).
//!
//! Operator memory: `project_track_p_v1_mvp_scope`,
//! `feedback_no_theater_in_demo`.

use argo_demo_seed::{run_canonical_v1_seed, SeedOptions};
use argo_store::cohort::list_cohorts;
use argo_store::SessionStore;
use argo_types::TenantId;
use std::sync::Arc;

async fn open_store() -> Arc<SessionStore> {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    Arc::new(store)
}

#[tokio::test]
async fn demo_seed_produces_six_cohorts_one_per_real_dept() {
    let store = open_store().await;
    let tenant = TenantId::new();
    let mut opts = SeedOptions::canonical_v1(tenant);
    opts.skip_pass_a_turns = true;
    opts.dressing_audit_rows = 0;

    let report = run_canonical_v1_seed(&store, &opts).await.unwrap();
    assert_eq!(
        report.workers_created, 7,
        "demo tenant seeds exactly the seven real hero teammates (incl. Portia + Grant)"
    );

    let cohorts = list_cohorts(store.as_ref(), tenant).await.unwrap();
    assert_eq!(
        cohorts.len(),
        7,
        "expected exactly 7 cohorts (one per real staffed dept); got {} ({:?})",
        cohorts.len(),
        cohorts.iter().map(|c| c.vertical.clone()).collect::<Vec<_>>()
    );
}

#[tokio::test]
async fn demo_seed_emits_pitch_agent_cohort_for_track_p() {
    let store = open_store().await;
    let tenant = TenantId::new();
    let mut opts = SeedOptions::canonical_v1(tenant);
    opts.skip_pass_a_turns = true;
    opts.dressing_audit_rows = 0;

    run_canonical_v1_seed(&store, &opts).await.unwrap();

    let cohorts = list_cohorts(store.as_ref(), tenant).await.unwrap();
    let pitch = cohorts
        .iter()
        .find(|c| c.vertical == "pitch-agent")
        .expect(
            "pitch-agent cohort must be present per Track P v1.0 scope (memory \
             project_track_p_v1_mvp_scope) — Portia is a hero teammate",
        );
    assert!(
        pitch.member_count >= 1,
        "pitch-agent cohort must have >= 1 member; got {}",
        pitch.member_count
    );
}

#[tokio::test]
async fn demo_seed_covers_each_hero_dept_vertical() {
    let store = open_store().await;
    let tenant = TenantId::new();
    let mut opts = SeedOptions::canonical_v1(tenant);
    opts.skip_pass_a_turns = true;
    opts.dressing_audit_rows = 0;

    run_canonical_v1_seed(&store, &opts).await.unwrap();

    let cohorts = list_cohorts(store.as_ref(), tenant).await.unwrap();
    let verticals: std::collections::HashSet<&str> =
        cohorts.iter().map(|c| c.vertical.as_str()).collect();
    for required in [
        "marketplace",
        "project-coordination",
        "recruiting",
        "support",
        "marketing",
        "pitch-agent",
        "gtm",
    ] {
        assert!(
            verticals.contains(required),
            "cohort vertical `{required}` missing from seed output; got {verticals:?}"
        );
    }
}
