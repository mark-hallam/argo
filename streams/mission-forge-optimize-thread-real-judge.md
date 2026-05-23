# mission-forge-optimize-thread-real-judge

**Authored against:** `origin/main` 2af54930f on 2026-05-23 (conductor tick-110).
**Validate before claim:** `bash scripts/check-brief-freshness.sh mission-forge-optimize-thread-real-judge`
**Owner:** claude
**Priority:** 🟡 P1 — forge fail-CLOSED judge gate wired-yet-unconsumed in production
**Source:** PR #2129 finding (eval-forge-promote-fail-closed-judge-wire), §FINDING.

## The problem

The fail-CLOSED judge gate landed via PR #940 — `argo-judge::LlmJudge` + the rubric thresholds (0.8/0.5) are in place. But `forge_optimize::run_with_events_and_run_id` accepts an `Option<Judge>` and **all three production callers pass `None`**:

- `crates/argo-cli/src/handlers/forge_optimize_loop.rs`
- `crates/argo-cli/src/handlers/forge_optimize_judge.rs`
- `crates/argo-cli/src/handlers/gateway/runs_streaming.rs` (calls `forge_optimize::run_with_events`, the sibling)

Effect: skill-refinement runs go through forge without judging. The substrate is right; the wire is missing.

## What to do

1. Add a helper `fn build_default_judge() -> Box<dyn Judge>` (or similar — match the trait shape already in `argo-judge`) that constructs `LlmJudge::with_default_rubric` using the existing `argo-provider-router` for transport.
2. Env-gate the wire-up: `ARGO_FORGE_JUDGE=on` returns `Some(judge)`, anything else (default) returns `None`. v1.x rollout discipline — opt-in until we trust it in prod.
3. Update all three callers to call the helper and thread the result.
4. Add an integration test that flips the env var and verifies the judge is invoked once per forge run (use a mock `Judge` impl that increments a counter).

## May-touch

- `crates/argo-cli/src/handlers/forge_optimize_loop.rs`
- `crates/argo-cli/src/handlers/forge_optimize_judge.rs`
- `crates/argo-cli/src/handlers/gateway/runs_streaming.rs`
- One new helper module — prefer extending an existing `argo-cli` module to introducing a new crate.
- `crates/argo-cli/tests/forge_judge_wire_test.rs` (new)

## Acceptance

- All 3 callers construct `LlmJudge` when `ARGO_FORGE_JUDGE=on`; `None` otherwise.
- New integration test pins the wire (env on → judge invoked; env off → not invoked).
- Existing PR #2129's `#[ignore]`'d FINDINGS-ONLY test (the one naming this brief) flipped to live and passes.
- Per-crate + workspace clippy clean.
- No new `pub` types — helper module is `pub(crate)`.

## Citations

- PR #2129 body §FINDING (mission-forge-optimize-thread-real-judge)
- `crates/argo-cli/src/handlers/forge_optimize_loop.rs`
- `crates/argo-cli/src/handlers/forge_optimize_judge.rs`
- `crates/argo-cli/src/handlers/gateway/runs_streaming.rs:272`

## LOC budget

~300-500 LOC.
