# fix-fn-loc-gate-baseline-and-enforce

Authored against: origin/main ca46b97f1e42d9d093d275763b55a04add3d65c3 on 2026-06-10
**Validate before claim**: bash scripts/check-brief-freshness.sh fix-fn-loc-gate-baseline-and-enforce

## Goal

Make the "no function > 80 LOC" CI invariant enforceable on the real merge
path: baseline the existing violations so the gate fails only on NEW ones,
then wire it into the conductor pre-merge gate.

## Why now (architecture audit 2026-06-10, P2-4)

- `bash scripts/check-fn-loc.sh` exits 1 with 315 violations (~195 in `src/`),
  e.g. `crates/argo-vertical/src/eval_score.rs` `score_fixture` 185 LOC,
  `crates/argo-workplace-knowledge/src/doc.rs:177` `update` 120 LOC,
  `crates/argo-workflow/src/executor_checkpoint.rs:50` `run_loop` 111 LOC.
- It's a gate in `scripts/local-ci-check.sh:147` (so local CI is permanently
  red on clean main — the "judge-around-it" disease) but ABSENT from
  `scripts/conductor-pre-merge-gate.sh` (so the invariant is unenforced on the
  merge path). Verified 2026-06-10: `grep -c check-fn-loc conductor-pre-merge-gate.sh` = 0.

## Fix

1. `scripts/check-fn-loc.sh` learns a baseline allowlist (a committed file of
   the 315 current `path:line fn` entries) and fails ONLY on violations not in
   the baseline (mirror the established baseline pattern other ratchet gates
   use, if one exists; else a simple sorted text file + diff).
2. Add the baselined check to `conductor-pre-merge-gate.sh` as a new gate step.
3. Document a burn-down: the baseline shrinks over time; never grows.

## May touch

- `scripts/check-fn-loc.sh`
- `scripts/fn-loc-baseline.txt` (NEW — the 315 entries)
- `scripts/conductor-pre-merge-gate.sh`
- `scripts/local-ci-check.sh` (point at the baselined check so local CI goes green)

## Acceptance / verifier

1. `bash scripts/check-fn-loc.sh` exits 0 on clean main (baseline honored).
2. A synthetic NEW 90-LOC fn FAILS the check.
3. Conductor gate includes the step; `local-ci-check.sh` green on clean main.

## Size / owner

~40 LOC script + generated baseline. Owner: claude (codex-eligible, mechanical).
🟡 P2 — stops a permanently-red gate training judge-around-it.
