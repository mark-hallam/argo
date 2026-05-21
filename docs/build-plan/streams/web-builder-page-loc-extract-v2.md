# web-builder-page-loc-extract-v2

**Authored against**: `origin/main` `c4a78625f` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-builder-page-loc-extract-v2`.

## Summary

`web/app/(app)/workflows/[verticalId]/[name]/build/page.tsx` is **656 LOC** post-#1964, well over the 500-LOC HARD RULE 6 ceiling. The #1951 split (`web-builder-page-split`) brought it down to 260, but subsequent merges (#1959 connect, #1960 delete-duplicate, #1961 keyboard-shortcuts, #1964 AI-proposal) re-grew it. This brief extracts coherent slices to get under the ceiling without touching behavior.

This is the v2 of `web-builder-page-split` — same goal, different cuts (the original split is settled and merged).

## Acceptance

- [ ] `build/page.tsx` reduced to ≤500 LOC (target: ~350, leaving headroom for future small additions).
- [ ] Behavior unchanged: every existing E2E spec and unit test passes.
- [ ] Suggested extractions (open to author judgment):
  - **`builder-handlers/delete-duplicate.ts`** — the `onNodesDelete`, `onEdgesDelete`, `duplicateNode`, `onNodeContextMenu`, `onEdgeContextMenu`, `onPaneClick`, `closeContextMenu`, `contextMenuItems` closures (~100 LOC).
  - **`builder-handlers/history.ts`** — the `captureSnapshot`, `onNodeDragStart`, `onNodeDragStop`, `handleUndo`, `handleRedo`, `handleSelectAll`, `handleDuplicateSelected` closures + `stateRef`/`preDragSnapshotRef` plumbing (~80 LOC).
  - **`builder-handlers/proposal.ts`** — the AI-proposal state + `handleProposeChange` (~50 LOC).
- [ ] Helpers consume `builder` (the `useWorkflowBuilderState` return) as a parameter so they're testable in isolation.
- [ ] No new dependencies on @xyflow internals — extracted helpers use the same Node/Edge types as today.
- [ ] All existing E2E specs (`tests/e2e/builder-*.spec.ts`) pass against the post-extract page.

## May touch

- `web/app/(app)/workflows/[verticalId]/[name]/build/page.tsx` (the shrinkage target)
- New files under `web/app/(app)/workflows/[verticalId]/[name]/build/builder-handlers/`
- Existing `builder-handlers.ts` (extend if cleaner than a new sub-dir)
- Test files for the new helpers (additive)

## Must NOT touch

- The `useWorkflowBuilderState` hook (settled by #1944).
- `BuilderCanvas.tsx`, `BuilderToolbar.tsx`, `BuilderInspector.tsx`, `BuilderCanvasNode.tsx`, `BuilderNodePalette.tsx`, `BuilderAiProposal.tsx` — components are correct; only the orchestrator page needs surgery.
- `KeyboardShortcutsModal.tsx` (settled by #1961).
- The streaming-proposal infrastructure in `propose-toml.ts` (#1964).

## Notes

- Sizing: 400-700 LOC delta total (page.tsx shrinks, new helper files appear).
- Pre-flight: read `build/page.tsx` end to end. Identify natural seams. The delete-duplicate group and the history group are the two biggest blocks.
- Operator memory: HARD RULE 6 is a hard ceiling, not a soft target. This brief restores compliance.
- Follow-up risk: future builder briefs (e.g., the deferred `web-builder-duplicate-handler-consolidate` from auditor's P1) will be easier to bound if this extract lands first.
