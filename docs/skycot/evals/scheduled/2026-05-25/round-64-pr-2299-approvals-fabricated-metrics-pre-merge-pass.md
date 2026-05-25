# Round 64 — PR #2299 `fix: remove fabricated approvals ops metrics` PRE-MERGE PASS

**Date**: 2026-05-25 (Melbourne 2026-05-26 00:14 AEST · UTC 2026-05-25T14:14Z)
**Round**: 64 (scheduled-task hourly fire; ~1h after round 63 ship 13:08Z)
**Pillar / Claim**: Anti-theater (operator-locked 2026-05-24 "NO THEATER IN DEMO") · Scene-6 demo-gate · Mission claim 6 partial (multi-tenant data honesty)
**PR head**: `f6504073c4ad0514595d8e6dd3b5a169e119aedb` · MERGEABLE per gh · 2 files +168/-25
**Verdict**: 🟢 **PRE-MERGE PASS · P0 demo-blocker fix · MERGE-READY**

---

## Setup

```bash
# 1. Fetch PR head into local ref
git fetch origin 'refs/pull/2299/head:refs/remotes/origin/pr-2299'

# 2. Create isolated worktree
git worktree add -f /tmp/argo-r64-pr2299-wt origin/pr-2299

# 3. Symlink node_modules (cp would OOM under memory pressure; vitest doesn't write into node_modules)
ln -s ~/code/argo/web/node_modules /tmp/argo-r64-pr2299-wt/web/node_modules

# 4. Run PR vitest suite on PR head
cd /tmp/argo-r64-pr2299-wt/web
pnpm exec vitest run components/skycot/approvals/ApprovalsOpsDashboard.test.tsx --reporter=verbose

# 5. Run the same test against main pre-fix (anti-theater proof)
cp /tmp/argo-r64-pr2299-wt/web/components/skycot/approvals/ApprovalsOpsDashboard.test.tsx \
   web/components/skycot/approvals/ApprovalsOpsDashboard.test.tsx
cd web && pnpm exec vitest run components/skycot/approvals/ApprovalsOpsDashboard.test.tsx --reporter=verbose
rm ~/code/argo/web/components/skycot/approvals/ApprovalsOpsDashboard.test.tsx
```

---

## Observed

### PR-head test run — 3/3 PASS in 892ms

```
✓ ApprovalsOpsDashboard > shows honest placeholders when no resolved rows or reviewer roster exist   39ms
✓ ApprovalsOpsDashboard > derives approval-rate visuals from resolved approval rows                   11ms
✓ ApprovalsOpsDashboard > does not render the old fabricated baseline copy                            6ms

Test Files  1 passed (1)
     Tests  3 passed (3)
  Duration  892ms
```

### Anti-theater run — main pre-fix code = 3/3 FAIL

The same test file, run against `origin/main`'s pre-fix `ApprovalsOpsDashboard.tsx`, fails on every assertion:

```
FAIL  components/skycot/approvals/ApprovalsOpsDashboard.test.tsx > does not render the old fabricated baseline copy
Error: expected document not to contain element, found <div>7-day baseline</div> instead
 ❯ components/skycot/approvals/ApprovalsOpsDashboard.test.tsx:97:55
     97|     expect(screen.queryByText(/7-day baseline/i)).not.toBeInTheDocumen…

Test Files  1 failed (1)
     Tests  3 failed (3)
```

### Production-code grep — fabricated constants exist on main, deleted in PR

`grep -n "7-day baseline\|78%\|14%\|8%\|0.78\|0.14\|0.82" web/components/skycot/approvals/ApprovalsOpsDashboard.tsx` on **main**:

```
77:  const approvalRate = decided.length > 0 ? approved / decided.length : 0.78;
102:      sub: decided.length > 0 ? "from resolved rows" : "7-day baseline",
138:          headline: "78%",
139:          slices: { approved: 0.78, refused: 0.14, expired: 0.08 },
163:            percent={totalAtRisk > 0 ? 0.82 : 0}
180:              <ApprovalRateRing approved={0.78} refused={0.14} expired={0.08} animate />
186:                  78% approved · 14% refused · 8% expired
```

PR head: zero matches (fabricated literals deleted; replaced by `computeApprovalRate()` derived from `item.decision` field, with `—` placeholders and `tone: "neutral"` when no resolved rows exist).

### What the production wire actually does

`ApprovalsOpsDashboard.tsx:74-90` (PR head):

```ts
function computeApprovalRate(items: ApprovalItem[]): ApprovalRateSummary | null {
  const decided = items.filter((item) => item.decision != null);
  if (decided.length === 0) return null;
  const approved = decided.filter((item) => item.decision === "approve").length;
  const refused = decided.filter((item) => item.decision === "refuse").length;
  return { total: decided.length, approved: approved / decided.length, refused: refused / decided.length, expired: 0 };
}
```

The KPI card consumes `approvalRate` (nullable) and renders `"—"` + `"reviewer roster pending"` / `"no resolved approvals"` placeholder strings when the function returns null. The donut card (`MetricDonutCard`) accepts `percent?: number` — when undefined, the conic-gradient fill switches to `var(--muted)` and `data-percent="none"` (used by the test).

---

## Anti-theater check

Three distinct regression-pin assertions, each grounded in production code:

1. **"7-day baseline" string deletion** — pre-fix has it at line 102; test queries `/7-day baseline/i` with `not.toBeInTheDocument()`. Test FAILS on main (would catch any revert).
2. **`approved={0.78}` / `refused={0.14}` / `expired={0.08}` hardcoded prop deletion** — pre-fix has these at line 180; test asserts the `<ApprovalRateRing>` props match derived percentages (50/50 in the decided-rows scenario, vs 78/14/8 fabricated baseline).
3. **`percent={0.82}` hardcoded donut fill deletion** — pre-fix has it at line 163; test asserts `data-percent="none"` when no monetary exposure exists.

Three resolved-decision items produce data-derived percentages (`50%` for the 1-approve + 1-refuse case, not the 78% fabricated baseline). The "old fabricated baseline copy" test is a pure anti-regression pin — checks 4 fabricated strings are absent. Test #2 (`derives ... from resolved approval rows`) requires the production wire to actually compute from `item.decision`, which the PR adds.

LOC budget: ApprovalsOpsDashboard.tsx is 374 LOC on PR head (well under 500).

---

## Gap / follow-up brief candidates

None new. The PR is shipped as designed.

**Boundary note from PR body** (operator may want to know): `@next/eslint-plugin-next` is not resolvable from `web/eslint.config.mjs` in fresh worktrees, so focused ESLint cannot start there. This matches the existing web-dependency boundary issue reported in prior PRs. Not a merge blocker; doesn't affect vitest or typecheck.

---

## Merge recommendation

🟢 **MERGE via `conductor-pre-merge-gate.sh --skip-clippy`** (operator-locked auth for evaluator-PRE-MERGE-PASS'd PRs within ~6h). This PR closes a P0 Scene-6 demo-gate finding — the ux-audit @ `docs/skycot/ux-audits/2026-05-25-1100-scene-6-claim-6-trust-and-approvals.md` called out the 78%/14%/8% fabricated baseline as camera-visible theater in the v1.0 demo flow.

48th anti-theater-honest verification this week.
