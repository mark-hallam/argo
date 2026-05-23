//! # Mission Claim 1 LIVE — workers persist across gateway restarts
//!
//! Operator-locked pass criteria from `docs/skycot/MISSION.md` (added
//! 2026-05-23 in evaluator round 15 with direct operator sign-off):
//!
//! > Claim 1 LIVE = 5 distinct workers persist across gateway
//! > restarts, with memory, and one of them can be re-summoned by
//! > name 24 hours later.
//!
//! This file is the regression contract for the **identity-persistence**
//! half of that promise. If it goes red, claim 1 has regressed.
//!
//! ## What this test proves
//!
//! 1. **Schema guarantee** — the `worker` table has NO `expires_at`,
//!    `ttl`, `valid_until`, or `expiry` column. Structural proof that
//!    "24h later" is not gated on a hidden TTL.
//! 2. **5 distinct hires succeed** — names + verticals + worker_ids
//!    are all distinct.
//! 3. **Workers survive a file-store restart** — drop every in-process
//!    handle, re-open the SAME SQLite file, list workers; all 5 must
//!    reappear with identical names, vertical ids, duty entries, and
//!    memory_namespace bindings.
//! 4. **Worker_ids are stable across restart** — Mariah's id pre-restart
//!    matches Mariah's id post-restart (no regeneration on re-resolve).
//!
//! ## What's deferred to a follow-up
//!
//! The "memory roundtrip survives second restart" half of the LIVE
//! criteria needs the `argo-memory::MemoryStore` write/read API
//! surface to be re-verified. The follow-up brief
//! `eval-claim-1-live-memory-roundtrip-extension` extends this test
//! to cover (a) write memory marker as Mariah, (b) restart, (c) read
//! marker back. Until that lands, this test covers the identity half
//! of claim 1, not the memory half.
//!
//! ## Authored
//!
//! Evaluator round 17, 2026-05-23.

use std::collections::HashSet;

use argo_identity::{
    ApprovalPolicyChoice, HireDraft, HireDutyDraft, IdentityStore,
};
use argo_store::SessionStore;
use argo_types::TenantId;

fn draft_for(name: &str, vertical: &str, charter: &str) -> HireDraft {
    HireDraft {
        suggested_name: name.into(),
        charter: charter.into(),
        suggested_vertical: vertical.into(),
        approval_policy: ApprovalPolicyChoice {
            level: "balanced".into(),
        },
        duties: vec![HireDutyDraft {
            cron: "0 9 * * MON-FRI".into(),
            entry_action: "morning_kickoff".into(),
            description: format!("Daily kickoff for {name}"),
            default_enabled: true,
        }],
        voice_profile: None,
        tagline: None,
    }
}

fn five_distinct_drafts() -> Vec<HireDraft> {
    vec![
        draft_for(
            "Mariah-Marketplace",
            "marketplace-seller",
            "Validate inventory data and post fresh listings to Facebook Marketplace. Approval required for listings priced below the floor.",
        ),
        draft_for(
            "Priya-Recruiter",
            "hr-screener",
            "First-pass candidate screen against a JD with skill-match scoring. Escalate ambiguous matches with notes.",
        ),
        draft_for(
            "Jamal-Support",
            "customer-support",
            "Front-line ticket triage. Answer FAQ from KB, escalate ambiguous, never reply without operator approval.",
        ),
        draft_for(
            "Eva-DataEntry",
            "data-entry-clerk",
            "Triage incoming PDF invoices into structured rows with per-field confidence scores. Never invent values.",
        ),
        draft_for(
            "Rio-LeadResearch",
            "lead-research",
            "Research inbound leads — company size, ICP fit, signal strength — before booking demos.",
        ),
    ]
}

