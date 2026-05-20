# Wave-13 GUI Workflow Canvas Deep Audit — SKYCOT `/workflows/<v>/<n>/build`

**Auditor:** Wave-13 Agent #2 (GUI Workflow Canvas)
**Date:** 2026-05-21
**Scope:** React Flow canvas at `web/app/(app)/workflows/[verticalId]/[name]/build/page.tsx` plus the custom `SkycotStepNode`, palette stub, side-panel, and trigger-config drawer
**Library:** `@xyflow/react` v12.10.2 (xyflow / React Flow v12, the canonical React Flow rewrite)
**Method:** Static read of `build/page.tsx` (1,121 LOC), `WorkflowBuilderSkeleton.tsx`, `WorkflowEdgeStates.tsx`, sibling `SkillCanvas.tsx` for comparison, plus xyflow v12 documented default behavior
**Competitive baseline:** n8n (open-source, the closest visual-builder reference), UIpath StudioWeb, Make.com (formerly Integromat), Zapier visual builder

---

## Executive summary

**Canvas score: 3.5 / 10.**

The canvas is a **read-only viewer dressed up as a builder**. xyflow's defaults give the operator a competent walk-through experience — nodes are draggable, pan works, zoom works, the minimap renders, the side-panel opens on click — but every single piece of state introduced by interaction is **discarded on the next render or page navigation**, and no editing primitives beyond "click a node to open a panel" are wired.

Concretely, looking at the React Flow props on line 995-1003:

```tsx
<ReactFlow
  defaultNodes={initial.nodes}
  defaultEdges={initial.edges}
  nodeTypes={nodeTypes}
  onNodeClick={onNodeClick}
  fitView
  fitViewOptions={{ padding: 0.18 }}
  attributionPosition="bottom-right"
>
```

