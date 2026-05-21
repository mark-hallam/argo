# web-builder-page-split

**Authored against**: `origin/main` 349b6acc on 2026-05-21.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-builder-page-split`.

## Summary

`web/app/(app)/workflows/[verticalId]/[name]/build/page.tsx` is 1063 LOC after #1944 landed (controlled-state foundation). HARD RULE 6 ceiling is 500. This brief splits the page into a `BuilderInner` wrapper + extracted panels so each file is under 500 LOC.

Pre-condition: #1944 (controlled-state foundation) MERGED. ✅ Met.

## Acceptance

- [ ] `build/page.tsx` reduced to ≤500 LOC (target: ~300 — provider wrapper + state hook wiring + top-level shell).
- [ ] Each extracted file under 500 LOC.
- [ ] Suggested extraction (open to author judgment):
  - `BuilderToolbar.tsx` (top bar: save / publish / test-run buttons + dirty indicator)
  - `BuilderNodePalette.tsx` (left drawer: draggable node types)
  - `BuilderInspector.tsx` (right drawer: selected-node editor)
  - `BuilderCanvas.tsx` (the `<ReactFlow>` mount + provider)
- [ ] `useWorkflowBuilderState` hook (from #1944) remains the single state source — extracted components consume via prop drilling or a thin context.
- [ ] All existing E2E + unit tests pass unchanged.
- [ ] No regression: drag-to-reorder still round-trips (verify the unit test `onNodesChange applies position updates via applyNodeChanges` from #1944 still passes).
- [ ] No regression: connect-and-add-step / delete-and-duplicate / keyboard-shortcuts briefs (still BLOCKED post-#1944) can land against the new shape without rework.

## May touch

- `web/app/(app)/workflows/[verticalId]/[name]/build/page.tsx`
- New files under `web/components/skycot/builder/` (or co-located in the build/ dir if more idiomatic)
- `web/lib/hooks/use-workflow-builder-state.ts` (only if a context shim is added — prefer prop drilling first)

## Must NOT touch

- Other workflow-builder pages or specs unrelated to `build/page.tsx`.
- ReactFlow library configuration outside the canvas component.

## Notes

- Declared as a follow-up gap by #1944 PR body. HARD RULE 6 pre-existed #1944 (page was 1046 → 1063) — this brief is paying that debt forward.
- The three downstream BLOCKED briefs (connect-and-add-step, delete-and-duplicate, keyboard-shortcuts-and-snap) can claim concurrently after this brief, but author chooses sequencing in their PR body.
