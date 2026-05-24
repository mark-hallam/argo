# eval-argo-core-crate-surface

**Owner**: claude
**Priority**: P2 - shared core substrate audit
**Authored against**: origin/main `ef55f5422` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh eval-argo-core-crate-surface`.

## Source

The mission-vs-code gap manifest marks `argo-core` as `NOT AUDITED`. The crate exports shared agent, audit, iteration-policy, resume, and steering primitives used as cross-cutting substrate.

## Goal

Audit and pin the `argo-core` public surface so shared core primitives remain stable and their relationship to higher-level runner/orchestrator code is explicit.

## May Touch

- `crates/argo-core/tests/surface_contract.rs` (NEW)
- `crates/argo-core/tests/` (NEW sibling tests if needed)
- `docs/skycot/evals/scheduled/2026-05-24/eval-argo-core-crate-surface.md` (NEW report)
- `docs/build-plan/BUILD_STATUS.md`

## Must Not Touch

- Do not change production `argo-core/src/**` code unless a contract test exposes an obvious compile-only drift and the fix is tiny.
- Do not edit agent-runner or orchestrator code.
- Do not add new mission behavior.

## Acceptance

- [ ] Pin exported modules/types/functions from `crates/argo-core/src/lib.rs`.
- [ ] Cover `Agent`, resume, steering, and iteration-policy contracts where public APIs exist.
- [ ] Static-grep current consumers and report whether the crate is actively used or legacy substrate.
- [ ] Write an eval report with verdict, gaps found, and follow-up brief names only for real findings.
- [ ] `cargo test -p argo-core --tests` passes, or the PR body records any unrelated clean-main failure with a focused green command.

## Notes

This is intentionally narrow: inventory and pin, then queue follow-up fixes only if the audit finds real drift.
