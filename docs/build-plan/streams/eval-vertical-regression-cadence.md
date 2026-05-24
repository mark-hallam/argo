# eval-vertical-regression-cadence

**Owner**: codex
**Priority**: P3 - regression substrate audit
**Authored against**: origin/main `7354f039d` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh eval-vertical-regression-cadence`.

## Source

The mission-vs-code gap manifest lists `argo-vertical-regression` as not audited.
This crate should explain whether vertical regression checks are a live guard,
manual tool, or unused substrate.

## Goal

Audit `argo-vertical-regression` end to end, pin its current public contract, and
document whether it is actually connected to CI, pre-commit, Genesis, or operator
runbooks.

## May Touch

- `crates/argo-vertical-regression/tests/cadence_contract.rs` (NEW)
- `crates/argo-vertical-regression/src/tests.rs` (only if local pattern requires)
- `docs/skycot/evals/scheduled/2026-05-24/eval-vertical-regression-cadence.md` (NEW report)
- `docs/build-plan/BUILD_STATUS.md`

## Must Not Touch

- Do not change Genesis or vertical loader production behavior.
- Do not add network or LLM calls.
- Do not invent a new regression scheduler in this brief.

## Acceptance

- [ ] Inventory exported public API and current in-repo callers.
- [ ] Pin at least two falsifiable regression contracts in tests.
- [ ] Report whether the crate is live in CI/pre-commit/runbooks or only available manually.
- [ ] Name follow-up brief(s) for any real wiring gap.
- [ ] `cargo test -p argo-vertical-regression` passes, or the PR body records any unrelated clean-main failure with a focused green command.

## Notes

If the crate is intentionally manual-only, make that explicit in the report rather
than force-wiring it.