That is the complete list of handlers and configuration. **No `onConnect`, no `onNodesChange`, no `onEdgesChange`, no `onNodesDelete`, no `onEdgesDelete`, no `onSelectionChange`, no `onDragEnd`, no `onConnectStart`, no `onConnectEnd`, no `snapToGrid`, no `snapGrid`, no `deleteKeyCode`, no `multiSelectionKeyCode`, no `selectionKeyCode`, no `nodesDraggable={false}` (so xyflow's drag-default applies), no `nodesConnectable={false}` either.**

Because `defaultNodes` + `defaultEdges` are passed (uncontrolled mode), React Flow maintains its **internal** node/edge state — drag a node and the new position lives inside the React Flow store. **It does not propagate back to the SKYCOT app state**, so:

- Operator drags a node — visible move happens.
- Operator clicks "Save + publish" — `handleSaveAndPublish` toasts a stub message ("Saved as draft — preview build" at line 712-716) and saves nothing.
- Operator navigates away and back — `useMemo(stepsToFlow(workflow.steps))` rebuilds `initial.nodes` from the SEED constant — drag is gone.

The "honest demo stub" comments on lines 679-696, 698-706, 708-716 acknowledge this (Wave-6 action-fires audit forced the rewrite). The builder is presented as a class-leading differentiator (page-header docblock at lines 44-69 calls it the n8n / UIpath / Replit competitive close) but **it does not yet build anything.**

The right-hand panel and the AI prompt bar are real UI; the canvas underneath them is theater.

---

## Walkthrough — answers to the 10 audit questions

### 1. Drag a node — does it persist position?

**No.** xyflow's default `nodesDraggable` is `true`, so drag visually works on the current page render. But:

- `onNodesChange` is not bound → no callback fires when the position updates internally
- `defaultNodes` is passed, not `nodes` → React Flow is in uncontrolled mode; the SPA layer never sees the new position
- `initial.nodes` is rebuilt every time `workflow` changes via `useMemo(stepsToFlow(workflow.steps))` at line 605
- `stepsToFlow` re-computes positions deterministically from `COL_BASE`, `ROW_GAP`, and step ordering — there is no `position` field on `WorkflowStep`

**Behavior:** Drag persists for the current React tree. Navigate to `/workflows` and back, or reload, and the node is back where it started. `Save + publish` does not capture the drag. Verdict: pure visual-only.

### 2. Drop on canvas — does it snap to grid?

**No.** `snapToGrid` and `snapGrid` props are absent. xyflow's default is **no snapping** — nodes follow the cursor at sub-pixel precision. Adjacent siblings `SkillCanvas` also omit snap props. **Critical n8n/UIpath gap** — both competitors snap to a 20px or 16px grid by default; misaligned nodes are a workflow-quality red flag for procurement reviewers.

### 3. Connect 2 nodes via handle — does it create an edge?

**No.** The custom `SkycotStepNode` renders source/target `Handle` components at lines 235 and 334 with correct positioning. Hovering over them surfaces the connection cursor (xyflow default). But:

- `onConnect` is not bound on the ReactFlow component
- `nodesConnectable` defaults to `true` in xyflow, so the **UI affordance exists** — the operator can drag a handle and see the connection line follow the cursor
- On drop, xyflow's default behavior fires `onConnect` if bound — since it isn't, **the connection is silently discarded**

This is the worst class of UX bug: it looks like it works, then nothing happens. The operator gets no feedback, no toast, no error. n8n shows a "+ Add node" insertion dialog at the connection point; UIpath inserts an unwired activity placeholder. SKYCOT does neither.

### 4. Disconnect (delete edge) — does it work?

**No.** `onEdgesDelete` and `onEdgesChange` are not bound. Edges have no `selectable: true` override but xyflow's default is `selectable: true`. Clicking an edge will set its `selected` state internally. Pressing Backspace/Delete with the edge selected triggers xyflow's default `deleteKeyCode` behavior — which mutates the **internal** store but does not call back into SKYCOT state (no `onEdgesChange` handler).

**Behavior:** Edge appears to delete from the canvas for the current session, then the next `useMemo` rebuild restores it.

### 5. Delete a node — keystroke? Right-click? Context menu?

**No keystroke handler, no right-click, no context menu.**

- xyflow's `deleteKeyCode` defaults to `Backspace` — selecting a node and pressing Backspace fires `onNodesChange` with a remove event. Since no handler is bound, the deletion is internal-only (same pattern as #4).
- No `onNodeContextMenu` is bound — right-click on a node does nothing app-specific. The browser's default context menu shows.
- No "Delete" affordance in the right-hand side panel. The panel has an "Edit step" button (line 1077-1100) but no delete.

**Behavior:** A keyboard user who knows xyflow can press Backspace and see the node disappear visually — until the next re-render restores it. A mouse user has no path to delete a node at all. **Critical builder gap.**

### 6. Multi-select via shift-click — works?

**Partial.** `multiSelectionKeyCode` defaults to `Shift` in xyflow v12, and `elementsSelectable` defaults to `true`. So shift-clicking nodes adds them to the internal selection set. **However:**

- `onSelectionChange` is not bound — the SPA layer never sees the selection
- `selectedNode` (line 614) is `useState<Node | null>` — single-select, set only by `onNodeClick` (line 624-627). So even if the operator multi-selects in the canvas, the right-hand panel only ever shows the **last clicked** node.
- No "bulk delete", "bulk move", or "bulk duplicate" affordance exists. Multi-select is a phantom feature.

### 7. Pan canvas (drag empty space) — smooth?

**Yes — xyflow default.** `panOnDrag` defaults to `true` on the primary mouse button. Drag on empty pane scrolls the viewport. No `onPaneClick`/`onPaneScroll` is bound, but xyflow handles the pan internally. **Performance:** with ~6 nodes (the seed workflows have 5-7 steps), pan is buttery. At 50+ nodes the absence of any virtualization config (no `nodeOrigin` adjustment, no `onlyRenderVisibleElements`) may bite — irrelevant at v1 demo scale.

### 8. Zoom in/out — keyboard shortcuts (Cmd+/Cmd-)? Mini-map zoom?

**Partial.**

- xyflow's `<Controls>` (rendered at line 1005) supplies plus/minus/fit-view/lock buttons — clicking those zooms.
- Scroll-wheel zoom: xyflow default is `zoomOnScroll = true`. Mouse-wheel works.
- Pinch zoom: `zoomOnPinch = true` default. Trackpad pinch works.
- Double-click zoom: `zoomOnDoubleClick = true` default. Double-click zooms in.
- **Cmd+ / Cmd- keyboard shortcuts: NOT WIRED.** xyflow does not bind these by default. There is no `onKeyDown` handler at the ReactFlow level. The operator cannot keyboard-zoom from outside the Controls widget (which is mouse-only — its buttons have no keyboard shortcut labels).
- MiniMap is `zoomable` (line 1013) — scroll-wheel inside the minimap zooms the main viewport.

**n8n gap:** n8n binds `Ctrl/Cmd +`, `Ctrl/Cmd -`, `Ctrl/Cmd 0` (reset), `Ctrl/Cmd 1` (fit-view) by default. SKYCOT has none of these.

### 9. Mini-map navigation — click + drag to pan?

**Yes — xyflow default.** `pannable` prop on MiniMap (line 1012) is set to `true`. Click + drag inside the minimap moves the main viewport. **This is the one canvas affordance that genuinely works as expected.**

### 10. Reset view — keyboard shortcut?

**No keyboard shortcut.** The `<Controls>` widget at line 1005 includes a "fit view" button (xyflow default). It is mouse-clickable. No `onKeyDown` mapping for `F` or `Cmd+0` or `Cmd+1`. No "Reset view" affordance elsewhere on the page. **Procurement-reviewer-flagged gap:** every commercial builder binds at least one fit-view shortcut (n8n uses `Ctrl/Cmd+1`, UIpath uses `Ctrl+Shift+H`).

---

## Other audit observations

### Add Step palette is a stub

`AddStepPalette` (line 449-559) renders 5 PaletteGroup buttons (Sources / Action / Logic / Governance / Data). **None of the buttons have an `onClick` handler.** The comment at line 451-453 acknowledges: "Per the brief, the palette is a stub: insertion wiring lands in the node-palette-sidebar brief." Hovering changes background color. Clicking does nothing — no node is added, no toast, no feedback. A buyer clicking through this in a demo will hit it within the first 30 seconds.

### "Edit step" button on the side panel is a stub for non-trigger nodes

Line 1077-1100. Clicking it calls `setTriggerConfigOpen(true)` — but the `TriggerConfigDrawer` only renders when `selectedIsTrigger && triggerConfigOpen && selectedStep` (line 1103). For a non-trigger node (skill, approval, tool_call, notify, decision, data, loop, wait), clicking "Edit step" sets a state flag that is never consumed by any drawer. **The button visibly does nothing.** Same hot-mic problem as #3 — looks like it works, then silence.

### Edge labels overlap on tight layouts

`linkReturn` at line 385-403 draws a dashed "↻ loop back" label for loop bodies. Hard-coded font size 9px, no collision detection. With nested loops (not yet seeded) or two loops stacked vertically the labels will overlap. Low priority but it'll show up the first time someone authors a `qa_audit`-style nested workflow.

### No selection persistence across drag/zoom

`selectedNode` state (line 614) is cleared by clicking the `<X>` close button (line 1033) but not by clicking empty pane. `onPaneClick` is not bound. Users have to click the close button to dismiss the side panel — clicking empty canvas does nothing. n8n and UIpath both close the side panel on pane click.

### `cursor: "grab"` on the node card but no `cursor: "grabbing"` on drag

Line 232. Minor polish gap — the cursor stays as `grab` (the open-hand icon) even during drag. xyflow doesn't override this. ~3 lines of CSS to fix.

### Custom node lacks `role="button"` and `tabIndex={0}` for a11y

The `SkycotStepNode` div has neither — relies on xyflow's wrapper for keyboard nav. Wave-7 keyboard audit already flagged: `onNodeKeyDown` is bound (line 633-639) for Enter/Space activation, but the visual focus ring from xyflow is faint against the white card background.

---

## Top 3 missing canvas affordances vs n8n / UIpath / Make.com

1. **Drag-to-create-edge with insertion dialog at drop.** n8n's signature affordance: drag from a source handle, drop on empty space, and a node-picker dialog appears at the drop point. SKYCOT discards the dragged connection silently. Closing this gap is the single highest-impact builder fix — without it, the canvas isn't a builder.

2. **Keyboard zoom + reset shortcuts (`Cmd+`, `Cmd-`, `Cmd+0`).** Every competitor binds these. SKYCOT operators have to find the Controls widget at bottom-left and click. Power-users will reach for the shortcuts first and conclude SKYCOT is "less polished."

3. **Right-click context menu on nodes (Delete / Duplicate / Disable / Copy / Edit).** n8n, UIpath, Make.com, Zapier all surface destructive + duplicate operations via right-click. SKYCOT has neither right-click nor a side-panel delete button — there is literally no path to remove a node short of editing the seed constant. This is a procurement-flag-level miss.

---

## Recommended brief sequencing (not in scope of this audit, but for handoff)

The audit's role ends at findings, but for the conductor: closing the read-only-canvas gap is a 4-brief sequence, each ~150-250 LOC:

- **B1 — controlled-state migration.** Replace `defaultNodes`/`defaultEdges` with `useNodesState`/`useEdgesState`; wire `onNodesChange`, `onEdgesChange`. Enables every subsequent fix.
- **B2 — connect + add-step.** Bind `onConnect` to create an edge; wire `AddStepPalette` buttons to `addNodes`.
- **B3 — delete + duplicate.** Bind `onNodesDelete`/`onEdgesDelete`; add right-click context menu (Radix `ContextMenu`); add side-panel delete button.
- **B4 — keyboard + snap polish.** Bind Cmd+/Cmd-/Cmd+0; set `snapToGrid` + `snapGrid={[20, 20]}`; `onPaneClick` clears selection.

Persistence to the gateway PUT-workflow endpoint is a separate brief (B0 in the sibling `web-workflow-builder-ai-prompt-wired` family).

---

## File references

- `web/app/(app)/workflows/[verticalId]/[name]/build/page.tsx:995-1016` — ReactFlow root, the load-bearing prop list
- `web/app/(app)/workflows/[verticalId]/[name]/build/page.tsx:449-559` — AddStepPalette stub
- `web/app/(app)/workflows/[verticalId]/[name]/build/page.tsx:215-361` — SkycotStepNode custom render
- `web/app/(app)/workflows/[verticalId]/[name]/build/page.tsx:675-716` — three "honest demo stub" handlers
- `web/components/skycot/canvas/SkillCanvas.tsx:178-197` — comparison sibling (intentionally read-only at L4; explicitly sets `nodesDraggable={false}`)
