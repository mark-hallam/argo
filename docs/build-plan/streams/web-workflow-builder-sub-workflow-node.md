# web-workflow-builder-sub-workflow-node — render sub-workflows as expandable group nodes

**Owner**: claude. **Sizing**: ~250-400 LOC.
**Authored against**: origin/main `fe35cb61` on `2026-05-20`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-workflow-builder-sub-workflow-node`.
**Priority**: P1 — n8n's most-requested power-user feature. Backend already supports sub-workflow invocation.

## Scope

1. New `SubWorkflowNode` component that renders a workflow-as-a-single-node card showing:
   - Sub-workflow name + vertical
   - Step count inside (e.g. "6 steps")
   - "Expand" affordance — clicking expands inline OR opens the sub-workflow in a new tab

2. Update `stepsToFlow()` to recognize step-kind `sub_workflow` and render with the new node type.

3. Add a "Call sub-workflow…" option to the Add step palette that opens a workflow picker.

## Acceptance criteria

- [ ] A workflow with a sub_workflow step renders the new node correctly
- [ ] Click "Expand" opens an inline overlay OR navigates to the sub-workflow detail
- [ ] Add Step → "Call sub-workflow…" lets you pick from existing workflows
- [ ] Mini-map represents sub-workflow nodes distinctly (e.g. different shape)
