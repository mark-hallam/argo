# M10-eval-results-dashboard - Track M (M-Eval), Phase 2, Tier 2

**Owner**: claude. **Depends on**: M9 MERGED.
**Authored against**: origin/main `5a5d3970` on `2026-05-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh M10-eval-results-dashboard`.

## Scope

Per-worker eval surface: latest run, history, per-fixture detail. Regression banner when a version causes a regression. Drill-in: per-fixture pass/fail with diff against prior versions output.

LOC: ~1100. Branch: `claude-M10-eval-results-dashboard`. PR title: `feat(web/eval): per-worker eval dashboard (M10)`.

## Files you may touch

- NEW web/app/(app)/workers/[id]/evals/page.tsx (<=350)
- NEW web/components/skycot/eval/EvalRunHistory.tsx (<=250)
- NEW web/components/skycot/eval/EvalFixtureTable.tsx (<=250)
- NEW web/components/skycot/eval/RegressionBanner.tsx (<=150)
- NEW web/lib/api/evals.ts
- NEW crates/argo-cli/src/handlers/gateway/routes_evals.rs

## Files you must NOT touch

- M9 regression engine (consume).
- Anything outside the brief's may-touch set per CLAUDE.md.

## Acceptance criteria

- [ ] Route loads, history renders.
- [ ] Drill-in shows per-fixture detail.
- [ ] Regression banner renders for synthetic regression.
- [ ] 6 unit tests.
- [ ] No file > 500 LOC.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean (Rust-side).
- [ ] `pnpm typecheck` clean (web-side).
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-M10-eval-results-dashboard`
- PR title: `feat(web/eval): per-worker eval dashboard (M10)`

## Notes for the implementer

- See `streams/M0-enterprise-parity-track-overview.md` for the authoritative spec for this brief.
- Mission-claim alignment: claim 4 (learning observability).


## Amendment 2026-05-14 (may-touch widening)

Pre-empted from the same self-BLOCK pattern that hit M1/M5/M9: workers reasonably reported the original may-touch was too narrow to ship a real (non-scaffold) implementation. Conductor adds these load-bearing edits to the allowed surface:

- The containing crate's `lib.rs` — narrowly: add `pub mod <new_module>;` declaration. **≤10 modified lines.**
- The containing crate's `Cargo.toml` — narrowly: add any required `[dependencies]` line for sibling argo-* crates that this brief consumes. **≤5 modified lines.**
- If a gateway endpoint is in scope: register the route in the appropriate `routes_dispatch_*.rs` sibling (post-#1601 split). **≤10 modified lines.**

These edits are mechanical and required for the new code to compile + be reachable. Do NOT widen beyond the brief's original scope (no business logic in lib.rs, no new dependencies beyond what the brief names).
