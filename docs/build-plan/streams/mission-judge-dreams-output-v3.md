# mission-judge-dreams-output-v3 — switch dreams_distillation to existing propose_for_review_judged

**Owner**: claude. **Depends on**: shipped `argo-skill::store_vertical_judge::{JudgeOutcome, classify_verdict}` + `SkillStore::propose_for_review_judged`.
**Authored against**: origin/main `94370ca5` on `2026-05-13`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-judge-dreams-output-v3`.

> **REWRITE 2026-05-13 by conductor (CEO-authorized after audit-3 INTENT-DRIFT finding).** V2 proposed a parallel `JudgeGate<T>` enum + `gate_with_verdict` helper, oblivious to the already-shipped `JudgeOutcome::{Allow, Reject, Unavailable}` + `classify_verdict(&JudgeVerdict, floor)` triple at `crates/argo-skill/src/store_vertical_judge.rs:14-99` and the consumer `SkillStore::propose_for_review_judged`. V3 ships ~150 LOC: reuse existing primitive, switch the dreams_distillation call site, optionally promote the primitive's visibility for argo-runtime / argo-memory.

## Scope

The judge gate is **already implemented and consumed** for skill-creation (#68). It just isn't called from Dreams output. Two changes:

1. **Switch `crates/argo-skill/src/dreams_distillation.rs:191`** from `propose_for_review` → `propose_for_review_judged` (or `_judged_with_evals` if appropriate). The function signatures + the `MockJudge` test pattern are documented in `store_vertical_eval_tests.rs`. ≤20 modified lines.

2. **(Optional, only if argo-runtime / argo-memory Dreams also need it)** Promote `JudgeOutcome` + `classify_verdict` to `pub` from `pub(crate)` so the other Dreams hosts can `use argo_skill::store_vertical_judge::*`. ≤5 modified lines.

3. **Audit emission**: the existing `propose_for_review_judged` already emits the right audit row on Reject. No new audit-kind needed.

**Not in scope**: new `JudgeGate<T>` enum (V2's antipattern); new `gate_with_verdict` helper (use `classify_verdict`); changing the existing `JudgeOutcome::{Allow, Reject, Unavailable}` 3-bucket shape; UI "Borderline" rendering (that's an operator-feedback layer concern, can be derived from `JudgeVerdict::Partial { reason }` reaching the approval queue).

## Files you may touch

- `crates/argo-skill/src/dreams_distillation.rs` — narrowly: line 191 call-site swap. ≤20 modified lines.
- `crates/argo-skill/src/store_vertical_judge.rs` — narrowly: promote `JudgeOutcome` + `classify_verdict` to `pub` if not already. ≤5 modified lines.
- `crates/argo-skill/src/lib.rs` — narrowly: re-export if needed. ≤2 modified lines.
- `crates/argo-skill/src/dreams_distillation_tests.rs` (or new sibling) — 3 unit tests: Allow case passes; Reject case logs + drops; Unavailable case logs warn + passes (existing audit-row pattern).

## Files you must NOT touch

- `argo-runtime::dreams_plan_templates` or `argo-memory::dreams_backend` — those are out-of-scope for this brief. V3 is argo-skill-only. A follow-up brief can extend if needed.
- The existing `propose_for_review_judged` API — locked.
- `classify_verdict` semantics — locked.
- `JudgeVerdict` enum — locked.

## Acceptance criteria

- [ ] `dreams_distillation.rs:191` calls `propose_for_review_judged` (or `_with_evals` if appropriate).
- [ ] Reject outcome: skill draft is dropped + audit row emitted (existing pattern).
- [ ] Allow outcome: skill draft proceeds to approval queue (existing behavior).
- [ ] Unavailable outcome: drops with `tracing::warn` + audit row (existing pattern).
- [ ] 3 unit tests against `MockJudge`.
- [ ] No new `JudgeGate<T>` enum. No new `gate_with_verdict` helper.
- [ ] `cargo clippy -p argo-skill --all-targets -- -D warnings` clean.
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-mission-judge-dreams-output-v3`
- PR title: `dreams: route distillation through propose_for_review_judged (mission-judge-dreams-output-v3)`

## Notes for the implementer

- This brief is a ~150-LOC delta. V2 was ~450 LOC because it duplicated existing infrastructure. V3 reuses.
- Per CLAUDE.md "subtract before adding" — the existing primitive IS the answer; the brief's job is to wire it in.
- If you find argo-runtime/argo-memory Dreams also need this, that's a separate follow-up brief (`mission-judge-dreams-output-v3-runtime-memory`).
