# eval-argo-judge-crate-surface

**Owner**: codex
**Priority**: P3 - substrate audit
**Authored against**: origin/main `7354f039d` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh eval-argo-judge-crate-surface`.

## Source

The mission-vs-code gap manifest still marks `argo-judge` as not audited. The
crate already has judge, rubric, evidence, persistence, eval-suite, and fixtures,
and it is mission-relevant because forge promotion must fail closed when judge
evidence is missing or adverse.

## Goal

Pin `argo-judge` public surface and core verdict/rubric contracts with focused
tests and a short eval report. If production wiring gaps appear, queue follow-up
briefs instead of wiring them here.

## May Touch

- `crates/argo-judge/tests/surface_contract.rs` (NEW)
- `crates/argo-judge/src/tests.rs` or sibling test files if the crate pattern is cleaner
- `docs/skycot/evals/scheduled/2026-05-24/eval-argo-judge-crate-surface.md` (NEW report)
- `docs/build-plan/BUILD_STATUS.md`

## Must Not Touch

- Do not change `argo-forge` production callers.
- Do not change judge persistence schema or migrations.
- Do not add live LLM/network calls.

## Acceptance

- [ ] Pin the exported inventory from `crates/argo-judge/src/lib.rs`.
- [ ] Cover `Judge` trait shape and representative verdict/rubric serde round trips.
- [ ] Cover default-rubric threshold behavior or document why no default exists.
- [ ] Cover at least one fixture-backed disagreement/evidence path.
- [ ] Write an eval report with verdict, production consumers, and follow-up brief names only for real findings.
- [ ] `cargo test -p argo-judge --tests` passes, or the PR body records any unrelated clean-main failure with a focused green command.

## Notes

Audit-only. Keep each new file under the 500 LOC budget.
