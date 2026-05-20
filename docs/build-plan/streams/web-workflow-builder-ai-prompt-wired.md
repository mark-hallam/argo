# web-workflow-builder-ai-prompt-wired — wire the visual builder's AI prompt to live streaming proposals

**Owner**: claude. **Sizing**: ~200-300 LOC.
**Authored against**: origin/main `fe35cb61` on `2026-05-20`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-workflow-builder-ai-prompt-wired`.
**Priority**: **P0** — closes the SKYCOT competitive moat (n8n / UIpath have no AI editing; Replit Agent has no visual canvas). The builder ships the UI but the "Propose" button is a stub today.

## Scope

The builder at `/workflows/<v>/<n>/build/page.tsx` has a prompt input + "Propose" button + suggestion chips. Today clicking Propose triggers a 1.2s spinner stub. This brief wires it to the existing `/api/workflows/propose` Next.js route (which forwards to gateway).

What ships:

1. **Stream the assistant response** — display tokens as they arrive in a panel below the prompt bar OR in a side overlay.
2. **Render the proposed diff** — the assistant returns a proposed workflow TOML; parse it back into `WorkflowStep[]` + show the new graph in a "before / after" overlay, with added/removed/changed nodes highlighted.
3. **Approve / Reject / Edit** buttons:
   - Approve → POST to gateway workflow-edit (P1 #6) + flash success toast + replace canvas with new graph
   - Reject → close panel + restore original
   - Edit → drop the proposed TOML into a textarea for manual tweaks before re-approving
4. **Streaming-error handling** — gateway 5xx / network → user-friendly retry CTA
5. **Suggestion chips** wire to set the prompt + auto-submit

## Acceptance criteria

- [ ] Typing in the prompt + Enter or click Propose sends the request to /api/workflows/propose
- [ ] Assistant response streams visibly (token-by-token)
- [ ] Proposed diff renders inline; original nodes stay visible faded behind
- [ ] Approve persists to gateway + updates canvas
- [ ] Reject discards proposal cleanly
- [ ] 5xx / network error shows retry path
- [ ] No console errors during the flow
- [ ] All current builder tests still pass
