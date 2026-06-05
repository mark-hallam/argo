# uf-hire-gate-wire-live-eval (UF Phase-A · A4)

**Owner**: claude / evaluator-lane.
**Depends on**: the live-agent eval runner + teammate scorecard. tenant keystone (#2676, MERGED).
**Authored against**: origin/main `2264560a2` on `2026-06-05`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh uf-hire-gate-wire-live-eval`.

## Dependency note (verified 2026-06-05 — read FIRST)
PR #2668 (`eval-teammate-live-agent-runner-and-scorecard`) shows **CLOSED**, but its **capability IS on `origin/main`**: `crates/argo-eval/src/teammate_scorecard.rs` exists and the `EvalSubject` trait is at `crates/argo-eval/src/runner.rs:13`. A4 reuses the LIVE module on main (NOT #2668's closed branch). **If on inspection the runner turns out to be backbone-only (not a working live-agent runner), STOP, flag it, and leave A4 BLOCKED** — do not build a parallel runner.

## Why
The hire gate scores the tautological `ContractSubject` (`routes_genesis_gate.rs`, 7 refs) — it ignores the task and substring-matches, so it passes on keyword-parroting. Replace it with the REAL live-agent runner (DEC-025 #7 fast-follow: deterministic floor first, LLM-graded rubric is the fast-follow). Do NOT rebuild the runner.

## Grep-before-write (verified 2026-06-05)
- `rg 'EvalSubject' crates/argo-eval/src/runner.rs` → trait at `:13`.
- reuse `crates/argo-eval/src/teammate_scorecard.rs` — grep it for the runner entry + scorecard scoring before wiring.

## Files you may touch
- `crates/argo-cli/src/handlers/gateway/routes_genesis_gate.rs`: swap `ContractSubject` → a subject that drives the live runner against the seeds; flag-gate for cost (run a capped subset live), and **document the cap (NO silent truncation)**.

## Acceptance
A deliberately-incompetent manifest **FAILS** the live floor; a competent one passes; the gate no longer passes on keyword-parroting alone.

## On ship (same commit)
Update `docs/build-plan/FEATURE_MATRIX.md` (hire-gate live-eval row) + regen HTML (`python3 scripts/render-feature-matrix.py`).

## Effort
~1 brief (M).
