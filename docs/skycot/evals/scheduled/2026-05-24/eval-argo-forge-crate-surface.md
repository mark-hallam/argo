# `argo-forge` crate surface audit

**Date**: 2026-05-24
**Brief**: `eval-argo-forge-crate-surface`
**Mission link**: Claim 4 (skills shared + learned) — Forge optimizer
orchestration is the substrate that lets a learned skill candidate
re-enter the loop with a score, cost, and verdict. Pinning this
surface protects every downstream caller (`argo-cli::handlers::forge_*`,
the per-tenant Forge cron feeder, the operator cost-report path,
the queued `forge_run` row, and the executor-side
`tokens_per_dollar_for_model` budget projection).
**Verdict**: Surface is coherent and now pinned by active tests. No
new `argo-forge` substrate fix-brief is needed.

## Public Surface Inventory

`crates/argo-forge/src/lib.rs` exposes **33 names across 7 modules**:

| Slice | Re-exports |
|---|---|
| `config` (2) | `FEEDBACK_CASE_WEIGHT`, `FEEDBACK_WINDOW_DAYS` |
| `cost` (6) | `accumulate_e2b_seconds`, `accumulate_llm_call`, `pricing_for_model`, `tokens_per_dollar_for_model`, `ForgeCost`, `PricingTable` |
| `cron` (2) | `spawn_forge_cron`, `ForgeCronConfig` |
| `feedback_eval` (6) | `ingest_operator_feedback`, `weighted_loss_score`, `CaseKind`, `ForgeExpectedOutcome`, `FeedbackEvalCase`, `ScoredCase` |
| `forge` (7) | `finalize_run`, `Forge`, `ForgeError`, `ForgeEvent`, `StubForge`, `FORGE_OPTIMIZER_STATUS_DOCS`, `FORGE_STUB_INVOCATION_EVENT` |
| `persist` (7) | `count_forge_runs`, `load_forge_cost`, `load_forge_status`, `persist_forge_run`, `query_forge_costs`, `CostQueryFilter`, `SkillCostRow` |
| `run` (3) | `ForgeOptions`, `ForgeRun`, `ForgeStatus` |

The new contract tests at
`crates/argo-forge/tests/surface_contract.rs` (orchestration half) and
`crates/argo-forge/tests/forge_data_contract.rs` (data / IO half) pin
all 33 with a compile-fence inventory plus an `inventory.len() == 33`
sentinel. The file is split because a single test module exceeded the
500-LOC per-file invariant; both files now sit at ~370 and ~310 LOC.

## Test Coverage Added

`surface_contract.rs` (orchestration) adds active coverage for:

| Contract | Pin |
|---|---|
| Re-export inventory | Compile-fence imports plus `inventory.len() == 33` plus function-pointer coercions across `pricing_for_model`, `tokens_per_dollar_for_model`, `finalize_run`, `accumulate_llm_call`, `accumulate_e2b_seconds`, `spawn_forge_cron`, `weighted_loss_score`, `ForgeRun::pending`, `StubForge::new`. |
| `Forge` trait | Object-safety assertion + a `Box<dyn Forge>` in-test impl that proves `&dyn Forge` works for downstream crates. |
| `ForgeStatus` labels | Every variant round-trips through `.label()` + `from_label()`; unknown labels yield `None`. |
| `ForgeStatus` serde | All six variants serialize as the documented snake-case JSON string (`"pending"`, `"running"`, `"completed"`, `"budget_exceeded"`, `"aborted"`, `"failed"`). |
| `ForgeOptions::new` defaults | §26.2 iterations = 12, §26.5 budget cap = $5, auto_promote threshold = 0.85, `include_operator_feedback` = true, empty `examples_path` so callers must opt in. |
| `ForgeOptions` minimal serde | A 2-required-field JSON payload (`skill_name`, `base_version`, empty `examples_path`) lands on the spec defaults via `#[serde(default = …)]`. |
| `StubForge::optimize` | Returns `ForgeError::OptimizerNotConfigured { skill_name, base_version, docs_url }` with `docs_url == FORGE_OPTIMIZER_STATUS_DOCS`. No network, no subprocess. |
| `ForgeError` → `ArgoError` | `From<ForgeError>` for `argo_types::ArgoError` carries the runbook URL into the rendered message. |
| `ForgeEvent` variants | Exhaustive match across `Iteration`, `CandidateScored`, `Aborted`, `Failed` so a variant add/rename fails compile. |
| `finalize_run` budget gate | §26.5 "hitting the budget cleanly aborts and persists partial state" — events after the cap are dropped, status flips to `BudgetExceeded`, `ended_at` is stamped, `candidate_version` stays `None`. |

`forge_data_contract.rs` (data/IO) adds active coverage for:

