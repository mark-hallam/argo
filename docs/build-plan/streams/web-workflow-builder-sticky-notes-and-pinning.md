# web-workflow-builder-sticky-notes-and-pinning — annotations + input-data pinning

**Owner**: claude. **Sizing**: ~200-350 LOC.
**Authored against**: origin/main `fe35cb61` on `2026-05-20`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-workflow-builder-sticky-notes-and-pinning`.
**Priority**: P2 — small but loved features that close the n8n parity feel.

## Scope

### Part A — Sticky notes
1. Right-click on empty canvas → context menu → "Add sticky note"
2. Yellow Post-it-style draggable note with editable text
3. Persists with the workflow (gateway will need a `notes[]` field on workflow doc)

### Part B — Input data pinning
1. On each input node, "Pin data" button opens a JSON editor
2. Pinned data overrides any live input during test runs
3. Visual indicator (📌) on nodes with pinned data
4. "Unpin" reverts to live

## Acceptance criteria

- [ ] Sticky notes can be added, edited, dragged, deleted
- [ ] Notes persist when the workflow is saved
- [ ] Pinning JSON on a node shows the pin icon + tooltip preview
- [ ] Test runs use pinned data when present
- [ ] No collision with React Flow's selection/drag interactions