#[tokio::test]
async fn claim_1_live_workers_persist_across_simulated_restart() {
    // ── Pre-flight: schema guarantee. The worker table must NOT have
    // any TTL / expiry column. If a future migration adds one, this
    // test must catch it — claim 1 LIVE assumes workers persist
    // indefinitely, not until a deadline.
    {
        let store = SessionStore::open(":memory:").await.expect("open");
        store.migrate().await.expect("migrate");
        let cols: Vec<(String,)> =
            sqlx::query_as("SELECT name FROM pragma_table_info('worker')")
                .fetch_all(store.pool())
                .await
                .expect("worker pragma");
        let names: HashSet<String> =
            cols.into_iter().map(|(c,)| c.to_lowercase()).collect();
        for forbidden in &["expires_at", "ttl", "valid_until", "expiry"] {
            assert!(
                !names.contains(*forbidden),
                "worker table has a `{forbidden}` column — claim 1 LIVE assumes workers do NOT expire on a hidden TTL. If this is intentional, update docs/skycot/MISSION.md and this test together."
            );
        }
    }

    // ── Phase 1: open file-backed store, hire 5 workers, drop everything.
    let tempdir = tempfile::tempdir().expect("tempdir");
    let db_path = tempdir.path().join("claim1.db");
    let drafts = five_distinct_drafts();
    let tenant = TenantId::new();

    let mut hired_ids: Vec<String> = Vec::with_capacity(5);
    let expected_names: HashSet<String> = drafts
        .iter()
        .map(|d| d.suggested_name.clone())
        .collect();
    let expected_verticals: HashSet<String> = drafts
        .iter()
        .map(|d| d.suggested_vertical.clone())
        .collect();
    {
        let store = SessionStore::open_unencrypted_for_tests(&db_path)
            .await
            .expect("open phase 1");
        store.migrate().await.expect("migrate phase 1");
        let identities = IdentityStore::new(&store);
        for draft in &drafts {
            let id = identities
                .create_worker_from_draft(draft, tenant, "human:evaluator-claim-1-live")
                .await
                .expect("hire phase 1");
            hired_ids.push(id.to_string());
        }
        drop(identities);
        drop(store);
    }

    assert_eq!(hired_ids.len(), 5, "all 5 hires must succeed");
    let unique_ids: HashSet<&String> = hired_ids.iter().collect();
    assert_eq!(unique_ids.len(), 5, "all 5 worker ids must be distinct");

    // ── Phase 2: simulate "gateway restart" — re-open SAME file.
    // All 5 workers must reappear with identical names + verticals +
    // memory_namespace bindings + duties.
    let mariah_id_after_restart;
    {
        let store = SessionStore::open_unencrypted_for_tests(&db_path)
            .await
            .expect("re-open after restart");
        store.migrate().await.expect("migrate idempotent on re-open");
        let identities = IdentityStore::new(&store);
        let listed = identities
            .list_workers(tenant, false)
            .await
            .expect("list after restart");
        assert_eq!(
            listed.len(),
            5,
            "post-restart: expected 5 workers, found {}",
            listed.len()
        );
        let actual_names: HashSet<String> =
            listed.iter().map(|w| w.persona.name.clone()).collect();
        let actual_verticals: HashSet<String> =
            listed.iter().map(|w| w.vertical_id.clone()).collect();
        assert_eq!(
            actual_names, expected_names,
            "post-restart: persona names diverged"
        );
        assert_eq!(
            actual_verticals, expected_verticals,
            "post-restart: vertical ids diverged"
        );
        for worker in &listed {
            assert_eq!(
                worker.memory_namespace.tenant_id, tenant,
                "post-restart: memory_namespace.tenant_id diverged for {}",
                worker.persona.name
            );
            assert_eq!(
                worker.memory_namespace.worker_id, worker.id,
                "post-restart: memory_namespace.worker_id diverged for {}",
                worker.persona.name
            );
            assert_eq!(
                worker.role.duties.len(),
                1,
                "post-restart: expected 1 duty, got {} for {}",
                worker.role.duties.len(),
                worker.persona.name
            );
        }

        // Re-summon Mariah by name (claim 1 "re-summoned by name"
        // criteria). Verify her id is stable across the restart.
        let mariah = listed
            .iter()
            .find(|w| w.persona.name == "Mariah-Marketplace")
            .expect("Mariah must be re-resolvable by name after restart");
        mariah_id_after_restart = mariah.id.to_string();
    }

    let mariah_id_before_restart = hired_ids
        .iter()
        .next()
        .cloned()
        .expect("Mariah was hired first");
    assert_eq!(
        mariah_id_before_restart, mariah_id_after_restart,
        "Mariah's worker_id must be stable across the simulated restart"
    );

    // ── Phase 3: simulate "24 hours later" — re-open YET AGAIN,
    //   re-resolve Mariah by name. Identity must still resolve.
    //
    // Note: the "memory marker survives second restart" half of the
    // LIVE criteria is deferred to a follow-up brief — see this
    // file's docstring.
    {
        let store = SessionStore::open_unencrypted_for_tests(&db_path)
            .await
            .expect("re-open phase 3 (simulated 24h later)");
        store
            .migrate()
            .await
            .expect("migrate idempotent on second re-open");
        let identities = IdentityStore::new(&store);
        let listed = identities
            .list_workers(tenant, false)
            .await
            .expect("list phase 3");
        assert_eq!(
            listed.len(),
            5,
            "phase 3 (simulated 24h): expected 5 workers, found {}",
            listed.len()
        );
        let mariah = listed
            .into_iter()
            .find(|w| w.persona.name == "Mariah-Marketplace")
            .expect("Mariah must STILL be re-resolvable by name across two restart cycles");
        assert_eq!(
            mariah.id.to_string(),
            mariah_id_after_restart,
            "Mariah's worker_id must be stable across the second restart"
        );
    }

    // If we got here, claim 1's identity-persistence half is LIVE.
    // The memory-roundtrip half is deferred to
    // `eval-claim-1-live-memory-roundtrip-extension`.
}
