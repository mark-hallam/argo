# eval-argo-forge-crate-surface

**Owner**: claude
**Priority**: P2 - substrate audit
**Authored against**: origin/main `50a3ea9f0` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh eval-argo-forge-crate-surface`.

## Source

The mission-vs-code gap manifest still tracks Forge substrate confidence separately from individual Forge fix briefs. `argo-forge` exports the orchestration surface for optimization runs, cost accounting, cron, feedback evals, persistence helpers, and the current `StubForge` placeholder.

## Goal

Audit and pin `argo-forge`'s public surface so optimization orchestration, stub behavior, status labels, and cost accounting cannot drift silently before the real worker plugin lands.

## May Touch

- `crates/argo-forge/tests/surface_contract.rs` (NEW)
- `crates/argo-forge/tests/` (NEW sibling tests if one file would exceed 500 LOC)
- `docs/skycot/evals/scheduled/2026-05-24/eval-argo-forge-crate-surface.md` (NEW report)
- `docs/build-plan/BUILD_STATUS.md`

## Must Not Touch

- Do not implement the real DSPy/GEPA optimizer.
- Do not change production `argo-forge/src/**` code unless a contract test exposes an obvious compile-only drift and the fix is tiny.
- Do not edit Python plugin scaffolding.

## Acceptance

- [ ] Pin the exported type/function inventory from `crates/argo-forge/src/lib.rs`.
- [ ] Cover `ForgeStatus` labels and serde behavior exhaustively enough to catch renamed persisted statuses.
- [ ] Cover `ForgeOptions::new` defaults and the fields a caller must supply.
- [ ] Cover `StubForge::optimize` returning the documented configure-the-worker error and event marker, without network or subprocess calls.
- [ ] Cover cost accumulation for at least one LLM call and one E2B-second path.
- [ ] Cover feedback eval case weighting or scoring fields sufficiently to catch schema drift.
- [ ] Write an eval report with verdict, any gaps found, and follow-up brief names only for real findings.
- [ ] `cargo test -p argo-forge --tests` passes, or the PR body records any unrelated clean-main failure with a focused green command.

## Notes

Keep this an audit/pin brief. Do not collapse it with open Forge promotion or judge-wire implementation work.
