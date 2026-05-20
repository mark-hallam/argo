# web-workflow-builder-multi-branch-rendering — render Decision-node branches side-by-side

**Owner**: claude. **Sizing**: ~150-250 LOC.
**Authored against**: origin/main `fe35cb61` on `2026-05-20`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-workflow-builder-multi-branch-rendering`.
**Priority**: P1 — fills the most-visible visual gap vs n8n. Decision nodes today render as a single linear step; n8n renders true/false branches side-by-side.

## Scope

Extend `SkycotStepNode` + `stepsToFlow()` in `/build/page.tsx` so that Decision-kind steps:

1. Render two output handles (left = "true / continue", right = "false / branch")
2. Subsequent steps lay out in 2 columns (left = continue path, right = branch path)
3. Edges connect the right handle to the branch's first node
4. Branches can re-merge — handle the "join" case where both paths converge on a later node

## Acceptance criteria

- [ ] Decision node renders two clearly-labeled output handles
- [ ] Auto-layout places branches in adjacent columns
- [ ] Edges colored differently for true (purple) vs false (amber) paths
- [ ] Re-merge case handled correctly (a single node receiving both incoming edges)
- [ ] Existing linear workflows still render correctly
- [ ] Mini-map shows the branched topology accurately
