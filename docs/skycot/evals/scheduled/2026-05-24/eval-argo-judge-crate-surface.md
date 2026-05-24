# `argo-judge` crate surface audit

**Date**: 2026-05-24  
**Brief**: `eval-argo-judge-crate-surface`  
**Mission link**: Claim 4 learned-skill review, claim 5/forge hardening, and judge-backed evals.  
**Verdict**: Surface is coherent and now pinned by active tests. No new `argo-judge` substrate fix-brief is needed.

## Public Surface Inventory

`crates/argo-judge/src/lib.rs` exposes 20 names:

| Slice | Re-exports |
|---|---|
| Disagreement eval | `load_fixtures`, `run_disagreement_eval`, `BucketBreakdown`, `CaseMismatch`, `DisagreementReport`, `DisagreementTag`, `JudgeDisagreementCase`, `DEFAULT_FIXTURES_DIR` |
| Evidence model | `GroundTruthCheck`, `JudgeEvidence`, `ProposedArtifact` |
| Judge seam | `verdict_from_scores`, `Judge`, `LlmJudge` |
| Persistence | `persist_judge_record` |
| Rubric | `Check`, `Rubric`, `VerdictThresholds`, `DEFAULT_RUBRIC_ID`, `DEFAULT_RUBRIC_VERSION` |

The new contract test at `crates/argo-judge/tests/surface_contract.rs` pins all 20 with a compile-fence and count sentinel.

## Test Coverage Added

`surface_contract.rs` adds active coverage for:

| Contract | Pin |
|---|---|
| Re-export inventory | Compile-fence imports plus `inventory.len() == 20`. |
| `Judge` trait shape | A `ScriptedJudge` impl proves `&dyn Judge` object-safety and `adjudicate(...) -> ArgoResult<JudgeRecord>`. |
| Verdict serde | `Success`, `Partial`, and `Failure` round-trip with stable tags. |
| Rubric shape | Default rubric ids, version, four checks, `0.8/0.5` thresholds, JSON/YAML round-trips. |
| Threshold behavior | Success scores allow, borderline `0.6` yields `Partial`, missing checks fail closed. |
| Evidence model | `JudgeEvidence` + `ProposedArtifact::SkillCreate` serialize, deserialize, and keep stable component hashes. |
| Fixture-backed path | All 30 real `fixtures/disagreement/*.json` cases run through `run_disagreement_eval` with a scripted judge; clear-allow, clear-block, and contentious buckets are present. |

This is intentionally no-network: all judge behavior is scripted in-process.

## Production Consumers

| Consumer | Symbols | Assessment |
|---|---|---|
| `argo-cli::handlers::judge_eval` | `LlmJudge`, `Rubric`, `load_fixtures`, `run_disagreement_eval`, `DisagreementReport`, `DEFAULT_FIXTURES_DIR` | Live CLI calibration path. Builds a provider transport and evaluates fixture sets. |
| `argo-cli::handlers::eval_skill_honesty` + `argo-eval::honesty` | `LlmJudge`, `Rubric`, `Judge`, `JudgeEvidence`, `ProposedArtifact` | Live diagnostic path. Judge errors intentionally fall through as agreement to avoid false suspicious verdicts. |
| `argo-skill::store_vertical` | `Judge`, `JudgeEvidence`, `ProposedArtifact` | Skill proposal judge gate is wired with fail-open semantics when a judge is supplied. Back-compat shim still permits `judge = None`. |
| `argo-skill::{per_worker_learning_cron,dreams_distillation}` | `Judge` | Threads optional judge into learned-skill proposal APIs. |
| `argo-executor-computer-use::learn` | `Judge` | Optional judged proposal API exists; current back-compat caller uses `None` unless a higher layer supplies a judge. |
| `argo-cli::handlers::forge_optimize*` | `Judge` | Gate exists, but production entry points still pass `judge: None`. This is the existing non-crate finding below. |

## Findings

No new finding in `argo-judge` itself.

Existing real production-wire finding remains: `forge_optimize::{run, run_with_events, run_claimed}` route to `run_with_events_and_run_id(..., judge: None)`, so the fail-closed forge judge gate is still unconsumed in production. This was already surfaced by `eval-forge-promote-fail-closed-judge-wire` and has the named follow-up `mission-forge-optimize-thread-real-judge`. That follow-up currently needs conductor rewrite or may-touch expansion because the required production files live under `crates/argo-cli/**`, outside the restored brief's allowed files.

## Follow-Up Briefs

- `mission-forge-optimize-thread-real-judge` — existing real finding; rewrite/expand may-touch before implementation.

## Verification

Required command:

```bash
cargo test -p argo-judge --tests
```

The PR body should report the final output from that command and the Codex pre-push gate.
