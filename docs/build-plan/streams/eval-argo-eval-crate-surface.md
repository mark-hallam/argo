# eval-argo-eval-crate-surface

**Owner**: claude
**Priority**: P2 - substrate audit
**Authored against**: origin/main `50a3ea9f0` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh eval-argo-eval-crate-surface`.

## Source

`docs/skycot/evals/scheduled/LATEST.md` and the mission-vs-code gap manifest still list several eval substrate crates that need surface-contract audits. `argo-eval` is the shared eval harness runtime for Forge bench, regression decisions, honesty sampling, and self-healing fixture proposals.

## Goal

Audit and pin the public surface of `argo-eval` so future refactors cannot silently change scoring, regression, honesty, streaming, or self-healing contracts.

## May Touch

- `crates/argo-eval/tests/surface_contract.rs` (NEW)
- `crates/argo-eval/tests/` (NEW sibling tests if one file would exceed 500 LOC)
- `docs/skycot/evals/scheduled/2026-05-24/eval-argo-eval-crate-surface.md` (NEW report)
- `docs/build-plan/BUILD_STATUS.md`

## Must Not Touch

- Do not change production `argo-eval/src/**` code unless a contract test exposes an obvious compile-only drift and the fix is tiny.
- Do not edit Forge code.
- Do not add network or LLM calls.

## Acceptance

- [ ] Pin the exported type/function inventory from `crates/argo-eval/src/lib.rs`.
- [ ] Cover `EvalMetric` scoring for exact match, substring contains, and Jaccard token behavior including at least one negative case.
- [ ] Cover `run_eval` success and subject-error behavior, including summary counts and mean score.
- [ ] Cover regression promotion decision thresholds for allowed and blocked cases.
- [ ] Cover at least one honesty verdict serde shape.
- [ ] Cover streaming line format stability for simple, verbose, and summary modes if exposed.
- [ ] Write an eval report with verdict, any gaps found, and follow-up brief names only for real findings.
- [ ] `cargo test -p argo-eval --tests` passes, or the PR body records any unrelated clean-main failure with a focused green command.

## Notes

This is an audit/pin brief. Findings-only ignored tests are acceptable only when paired with an explicit follow-up row recommendation.
