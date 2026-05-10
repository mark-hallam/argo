//! End-to-end exercise of the auto-approval budget + time-window
//! envelope (mission-auto-approval-budget-e2e).
//!
//! Mission claim: operators can set budget caps and a weekly time
//! window and trust that approval-gated actions inside the
//! envelope auto-execute while everything outside falls to a
//! human Ask. The primitives —
//! [`argo_identity::auto_approval::evaluate`] for the decision
//! and the persisted `worker_budget` ledger
//! ([`argo_identity::worker_budget::record_charge`] +
//! [`argo_identity::worker_budget::spend_today_for_worker`]) for
//! cumulative spend — already ship in the platform. This test is
//! the cheap insurance that the wiring stays correct end-to-end.
//!
//! Scenario (matches `docs/build-plan/streams/mission-auto-approval-budget-e2e.md`):
//!
//! | # | When (UTC)                     | Estimated cost | Expected verdict        |
//! |---|--------------------------------|----------------|-------------------------|
//! | 1 | Wed 2026-05-13 18:00 (in)      | $3.00          | AutoApprove             |
//! | 2 | Wed 2026-05-13 18:00 (in)      | $4.00          | AutoApprove (today=$7)  |
//! | 3 | Wed 2026-05-13 18:00 (in)      | $6.00          | Ask (per-action cap)    |
//! | 4 | Sat 2026-05-16 15:00 (out)     | $2.00          | Ask (outside window)    |
//!
//! Final assertion: the persisted ledger reconciles to $7.00 — only
//! the auto-executed actions consume budget; Asks do not. Daily-cap
//! Block path is exercised by the existing
//! `argo-identity::auto_approval` unit suite; this test focuses on
//! the per-action + window envelope plus the persistence round-trip
//! the unit tests can't reach.
//!
//! ## Sensible deviations from the brief
//!
//! - The brief sketches a `[budget] / [window]` TOML schema with a
//!   `timezone = "America/New_York"` field. The actual
//!   [`argo_identity::AutoApprovalPolicy`] schema is flat
//!   (`max_usd_micros_per_action`, `daily_budget_usd_micros`,
//!   `[[time_windows]]`) and has no timezone field — `TimeWindow`
//!   start/end are read against the same UTC clock the evaluator
//!   uses. The fixture pre-translates the NY business window to its
//!   UTC equivalent (13:00–21:00 during EDT) so the on-disk shape
//!   matches what the platform actually parses. The brief's
//!   "Sensible deviations" allowance covers this.
//! - The 4 candidate actions are constructed in-memory rather than
//!   serialised to per-file `actions_*.json` fixtures; the
//!   evaluator inspects fields directly from a `PendingAction`, so
//!   on-disk JSON would only add round-trip noise.

use argo_identity::auto_approval::{
    dollars_to_micros, evaluate, Decision, PendingAction, SpendSnapshot,
};
use argo_identity::worker_budget::{record_charge, spend_today_for_worker};
use argo_identity::AutoApprovalPolicy;
use argo_store::SessionStore;
use chrono::{DateTime, TimeZone, Utc};

const FIXTURE_BUDGET_TOML: &str =
    include_str!("fixtures/auto_approval/auto_approval_budget.toml");

const WORKER_ID: &str = "worker-budget-e2e";

#[derive(Debug, Clone, Copy)]
struct Candidate {
    id: &'static str,
    estimated_usd: f64,
    fired_at: DateTime<Utc>,
}

fn load_policy() -> AutoApprovalPolicy {
    toml::from_str(FIXTURE_BUDGET_TOML).expect(
        "fixture auto_approval_budget.toml must deserialise into AutoApprovalPolicy",
    )
}

fn pending_for(c: &Candidate) -> PendingAction<'static> {
    PendingAction {
        risk_level: "low",
        requires_approval: true,
        estimated_usd_micros: Some(dollars_to_micros(c.estimated_usd)),
        action_type: "send_invoice",
    }
}

