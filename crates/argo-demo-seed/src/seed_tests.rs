//! Unit tests for the top-level seed orchestration in `seed.rs`.
//!
//! Split out of `seed.rs` to keep that file under the 500-LOC ceiling
//! (mechanical extraction, no behavior change). Loaded via
//! `#[path = "seed_tests.rs"] mod tests;` from `seed.rs`.

use super::*;
use argo_identity::IdentityStore;
use argo_store::approvals::ApprovalDecision;
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
    assert_eq!(report.workers_created, 8);
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
    // 5 pending + 9 pre-decided rows (7 approved / 2 refused = ~78%) so
    // the /approvals ring renders a healthy, trusted-AI percentage on
    // first paint, with role-appropriate variety (refund / hire /
    // contract / marketing / marketplace / comms) — not an all-marketing
    // queue (see `fix-approvals-seed-decided-approvals-for-ring`).
    assert_eq!(report.approval_requests, 14);
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
    // One clean v1→v2→v3 version chain per catalog skill so the Version
    // History panel aligns with the Eval Re-run History (v1/v2/v3).
    assert_eq!(
        report.skill_version_chains_built,
        crate::demo_marketplace_catalog_size()
    );
    assert!(!report.pass_a_real_llm);

    // ── Seed-enrichment surfaces (REAL-but-empty gaps closed) ─────
    // Broadcast bell notifications so the SPA never falls back to the
    // SAMPLE_NOTIFICATIONS mock (it only fires on 404; an empty 200
    // would render the "Nothing new." empty state).
    assert_eq!(report.notifications_seeded, 5);
    // Run history for the Scene-5 workflow so the run-history panel
    // populates (7 runs: 5 ok, 1 failed, 1 running).
    assert_eq!(report.workflow_runs_seeded, 7);
    // Handbook docs so /settings/knowledge lists real pages.
    assert_eq!(report.knowledge_docs_seeded, 4);
    // Honest trust grade on every one of the eight hero teammates.
    assert_eq!(report.trust_grades_set, 8);
    // Real worker-memory facts on the live append path + open conversation
    // threads (v87) so the Memory tab renders real persona-tied data.
    assert!(
        report.worker_memory_facts_seeded >= 8,
        "expected >= 1 memory fact per hero; got {}",
        report.worker_memory_facts_seeded
    );
    assert!(
        report.worker_conversation_threads_seeded >= 4,
        "expected several seeded conversation threads; got {}",
        report.worker_conversation_threads_seeded
    );
    // A fresh seed has no stray workers — the prune archives 0.
    assert_eq!(report.workers_pruned, 0);

    // Vacuous-pass guard (round-37 pattern): seeded tenant sees the
    // eight real hero teammates (incl. Marcus, DEC-028); a DIFFERENT
    // tenant sees zero.
    let identities = IdentityStore::new(&store);
    let seeded_workers = identities.list_workers(tenant, false).await.unwrap();
    assert_eq!(seeded_workers.len(), 8);
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
    assert_eq!(first.approval_requests, 14, "first seed creates the 14-row demo set");

    // Re-seeding the SAME tenant (e.g. to add a new teammate like
    // Grant) must NOT duplicate the approval inbox — the sentinel
    // guard skips the already-seeded set.
    let second = run_canonical_v1_seed(&store, &opts).await.unwrap();
    assert_eq!(
        second.approval_requests, 0,
        "re-seed must skip the already-seeded approval set (got {})",
        second.approval_requests
    );

    // Ground truth: still exactly 5 pending demo approvals (9 of the
    // 14 were pre-decided), not 10.
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

    // Enrichment surfaces are seeded once and skip on re-seed (the
    // notifications / workflow-runs / knowledge guards), so a re-seed
    // to add a teammate never duplicates the bell, run history, or
    // handbook. Trust grades re-apply the same value (no-op UPDATE).
    assert_eq!(first.notifications_seeded, 5);
    assert_eq!(second.notifications_seeded, 0, "re-seed must not duplicate the bell");
    assert_eq!(first.workflow_runs_seeded, 7);
    assert_eq!(second.workflow_runs_seeded, 0, "re-seed must not duplicate run history");
    assert_eq!(first.knowledge_docs_seeded, 4);
    assert_eq!(second.knowledge_docs_seeded, 0, "re-seed must not duplicate the handbook");
    assert_eq!(first.trust_grades_set, 8);
    assert_eq!(second.trust_grades_set, 8, "re-seed re-applies the same grades");
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

