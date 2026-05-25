# Round 64 — PR #2301 `fix: source sub-workflow picker from workflows API` PRE-MERGE PASS

**Date**: 2026-05-25 (Melbourne 2026-05-26 00:14 AEST · UTC 2026-05-25T14:14Z)
**Round**: 64
**Pillar / Claim**: Scene-2 demo flow (workflow builder camera-visible) · Mission claim 3 partial (platform-builds-workers — workflow definitions sourced from real gateway, not hardcoded fixture)
**PR head**: `c9ad41aefcc4c0f975a699e5a2a9fb9aa3189cd8` · MERGEABLE per gh · 2 files +161/-14
**Verdict**: 🟢 **PRE-MERGE PASS · P0 demo-blocker fix · MERGE-READY**

---

## Setup

```bash
git fetch origin 'refs/pull/2301/head:refs/remotes/origin/pr-2301'
git worktree add -f /tmp/argo-r64-pr2301-wt origin/pr-2301
ln -s ~/code/argo/web/node_modules /tmp/argo-r64-pr2301-wt/web/node_modules

# PR-head suite
cd /tmp/argo-r64-pr2301-wt/web
pnpm exec vitest run 'app/(app)/workflows/[verticalId]/[name]/build/SubWorkflowPicker.test.tsx' --reporter=verbose

# Anti-theater: same test against main pre-fix
cp '/tmp/argo-r64-pr2301-wt/web/app/(app)/workflows/[verticalId]/[name]/build/SubWorkflowPicker.test.tsx' \
   '~/code/argo/web/app/(app)/workflows/[verticalId]/[name]/build/SubWorkflowPicker.test.tsx'
cd web && pnpm exec vitest run 'app/(app)/workflows/[verticalId]/[name]/build/SubWorkflowPicker.test.tsx' --reporter=verbose
rm '~/code/argo/web/app/(app)/workflows/[verticalId]/[name]/build/SubWorkflowPicker.test.tsx'
```

---

## Observed

### PR-head test run — 3/3 PASS in 853ms

```
✓ SubWorkflowPicker > uses gateway workflows before local seed workflows               25ms
✓ SubWorkflowPicker > falls back to seed workflows only when the gateway list is empty  9ms
✓ SubWorkflowPicker > shows a loading state without rendering seed options while the
   query is pending                                                                     3ms

Test Files  1 passed (1)
     Tests  3 passed (3)
  Duration  853ms
```

### Anti-theater run — main pre-fix code = 2/3 FAIL · 1/3 vacuous-pass

```
FAIL > uses gateway workflows before local seed workflows
  expected `screen.getByTestId("sub-workflow-picker-option-sales-customer_research_followup")` to render
  found instead: <button data-testid="sub-workflow-picker-option-gtm-engineer-post_listing"> (from seed)

FAIL > shows a loading state without rendering seed options while the query is pending
  expected `getByTestId("sub-workflow-picker-loading")` to exist
  found instead: seed-workflow `gtm-engineer/post_listing` rendered

PASS > falls back to seed workflows only when the gateway list is empty
  (vacuously passes — main always renders seed workflows regardless of gateway state)

Test Files  1 failed (1)
     Tests  2 failed | 1 passed (3)
```

The 1 passing test is the "fallback when gateway empty" case. On main pre-fix, the picker **always** uses `SEED_WORKFLOWS` (no `useWorkflows()` call exists), so the "fall back when empty" assertion vacuously holds. The 2 failing tests are the **contract-shifting** assertions (gateway-first lookup + loading state) — both fail because the production code on main has no gateway integration whatsoever.

### Production wire (PR head)

`SubWorkflowPicker.tsx:81-95` (PR head):

```tsx
const { data, isLoading } = useWorkflows();
...
const candidates = useMemo(() => {
  if (data?.workflows.length) {
    return buildWorkflowCandidates(data.workflows, currentVerticalId, currentName);
  }
  if (isLoading) return [];
  return buildFallbackCandidates(currentVerticalId, currentName);
}, [currentVerticalId, currentName, data?.workflows, isLoading]);
```

Three-state branch:
1. Gateway returns >0 workflows → use them, ignore `SEED_WORKFLOWS`.
2. Gateway is loading (initial render) → render empty + `Loading workflows…` row.
3. Gateway returned empty list (or errored, since `useWorkflows()` collapses errors to `data: undefined` per existing hook shape) → fall back to `SEED_WORKFLOWS` so the demo doesn't regress to a blank picker.

This is the right contract: real workflows when the gateway has them, graceful fallback when not, no static fixture leaking into camera-visible Scene-2 if the gateway is alive.

`buildWorkflowCandidates()` excludes the current workflow from the picker (`workflow.vertical === currentVerticalId && workflow.name === currentName`) — matches the seed-fallback exclusion semantics on main.

LOC budget: SubWorkflowPicker.tsx is 332 LOC on PR head (51 LOC added; well under 500).

---

## Anti-theater check

Three test cases, each with a deliberate failure mode on main:

1. **"uses gateway workflows before local seed workflows"** — mocks `useWorkflows()` to return a single gateway workflow `customer_research_followup`. Test asserts that test-ID is present AND that the seed test-ID `gtm-engineer-post_listing` is NOT. On main, the gateway workflow is never queried, so the seed `gtm-engineer/post_listing` renders — assertion fails.
2. **"shows a loading state"** — mocks `useWorkflows()` to return `{data: undefined, isLoading: true}`. Test asserts `sub-workflow-picker-loading` test-ID exists AND no seed options. On main, no loading state exists, so `getByTestId('sub-workflow-picker-loading')` throws not-found.
3. **"falls back to seed when gateway empty"** — mocks `useWorkflows()` to return `{data: {workflows: []}}`. Test asserts the seed option renders. **Vacuously passes on main** because main always renders seed; passes on PR head because empty-list branch falls through to `buildFallbackCandidates()`. Useful as positive control once PR ships.

The two failing-on-main tests are real contract-shifting assertions; the one vacuously-passing test is a positive control that becomes load-bearing post-merge (catches regression if someone removes the fallback branch).

---

## Gap / follow-up brief candidates

None new. PR closes a Scene-2 demo-blocker without scope creep.

**Boundary note from PR body**: same `@next/eslint-plugin-next` resolution issue as #2299 in fresh checkouts; doesn't affect vitest or typecheck. Not a merge blocker.

**Follow-on (already authored elsewhere)**: gateway `/api/v1/verticals` envelope mismatch surfaced in round-63 (#2300 P0 BLOCK) is independent of this PR — `useWorkflows()` hits the workflows endpoint, not verticals — so #2301 doesn't depend on #2300's fix.

---

## Merge recommendation

🟢 **MERGE via `conductor-pre-merge-gate.sh --skip-clippy`** — Scene-2 demo flow is camera-visible (workflow builder → sub-workflow picker). PR is well-scoped (1 production file + 1 new test file), tests are real regression-pins, and the fallback branch preserves the demo even if the gateway is offline.

49th anti-theater-honest verification this week.
