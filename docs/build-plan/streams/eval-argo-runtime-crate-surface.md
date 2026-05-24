# eval-argo-runtime-crate-surface

**Owner**: claude
**Priority**: P2 - runtime substrate audit
**Authored against**: origin/main `ef55f5422` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh eval-argo-runtime-crate-surface`.

## Source

The mission-vs-code gap manifest marks `argo-runtime` as `NOT AUDITED` and says its relationship to `argo-agent-runner` is unclear. The crate contains planning, dispatch gates, context compression/truncation, worker resource cages, anomaly, throttle, and pool-elasticity primitives.

## Goal

Audit and pin `argo-runtime`'s public surface and clarify which primitives are live substrate versus legacy/shared helpers.

## May Touch

- `crates/argo-runtime/tests/surface_contract.rs` (NEW)
- `crates/argo-runtime/tests/` (NEW sibling tests if needed)
- `docs/skycot/evals/scheduled/2026-05-24/eval-argo-runtime-crate-surface.md` (NEW report)
- `docs/build-plan/BUILD_STATUS.md`

## Must Not Touch

- Do not change production `argo-runtime/src/**` code unless a contract test exposes an obvious compile-only drift and the fix is tiny.
- Do not edit `argo-agent-runner` or gateway code.
- Do not add network or LLM calls.

## Acceptance

- [ ] Pin the exported module/type/function inventory from `crates/argo-runtime/src/lib.rs`.
- [ ] Cover at least one contract each for dispatch gating, context compression/truncation, planner or planner-bias behavior, and worker resource cage behavior.
- [ ] Identify current production consumers via static grep and report whether the crate is live substrate or partially legacy.
- [ ] Write an eval report with verdict, gaps found, and follow-up brief names only for real findings.
- [ ] `cargo test -p argo-runtime --tests` passes, or the PR body records any unrelated clean-main failure with a focused green command.

## Notes

This is an audit/pin brief. If live-wiring gaps appear, queue narrow follow-ups rather than wiring them here.
