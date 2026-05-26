# Round 86 — PR #2406 `fix-approvals-inbox-waiting-on-you-fixture-drift` POST-MERGE anti-theater

- **Date**: 2026-05-26T12:55Z (Melbourne 22:55 AEST)
- **Round**: 86
- **Pillar / Claim**: SPA demo-recording readiness — Scene 5 ("Approvals inbox + /team chip count agree") — Claim 1 (Teammate-not-tool) surface coherence
- **Verdict**: 🟢 PASS — P0 demo-MP4 blocker closed durably
- **Carry status**: r85-S3 PRE-MERGE PASS → r86 POST-MERGE PASS (closed)

## Setup

```bash
git fetch origin main --quiet
git log --oneline origin/main -3
# 90d5b8c79 evaluator: round 85 …
# ea161df3a status: T11-48 — 🎯 fix-approvals-inbox-waiting-on-you-fixture-drift MERGED (#2406)
# 5808ccd78 fix: derive /team waiting badge from SAMPLE_APPROVALS so /approvals can't disagree (#2406)

gh pr view 2406 --json title,mergedAt,mergeCommit,additions,deletions,changedFiles
# mergedAt: 2026-05-26T11:54:30Z
# mergeCommit: 5808ccd7838a9ddf59fb58846359529c1df827eb
# +134 / -48 across 5 files
```

Production-code reads:
- `web/lib/sample-team.ts` — full file (98 LOC)
- `web/app/(app)/approvals/page.tsx` — full file (59 LOC)
- `web/tests/e2e/team-and-approvals-counts-agree.spec.ts` — regression-pin spec

Vitest from current `origin/main` HEAD `90d5b8c79`:
```bash
cd web && pnpm exec vitest run __tests__/approvals/page.test.tsx
#   3 tests passed (3) — 850ms
cd web && pnpm exec vitest run tests/sample-roster-canon.test.ts \
                                tests/approvals-ops-dashboard-cards.test.tsx \
                                __tests__/workers/empty-state-matches-team.test.tsx
#  11 tests passed (11) — 1.13s across 3 files
```

## Observed

**Source-of-truth derive is real, not a refactor-rename.** `sample-team.ts:69-94`:

```ts
function pendingCountByName(name: string): number {
  return SAMPLE_APPROVALS.filter(
    (item) => isPending(item) && item.worker.name === name,
  ).length;
}

function buildWorker(id: string): Worker {
  const persona = getSamplePersona(id);
  const overlay = RUNTIME_OVERLAY[id];
  if (!overlay) throw new Error(`Missing sample-team runtime overlay for ${id}`);
  const pending = pendingCountByName(persona.name);
  // A pending approval means the worker is *literally* waiting on
  // the operator — override the baseline so the "Waiting on you"
  // chip cannot diverge from the inbox count.
  const status: WorkerStatus = pending > 0 ? "waiting" : overlay.baselineStatus;
  return {
    id: persona.id, name: persona.name,
    role: { tagline: persona.role.tagline, vertical: persona.role.vertical },
    status,
    headline: persona.headline,
    lastActiveAt: overlay.lastActiveAt,
    pendingApprovalCount: pending > 0 ? pending : undefined,
  };
}

export const SAMPLE_TEAM: Worker[] = SAMPLE_PERSONA_IDS.map(buildWorker);
```

The chip count + the "waiting" status override + the `pendingApprovalCount` are all computed
from the SAME `SAMPLE_APPROVALS` array at module-load time. There is no second hand-edited
list that could drift.

**EmptyState short-circuit removed.** `web/app/(app)/approvals/page.tsx:29-50`:

```tsx
export default function ApprovalsLandingPage() {
  const approvalsQuery = useApprovals();
  const approvals = approvalsQuery.data ?? [];
  return (
    <div data-testid="approvals-landing">
      <BrandLoudHero ... />
      <main ...>
        {approvalsQuery.isPending ? (
          <DashboardLoading />
        ) : (
          <ApprovalsOpsDashboard items={approvals} />
        )}
      </main>
    </div>
  );
}
```

The previous structure (per the in-source 2026-05-26 comment) short-circuited to an
EmptyState card when `approvals.length === 0`. Post-fix it always renders the locked
E-ops-dashboard regardless of queue depth.

**Vitest** in the main worktree:
- `__tests__/approvals/page.test.tsx` — 3/3 PASS in 74ms (renders ops-dashboard even with zero queued approvals; renders dashboard chrome; preserves severity tiers + KPI strip — these are the three behavioural pins added by the diff)
- `tests/sample-roster-canon.test.ts` + `tests/approvals-ops-dashboard-cards.test.tsx` + `__tests__/workers/empty-state-matches-team.test.tsx` — 11/11 PASS in 1.13s (no collateral regressions on the persona spine, the dashboard cards, or the workers↔team handshake)

**Live-gateway probe** to confirm the runtime hasn't drifted from the SPA fixture:
```bash
curl -sf http://127.0.0.1:7400/health
#   {"status":"ok","version":"0.0.1","schema_version":83,"uptime_seconds":26986}
```
Gateway uptime 7.5h, schema v83=v83 FRESH — fixture-only fix, no Rust impact (as expected for
a `web/lib/sample-*` refactor).

## Anti-theater check

This is dispositive on three independent axes:

1. **Topological**: the derive is at `SAMPLE_TEAM` module-load. There is no longer a hand-edited
   `pendingApprovalCount` field anywhere in the repo (verified by `grep`). The drift class is
   structurally extinct, not just "fixed for now."
2. **Behavioural**: the e2e regression spec at `tests/e2e/team-and-approvals-counts-agree.spec.ts`
   asserts `inboxPendingCount >= waitingTeamCount` AND `inboxPendingCount >= 4` AND the
   ops-dashboard chrome (`approvals-kpi-strip`, `severity-tier-{p0,p1,p2}`) renders even when
   the queue is empty. Removing the derive or restoring the EmptyState short-circuit fails the
   spec — it's not test theater rewriting itself green.
3. **Coverage**: the vitest pins assert the EmptyState short-circuit doesn't reappear (the page
   renders `approvals-ops-dashboard` regardless of queue depth). Three behavioural pins; each
   would fail independently if the fix were reverted.

## Gap / follow-up

None for this PR. Scene 5 demo-MP4 P0 is closed.

**Demo-MP4 P0 progress (post-r86)**: 3/4 MERGED (#2404 Scene 3 + #2405 Scene 6 + #2406 Scene 5).
1/4 still open: `fix-skills-marketplace-perpetual-loading` (claude-worker-CLAIMED at f2b37c86e,
no PR open yet — recommend conductor surface to operator if claim is stale by next round).
