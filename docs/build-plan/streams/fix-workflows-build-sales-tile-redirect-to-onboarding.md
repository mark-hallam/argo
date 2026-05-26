# fix-workflows-build-sales-tile-redirect-to-onboarding

**Owner**: claude.
**Authored against**: origin/main `ddf44dfb4` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-workflows-build-sales-tile-redirect-to-onboarding`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-2007/scene-4-and-5-claim-4-and-5.md` F8 [P2].

🟡 **P2 — `/workflows/build` → click "Sales" tile → navigates to `/workflows/sales/untitled/build` → the SPA hydrates, finds no workflow named "untitled" in the gateway, and REDIRECTS to `/onboarding/workspace` instead of showing the blank-canvas mode. Operator picks "build a new workflow" and lands on "Name your workspace" — confusing wrong flow.**

## Root cause

`web/app/(app)/workflows/[verticalId]/[name]/build/page.tsx:69` defines `NEW_WORKFLOW_SENTINEL = "untitled"` and the `isNewWorkflow` branch is supposed to render the blank-canvas state. But in the live SPA something nudges the route to `/onboarding/workspace` instead. Likely an `onError` / `fallback` path on the builder page that calls `router.push("/onboarding/workspace")` when `useWorkflow` returns `null` — without checking whether the sentinel was the requested name.

## Findings this brief closes

- scene-4-and-5-claim-4-and-5 F8 [P2] — `/workflows/build` sales-tile redirect to /onboarding/workspace instead of blank canvas

## Fix recipe

1. **Trace the route divergence** in `web/app/(app)/workflows/[verticalId]/[name]/build/page.tsx:69-129`:
   - The `isNewWorkflow` branch should render `<BlankCanvas verticalId={verticalId} />` and bypass `useWorkflow` entirely
   - Any `useEffect` that redirects on a "workflow not found" condition must FIRST check `if (isNewWorkflow) return;` to skip the redirect

2. **Likely fix**:
   ```tsx
   const isNewWorkflow = name === NEW_WORKFLOW_SENTINEL;
   const { data: workflow, isError } = useWorkflow(verticalId, name, { enabled: !isNewWorkflow });
   
   useEffect(() => {
     if (isNewWorkflow) return; // don't redirect for blank-canvas
     if (isError) router.push(`/workflows/${verticalId}`);
   }, [isError, isNewWorkflow]);
   
   if (isNewWorkflow) return <BlankCanvas verticalId={verticalId} />;
   ```

3. **Add a Playwright spec** at `web/playwright/workflows/build-sales-tile-lands-blank-canvas.spec.ts`:
   - Navigate to `/workflows/build`
   - Click the Sales tile
   - Assert URL matches `/workflows/sales/untitled/build`
   - Assert blank-canvas UI is visible (e.g. `[data-testid="workflow-blank-canvas"]`)
   - Assert URL is NOT `/onboarding/workspace`

## Acceptance criteria

- [ ] `/workflows/build` → Sales tile → lands on blank canvas at `/workflows/sales/untitled/build`
- [ ] Same for the other 4 tiles (Support / GTM / People Ops / Finance)
- [ ] No redirect to /onboarding under any condition reachable from the builder picker
- [ ] New Playwright spec passes

## May-touch

- `web/app/(app)/workflows/[verticalId]/[name]/build/page.tsx`
- `web/app/(app)/workflows/build/page.tsx` (the picker, if its link handler is the bug)
- `web/playwright/workflows/build-sales-tile-lands-blank-canvas.spec.ts` (NEW)

## Out of scope

- The blank-canvas UI itself (presumed correct; preserve)
- The MCP/CU step kinds in the palette (covered by `fix-scene-5-customer-research-followup-workflow-and-mcp-cu-step-kinds`)
- The Builder regression cluster (covered by `fix-playwright-workflow-builder-regression-cluster`)

## Effort

~30-80 LOC + 1 spec. ~1-2 hours. P2 — small fix but unblocks the "new workflow from scratch" entry point.