#[tokio::test]
async fn canonical_seed_prunes_a_stray_to_exactly_eight() {
    let store = open_store().await;
    let tenant = TenantId::new();

    // A stray Genesis-hired worker like "Maya" lands in the demo tenant
    // (an interactive-session / verify-test artifact).
    let stray = Worker::draft(
        tenant,
        "support",
        Persona::named("Maya", "genesis"),
        Role::new("Stray hired worker."),
    );
    IdentityStore::new(&store)
        .create_worker(&stray)
        .await
        .unwrap();

    let mut opts = SeedOptions::canonical_v1(tenant);
    opts.skip_pass_a_turns = true;
    opts.dressing_audit_rows = 0;
    let report = run_canonical_v1_seed(&store, &opts).await.unwrap();

    // The seed pruned exactly the one stray; the tenant is now exactly the
    // canonical eight live (Maya is archived, not visible).
    assert_eq!(report.workers_pruned, 1, "the stray Maya should be pruned");
    let live = IdentityStore::new(&store)
        .list_workers(tenant, false)
        .await
        .unwrap();
    assert_eq!(live.len(), 8, "demo tenant is exactly the canonical eight after seed");
    assert!(
        !live.iter().any(|w| w.persona.name == "Maya"),
        "Maya must be gone from the live roster"
    );
}

#[tokio::test]
async fn version_history_aligns_with_eval_rerun_history() {
    // Regression for the LIVE walkthrough: the Version History panel
    // showed "6× v0.1.0, all No eval runs yet" — incoherent with the
    // Eval Re-run History (v1/v2/v3) above it. After the seed, the
    // catalog version labels (v1/v2/v3) must MATCH the seeded
    // `skill_reeval_history.skill_version` labels for the same name, so
    // the route's `(skill_name, skill_version)` join lands a real eval
    // pass-rate on each version.
    let store = open_store().await;
    let tenant = TenantId::new();
    let mut opts = SeedOptions::canonical_v1(tenant);
    opts.skip_pass_a_turns = true;
    opts.dressing_audit_rows = 0;
    run_canonical_v1_seed(&store, &opts).await.unwrap();

    let name = "pricing.competitor_scrape";
    let versions = argo_skill::list_versions_for_tenant(&store, name, &tenant.to_string())
        .await
        .unwrap();
    let version_labels: std::collections::HashSet<&str> =
        versions.iter().map(|v| v.version.as_str()).collect();
    assert!(
        version_labels.contains("v1")
            && version_labels.contains("v2")
            && version_labels.contains("v3"),
        "catalog chain must be v1/v2/v3, got {version_labels:?}"
    );

    // Every catalog version label has a matching reeval row for the same
    // skill name → the version-history join is non-empty (no "No eval
    // runs yet" on a version that has an eval run).
    for v in &versions {
        let matches: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM skill_reeval_history WHERE skill_name = ? AND skill_version = ?",
        )
        .bind(name)
        .bind(&v.version)
        .fetch_one(store.pool())
        .await
        .unwrap();
        assert!(
            matches >= 1,
            "version {} must join a reeval row for {name}",
            v.version
        );
    }
}

