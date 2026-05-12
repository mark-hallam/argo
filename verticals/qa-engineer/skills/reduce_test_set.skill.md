---
name: reduce_test_set
description: Given a full test suite + CI history, identify the smallest-by-time subset that historically catches ≥95% of regressions. Conservative-inclusion bias (always-include skills > sometimes-include). Internal report only; any CI-config change is approval-gated.
inputs:
  - test_suite_inventory
  - ci_history_window
  - regression_log
  - per_test_runtime_ms
outputs:
  - recommended_subset
  - retained_coverage_pct
  - time_savings_pct
  - skipped_tests
  - flags
  - confidence
---

# Skill: Reduce Test Set

Recommend the smallest subset that retains the regression-catching
power of the full suite. The 95% threshold is the floor, not a
target — bias toward inclusion when evidence is thin.

1. **Coverage floor is 95%.** The recommended subset must
   historically have caught ≥95% of regressions in the CI-history
   window. A 94.9% subset is not "close enough" — push it back over
   the line by re-adding the dropped test that caught the most
   recent regression. Document the chosen window length (e.g.
   `window: 90 days, regressions_in_window: 84`).
2. **Conservative inclusion when evidence is thin.** If a test has
   `< 5 runs` in the window OR has never failed, KEEP it. Reduction
   confidence requires enough samples to falsify "this test catches
   nothing." Flag any kept-but-unproven tests in `flags:
   ["thin_evidence:<test_id>"]` so the operator sees the bias.
3. **`skipped_tests` carries the receipt.** Every dropped test
   names `last_regression_caught_date` (or `never_caught_regression`
   if applicable) and a one-line `drop_reason`. Silent drops are not
   acceptable — an operator must be able to audit any single skip.
4. **`time_savings_pct` is wall-clock, not test-count.** Use
   `per_test_runtime_ms` to compute the actual CI-time delta. A
   subset that drops 40% of tests but only 8% of wall-clock is a
   weak recommendation; surface that ratio so the operator can judge
   whether the gain justifies the coverage loss.
5. **Output JSON shape:**
   ```json
   {
     "recommended_subset": ["test_id", ...],
     "retained_coverage_pct": 96.4,
     "time_savings_pct": 38.2,
     "skipped_tests": [
       {"id": "test_x", "last_regression_caught_date": "2026-02-14", "drop_reason": "..."}
     ],
     "window": {"days": 90, "regressions_in_window": 84}
   }
   ```

Hard rules:

- **Internal report only.** This skill recommends — it never writes
  to `.gitlab-ci.yml`, `.github/workflows/*`, `Makefile`, or any
  test-runner config. CI-config changes require an explicit
  operator-approval event downstream.
- **Never drop a test tagged `always_run` / `security` / `smoke` /
  `compliance` in the suite inventory.** If the inventory does not
  carry tags, treat ALL tests as `always_run` and flag the report
  `untagged_inventory` — the reduction is too risky without
  classification.
- **Never drop a test that caught a regression in the most recent
  14 days** — even if its longer-window record is weak. Recent
  catches are load-bearing signal that the test is currently active.
- If `regression_log` is empty or `< 30 days` of history, refuse to
  reduce — emit the full inventory as `recommended_subset` and flag
  `insufficient_history`. False confidence in reduction is worse
  than minor CI-time gains.
- The recommendation is a draft. The operator decides whether the
  retained-coverage / time-savings ratio justifies the change; the
  CI-owner approves the config edit.