| Contract | Pin |
|---|---|
| `ForgeCost` accumulator | Pinned Sonnet rate maths for one LLM call (1000 input + 500 output = 10_500 USD-micros) and the E2B path (2.4s → 3 whole seconds × 290 USD-micros/s). `total_cost_usd_micros` = `llm + e2b` post-accumulation. |
| `pricing_for_model` snapshot | Sonnet ($3/$15/$0.30) and Opus ($15/$75/$1.50) match the 2026-05-11 snapshot; unknown model ids fall through to Sonnet rates (loud not silent). |
| `tokens_per_dollar_for_model` | Pinned Opus value (13_333 tokens/$) and the `None` return for registry miss. |
| `ForgeExpectedOutcome` serde | `Pass` / `Fail` use `snake_case` JSON. |
| `CaseKind` serde | `Synth` / `OperatorFeedback` use `snake_case` JSON. |
| `FeedbackEvalCase` constructor | `new` stamps `source = "operator_feedback"`; serde round-trip preserves both `source` and `expected` tags. |
| `ScoredCase::weight` | `Synth` → 1.0, `OperatorFeedback` → `FEEDBACK_CASE_WEIGHT` (1.5×). |
| `weighted_loss_score` | Two-case fold (`{Synth, 1.0}`, `{OperatorFeedback, 0.0}`) yields weighted=1.0 / total=2.5 / normalized=0.4. |
| `ForgeExpectedOutcome::from_vote` | `Vote::Up` → `Pass`, `Vote::Down` → `Fail`. |
| `persist_forge_run` round-trip | Insert + bump-to-Completed upserts a single row; `load_forge_status` reflects the latest status. |
| Cost columns persistence | `persist_forge_run` round-trips `llm_input_tokens`, `llm_output_tokens`, `llm_cache_read_tokens`, `e2b_seconds`, `total_cost_usd_micros`; missing id returns `None` from `load_forge_cost`. |
| `query_forge_costs` shape | One-row aggregate against a filter (`skill = "alpha"`); `mean_cost_usd_micros == total_cost_usd_micros` for `run_count == 1`; `CostQueryFilter::default()` returns the same row (no filter pushed down). |
| `ForgeCronConfig::default` | Pinned v1 caps: 3 skills/tick, 70.0 quality-score threshold, 3600s interval, dry_run=false, $0.50 budget. |
| `ingest_operator_feedback` shape | `(store, skill_name, Duration)` signature; empty store yields an empty `Vec`, not an error. |

This is intentionally no-network and no-subprocess: `SessionStore::open(":memory:")` runs the persist/cost paths
fully in-process, and `StubForge::optimize` never touches the
Python worker plugin path.

## Production Consumers

| Consumer | Symbols | Assessment |
|---|---|---|
| `argo-cli::handlers::forge_cost_report` | `query_forge_costs`, `CostQueryFilter`, `SkillCostRow` | Live operator path. Reads cost rollups grouped by `skill_name`. |
| `argo-cli::handlers::gateway::boot_serve` | `spawn_forge_cron`, `ForgeCronConfig` | Live: boots the per-tenant Forge cron feeder when the gateway starts. |
| `argo-cli::tests::forge_cost_smoke` | `ForgeOptions`, `ForgeRun`, `accumulate_llm_call`, etc. | Smoke test for the end-to-end cost-report path. |
| `argo-cli::tests::forge_cron_smoke` | `spawn_forge_cron`, `ForgeCronConfig` | Cron-feeder smoke test. |
| `argo-executor-computer-use::executor` | `tokens_per_dollar_for_model` | Live: budget projection on each computer-use step. |
| `argo-trust-engine::cron` | Doc-link reference only | Mirrors the cron shape but does not import the symbols. |
| `argo-store::skill_flag` + `migrations/59__skill_flag.sql` | `FLAG_NO_FORGE_FEEDBACK` (string) | The `no-forge-feedback` per-skill opt-out flag that `ingest_operator_feedback` reads. |

## Findings

No new finding in `argo-forge` itself. The crate's exported surface is
stable, the orchestration seam is honest (the stub error carries the
documented runbook URL), the cost math is reproducible, and the
persisted column inventory matches the `ForgeCost` struct shape.

Pre-existing finding from the sibling judge audit
(`eval-argo-judge-crate-surface`, 2026-05-24) remains unrelated to this
audit's may-touch list: `argo-cli::handlers::forge_optimize::{run,
run_with_events, run_claimed}` still call
`run_with_events_and_run_id(…, judge: None)`, so the fail-closed
forge judge gate is unconsumed in production. That finding has a
named follow-up brief (`mission-forge-optimize-thread-real-judge`)
which is currently BLOCKED pending may-touch widening.

## Follow-Up Briefs

None from this audit. The pre-existing follow-up
(`mission-forge-optimize-thread-real-judge`) is tracked by the
`eval-argo-judge-crate-surface` audit and does not need a new
copy here.

## Verification

Required command:

```bash
cargo test -p argo-forge --tests
```

The PR body reports the final output from that command plus the
cross-crate drift check
(`bash scripts/workspace-clippy-fast-fail.sh`).
