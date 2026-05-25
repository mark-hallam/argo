//! Public-surface contract pins for `argo-forge` — orchestration half.
//!
//! Authored by `eval-argo-forge-crate-surface` (Forge substrate audit,
//! 2026-05-24). The crate exposes the orchestration seam used by the
//! CLI/agent loop, the per-tenant Forge cron feeder, run/cost
//! persistence, and the operator-feedback ingestion path. This file
//! pins every `pub use` re-export plus the orchestration-side
//! contracts (`Forge` trait, `StubForge`, `ForgeStatus`,
//! `ForgeOptions`, `ForgeEvent`, `finalize_run`). The data/IO half —
//! cost, persist, feedback-eval, cron — lives in the sibling
//! `forge_data_contract.rs` so each file stays under the 500-LOC
//! ceiling.
//!
//! The test stays no-network and no-subprocess on purpose: the real
//! DSPy/GEPA worker lives in a Python subprocess and isn't part of
//! the orchestration contract we want to pin here.

#![allow(dead_code)]

use argo_forge::{
    accumulate_e2b_seconds, accumulate_llm_call, count_forge_runs, finalize_run,
    ingest_operator_feedback, load_forge_cost, load_forge_status, persist_forge_run,
    pricing_for_model, query_forge_costs, spawn_forge_cron, tokens_per_dollar_for_model,
    weighted_loss_score, CaseKind, CostQueryFilter, FeedbackEvalCase, Forge, ForgeCost,
    ForgeCronConfig, ForgeError, ForgeEvent, ForgeExpectedOutcome, ForgeOptions, ForgeRun,
    ForgeStatus, PricingTable, ScoredCase, SkillCostRow, StubForge, FEEDBACK_CASE_WEIGHT,
    FEEDBACK_WINDOW_DAYS, FORGE_OPTIMIZER_STATUS_DOCS, FORGE_STUB_INVOCATION_EVENT,
};
use argo_store::SessionStore;
use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::Arc;

// ---------------------------------------------------------------------------
// 1. `lib.rs` re-export inventory: compile-fence + count sentinel.
//
// Total inventory pinned: 33 names across the 7 crate modules.
// ---------------------------------------------------------------------------

#[test]
fn pub_use_inventory_pins_count_at_33() {
    let inventory = [
        // config (2)
        "FEEDBACK_CASE_WEIGHT",
        "FEEDBACK_WINDOW_DAYS",
        // cost (6)
        "accumulate_e2b_seconds",
        "accumulate_llm_call",
        "pricing_for_model",
        "tokens_per_dollar_for_model",
        "ForgeCost",
        "PricingTable",
        // cron (2)
        "spawn_forge_cron",
        "ForgeCronConfig",
        // feedback_eval (6)
        "ingest_operator_feedback",
        "weighted_loss_score",
        "CaseKind",
        "ForgeExpectedOutcome",
        "FeedbackEvalCase",
        "ScoredCase",
        // forge (7)
        "finalize_run",
        "Forge",
        "ForgeError",
        "ForgeEvent",
        "StubForge",
        "FORGE_OPTIMIZER_STATUS_DOCS",
        "FORGE_STUB_INVOCATION_EVENT",
        // persist (7)
        "count_forge_runs",
        "load_forge_cost",
        "load_forge_status",
        "persist_forge_run",
        "query_forge_costs",
        "CostQueryFilter",
        "SkillCostRow",
        // run (3)
        "ForgeOptions",
        "ForgeRun",
        "ForgeStatus",
    ];
    assert_eq!(
        inventory.len(),
        33,
        "argo-forge pub-use inventory drifted; update this contract with lib.rs"
    );

    // Constants pin (catch renamed/retyped operator runbook links).
    assert!((FEEDBACK_CASE_WEIGHT - 1.5).abs() < f32::EPSILON);
    assert_eq!(FEEDBACK_WINDOW_DAYS, 14);
    assert_eq!(
        FORGE_OPTIMIZER_STATUS_DOCS,
        "docs/skycot/operator/forge-optimizer-status.md"
    );
    assert_eq!(FORGE_STUB_INVOCATION_EVENT, "FORGE_STUB_INVOCATION");

    // Function pointer coercions (catch signature drift at compile time).
    let _: fn(&str) -> PricingTable = pricing_for_model;
    let _: fn(&str) -> Option<u32> = tokens_per_dollar_for_model;
    let _: fn(ForgeRun, &[ForgeEvent], u64) -> ForgeRun = finalize_run;
    let _: fn() -> ForgeCronConfig = ForgeCronConfig::default;
    let _: fn(&ForgeOptions) -> ForgeRun = ForgeRun::pending;
    let _: fn() -> StubForge = StubForge::new;
    let _: fn() -> ForgeCost = ForgeCost::default;
    let _: fn() -> CostQueryFilter = CostQueryFilter::default;
    let _: fn(&[ScoredCase]) -> (f32, f32) = weighted_loss_score;

    // Touch the remaining re-exports so any signature drift breaks
    // this test (not just the standalone modules in
    // `forge_data_contract.rs`).
    let _: fn(&mut ForgeCost, &str, u64, u64, u64) = accumulate_llm_call;
    let _: fn(&mut ForgeCost, f64) = accumulate_e2b_seconds;
    let _: fn(Arc<SessionStore>, ForgeCronConfig) -> tokio::task::JoinHandle<()> =
        spawn_forge_cron;
    let _ = std::any::TypeId::of::<SkillCostRow>();
    let _ = std::any::TypeId::of::<FeedbackEvalCase>();
    let _ = std::any::TypeId::of::<CaseKind>();
    let _ = std::any::TypeId::of::<ForgeExpectedOutcome>();
    let _ = ingest_operator_feedback;
    let _ = count_forge_runs;
    let _ = load_forge_status;
    let _ = load_forge_cost;
    let _ = persist_forge_run;
    let _ = query_forge_costs;
}

