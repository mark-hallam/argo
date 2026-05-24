# eval-team-headline-crate-surface

**Owner**: codex
**Priority**: P3 - claim 1 last unaudited cell
**Authored against**: origin/main `7354f039d` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh eval-team-headline-crate-surface`.

## Source

The gap manifest marks `argo-team-headline` as the remaining unaudited Claim 1
cell. It is consumed by team-list surfaces and affects teammate-feel UX rather
than data-loss risk.

## Goal

Pin the crate's public surface, current consumers, and tenant-safety assumptions
with a small structural contract and report.

## May Touch

- `crates/argo-team-headline/tests/surface_pin_test.rs` (NEW)
- `crates/argo-team-headline/src/lib_tests.rs` only if extending local tests is cleaner
- `docs/skycot/evals/scheduled/2026-05-24/eval-team-headline-crate-surface.md` (NEW report)
- `docs/build-plan/BUILD_STATUS.md`

## Must Not Touch

- Do not change gateway team routes.
- Do not change headline algorithms unless a test exposes an obvious compile-only drift.
- Do not add network or LLM calls.

## Acceptance

- [ ] Pin exported inventory from `crates/argo-team-headline/src/lib.rs`.
- [ ] Identify production consumers and report whether tenant context is carried or irrelevant.
- [ ] Cover at least one deterministic headline/KPI contract.
- [ ] Write an eval report with verdict and follow-up brief names only for real findings.
- [ ] `cargo test -p argo-team-headline` passes, or the PR body records any unrelated clean-main failure with a focused green command.

## Notes

Small audit, fast-mode suitable.
