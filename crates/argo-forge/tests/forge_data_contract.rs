//! Public-surface contract pins for `argo-forge` — data/IO half.
//!
//! Sibling to `surface_contract.rs`; that file pins the orchestration
//! surface (`Forge` trait, `StubForge`, `ForgeStatus`, `ForgeOptions`,
//! `ForgeEvent`, `finalize_run`). This file pins:
//!
//! * cost accumulator math (`ForgeCost`, `accumulate_llm_call`,
//!   `accumulate_e2b_seconds`, `pricing_for_model`,
//!   `tokens_per_dollar_for_model`),
//! * `forge_run` persistence + cost rollup (`persist_forge_run`,
//!   `count_forge_runs`, `load_forge_status`, `load_forge_cost`,
//!   `query_forge_costs`, `SkillCostRow`, `CostQueryFilter`),
//! * feedback-eval surface (`FeedbackEvalCase`, `ScoredCase`,
//!   `weighted_loss_score`, `ingest_operator_feedback`,
//!   `CaseKind`, `ForgeExpectedOutcome`),
//! * cron-feeder defaults (`ForgeCronConfig`, `spawn_forge_cron`).
//!
//! Split out so each file stays under the 500-LOC per-file ceiling.
//! Authored by `eval-argo-forge-crate-surface` (Forge substrate audit,
//! 2026-05-24). No network, no subprocess — uses an in-memory SQLite
//! `SessionStore` so the tests run in `cargo test -p argo-forge` flat.

#![allow(dead_code)]

use argo_forge::{
    accumulate_e2b_seconds, accumulate_llm_call, count_forge_runs, ingest_operator_feedback,
    load_forge_cost, load_forge_status, persist_forge_run, pricing_for_model, query_forge_costs,
    tokens_per_dollar_for_model, weighted_loss_score, CaseKind, CostQueryFilter, FeedbackEvalCase,
    ForgeCost, ForgeCronConfig, ForgeExpectedOutcome, ForgeOptions, ForgeRun, ForgeStatus,
    ScoredCase, SkillCostRow, FEEDBACK_CASE_WEIGHT,
};
use argo_store::SessionStore;
use argo_types::ForgeRunId;
use std::time::Duration;

async fn fresh_store() -> SessionStore {
    let store = SessionStore::open(":memory:").await.expect("open store");
    store.migrate().await.expect("migrate store");
    store
}

// ---------------------------------------------------------------------------
// 1. Cost accumulation — at least one LLM call and one E2B-second
//    path. Pin the rate maths so operators can trust `forge_run.cost`
//    on the persisted row.
// ---------------------------------------------------------------------------

#[test]
fn cost_accumulator_pins_llm_call_and_e2b_paths() {
    let mut cost = ForgeCost::default();
    // LLM call: Sonnet input 1000 tokens + output 500 + cache_read 0.
    accumulate_llm_call(&mut cost, "claude-sonnet-4-6", 1_000, 500, 0);
    assert_eq!(cost.llm_input_tokens, 1_000);
    assert_eq!(cost.llm_output_tokens, 500);
    assert_eq!(cost.llm_cache_read_tokens, 0);
    // $3/MTok input × 1000 = 3000 USD-micros;
    // $15/MTok output × 500 = 7500 USD-micros; total 10_500.
    assert_eq!(cost.llm_cost_usd_micros, 10_500);
    assert_eq!(cost.total_cost_usd_micros, 10_500);

    // E2B path: 2.4 seconds rounds UP to 3 whole seconds.
    accumulate_e2b_seconds(&mut cost, 2.4);
    assert_eq!(cost.e2b_seconds, 3);
    // Sandbox rate is 290 USD-micros/second (2026-05-11 snapshot).
    assert_eq!(cost.e2b_cost_usd_micros, 3 * 290);
    assert_eq!(
        cost.total_cost_usd_micros,
        cost.llm_cost_usd_micros + cost.e2b_cost_usd_micros
    );
}

#[test]
fn pricing_for_known_models_matches_2026_05_11_snapshot() {
    let sonnet = pricing_for_model("claude-sonnet-4-6");
    assert!((sonnet.input_usd_per_mtok - 3.0).abs() < f64::EPSILON);
    assert!((sonnet.output_usd_per_mtok - 15.0).abs() < f64::EPSILON);
    assert!((sonnet.cache_read_usd_per_mtok - 0.30).abs() < f64::EPSILON);

    let opus = pricing_for_model("claude-opus-4-7");
    assert!((opus.input_usd_per_mtok - 15.0).abs() < f64::EPSILON);
    assert!((opus.output_usd_per_mtok - 75.0).abs() < f64::EPSILON);
    assert!((opus.cache_read_usd_per_mtok - 1.5).abs() < f64::EPSILON);

    // Unknown model id falls through to Sonnet-4 rates so a typo is
    // loud, not silently zero.
    let unknown = pricing_for_model("not-a-real-model-2099");
    assert!((unknown.input_usd_per_mtok - 3.0).abs() < f64::EPSILON);

    // Conservative tokens-per-dollar uses the most expensive class.
    assert_eq!(tokens_per_dollar_for_model("claude-opus-4-7"), Some(13_333));
    assert_eq!(tokens_per_dollar_for_model("not-a-real-model"), None);
}

