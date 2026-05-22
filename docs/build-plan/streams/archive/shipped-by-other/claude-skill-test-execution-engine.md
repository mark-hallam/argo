# claude-skill-test-execution-engine — eval-set runner before promotion

**Owner**: claude. **Depends on**: `propose_for_review_judged` in
`crates/argo-skill/src/store_vertical.rs` (#936); existing `SkillCandidate`
type in `crates/argo-skill/src/learn.rs`.

## Why

Promotion today gates on quality scores aggregated from real session traffic.
For brand-new candidates that signal is thin. If a skill author declares an
eval set in frontmatter (a fixture-and-expected-outcome list), we should
actually run it before letting the skill graduate from `pending_review`. The
evaluation result becomes another input to the trust signal — separate from
operator votes and from quality history.

## Scope

- New module `argo_skill::eval_runner` exposing:
  - `pub struct EvalCase { input: Value, expected: ExpectedOutcome }`.
  - `pub async fn run_evals(skill: &Skill, executor: &dyn EvalExecutor) -> ArgoResult<EvalReport>`.
  - `pub trait EvalExecutor { async fn run_one(&self, skill: &Skill, case: &EvalCase) -> ArgoResult<ActualOutcome>; }`.
- Frontmatter extension: `evals: [...]` array of cases (parser additive,
  empty/missing is fine — backward-compatible).
- Hook into `propose_for_review_judged`: when called with a non-empty
  `evals` set AND the caller passes an `EvalExecutor`, run them and attach
  `eval_pass_rate` to the resulting `SkillCandidate`.
- Pass-rate threshold (default 0.8) blocks promotion when below.

## Approach

1. **NEW** `crates/argo-skill/src/eval_runner.rs` (~320 LOC):
   - Trait + struct definitions.
   - `run_evals` iterates cases, collects outcomes, computes pass-rate.
   - `MockEvalExecutor` for tests (in `cfg(test)` block of same file).
2. Extend `SkillCandidate` with `eval_pass_rate: Option<f32>` — additive,
   no breaking change.
3. Modify `propose_for_review_judged` (`store_vertical.rs`) — narrowly: take
   an optional `&dyn EvalExecutor`, run evals when present, attach result.
   When trait not provided, behaviour is unchanged.
4. Frontmatter parser: add `evals` field as `Option<Vec<EvalCase>>` —
   serde-default empty.

## Files you may touch

- `crates/argo-skill/src/eval_runner.rs` (NEW, ≤ 360 LOC).
- `crates/argo-skill/src/lib.rs` — narrowly: 1 `pub mod eval_runner;` line.
- `crates/argo-skill/src/skill.rs` — narrowly: add `evals` field to `SkillFrontmatter`.
- `crates/argo-skill/src/learn.rs` — narrowly: add `eval_pass_rate` field to `SkillCandidate`.
- `crates/argo-skill/src/store_vertical.rs` — narrowly: extend `propose_for_review_judged` signature.
- `crates/argo-skill/src/tests.rs` — narrowly: existing test module, add eval-runner tests.

## Files you must NOT touch

- CLI surface — this brief is library-only. CLI integration lands in a follow-up.
- Migration files — no schema change; `eval_pass_rate` is in-memory.
- `crates/argo-skill/src/quality.rs` — orthogonal scoring pipeline.

## Acceptance criteria

- [ ] `run_evals` returns `EvalReport { pass: N, fail: M, rate: f32 }`.
- [ ] Skills with no evals → `eval_pass_rate = None`, no behavior change.
- [ ] `propose_for_review_judged` with evals + executor → pass-rate attached.
- [ ] Below-threshold (< 0.8 default) → candidate marked failed_eval.
- [ ] Frontmatter parsing of `evals` is additive — old skills still parse.
- [ ] 7+ tests: empty evals, all-pass, all-fail, mixed, threshold-edge, executor error, signature back-compat.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] Every authored file < 500 LOC.

## Branch + PR

- Branch: `claude-skill-test-execution-engine`
- PR title: `skill: eval-set runner for candidate gating (claude-skill-test-execution-engine)`

## Notes

- The trait is deliberately abstract — callers (CLI, runner, gateway) supply
  their own `EvalExecutor` implementation. This brief doesn't ship a real
  executor backed by the runtime; that's a follow-up.
- Threshold must be configurable in a follow-up; default 0.8 hard-coded here.
