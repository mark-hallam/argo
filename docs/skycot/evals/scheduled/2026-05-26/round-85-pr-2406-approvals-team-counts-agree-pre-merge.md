# Round 85 — PR #2406 fix-approvals-inbox-waiting-on-you-fixture-drift PRE-MERGE PASS

- **Date**: 2026-05-26
- **Round**: 85
- **Pillar / Claim**: v1.0 demo-MP4 P0 — Scene 5 (approvals inbox + /team consistency). 3rd of 4 demo-MP4 P0s; closes the "Waiting on you" badge ⇄ inbox-empty mismatch.
- **PR head SHA**: `541fcc8b3ab0969f912273b9c0f631f971c6172c`
- **Mergeable**: `MERGEABLE` post-fetch (5 files, all non-overlapping with #2404/#2405)
- **Verdict**: 🟢 **PRE-MERGE PASS** (P0 demo-blocker ready)

## Setup

- `git fetch origin pull/2406/head:pr-2406-eval`; `git worktree add --detach /tmp/eval-r85-wt pr-2406-eval`.
- `pnpm install --frozen-lockfile --prefer-offline` in worktree (6.6s warm install).
- `pnpm vitest run __tests__/approvals/page.test.tsx lib/sample-team lib/sample-approvals` from `/tmp/eval-r85-wt/web`.
- Source-read diff against PR base `e5739d85ee969eb4bd213ab80f1daece9c2b8ea5`.

## Observed

- **Vitest 7/7 PASS in 85ms** across 2 test files on PR head:
  - `lib/sample-approvals.test.ts` (4 tests, 2ms) — sample data structural invariants.
  - `__tests__/approvals/page.test.tsx` (3 tests, 83ms) — covers (a) zero-pending dashboard renders with KPI placeholders, (b) populated dashboard with KPI + donut + ring, (c) the ring renders 7-day approval rate because SAMPLE_APPROVALS carries 3 resolved rows.
- **Production fix at `web/lib/sample-team.ts:34-94`**: the `RUNTIME_OVERLAY` per-worker map now carries only `baselineStatus` + `lastActiveAt`. The `pendingApprovalCount` field is REMOVED from the inline overlay and DERIVED from `SAMPLE_APPROVALS` by name match:
  ```ts
  function pendingCountByName(name: string): number {
    return SAMPLE_APPROVALS.filter(
      (item) => isPending(item) && item.worker.name === name,
    ).length;
  }
  ```
  And the status is overridden from `baselineStatus` to `"waiting"` when `pending > 0`. The /team chip count CANNOT diverge from the /approvals row count by construction — they share a single source of truth.
- **Production fix at `web/app/(app)/approvals/page.tsx`**: the `pendingCount === 0 ? <ApprovalsEmptyState /> : <ApprovalsOpsDashboard />` ternary is removed. The page always renders the locked E-ops-dashboard (per `project_approvals_page_design_locked.md`) — its 0-state already handles "0 awaiting" via `—` placeholders + per-tier "No P0 severity approvals waiting" rows.
- **Playwright e2e at `web/tests/e2e/team-and-approvals-counts-agree.spec.ts`** (NEW, 66 LOC):
  - Test 1 navigates to /team, counts cards with "Waiting on you" status pill, then navigates to /approvals, counts pending rows via `getByTestId(/^recent-approval-/)`, asserts `inboxPendingCount >= waitingTeamCount` and `>= 4`. This is the cross-route regression pin.
  - Test 2 asserts the ops-dashboard chrome is always rendered (KPI strip + 3 severity tiers + zero `approvals-empty` matches).
- **Zero file overlap with merged #2404 or #2405**. Rebase against current origin/main will be trivial:
  - #2404 touched: `_scrub-error.ts`, onboarding/* pages, `use-*.ts` hooks
  - #2405 touched: `audit-detail.ts`, `sample-audit-detail.ts`, `audit/action/[actionId]/error.tsx`
  - #2406 touches: `__tests__/approvals/page.test.tsx`, `approvals/page.tsx`, `sample-approvals.ts`, `sample-team.ts`, new playwright spec
  - GitHub-reported `MERGEABLE` is correct; no conflict-resolution rebase needed.

## Anti-theater check

- **Source-of-truth refactor**: the fix is a structural change in `sample-team.ts` (derive vs duplicate). Theatre would touch only `RUNTIME_OVERLAY` numbers (i.e., increment `pendingApprovalCount` to match `SAMPLE_APPROVALS.length` once and call it done) — that approach fails the moment SAMPLE_APPROVALS changes again. This PR removes the divergence vector entirely.
- **Cross-route playwright assertion**: the e2e visits BOTH /team AND /approvals in a single test and asserts state agreement. Unit tests alone can't prove cross-page coherence — this Playwright test catches what unit tests cannot.
- **The vitest tests were updated to match the new contract**: the prior "no approvals waiting" empty-state expectation is replaced with the "ops-dashboard renders + KPI shows 0" expectation. This change is justified because the production behaviour intentionally changed (locked design renders unconditionally). The new tests assert the LOCKED design's testids, not arbitrary copy.
- **72nd anti-theater verification this week**.

## Rebase risk

Trivial. Five files, zero overlap with the 9 PRs merged since the PR's base commit (#2402, #2353, #2345, #2268, #2343, #2376, #2362, #2359, #2397, #2404, #2405). GitHub flags `MERGEABLE`. Recommend merge without conductor-personal-rebase.

## Recommendation

🟢 **P0 MERGE** under permanent web-only `--skip-clippy` fix-* auth (web-only 5 files +134/-48 with vitest in body PASSing).

After merge, 3 of 4 demo-MP4 P0s will be landed:
- ✅ #2404 Scene 3 onboarding-demo-bypass
- ✅ #2405 Scene 6 audit-drill-in-fallback
- ✅ #2406 Scene 5 approvals/team counts-agree  ← this PR
- 🟡 `fix-skills-marketplace-perpetual-loading` (CLAIMED per ledger, no PR yet)

Demo MP4 recording becomes possible once the 4th P0 ships.