// ---------------------------------------------------------------------------
// 2. Feedback-eval surface — case weighting, scoring discriminants,
//    and the vote→outcome map. These pin the schema and the loss-
//    function math so the eval signal stays comparable across runs.
// ---------------------------------------------------------------------------

#[test]
fn feedback_expected_outcome_serde_uses_snake_case() {
    for (variant, label) in [
        (ForgeExpectedOutcome::Pass, "\"pass\""),
        (ForgeExpectedOutcome::Fail, "\"fail\""),
    ] {
        let actual = serde_json::to_string(&variant).expect("serialize");
        assert_eq!(actual, label);
        let back: ForgeExpectedOutcome = serde_json::from_str(label).expect("deserialize");
        assert_eq!(back, variant);
    }
}

#[test]
fn case_kind_serde_uses_snake_case() {
    let synth = serde_json::to_string(&CaseKind::Synth).unwrap();
    assert_eq!(synth, "\"synth\"");
    let op = serde_json::to_string(&CaseKind::OperatorFeedback).unwrap();
    assert_eq!(op, "\"operator_feedback\"");

    let back: CaseKind = serde_json::from_str("\"operator_feedback\"").unwrap();
    assert_eq!(back, CaseKind::OperatorFeedback);
}

#[test]
fn feedback_eval_case_pins_source_tag() {
    // The "operator_feedback" tag is the provenance JSON consumers
    // filter on. Pinning it here catches accidental rename.
    let case = FeedbackEvalCase::new(
        "act-1".into(),
        "operator notes".into(),
        ForgeExpectedOutcome::Pass,
    );
    assert_eq!(case.invocation_id, "act-1");
    assert_eq!(case.action_summary, "operator notes");
    assert_eq!(case.expected, ForgeExpectedOutcome::Pass);
    assert_eq!(case.source, "operator_feedback");

    let json = serde_json::to_string(&case).expect("serialize FeedbackEvalCase");
    let back: FeedbackEvalCase = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back, case);
    assert!(json.contains("\"source\":\"operator_feedback\""));
    assert!(json.contains("\"expected\":\"pass\""));
}

#[test]
fn scored_case_weight_uses_feedback_constant_for_operator_cases() {
    let synth = ScoredCase {
        kind: CaseKind::Synth,
        score: 1.0,
    };
    let operator = ScoredCase {
        kind: CaseKind::OperatorFeedback,
        score: 1.0,
    };
    assert!((synth.weight() - 1.0).abs() < f32::EPSILON);
    assert!((operator.weight() - FEEDBACK_CASE_WEIGHT).abs() < f32::EPSILON);
}

#[test]
fn weighted_loss_score_blends_synth_and_operator_cases() {
    // Two cases: synth scored 1.0 (weight 1.0), operator-feedback
    // scored 0.0 (weight 1.5). Weighted sum = 1.0; total weight = 2.5;
    // normalized score should be 0.4.
    let scores = [
        ScoredCase {
            kind: CaseKind::Synth,
            score: 1.0,
        },
        ScoredCase {
            kind: CaseKind::OperatorFeedback,
            score: 0.0,
        },
    ];
    let (weighted, total) = weighted_loss_score(&scores);
    assert!((weighted - 1.0).abs() < f32::EPSILON);
    assert!((total - 2.5).abs() < f32::EPSILON);
    assert!(
        ((weighted / total) - 0.4).abs() < 1e-6,
        "operator cases must out-weight synth cases by FEEDBACK_CASE_WEIGHT"
    );
}

#[test]
fn forge_expected_outcome_from_vote_maps_up_and_down() {
    use argo_store::feedback::Vote;
    assert_eq!(
        ForgeExpectedOutcome::from_vote(Vote::Up),
        ForgeExpectedOutcome::Pass
    );
    assert_eq!(
        ForgeExpectedOutcome::from_vote(Vote::Down),
        ForgeExpectedOutcome::Fail
    );
}

