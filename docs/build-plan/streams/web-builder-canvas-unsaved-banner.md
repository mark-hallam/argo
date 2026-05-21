# web-builder-canvas-unsaved-banner

**Authored against**: `origin/main` `72e782df` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-builder-canvas-unsaved-banner`.

## Summary

Deferred by #1944 (canvas controlled-state foundation). The `useWorkflowBuilderState` hook exposes a `dirty` flag, but #1944 did not surface an unsaved-changes banner at the top of the canvas. Land that banner now.

Pre-condition: #1944 MERGED ✅. Pre-condition: #1951 (`web-builder-page-split`) is in-flight — if it MERGES before this brief is claimed, the worker should consume the new component shape; if not, the worker adds the banner inside the existing `build/page.tsx` structure.

## Acceptance

- [ ] When `builder.dirty === true`, a banner renders at the top of the canvas region: "You have unsaved changes" + Save button + Discard button.
- [ ] Save button calls the existing save-publish path (TBD by #1882 status — if #1882 is still DIRTY, wire to the placeholder toast).
- [ ] Discard button calls `builder.reset()` (already exposed by the hook per #1944).
- [ ] Banner disappears when `dirty === false`.
- [ ] `prefers-reduced-motion` respected (no slide-down animation when reduced).
- [ ] Unit test: dragging a node flips `dirty` true → banner appears; clicking Discard → banner disappears + canvas returns to initial nodes.
- [ ] Banner uses the canonical D3 register (consistent with the locked builder design — Inter, restrained, no Audiowide).

## May touch

- `web/app/(app)/workflows/[verticalId]/[name]/build/page.tsx` (or wherever the canvas region renders after #1951 splits it)
- `web/components/skycot/builder/UnsavedBanner.tsx` (new file)
- New unit test under the builder test dir

## Must NOT touch

- `web/lib/hooks/use-workflow-builder-state.ts` (the hook is already correct; consume its `dirty` and `reset`)
- ReactFlow canvas internals
- Save-publish endpoint wiring (that's #1882's scope)
- Other workflow-builder pages

## Notes

- The `dirty` flag was exposed at #1944. See `web/lib/hooks/use-workflow-builder-state.ts` for the surface.
- If #1951 (`web-builder-page-split`) has merged, expect `BuilderToolbar.tsx` to be the natural mount point. Read the current shape before authoring.
- Brief explicitly declared by the author of #1944 in the PR body ("dirty flag is exposed for a follow-up").
