# web-workflow-builder-live-execution-overlay — n8n-parity green-checks-as-steps-execute

**Owner**: claude. **Sizing**: ~300-500 LOC.
**Authored against**: origin/main `fe35cb61` on `2026-05-20`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-workflow-builder-live-execution-overlay`.
**Priority**: P1 — n8n's "node turns green as it executes" is one of the most-celebrated UX moments in the category.

## Scope

When the user clicks "Test run" in the builder header:

1. Open an SSE stream to gateway `/api/v1/workflows/<id>/runs?stream=true` (gateway-side brief authoring needed if endpoint doesn't exist)
2. As each step starts: pulse the node's border in yellow + show a tiny spinner overlay
3. As each step succeeds: green-check overlay + edge animates faster
4. As each step fails: red border + error tooltip on hover
5. Final summary banner: "Test run complete. 6/6 steps succeeded. Total: 4.2s, $0.014."
6. Below the canvas, a collapsible "Run details" tray with per-step timing + token usage + LLM responses

## Acceptance criteria

- [ ] Test run triggers a real workflow execution on the gateway
- [ ] Each step's status updates live on the canvas
- [ ] Run details tray collapsible, shows per-step metadata
- [ ] Failures surface error messages helpfully
- [ ] Mini-map indicates per-step status too
- [ ] No memory leaks when stream closes
