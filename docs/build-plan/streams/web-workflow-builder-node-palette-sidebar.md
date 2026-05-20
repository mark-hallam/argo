# web-workflow-builder-node-palette-sidebar — drag-to-add node palette

**Owner**: claude. **Sizing**: ~250-400 LOC.
**Authored against**: origin/main `fe35cb61` on `2026-05-20`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-workflow-builder-node-palette-sidebar`.
**Priority**: P2 — fills the n8n / UIpath "drag-to-add" power-user pattern.

## Scope

1. Add a left sidebar (collapsible) listing all step kinds + skill catalog
2. Each entry is draggable to the canvas
3. Drop creates the new node at the drop location + wires it into the nearest predecessor
4. Search field at the top filters the palette
5. Group palette items by type:
   - Triggers (manual, schedule, webhook, email, slack)
   - Skills (browse the skill marketplace inline)
   - Logic (decision, switch, loop, wait, merge)
   - Approval gates (Slack, Email, Web)
   - Tools (HTTP, browser, code)
   - Notify (Slack DM, email, web inbox)

## Acceptance criteria

- [ ] Left palette renders with grouped items
- [ ] Drag from palette → drop on canvas creates a new node with sensible defaults
- [ ] Search filters items in real time
- [ ] Palette collapsible (header click) so power users can hide it
- [ ] Existing FAB + AI prompt suggestion chips still work
- [ ] No drag-collision with React Flow's existing drag handles
