# mission-forge-optimize-thread-real-judge

**Owner**: codex
**Priority**: P1 - broken-wire closure
**Authored against**: origin/main `7354f039d` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-forge-optimize-thread-real-judge`.

## Source

The forge promote fail-closed audit found that the judge substrate exists, but
production callers of `forge_optimize::run_with_events_and_run_id` pass
`judge: None`. This makes the fail-closed judge contract real in tests but not
live in the optimize thread.

## Goal

Wire the default judge into the forge optimize thread behind an explicit operator
gate so production can fail closed when the judge is enabled.

## May Touch

- `crates/argo-forge/src/**`
- `crates/argo-judge/src/**` only for existing constructor reuse or a tiny additive helper
- `crates/argo-forge/tests/**` (new or existing focused tests)
- `docs/build-plan/BUILD_STATUS.md`

## Must Not Touch

- Do not change unrelated forge cost/persistence behavior.
- Do not make live LLM calls in tests.
- Do not enable judge-by-default unless existing config already demands it.

## Acceptance

- [ ] `ARGO_FORGE_JUDGE=on` or the existing equivalent config path wires a real judge into optimize.
- [ ] Judge-disabled behavior remains unchanged.
- [ ] A fail-closed test proves adverse/missing judge evidence prevents promotion.
- [ ] Any existing FINDINGS-ONLY test from the audit is unignored or replaced by an active equivalent.
- [ ] Crate-scoped verification claimed in the PR body with any unrelated clean-main failures clearly separated.

## Notes

Prefer the smallest config seam that makes the production caller honest.
