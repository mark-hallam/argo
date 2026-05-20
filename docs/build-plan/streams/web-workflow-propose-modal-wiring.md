# web-workflow-propose-modal-wiring — wire "Propose new" CTA to chat-propose flow

**Owner**: claude. **Sizing**: ~200-350 LOC (modal wrapper + worker-id picker + chat embed).
**Authored against**: origin/main `3121e32e` on `2026-05-20`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-workflow-propose-modal-wiring`.
**Priority**: P1 — the /workflows catalog has a brand-loud "Propose new" gradient CTA that currently does nothing. The `ProposeWorkflowChat` component exists and the gateway `/api/workflows/propose` route exists. This brief connects them.

## Scope

### 1. New modal: `WorkflowProposeModal`
Path: `web/components/skycot/WorkflowProposeModal.tsx`
- Wraps `<ProposeWorkflowChat>`
- Worker-id picker at top (defaults to first eligible worker; dropdown to switch)
- Vertical-id picker (auto-fills based on worker)
- Title + "x" close button
- Approval flow: streaming chat → user approves → modal closes + toast "Proposed workflow lands when reviewed"

### 2. Wire the CTA in `/workflows/page.tsx`
Replace the existing `<button>` Propose new with one that opens the modal. State managed at the page level.

### 3. Add "Edit in chat" affordance
The `/workflows/[v]/[n]` visual viewer has an "Edit in chat" button that currently does nothing. Wire it to open the same modal pre-filled with the existing workflow's TOML, so the chat can describe diffs.

## Acceptance criteria

- [ ] Clicking "Propose new" opens the modal with workflow chat ready to accept input.
- [ ] Submitting a proposal streams the assistant response.
- [ ] User can approve → modal closes + success toast.
- [ ] "Edit in chat" on a viewer page opens modal with workflow context preloaded.
- [ ] Modal traps focus, closes on Escape, has a working "x" button.
- [ ] Mobile responsive (full-screen modal under 720px).

## Out of scope

- The streaming endpoint behavior — already implemented in `/api/workflows/propose`.
- Workflow approval persistence — already implemented in gateway (#1860).