#[test]
fn demo_approval_set_is_role_appropriate_and_trusted() {
    let rows = demo_approval_rows();
    // Every owner is a real canonical persona (no orphan attribution).
    for r in &rows {
        assert!(
            DEMO_SEED_PERSONAS.iter().any(|p| p.name == r.owner),
            "approval owner {} is not in the canonical roster",
            r.owner
        );
    }
    // Composition: 5 pending + 9 decided.
    let pending = rows.iter().filter(|r| r.decision.is_none()).count();
    let decided = rows.iter().filter(|r| r.decision.is_some()).count();
    assert_eq!(pending, 5, "expected 5 pending demo approvals");
    assert_eq!(decided, 9, "expected 9 decided demo approvals");

    // Trusted-AI ratio: the approve rate among DECIDED rows sits in the
    // healthy 70-85% band the brief calls for (not the 24% distrust
    // signal). 7 approved / 9 decided ≈ 78%.
    let approved = rows
        .iter()
        .filter(|r| matches!(r.decision, Some(ApprovalDecision::Approve)))
        .count();
    let rate = (approved as f64) / (decided as f64);
    assert!(
        (0.70..=0.85).contains(&rate),
        "approval rate {rate:.2} must be a healthy 70-85% (got {approved}/{decided})"
    );

    // Variety: the inbox is NOT all marketing drafts — the banner promises
    // refunds, hires, and contracts, so each must appear. Match on the
    // human label keywords the cards render.
    let has = |needle: &str| rows.iter().any(|r| r.label.contains(needle));
    assert!(has("refund"), "demo set must include a refund approval");
    assert!(
        rows.iter().any(|r| r.label.contains("hire") || r.label.contains("interview")),
        "demo set must include a hiring approval"
    );
    assert!(has("contract"), "demo set must include a contract approval");
    assert!(
        rows.iter().any(|r| r.label.contains("marketing") || r.label.contains("post")),
        "demo set must include a marketing draft"
    );
    // And not every row is Marcus's marketing work.
    let marcus_rows = rows.iter().filter(|r| r.owner == "Marcus").count();
    assert!(
        marcus_rows <= 2,
        "the queue must not be dominated by Marcus drafts (got {marcus_rows})"
    );
}

#[tokio::test]
async fn recent_decisions_are_attributed_to_the_right_worker() {
    // Regression for the LIVE walkthrough: a *recruiting* "candidate
    // rejection email" must be attributed to the Recruiter (Priya), not
    // mis-assigned to Marketing (Marcus) by a blind round-robin. The
    // `approval.asked` audit row's `worker_id` is what the Memory tab's
    // Recent-decisions query filters on.
    let store = open_store().await;
    let tenant = TenantId::new();
    let mut opts = SeedOptions::canonical_v1(tenant);
    opts.skip_pass_a_turns = true;
    opts.dressing_audit_rows = 0;
    run_canonical_v1_seed(&store, &opts).await.unwrap();

    // Resolve Priya + Marcus worker ids from the seeded roster.
    let identities = IdentityStore::new(&store);
    let workers = identities.list_workers(tenant, false).await.unwrap();
    let id_of = |name: &str| {
        workers
            .iter()
            .find(|w| w.persona.name == name)
            .map(|w| w.id.to_string())
            .unwrap_or_default()
    };
    let priya = id_of("Priya");
    let marcus = id_of("Marcus");
    assert!(!priya.is_empty() && !marcus.is_empty());

    // Find the `approval.asked` audit row whose label is the rejection
    // email and confirm its worker_id is Priya's, not Marcus's.
    let payloads: Vec<(String,)> = sqlx::query_as(
        "SELECT payload_json FROM audit_record \
         WHERE tenant_id = ? AND kind = 'approval.asked'",
    )
    .bind(tenant.to_string())
    .fetch_all(store.pool())
    .await
    .unwrap();
    let mut found = false;
    for (raw,) in &payloads {
        let v: serde_json::Value = serde_json::from_str(raw).unwrap();
        if v.get("label").and_then(|l| l.as_str()) == Some("candidate rejection email") {
            let wid = v.get("worker_id").and_then(|w| w.as_str()).unwrap_or("");
            assert_eq!(wid, priya, "rejection email must be Priya's, not {wid}");
            assert_ne!(wid, marcus, "rejection email must NOT land on Marcus");
            found = true;
        }
    }
    assert!(found, "seeded `candidate rejection email` approval.asked row not found");
}
