# web-workflows-loop-and-wait-nodes — Loop-over-items + Wait/delay node kinds

**Owner**: claude. **Sizing**: ~150-250 LOC.
**Authored against**: origin/main `1937a225` on `2026-05-20`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-workflows-loop-and-wait-nodes`.
**Priority**: **P0** — n8n parity. Loops + Waits are the two most-used control-flow primitives in n8n. The SKYCOT builder ships Decision + Approval but has no Loop or Wait node, leaving a visible gap in any non-trivial workflow.

## Scope

Extend the visual builder at `web/app/(app)/workflows/[verticalId]/[name]/build/page.tsx` (plus `SkycotStepNode` + `stepsToFlow()` helpers) so two new step kinds render and connect cleanly in React Flow.

### Part A — Loop-over-items node

1. New step kind `loop` with config:
   - `items_path` (string) — JSON-path / variable reference into the upstream payload (e.g. `$.leads`)
   - `item_var_name` (string, default `item`) — name the per-iteration variable gets bound to
   - `max_iterations` (number, default 100) — guard rail
2. Node renders as a distinct shape (rounded rectangle with a small loop-arrow glyph in the header) and a single output handle labelled "for each item".
3. Children of the loop step layout inline beneath the loop, indented one column, with a dashed left rail indicating "this is the loop body".
4. Last node in the loop body wires back to the loop header with a dashed return edge (visual only; runtime semantics are gateway-owned).

### Part B — Wait / delay node

1. New step kind `wait` with config:
   - `duration_seconds` (number) — fixed delay
   - OR `until_iso` (string) — absolute timestamp
   - OR `until_field` (string) — JSON-path into the payload that resolves to an ISO timestamp
2. Node renders as a clock-icon header + duration summary ("Wait 30s" / "Wait until 14:00 UTC" / "Wait until $.followup_at").
3. Single input handle, single output handle. Connects like any linear step.

### Shared

- Both nodes appear in the Add-step palette (existing FAB menu) under a new "Logic" group alongside Decision.
- Both nodes inherit the existing SkycotStepNode visual treatment (gradient header on hover, drag handles).
- TOML/JSON encoding (when builder writes back to gateway) matches the existing step schema — add `kind: "loop"` and `kind: "wait"` cases to the encoder.
- TypeScript types: extend `WorkflowStepKind` union in `web/lib/types/workflow.ts` (or wherever the step types live — grep first).

## Acceptance criteria

- [ ] Loop node renders with iterator config (`items_path`, `item_var_name`, `max_iterations`).
- [ ] Loop body indents one column with dashed left rail.
- [ ] Loop's last child wires back to the loop header with a dashed return edge.
- [ ] Wait node renders with duration config (one of `duration_seconds`, `until_iso`, `until_field`).
- [ ] Wait node summary text reflects the chosen mode ("Wait 30s" / "Wait until 14:00 UTC" / "Wait until $.followup_at").
- [ ] Both kinds connect through React Flow with proper input/output handles.
- [ ] Both kinds appear in the Add-step palette under "Logic".
- [ ] Existing linear + decision workflows still render correctly.
- [ ] `pnpm typecheck && pnpm test` clean.

## Out of scope

- Backend execution semantics (gateway owns loop iteration + wait scheduling — separate brief if gateway support is missing).
- Loop concurrency controls (parallel vs sequential) — defer to v1.x.
- Persistent wait-state recovery on worker restart — gateway concern.
