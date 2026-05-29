# fix-stale-web-tests-27-reds-asserting-pre-change-behavior

**Priority**: P0 — the web vitest suite has 27 failing tests across 13 files; they assert pre-change behaviour after the last 24-48h of onboarding/audit/approvals/banner waves landed. Reds drown signal: a real regression can no longer be told apart from the stale noise.
**Source**: 2026-05-29 test-coverage audit (item 3 of operator's 6-item program)
**Authored against**: origin/main `65fa0a3223ef521c1c43452a6794216be7d44a25` on 2026-05-29
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-stale-web-tests-27-reds-asserting-pre-change-behavior`
**Authored by**: claude conductor (interactive session)

## Why this brief exists

`pnpm exec vitest run` in `web/` reports **2189 pass / 27 fail across 13 of 293 files**. Every failure was traced to a test asserting behaviour that a recently-merged PR *deliberately changed* — the tests were not updated in lockstep. These are STALE tests, **not regressions**. The fix is to UPDATE each test to pin the NEW (shipped, correct) contract — NOT to delete them, and NOT to revert the production change.

**Hard rule for this brief: do not change any production component to make a test pass. The production behaviour on main is the intended behaviour. Only the test assertions move.** If any failing test turns out to be guarding behaviour that is actually wrong on main, STOP and flag it in the PR body rather than editing production code under this brief.

## The 13 failing files + why each is stale

1. `app/(setup)/onboarding/connect-provider/__tests__/demo-mode-placeholder.test.tsx` (2 fail) — asserts old placeholder `"sk-ant-..."`; PR #2650 changed the demo-mode placeholder copy.
2. `app/(setup)/onboarding/connect-provider/__tests__/cta-hierarchy.test.tsx` (3 fail) — asserts old Continue/Skip CTA layout; changed by #2650/#2660.
3. `app/(setup)/onboarding/connect-provider/onboarding-connect-provider.test.ts` (1 fail) — #2660 re-gated the inputs.
4. `components/skycot/.../WizardEditorialShell.polish.test.tsx` (3 fail) — DemoBanner pathname-gate / mount-suppress rules changed (+83 LOC via #2655/#2662; banner copy now branches on data source).
5. `components/skycot/approvals/approvals-ops-dashboard-cards.test.tsx` (1 fail) — ApprovalsOpsDashboard card behaviour changed.
6. `__tests__/approvals/page.test.tsx` (3 fail) — empty-state / "$ at risk" card behaviour changed.
7. `app/(app)/audit/page.test.tsx` (2 fail) — row→route + TODAY/YESTERDAY grouping copy/route drift from the audit-feed wave.
8. `components/skycot/workflows/__tests__/intro-band.test.tsx` (3 fail) — copy drift.
9. `app/(setup)/onboarding/wizard.test.tsx` (1 fail) — copy/route drift.
10. `.../hire-wizard-auth.test.tsx` (4 fail) — copy/route drift.
11. `lint/route-metadata.test.ts` (2 fail) — route-shape drift.
12. `__tests__/workers/empty-state-matches-team.test.tsx` (1 fail) — empty-state copy drift.
13. (the 13th file is one of the above clusters spanning two paths — enumerate via the command below.)

## Scope

- For each failing test, read the production component it covers ON MAIN, then update the assertion to match the shipped behaviour. Each updated assertion must pin a real, current contract (new placeholder string, new CTA order, new banner gate, new audit grouping copy, etc.) — not a tautology.
- Where a test asserts a copy string, prefer asserting a stable role/testid + a substring of the new copy over an exact full-string match, so future copy tweaks don't re-break it (judgement: keep exact-match where the exact string IS the contract, e.g. metadata `<title>`).
- Re-run the full `web/` vitest suite; the run must end **0 fail**. Do not introduce `.skip`/`.todo` to hide a red — every test either asserts the new contract or, if genuinely obsolete, is removed WITH a one-line PR-body justification per removal.

## How to enumerate the exact failures

```bash
cd web && pnpm exec vitest run 2>&1 | grep -E "(FAIL|✗|×)" | sort -u
```

## Acceptance

- [ ] `cd web && pnpm exec vitest run` → 0 failing tests (was 27); pass count ≥ 2189 + (any net-new) − (any justified removals)
- [ ] No production `.tsx`/`.ts` component file changed (test files + test-only fixtures only) — `git diff --name-only` shows only `*.test.tsx`/`*.test.ts`/`__tests__/**` paths
- [ ] No `.skip`/`.only`/`.todo` added to dodge a red
- [ ] `pnpm exec tsc --noEmit` clean
- [ ] PR body lists, per file, the one-line reason the test was stale + which shipped PR changed the contract

## Out of scope

- Any production-component change (that would be a separate brief — flag, don't fix here)
- The forge `cost.rs` opus-4-8 pricing test (fixed directly by conductor 2026-05-29)
- Adding NEW coverage (e.g. Pitch LLM Playwright e2e — separate P1 brief)

## May-touch

- The 13 test files enumerated above + their `__tests__/` siblings
- Test-only fixtures under `web/**/__fixtures__/` or `web/test/` if a fixture also encodes stale copy

## Operator authority

Web-only / test-only PR — eligible for the test-only admin-merge lane (operator-locked 2026-05-26, no evaluator pre-verification required) PROVIDED the diff is genuinely test-only (acceptance criterion 2). If any production file must change, the PR drops out of the fast lane and needs evaluator cold-cargo + operator review.
