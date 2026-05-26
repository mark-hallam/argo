# fix-pre-existing-vitest-flakes-bundle

**Owner**: claude or codex.
**Authored against**: origin/main `bded3714e` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-pre-existing-vitest-flakes-bundle`.
**Source**: PR #2384 body footnote — 10 test failures across 6 files on `origin/main` `ded274874` confirmed by re-running on a clean stash. Pre-existing, NOT introduced by any specific recent PR.

🟡 **P2 — pre-existing vitest failures pollute the workspace test signal.** When workers run `pnpm exec vitest run`, they get 10 RED tests across 6 files even on clean main — making it impossible to tell at-a-glance whether a PR introduces regressions. Per HARD RULE 2 every PR has had to write a "Boundary note" footnote confirming pre-existing on main. Real ongoing tax on every PR.

## Affected files (6)

1. `web/__tests__/a11y-contrast.test.ts`
2. `web/__tests__/a11y-headings.test.ts`
3. `web/__tests__/brand-execution-polish.test.tsx`
4. `web/__tests__/approvals/page.test.tsx`
5. `web/__tests__/settings/page.test.tsx`
6. `web/__tests__/workflows/sub-workflow-node.test.tsx`

10 tests total fail; specific failing test names not yet enumerated.

## Fix recipe

Run `cd web && pnpm exec vitest run __tests__/a11y-contrast.test.ts` (and each of the other 5) on a fresh main checkout to capture the **exact** failure modes. For each failing test, pick the smallest-touch fix:

- **a11y-contrast / a11y-headings** — likely a design-token drift (a color got changed in `globals.css` and the test now snapshots a wrong value, or a heading-level changed in a marketing/onboarding page). Fix at the snapshot OR the design token, whichever matches current intent.
- **brand-execution-polish** — likely outdated DOM assertions after recent brand-consistency pass. Update the assertion to match current DOM.
- **approvals/page** + **settings/page** — likely route-component drift after recent T00-28 batch. Re-run against current page.tsx and update assertions.
- **workflows/sub-workflow-node** — likely a workflow-graph-shape change. Update graph assertions.

Acceptance criteria:
- [ ] `cd web && pnpm exec vitest run` exits 0 (all tests pass on clean main).
- [ ] No test deleted to make it pass — assertions updated to match correct current behavior.
- [ ] If a fix reveals a real product bug (test was right, component is wrong), file a sibling brief and document the choice in the PR body.
- [ ] PR body table mapping each of the 10 failing tests to (a) test-update OR (b) component-fix OR (c) sibling-brief-filed.

## May-touch

- All 6 test files listed above.
- Any component the tests reference (one of the SPA app pages or `web/components/`). Worker uses judgment on the scope.

## Out of scope

- The `/brand/components` prerender error (separate brief: `fix-brand-components-page-event-handler-prerender-error`).
- Adding NEW tests — only fix existing ones.
- Snapshot regeneration for visual diffs unrelated to the 10 failing tests.

## Worker note

This is a "clean up the signal" brief — the value isn't user-visible, it's swarm-velocity. Once tests pass on clean main, every subsequent PR can run `pnpm exec vitest run` and trust an exit-0 result as "I introduced no regressions." Currently every worker has to manually compare against the same 10 failures, which wastes ~5-10 min per PR.
