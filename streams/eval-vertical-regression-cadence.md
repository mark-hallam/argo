# eval-vertical-regression-cadence

**Authored against:** `origin/main` 2af54930f on 2026-05-23 (conductor tick-110).
**Validate before claim:** `bash scripts/check-brief-freshness.sh eval-vertical-regression-cadence`
**Owner:** codex
**Priority:** 🟢 P3 — substrate audit, no production change
**Source:** mission-vs-code-gap-manifest.md row argo-vertical-regression (🔴 NOT AUDITED).

## What to do

Read `crates/argo-vertical-regression/` end to end and answer:

1. **What does the crate test?** Inventory its `pub` regression-test surface — what inputs it accepts, what outputs/verdicts it emits.
2. **Who invokes it?** `rg 'argo_vertical_regression' crates/ verticals/ scripts/` — list every caller in the workspace.
3. **Is it wired into CI / pre-commit / Genesis pipeline?** Check `.github/workflows/`, `scripts/`, and `crates/argo-genesis/` for invocations.
4. **What's the drift-detection contract?** Does it diff against a baseline? Where's the baseline stored?
5. **Coverage gaps:** which vertical operations (rename, add-action, remove-action, system-prompt change) trigger regression checks vs not?

Write `crates/argo-vertical-regression/tests/cadence_pin_test.rs` to pin the answers as falsifiable contracts (the trait shape, caller count, any "every vertical edit re-runs this" invariant if it holds).

Write report `docs/skycot/evals/scheduled/2026-05-23/eval-vertical-regression-cadence.md` summarizing findings + recommending follow-up fix-briefs if cadence is broken.

## May-touch

- `crates/argo-vertical-regression/src/lib.rs` (rustdoc only — no behavior change)
- `crates/argo-vertical-regression/tests/cadence_pin_test.rs` (new)
- `docs/skycot/evals/scheduled/2026-05-23/eval-vertical-regression-cadence.md` (new)

## Acceptance

- Caller inventory + cadence assessment in report.
- Test pins at least 2 falsifiable contracts (trait shape + caller-existence-or-absence).
- Per-crate + workspace clippy clean.

## LOC budget

~250-500 LOC.
