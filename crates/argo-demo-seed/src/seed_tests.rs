//! Unit tests for the top-level seed orchestration in `seed.rs`.
//!
//! Split out of `seed.rs` to keep that file under the 500-LOC ceiling
//! (mechanical extraction, no behavior change). Loaded via
//! `#[path = "seed_tests.rs"] mod tests;` from `seed.rs`.

use super::*;
use argo_identity::IdentityStore;
use std::sync::Arc;

async fn open_store() -> Arc<SessionStore> {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    Arc::new(store)
}

#[tokio::test]
async fn fails_closed_without_provider_key() {
    let store = open_store().await;
    let tenant = TenantId::new();
    // Make sure the env var is unset for this test.
    std::env::remove_var(PROVIDER_KEY_ENV);
    let opts = SeedOptions::canonical_v1(tenant);
    let err = run_canonical_v1_seed(&store, &opts).await.unwrap_err();
    assert!(matches!(err, SeedError::MissingProviderKey));
}

#[tokio::test]
async fn skip_pass_a_seeds_dressing_only() {
    let store = open_store().await;
    let tenant = TenantId::new();
    let mut opts = SeedOptions::canonical_v1(tenant);
    opts.skip_pass_a_turns = true;
    opts.dressing_audit_rows = 25;
    let report = run_canonical_v1_seed(&store, &opts).await.unwrap();
    assert_eq!(report.workers_created, 7);
    assert_eq!(report.foreign_workers_created, 1);
    assert_eq!(report.workflows_created, 1);
    assert_eq!(report.skills_created, 3);
    assert_eq!(report.foreign_skills_created, 1);
    assert!(
        report.marketplace_skills_created >= 6,
        "marketplace catalog must seed >= 6 rows (got {})",
        report.marketplace_skills_created
    );
    assert_eq!(report.pass_a_audit_rows, 0);
    assert_eq!(report.pass_b_audit_rows, 25);
    assert_eq!(report.audit_backstory_rows, crate::backstory_row_count());
    assert_eq!(report.foreign_audit_rows, 1);
    // 5 pending + 3 pre-decided rows so the /approvals ring renders
    // a real percentage on first paint (see the brief
    // `fix-approvals-seed-decided-approvals-for-ring`).
    assert_eq!(report.approval_requests, 8);
    // Five committed retrains (one per classification) seeded for the
    // lead heroes so the Recent-changes panel renders real history.
    assert_eq!(report.retrain_records_seeded, 5);
    // One real skill_ir graph per catalog skill (so /skills/<id>/ir
    // returns an IR) and three reeval rows per catalog skill (so
    // /workers/<id>/evals shows a real trend).
    assert_eq!(
        report.skill_ir_seeded,
        crate::demo_marketplace_catalog_size()
    );
    assert_eq!(
        report.skill_reeval_rows_seeded,
        crate::demo_marketplace_catalog_size() * 3
    );
    assert!(!report.pass_a_real_llm);

    // Vacuous-pass guard (round-37 pattern): seeded tenant sees the
    // seven real hero teammates; a DIFFERENT tenant sees zero.
    let identities = IdentityStore::new(&store);
    let seeded_workers = identities.list_workers(tenant, false).await.unwrap();
    assert_eq!(seeded_workers.len(), 7);
    let other_tenant = TenantId::new();
    let other_workers = identities.list_workers(other_tenant, false).await.unwrap();
    assert_eq!(other_workers.len(), 0);
}

#[tokio::test]
async fn reseed_does_not_duplicate_approvals() {
    let store = open_store().await;
    let tenant = TenantId::new();
    let mut opts = SeedOptions::canonical_v1(tenant);
    opts.skip_pass_a_turns = true;
    opts.dressing_audit_rows = 0;

    let first = run_canonical_v1_seed(&store, &opts).await.unwrap();
    assert_eq!(first.approval_requests, 8, "first seed creates the 8-row demo set");

    // Re-seeding the SAME tenant (e.g. to add a new teammate like
    // Grant) must NOT duplicate the approval inbox — the sentinel
    // guard skips the already-seeded set.
    let second = run_canonical_v1_seed(&store, &opts).await.unwrap();
    assert_eq!(
        second.approval_requests, 0,
        "re-seed must skip the already-seeded approval set (got {})",
        second.approval_requests
    );

    // Ground truth: still exactly 5 pending demo approvals (3 of the
    // 8 were pre-decided), not 10.
    let pending = list_pending(store.pool(), tenant, 100).await.unwrap();
    assert_eq!(
        pending.len(),
        5,
        "expected 5 pending demo approvals after re-seed, got {}",
        pending.len()
    );

    // Retrain history is seeded once: 5 on the first run, 0 on re-seed
    // (the committed-retrain sentinel guards against duplication).
    assert_eq!(first.retrain_records_seeded, 5);
    assert_eq!(
        second.retrain_records_seeded, 0,
        "re-seed must skip already-seeded retrains (got {})",
        second.retrain_records_seeded
    );
}

#[tokio::test]
async fn assigns_real_skills_to_workers_for_claim_4_adoption() {
    let store = open_store().await;
    let tenant = TenantId::new();
    let mut opts = SeedOptions::canonical_v1(tenant);
    opts.skip_pass_a_turns = true;
    opts.dressing_audit_rows = 0;

    let report = run_canonical_v1_seed(&store, &opts).await.unwrap();
    // 7 hero workers × 2 skills each.
    assert!(
        report.worker_skills_assigned >= 7,
        "every hero worker should learn at least one skill; got {}",
        report.worker_skills_assigned
    );
    // Ground truth: worker_skill rows exist for the tenant's workers,
    // so /skills/<id>/usage + "learned this month" are non-empty.
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM worker_skill ws \
         JOIN worker w ON w.id = ws.worker_id WHERE w.tenant_id = ?",
    )
    .bind(tenant.to_string())
    .fetch_one(store.pool())
    .await
    .unwrap();
    assert!(count >= 7, "expected worker_skill rows seeded; got {count}");
}

#[tokio::test]
async fn pass_a_scaffolding_runs_when_provider_key_set() {
    let store = open_store().await;
    let tenant = TenantId::new();
    std::env::set_var(PROVIDER_KEY_ENV, "sandbox-key-for-tests");
    let mut opts = SeedOptions::canonical_v1(tenant);
    opts.pass_a_turns = 4;
    opts.dressing_audit_rows = 10;
    let report = run_canonical_v1_seed(&store, &opts).await.unwrap();
    std::env::remove_var(PROVIDER_KEY_ENV);
    assert!(report.pass_a_real_llm);
    // 4 turns × 3 audit rows each (TurnStart + AssistantMessage + TurnEnd).
    assert_eq!(report.pass_a_audit_rows, 12);
    assert_eq!(report.pass_b_audit_rows, 10);
}

#[tokio::test]
async fn unknown_profile_is_rejected() {
    let store = open_store().await;
    let tenant = TenantId::new();
    let mut opts = SeedOptions::canonical_v1(tenant);
    opts.profile = "bogus".into();
    opts.skip_pass_a_turns = true;
    let err = run_canonical_v1_seed(&store, &opts).await.unwrap_err();
    assert!(matches!(err, SeedError::UnknownProfile(p) if p == "bogus"));
}
