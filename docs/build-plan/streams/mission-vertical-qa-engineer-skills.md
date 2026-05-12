# Mission load-bearing wire — QA-Engineer vertical: 2+ skills to reach 6

**Owner**: claude. **Depends on**: `verticals/qa-engineer/` (4 existing skills).
**Authored against**: origin/main `281ba83d` on `2026-05-12`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-vertical-qa-engineer-skills`.

> SOURCE: MISSION_BACKLOG.md Section 4 #137. qa-engineer ships with 4 skills (author_bug_report / investigate_flake / propose_regression_test / propose_test_plan). Brief brings to 6.

## Scope

2 NEW skill files:

1. **`reduce_test_set.skill.md`** — given a full test suite + CI history (operator-provided), identify the smallest-by-time subset that historically catches ≥95% of regressions. Output JSON: `{ recommended_subset: [test_id], retained_coverage_pct, time_savings_pct, skipped_tests: [{id, last_regression_caught_date}] }`. Conservative on inclusion (always-include skills > sometimes-include). Internal report; CI-config changes approval-gated.
2. **`triage_failing_build.skill.md`** — given a failing build's test report + recent commit log, categorize each failure: (a) genuine regression with most-likely-commit-id, (b) flaky-test (rerun-likely-passes), (c) infra-related (timeout/oom/dep-pull), (d) environment-drift. Output JSON with severity + suggested-next-action per failure. Autonomous.

## Files you may touch

- `verticals/qa-engineer/skills/reduce_test_set.skill.md` (NEW, ≤ 90 LOC)
- `verticals/qa-engineer/skills/triage_failing_build.skill.md` (NEW, ≤ 90 LOC)

## Files you must NOT touch

- Existing 4 skill files; `vertical.toml`; prompts.

## Acceptance criteria

- [ ] 2 new `.skill.md` files with full frontmatter + sections
- [ ] `reduce_test_set` 95% retained-coverage threshold + conservative-inclusion bias documented
- [ ] `triage_failing_build` 4-category vocab + most-likely-commit-id when categorizing as regression
- [ ] No file > 100 LOC; BUILD_STATUS.md updated

## Branch + PR

- Branch: `claude-mission-vertical-qa-engineer-skills`
- PR title: `verticals: 2 skills for qa-engineer (mission-vertical-qa-engineer-skills)`

## Notes

- Conservative inclusion in `reduce_test_set` is operator-meaningful — false-confidence drops worse than minor CI-time gains. Document the bias.
- `triage_failing_build`'s commit-id-attribution is a guess, not a certainty. Always output as `most_likely_commit: <id>` with a `confidence: low|medium|high` flag — never absolute.