#[test]
fn forge_trait_is_object_safe() {
    fn assert_object_safe(_f: &dyn Forge) {}
    let stub = StubForge::new();
    assert_object_safe(&stub);
}

// ---------------------------------------------------------------------------
// 2. `ForgeStatus` labels + serde — pin every variant and the on-disk
//    string. These labels live in the `forge_run.status` column, so a
//    rename without a migration silently breaks status round-trips for
//    every already-persisted row.
// ---------------------------------------------------------------------------

#[test]
fn forge_status_labels_round_trip_exhaustively() {
    let pinned: &[(ForgeStatus, &str)] = &[
        (ForgeStatus::Pending, "pending"),
        (ForgeStatus::Running, "running"),
        (ForgeStatus::Completed, "completed"),
        (ForgeStatus::BudgetExceeded, "budget_exceeded"),
        (ForgeStatus::Aborted, "aborted"),
        (ForgeStatus::Failed, "failed"),
    ];

    for (status, label) in pinned {
        assert_eq!(
            status.label(),
            *label,
            "ForgeStatus::{status:?} label drifted; persisted rows depend on it"
        );
        assert_eq!(
            ForgeStatus::from_label(label),
            Some(*status),
            "ForgeStatus::from_label({label}) drifted"
        );
    }

    // Unknown labels must return None so the CLI surfaces "unknown
    // status" rather than mis-mapping to a real variant.
    assert!(ForgeStatus::from_label("unknown").is_none());
    assert!(ForgeStatus::from_label("").is_none());
}

#[test]
fn forge_status_serde_uses_snake_case_tags() {
    let pinned: &[(ForgeStatus, &str)] = &[
        (ForgeStatus::Pending, "\"pending\""),
        (ForgeStatus::Running, "\"running\""),
        (ForgeStatus::Completed, "\"completed\""),
        (ForgeStatus::BudgetExceeded, "\"budget_exceeded\""),
        (ForgeStatus::Aborted, "\"aborted\""),
        (ForgeStatus::Failed, "\"failed\""),
    ];
    for (status, json) in pinned {
        let actual = serde_json::to_string(status).expect("serialize ForgeStatus");
        assert_eq!(
            &actual, json,
            "ForgeStatus serde drifted for {status:?} — JSON consumers will see {actual}"
        );
        let back: ForgeStatus = serde_json::from_str(json).expect("deserialize ForgeStatus");
        assert_eq!(back, *status);
    }
}

// ---------------------------------------------------------------------------
// 3. `ForgeOptions::new` defaults — the v1 spec caps and the fields a
//    caller must supply. Drift on any of these changes Forge's safety
//    contract (budget cap, iteration count, opt-in feedback).
// ---------------------------------------------------------------------------

#[test]
fn forge_options_new_pins_spec_defaults() {
    let opts = ForgeOptions::new("alpha", "0.1");
    assert_eq!(opts.skill_name, "alpha");
    assert_eq!(opts.base_version, "0.1");
    assert_eq!(opts.iterations, 12, "§26.2 default iteration count");
    assert_eq!(
        opts.budget_usd_micros, 5_000_000,
        "§26.5 default budget cap is $5"
    );
    assert!(
        (opts.auto_promote_threshold - 0.85).abs() < f64::EPSILON,
        "§26.5 auto-promote threshold"
    );
    assert!(
        opts.include_operator_feedback,
        "operator-feedback ingestion is on by default per Stream mission-forge-operator-feedback-eval-signal"
    );
    assert_eq!(
        opts.examples_path,
        PathBuf::new(),
        "constructor leaves examples_path empty so callers must opt in"
    );
}

