//! Acceptance test for brief
//! `fix-demo-seed-add-pitch-agent-and-missing-cohort-rows`:
//! the demo seed must produce ≥9 cohorts (one per dept tab declared
//! in `web/components/skycot/cohorts/dept-registry.ts`) with the
//! `pitch-agent` cohort present so Track P's v1.0-scope claim holds
//! at the /cohorts surface.
//!
//! UX audit source: whole-site F06 (docs/skycot/ux-audits/2026-05-28-0252/).
//! Operator memory: `project_track_p_v1_mvp_scope`.

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
async fn demo_seed_produces_at_least_nine_cohorts_one_per_dept() {
    let store = open_store().await;
    let tenant = TenantId::new();
    let mut opts = SeedOptions::canonical_v1(tenant);
    opts.skip_pass_a_turns = true;
    opts.dressing_audit_rows = 0;

    let report = run_canonical_v1_seed(&store, &opts).await.unwrap();
    assert_eq!(
        report.workers_created, 5,
        "canonical Scene-1 personas must remain at 5"
    );
    assert_eq!(
        report.dept_filler_workers_created, 4,
        "dept-filler personas cover the 4 missing dept tabs"
    );

    let cohorts = list_cohorts(store.as_ref(), tenant).await.unwrap();
    assert!(
        cohorts.len() >= 9,
        "expected >= 9 cohorts (one per dept tab) after demo seed; got {} ({:?})",
        cohorts.len(),
        cohorts
            .iter()
            .map(|c| c.vertical.clone())
            .collect::<Vec<_>>()
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
             project_track_p_v1_mvp_scope)",
        );
    assert!(
        pitch.member_count >= 1,
        "pitch-agent cohort must have >= 1 member; got {}",
        pitch.member_count
    );
}

#[tokio::test]
async fn demo_seed_covers_each_filler_dept_vertical() {
    let store = open_store().await;
    let tenant = TenantId::new();
    let mut opts = SeedOptions::canonical_v1(tenant);
    opts.skip_pass_a_turns = true;
    opts.dressing_audit_rows = 0;

    run_canonical_v1_seed(&store, &opts).await.unwrap();

    let cohorts = list_cohorts(store.as_ref(), tenant).await.unwrap();
    let verticals: std::collections::HashSet<&str> =
        cohorts.iter().map(|c| c.vertical.as_str()).collect();
    for required in ["pitch-agent", "finance-ar", "devex", "legal"] {
        assert!(
            verticals.contains(required),
            "cohort vertical `{required}` missing from seed output; got {verticals:?}"
        );
    }
}
