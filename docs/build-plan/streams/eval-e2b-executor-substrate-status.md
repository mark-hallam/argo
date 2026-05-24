# eval-e2b-executor-substrate-status

**Owner**: claude
**Priority**: P3 - v1.x-deferred computer-use substrate audit
**Authored against**: origin/main `ef55f5422` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh eval-e2b-executor-substrate-status`.

## Source

The mission-vs-code gap manifest lists `argo-executor-computer-use` (E2B Firecracker microVM) as `NOT AUDITED` and notes it is DEFERRED-v1.x per operator strategy. PR #2171 now pins that deferred substrates must not gain surprise production callers.

## Goal

Audit the E2B computer-use substrate status: public surface, offline test coverage, production callsites, and whether the current deferral is accurately documented.

## May Touch

- `crates/argo-executor-computer-use/tests/substrate_status_contract.rs` (NEW)
- `crates/argo-executor-computer-use/tests/` (NEW sibling tests if needed)
- `docs/skycot/evals/scheduled/2026-05-24/eval-e2b-executor-substrate-status.md` (NEW report)
- `docs/build-plan/BUILD_STATUS.md`

## Must Not Touch

- Do not wire E2B into v1.0 production paths.
- Do not start real E2B sandboxes or make network calls.
- Do not edit macOS computer-use code.

## Acceptance

- [ ] Pin the exported public surface from `crates/argo-executor-computer-use/src/lib.rs`.
- [ ] Cover offline parsing/config/budget/pool/restart contracts using existing mock/fake fixtures.
- [ ] Static-grep production callers and confirm they match the v1.x-deferred allowlist from #2171.
- [ ] Document exactly what works offline today and what remains deferred behind operator policy.
- [ ] Write an eval report with verdict, gaps found, and follow-up brief names only for real findings.
- [ ] `cargo test -p argo-executor-computer-use --tests` passes, or the PR body records any unrelated clean-main failure with a focused green command.

## Notes

This brief should make deferral honest and test-backed. It must not promote E2B into the v1.0 live path.