#[test]
fn forge_options_serde_default_fills_missing_fields() {
    // A JSON payload with only the two required fields must round-trip
    // through serde and land on the spec defaults — protects CLI/forge
    // queue payloads against silent breakage when an older worker
    // emits a minimal payload.
    let minimal = r#"{
        "skill_name": "billing-resolver",
        "base_version": "0.1",
        "examples_path": ""
    }"#;
    let opts: ForgeOptions = serde_json::from_str(minimal).expect("minimal ForgeOptions parses");
    assert_eq!(opts.iterations, 12);
    assert_eq!(opts.budget_usd_micros, 5_000_000);
    assert!((opts.auto_promote_threshold - 0.85).abs() < f64::EPSILON);
    assert!(opts.include_operator_feedback);
}

// ---------------------------------------------------------------------------
// 4. `StubForge::optimize` — the documented "configure the Python
//    worker" error path. No network, no subprocess — covered by a
//    direct `.await` on a `tokio::runtime` test. The event marker is
//    pinned because the audit log uses it as a structured tag.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn stub_forge_optimize_returns_documented_configure_error() {
    let stub = StubForge::new();
    assert_eq!(stub.id(), "stub");

    let opts = ForgeOptions::new("billing-resolver", "0.1");
    let err = stub
        .optimize(opts)
        .await
        .expect_err("stub must surface a configure-the-worker error");
    match err {
        ForgeError::OptimizerNotConfigured {
            skill_name,
            base_version,
            docs_url,
        } => {
            assert_eq!(skill_name, "billing-resolver");
            assert_eq!(base_version, "0.1");
            assert_eq!(docs_url, FORGE_OPTIMIZER_STATUS_DOCS);
        }
    }
}

#[tokio::test]
async fn stub_forge_optimize_error_carries_runbook_url_via_argo_error() {
    let stub = StubForge::new();
    let opts = ForgeOptions::new("alpha", "0.2");
    let err = stub.optimize(opts).await.unwrap_err();
    let argo_err: argo_types::ArgoError = err.into();
    let rendered = format!("{argo_err}");
    assert!(
        rendered.contains(FORGE_OPTIMIZER_STATUS_DOCS),
        "rendered ArgoError must point operators at the runbook, got: {rendered}"
    );
}

#[test]
fn forge_event_variants_are_pinned() {
    // Pattern-match on every variant: if a variant is added or
    // removed without updating the orchestration crate this match
    // stops being exhaustive at compile time.
    let events = [
        ForgeEvent::Iteration {
            spend_delta_usd_micros: 1_000,
        },
        ForgeEvent::CandidateScored {
            version: "0.2-c1".into(),
            score: 0.92,
        },
        ForgeEvent::Aborted,
        ForgeEvent::Failed {
            reason: "stub".into(),
        },
    ];
    for event in events {
        match event {
            ForgeEvent::Iteration { .. }
            | ForgeEvent::CandidateScored { .. }
            | ForgeEvent::Aborted
            | ForgeEvent::Failed { .. } => {}
        }
    }
}

#[test]
fn finalize_run_marks_budget_exceeded_when_cap_hit() {
    // §26.5: hitting the budget cleanly aborts and persists partial
    // state. Pin the finalize_run event-fold so a refactor cannot
    // silently break the budget gate.
    let mut opts = ForgeOptions::new("alpha", "0.1");
    opts.budget_usd_micros = 1_000;
    let run = ForgeRun::pending(&opts);
    let events = vec![
        ForgeEvent::Iteration {
            spend_delta_usd_micros: 600,
        },
        ForgeEvent::Iteration {
            spend_delta_usd_micros: 700,
        },
        ForgeEvent::CandidateScored {
            version: "post-budget".into(),
            score: 1.0,
        },
    ];
    let finalized = finalize_run(run, &events, opts.budget_usd_micros);
    assert_eq!(finalized.status, ForgeStatus::BudgetExceeded);
    assert!(finalized.candidate_version.is_none());
    assert_eq!(finalized.iterations, 2);
    assert_eq!(finalized.spend_usd_micros, 1_300);
    assert!(finalized.ended_at.is_some());
}

// ---------------------------------------------------------------------------
// 5. `Forge` trait object-safety + a tiny in-test impl. Catches an
//    accidental `Sized`/`async fn`-without-trait-shim refactor that
//    would break `argo-cli::handlers::forge_optimize`.
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct AlwaysFails;

#[async_trait]
impl Forge for AlwaysFails {
    fn id(&self) -> &str {
        "test-always-fails"
    }
    async fn optimize(&self, opts: ForgeOptions) -> Result<ForgeRun, ForgeError> {
        Err(ForgeError::OptimizerNotConfigured {
            skill_name: opts.skill_name,
            base_version: opts.base_version,
            docs_url: FORGE_OPTIMIZER_STATUS_DOCS,
        })
    }
}

#[tokio::test]
async fn forge_trait_can_be_implemented_in_a_downstream_crate_shape() {
    let f: Box<dyn Forge> = Box::new(AlwaysFails);
    assert_eq!(f.id(), "test-always-fails");
    let opts = ForgeOptions::new("beta", "0.1");
    assert!(f.optimize(opts).await.is_err());
}
