# stage-3-eval-replay-scorecard-surface

**Owner**: claude or codex — web-only, DEC-006 fast lane; LOCKED BRAND (consume tokens).
**Depends on**: none.
**Authored against**: origin/main `de0e01ea3` on `2026-06-12`.
**Validate before claim**: bash scripts/check-brief-freshness.sh stage-3-eval-replay-scorecard-surface

## Problem (discovery-verified + re-scoped 2026-06-12)

DEC-042 §3 Stage-3 wants the "AI workforce track record" measurability
surfaces user-facing. The discovery sweep found **most of it already
ships** (per-worker activity timeline, goal-vs-actual scorecard, team KPI
rollup, skills list are all live SPA surfaces) — so this brief is the ONE
genuinely-unsurfaced piece, NOT a parallel rebuild:

The **6-axis eval-REPLAY scorecard** — `GET
/api/v1/workers/<id>/evals/replay`
(`crates/argo-cli/src/handlers/gateway/routes_worker_evals_replay.rs`) — is
referenced **nowhere in `web/`** (zero matches incl. tests). The teammate-
performance eval harness produces this scorecard but no SPA surface renders
it. (No backend work — the route is live.)

## Fix (consume the existing API; no parallel surface)

Add an "Evals" / "Performance" panel on the worker detail page
(`web/app/(app)/workers/[workerId]/`) that fetches the eval-replay
scorecard and renders the 6 axes (+ pass/fail + any historical trend the
route returns). Reuse the existing `ScorecardCard` / `WorkerDetailTabs`
visual patterns (do NOT build a new card system); brand tokens from
`globals.css` (DESIGN_SYSTEM.md locked). Mock-gated via `hasRealSession()`
like the other worker surfaces; honest empty state when a worker has no
eval runs.

## May touch

- `web/app/(app)/workers/[workerId]/` (new tab/panel + page wiring)
- `web/components/skycot/worker/` (new EvalScorecard component, mirror
  ScorecardCard)
- `web/lib/api/` (a fetcher for `/workers/<id>/evals/replay`)

## Acceptance / verifier

1. The panel renders the real eval-replay scorecard for a worker that has
   eval runs (live fetch, not mock); honest empty state otherwise.
2. vitest on the new component green; `tsc --noEmit` clean; full `vitest
   run` stays 0-failed (don't regress the green suite); each file <500 LOC.

~150-250 LOC. **🟡 P3 — DEC-042 Stage-3 first user-facing slice (the one
real gap; the rest of the track-record surface already ships).**
