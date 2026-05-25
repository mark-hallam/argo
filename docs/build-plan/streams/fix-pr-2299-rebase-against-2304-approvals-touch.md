# fix-pr-2299-rebase-against-2304-approvals-touch

**🟡 P1 fast-mode rebase brief**

Authored against: origin/main `0c48732b8` on 2026-05-25
Validate before claim: bash scripts/check-brief-freshness.sh fix-pr-2299-rebase-against-2304-approvals-touch

## Why this exists

PR #2299 (`fix: remove fabricated approvals ops metrics`) was authored before the wave-1 merge train shipped #2304 (`approvals: link rows to audit detail`). Both touch `web/components/skycot/approvals/ApprovalsOpsDashboard.tsx` and the matching test file. The two PRs are NOT conflicting in INTENT — #2299 removes hardcoded KPIs, #2304 wires row-click navigation — but they overlap on the same React component lines and git refuses to merge cleanly.

## What this brief does

Rebase `codex-fix-approvals-ops-dashboard-hardcoded-kpis-and-donuts` onto current `origin/main`, resolving line-level conflicts in:
- `web/components/skycot/approvals/ApprovalsOpsDashboard.tsx`
- `web/components/skycot/approvals/ApprovalsOpsDashboard.test.tsx`

Preserve BOTH changes:
- **#2299 contribution**: remove all hardcoded `78/82/14/8/active-reviewers=1` KPI strip values + $-at-risk donut + 7-day rate ring fabrications. If real metrics aren't available from the gateway, render an empty-state skeleton with "Not enough activity yet" copy (DO NOT re-introduce hardcoded numbers).
- **#2304 contribution (already on main)**: row-click navigation to `/audit/action/{action_id}`. Preserve the existing onClick handler + focus ring + ARIA accessibility per #2304.

## Acceptance criteria

1. `git rebase origin/main` completes cleanly with manual conflict resolution
2. `pnpm --dir web test --run ApprovalsOpsDashboard` passes (post-rebase tests cover both fabricated-metrics-removal and row-click-navigation)
3. `pnpm --dir web typecheck` clean
4. Force-push to existing branch `codex-fix-approvals-ops-dashboard-hardcoded-kpis-and-donuts` (do NOT close + re-open the PR; preserve PR #2299 review history)
5. PR #2299 flips back to MERGEABLE/CLEAN, ready for admin-merge

## May-touch

- `web/components/skycot/approvals/ApprovalsOpsDashboard.tsx`
- `web/components/skycot/approvals/ApprovalsOpsDashboard.test.tsx`

## Must NOT touch

- The PR description on #2299 (preserve original)
- Any file outside `web/components/skycot/approvals/`
- BUILD_STATUS.md (conductor will flip the row post-merge)

## Verification

```bash
git fetch origin main
git checkout codex-fix-approvals-ops-dashboard-hardcoded-kpis-and-donuts
git rebase origin/main
# resolve conflicts in ApprovalsOpsDashboard.{tsx,test.tsx}
pnpm --dir web typecheck
pnpm --dir web test --run ApprovalsOpsDashboard
git push --force-with-lease origin codex-fix-approvals-ops-dashboard-hardcoded-kpis-and-donuts
gh pr view 2299 --json mergeable -q .mergeable  # should now be MERGEABLE
```

## Fast-mode candidate

Yes — pure mechanical rebase, no design decisions. ~30-60 min typical.

## Reference

- Conductor rebase recipe comment on PR #2299: https://github.com/mark-hallam/Argo-Agent/pull/2299#issuecomment-4535209144
- Originating UX audit finding: agent-4 F4+F5+F10+F11 (UX-AUDIT-T11-00 synthesis batch)