#[test]
fn budget_and_window_envelope_reconciles_end_to_end() {
    let policy = load_policy();

    // Pin the fixture's policy shape so a future TOML drift surfaces
    // here, not three lines into the assertion mosaic below.
    assert!(policy.enabled, "fixture must enable the policy");
    assert_eq!(
        policy.max_usd_micros_per_action,
        Some(dollars_to_micros(5.0)),
        "per-action cap must be $5.00",
    );
    assert_eq!(
        policy.daily_budget_usd_micros,
        Some(dollars_to_micros(50.0)),
        "daily cap must be $50.00",
    );
    assert_eq!(policy.time_windows.len(), 1, "one [[time_windows]] entry");

    // Wed 2026-05-13 18:00 UTC = 14:00 America/New_York (EDT) — inside.
    let inside = Utc.with_ymd_and_hms(2026, 5, 13, 18, 0, 0).unwrap();
    // Sat 2026-05-16 15:00 UTC = 11:00 America/New_York (EDT) — outside
    // because Saturday isn't in the weekday set.
    let outside = Utc.with_ymd_and_hms(2026, 5, 16, 15, 0, 0).unwrap();

    let candidates = [
        Candidate { id: "act-1", estimated_usd: 3.0, fired_at: inside },
        Candidate { id: "act-2", estimated_usd: 4.0, fired_at: inside },
        Candidate { id: "act-3", estimated_usd: 6.0, fired_at: inside },
        Candidate { id: "act-4", estimated_usd: 2.0, fired_at: outside },
    ];

    // ARGO_HOME-style isolated tmpdir — every test gets a fresh
    // sessions.db so no two parallel runs can collide.
    let home = tempfile::tempdir().expect("tmpdir for ARGO_HOME");
    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
    runtime.block_on(async move {
        let store = SessionStore::open(home.path().join("sessions.db"))
            .await
            .expect("open SessionStore");
        store.migrate().await.expect("migrate SessionStore");
        let pool = store.pool();

        let mut auto_approved: Vec<&'static str> = Vec::new();
        let mut asked: Vec<(&'static str, String)> = Vec::new();

        for c in candidates {
            // Today's persisted ledger drives the daily-cap projection.
            // Re-read for every iteration so cumulative spend reflects
            // the just-recorded charge from the previous loop.
            let today = spend_today_for_worker(pool, WORKER_ID, c.fired_at)
                .await
                .expect("spend_today_for_worker");
            let snap = SpendSnapshot {
                today_usd_micros: today,
                this_month_usd_micros: today,
            };
            let decision = evaluate(&policy, &pending_for(&c), snap, c.fired_at);
            match decision {
                Decision::AutoApprove { .. } => {
                    record_charge(
                        pool,
                        c.id,
                        WORKER_ID,
                        dollars_to_micros(c.estimated_usd),
                        c.fired_at,
                    )
                    .await
                    .expect("record_charge");
                    auto_approved.push(c.id);
                }
                Decision::Ask { reason, .. } => {
                    asked.push((c.id, reason));
                }
                Decision::Block { reason } => {
                    panic!("{} unexpectedly Blocked: {reason}", c.id);
                }
            }
        }

        assert_eq!(
            auto_approved,
            vec!["act-1", "act-2"],
            "in-window actions under the per-action cap auto-execute",
        );
        let ask_ids: Vec<&'static str> = asked.iter().map(|(id, _)| *id).collect();
        assert_eq!(
            ask_ids,
            vec!["act-3", "act-4"],
            "over-cap and out-of-window actions fall to Ask",
        );

        // Reason-tagging: a regression that misroutes either
        // candidate (e.g. window math drift, cap inversion) shows
        // up here with a usable diagnostic instead of a bare
        // `assert_eq!` mismatch.
        let act3_reason = &asked[0].1;
        assert!(
            act3_reason.contains("per_action_cap"),
            "act-3 should Ask for per-action cap, got: {act3_reason}",
        );
        let act4_reason = &asked[1].1;
        assert!(
            act4_reason.contains("outside_time_window"),
            "act-4 should Ask for outside-window, got: {act4_reason}",
        );

        // Spend reconciliation. `spend_today_for_worker` rolls every
        // persisted `worker_budget` row for the worker on the same
        // UTC calendar day as `inside`; act-1 + act-2 = $7.00.
        // act-3 hit the cap and act-4 fell outside the window, so
        // neither was charged.
        let total_today = spend_today_for_worker(pool, WORKER_ID, inside)
            .await
            .expect("final spend_today_for_worker");
        assert_eq!(
            total_today,
            dollars_to_micros(7.0),
            "ledger reconciles to $7.00 — only auto-executed actions consume budget",
        );

        // Cross-day isolation: querying on the Saturday "outside"
        // clock returns $0 because no charge was recorded that day.
        let total_outside = spend_today_for_worker(pool, WORKER_ID, outside)
            .await
            .expect("outside-day spend_today_for_worker");
        assert_eq!(
            total_outside, 0,
            "no charges land on the outside-window day",
        );
    });
}