// ---------------------------------------------------------------------------
// 3. Persist surface — pin the upsert + read-back round-trip for a
//    pending run, plus the SkillCostRow / CostQueryFilter shapes.
//    Keeps the persisted column inventory and the cost-report read
//    path honest.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn persist_and_load_status_round_trip_a_pending_run() {
    let store = fresh_store().await;
    let opts = ForgeOptions::new("alpha", "0.1");
    let mut run = ForgeRun::pending(&opts);
    assert_eq!(run.status, ForgeStatus::Pending);

    persist_forge_run(&store, &run).await.expect("persist run");
    assert_eq!(count_forge_runs(&store).await.expect("count rows"), 1);
    assert_eq!(
        load_forge_status(&store, run.id)
            .await
            .expect("load status"),
        Some(ForgeStatus::Pending)
    );

    // Bump status to Completed; upsert keeps a single row.
    run.status = ForgeStatus::Completed;
    run.final_score = Some(0.91);
    persist_forge_run(&store, &run).await.expect("persist again");
    assert_eq!(count_forge_runs(&store).await.expect("count rows"), 1);
    assert_eq!(
        load_forge_status(&store, run.id)
            .await
            .expect("load status"),
        Some(ForgeStatus::Completed)
    );
}

#[tokio::test]
async fn persist_round_trip_preserves_cost_columns() {
    let store = fresh_store().await;
    let opts = ForgeOptions::new("alpha", "0.1");
    let mut run = ForgeRun::pending(&opts);
    run.accumulate_llm_call("claude-sonnet-4-6", 1_000, 500, 100);
    run.accumulate_e2b_seconds(2.0);
    persist_forge_run(&store, &run).await.expect("persist run");

    let loaded = load_forge_cost(&store, run.id)
        .await
        .expect("load cost")
        .expect("row exists");
    assert_eq!(loaded.llm_input_tokens, 1_000);
    assert_eq!(loaded.llm_output_tokens, 500);
    assert_eq!(loaded.llm_cache_read_tokens, 100);
    assert_eq!(loaded.e2b_seconds, 2);
    assert!(loaded.total_cost_usd_micros > 0);

    // Missing id returns None.
    let absent = load_forge_cost(&store, ForgeRunId::new())
        .await
        .expect("load cost");
    assert!(absent.is_none());
}

#[tokio::test]
async fn query_forge_costs_pins_skillcostrow_shape() {
    let store = fresh_store().await;
    let opts = ForgeOptions::new("alpha", "0.1");
    let mut run = ForgeRun::pending(&opts);
    run.accumulate_llm_call("claude-sonnet-4-6", 1_000, 500, 0);
    persist_forge_run(&store, &run).await.expect("persist run");

    let filter = CostQueryFilter {
        skill: Some("alpha".into()),
        since: None,
    };
    let rows = query_forge_costs(&store, &filter)
        .await
        .expect("query costs");
    assert_eq!(rows.len(), 1, "single skill must aggregate to one row");
    let row: &SkillCostRow = &rows[0];
    assert_eq!(row.skill_name, "alpha");
    assert_eq!(row.run_count, 1);
    assert!(row.total_cost_usd_micros > 0);
    assert!(row.llm_input_tokens > 0);
    assert_eq!(row.mean_cost_usd_micros, row.total_cost_usd_micros);

    // Empty CostQueryFilter must also return the same row (no filter
    // pushed down). Pin the Default impl while we're here.
    let all = query_forge_costs(&store, &CostQueryFilter::default())
        .await
        .expect("query all");
    assert_eq!(all.len(), 1);
}

// ---------------------------------------------------------------------------
// 4. Cron surface — pin defaults without starting the spawn loop
//    (tokio task lifetime is out of scope here).
// ---------------------------------------------------------------------------

#[test]
fn forge_cron_config_default_matches_pinned_v1_caps() {
    let cfg = ForgeCronConfig::default();
    assert_eq!(cfg.max_skills_per_tick, 3);
    assert!((cfg.score_threshold - 70.0).abs() < f64::EPSILON);
    assert_eq!(cfg.interval_secs, 3_600);
    assert!(!cfg.dry_run);
    assert!((cfg.budget_usd - 0.50).abs() < f64::EPSILON);
}

#[tokio::test]
async fn ingest_operator_feedback_signature_pins_window_duration() {
    // Pin the (store, skill_name, window) -> ArgoResult<Vec<_>> shape
    // by calling it against an empty store — graceful empty result.
    let store = fresh_store().await;
    let cases = ingest_operator_feedback(&store, "nonexistent-skill", Duration::from_secs(60))
        .await
        .expect("empty result is graceful");
    assert!(cases.is_empty());
}
