# claude-web-test-flake-sweep-wave-5

**Authored against**: `origin/main` 349b6acc on 2026-05-21.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-web-test-flake-sweep-wave-5`.

## Summary

Every PR in the wave-15 merge wave (#1931–#1945) declared the same 5–7 pre-existing vitest failures in their boundary notes per HARD RULE 2. They are accurate (the failures pre-date each PR), but they are real test breakages that mask new regressions in the suite. This brief clears them.

## Known pre-existing failures (verified consistent across PRs)

1. `brand-execution-polish` — off-grid 1.5 spacing (cited as `AppShell.tsx:138 py-1.5`)
2. `workflow-templates` — `prefers-reduced-motion` guard missing
3. `TeammateCard` — 3 routing test failures (click-to-detail paths)
4. `team/page` — heading-level assertion failure
5. `team` — `Clear all` button query failure (sometimes cited as 6th)
6. `auth.config.ts` — `next/headers` in pages dir (cited as pre-existing build failure by #1941)

## Acceptance

- [ ] Each of the 6 failures above either FIXED or DELETED with a 1-line PR-body justification per failure.
- [ ] `pnpm -C web test` returns 0 failures on this branch.
- [ ] Boundary note in PR body lists what was fixed vs deleted and why.
- [ ] No production code change (test-only sweep), unless a failure is a real bug surfaced — in which case author MAY fix the bug and cite the audit chain.
- [ ] Followup brief authored if any "real bug" surfaces during the sweep.

## May touch

- `web/**/__tests__/**` and `web/**/*.test.{ts,tsx}` (test files only).
- `web/components/skycot/brand-execution-polish.test.tsx` and similar test specs (if the right fix is to update the assertion).
- `web/components/AppShell.tsx` (only the `py-1.5` line if the audit chain shows that off-grid spacing is actually wrong — operator hasn't decided; if the spec is the wrong assertion, fix the spec).
- `web/auth.config.ts` — only if the `next/headers`-in-pages-dir warning is a real bug worth fixing in this sweep.

## Must NOT touch

- Locked design files (cohorts D3, hire-done celebration, worker-detail).
- Seed files (`web/lib/sample-*.ts`).
- Any non-test production code unless an audit chain in the PR body justifies the fix.

## Notes

- Boundary-flake declarations across #1931, #1932, #1933, #1934, #1935, #1936, #1937, #1938, #1939, #1940, #1941, #1942, #1943, #1944, #1945 are remarkably consistent — same 5–7 names. This is a maintenance-debt sweep, not a feature.
- Operator priority: keep the suite green so new regressions surface immediately. Pre-launch hygiene.
- HARD RULE 2 boundary declarations are not a substitute for fixing the flakes — they are an interim discipline.
