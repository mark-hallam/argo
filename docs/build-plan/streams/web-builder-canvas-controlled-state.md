# web-builder-canvas-controlled-state — move ReactFlow from uncontrolled to controlled

**Owner**: claude.
**Authored against**: origin/main `50e65e9f` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-builder-canvas-controlled-state`.
**Priority**: P0 — wave-13 GUI canvas audit verdict was **3.5/10 ("canvas is
theater")**. The root cause is that `<ReactFlow>` is mounted in uncontrolled
mode (`defaultNodes` / `defaultEdges`) and the parent component has zero
visibility into drag, connect, or delete events. This brief is the FOUNDATION
— sibling briefs (`web-builder-connect-and-add-step`,
`web-builder-delete-and-duplicate`, `web-builder-keyboard-shortcuts-and-snap`)
all depend on the controlled-state plumbing landing first.
**Sizing**: ~300-400 LOC across the builder page (state lift) + a new helper
hook + 6 unit tests + 1 E2E smoke. NO new dependencies.

---

## Why this brief exists

Current state at `web/app/(app)/workflows/[verticalId]/[name]/build/page.tsx:782`:

```tsx
<ReactFlow
  defaultNodes={initial.nodes}     // ← uncontrolled
  defaultEdges={initial.edges}     // ← uncontrolled
  nodeTypes={nodeTypes}
  onNodeClick={onNodeClick}
  fitView
  ...
>
```

In uncontrolled mode, ReactFlow internally manages node positions, edges, and
selection. The parent has NO `onNodesChange` / `onEdgesChange` listeners, so:

- Dragging a node moves it visually but the parent state never sees the new position. Save → drag is lost.
- The "Add step" palette buttons (lines 891-924) appear to spawn new nodes but actually do nothing because there's no setter to push into.
- Delete key on a selected node does nothing.
- Wave-13 audit verbatim: "Builder is a high-fidelity FAKE — clicks register, animations play, but nothing persists."

This brief flips to controlled mode (`nodes` + `onNodesChange`, `edges` +
`onEdgesChange`) and surfaces a single source of truth that sibling briefs
write into.

---

## Pre-flight verification

```bash
# 1. Confirm the file still uses uncontrolled props.
rg "defaultNodes|defaultEdges" web/app/\(app\)/workflows/\[verticalId\]/\[name\]/build/page.tsx

# 2. Confirm no other builder page has been spun up that would conflict.
rg "WorkflowBuilder|builder-canvas" web/app/ -l

# 3. Confirm ReactFlow version (need ≥ v12 for the controlled API shape).
rg '"@xyflow/react"' web/package.json
```

---

## Architecture

### State shape

Lift to two `useState` slots at the top of the page component (after the
existing `useState` declarations around line 250):

```ts
const [nodes, setNodes] = useState<Node[]>(initial.nodes);
const [edges, setEdges] = useState<Edge[]>(initial.edges);
```

### Change handlers

Use ReactFlow's `applyNodeChanges` / `applyEdgeChanges` helpers — these are the
canonical reducers for the change events:

```ts
import { applyNodeChanges, applyEdgeChanges, type OnNodesChange, type OnEdgesChange } from "@xyflow/react";

const onNodesChange: OnNodesChange = useCallback(
  (changes) => setNodes((nds) => applyNodeChanges(changes, nds)),
  [],
);
const onEdgesChange: OnEdgesChange = useCallback(
  (changes) => setEdges((eds) => applyEdgeChanges(changes, eds)),
  [],
);
```

### Component swap

Replace the `<ReactFlow>` props:

```tsx
<ReactFlow
  nodes={nodes}                          // ← was defaultNodes
  edges={edges}                          // ← was defaultEdges
  onNodesChange={onNodesChange}          // ← NEW
  onEdgesChange={onEdgesChange}          // ← NEW
  nodeTypes={nodeTypes}
  onNodeClick={onNodeClick}
  fitView
  fitViewOptions={{ padding: 0.18 }}
  attributionPosition="bottom-right"
>
```

### Helper hook for sibling briefs

Extract the state + handlers into a hook so sibling briefs have a single
import surface:

**File**: `web/lib/hooks/use-workflow-builder-state.ts` (new)

```ts
import { useCallback, useState } from "react";
import {
  applyEdgeChanges,
  applyNodeChanges,
  type Edge,
  type Node,
  type OnEdgesChange,
  type OnNodesChange,
} from "@xyflow/react";

/**
 * Single source of truth for the visual workflow builder. Wraps
 * ReactFlow's controlled-mode change handlers and exposes a
 * paint-by-numbers API for sibling features (connect, add, delete,
 * keyboard shortcuts).
 *
 * Returns:
 *   - `nodes`, `edges` — pass directly to `<ReactFlow>` props.
 *   - `onNodesChange`, `onEdgesChange` — wire to the matching props.
 *   - `setNodes`, `setEdges` — for sibling features that need direct
 *     mutation (Add Step adds a new node, Delete removes one, etc.).
 *   - `addNode`, `removeNode`, `addEdge`, `removeEdge` — typed
 *     helpers that maintain ReactFlow invariants (unique ids, no
 *     orphan edges).
 */
export function useWorkflowBuilderState(
  initialNodes: Node[],
  initialEdges: Edge[],
) {
  const [nodes, setNodes] = useState<Node[]>(initialNodes);
  const [edges, setEdges] = useState<Edge[]>(initialEdges);

  const onNodesChange: OnNodesChange = useCallback(
    (changes) => setNodes((nds) => applyNodeChanges(changes, nds)),
    [],
  );
  const onEdgesChange: OnEdgesChange = useCallback(
    (changes) => setEdges((eds) => applyEdgeChanges(changes, eds)),
    [],
  );

  const addNode = useCallback((node: Node) => {
    setNodes((nds) => {
      if (nds.some((n) => n.id === node.id)) return nds;  // dedupe
      return [...nds, node];
    });
  }, []);

  const removeNode = useCallback((id: string) => {
    setNodes((nds) => nds.filter((n) => n.id !== id));
    setEdges((eds) => eds.filter((e) => e.source !== id && e.target !== id));
  }, []);

  const addEdge = useCallback((edge: Edge) => {
    setEdges((eds) => {
      if (eds.some((e) => e.id === edge.id)) return eds;
      return [...eds, edge];
    });
  }, []);

  const removeEdge = useCallback((id: string) => {
    setEdges((eds) => eds.filter((e) => e.id !== id));
  }, []);

  return {
    nodes, edges,
    setNodes, setEdges,
    onNodesChange, onEdgesChange,
    addNode, removeNode, addEdge, removeEdge,
  };
}
```

### Builder page wiring

Replace the two top-of-component `useState<Node[]>` declarations + manual
handlers with one hook call:

```tsx
const builder = useWorkflowBuilderState(initial.nodes, initial.edges);

// Then in JSX:
<ReactFlow
  nodes={builder.nodes}
  edges={builder.edges}
  onNodesChange={builder.onNodesChange}
  onEdgesChange={builder.onEdgesChange}
  ...
/>
```

### Dirty-state tracking (foundation for save-button enablement)

A workflow that has been edited but not saved must be visually distinguishable
from a clean one. Add a `dirty` boolean computed from `nodes` / `edges`
diverging from `initial`:

```ts
const dirty = useMemo(
  () =>
    nodes.length !== initial.nodes.length
    || edges.length !== initial.edges.length
    || JSON.stringify(nodes) !== JSON.stringify(initial.nodes)
    || JSON.stringify(edges) !== JSON.stringify(initial.edges),
  [nodes, edges, initial],
);
```

(JSON.stringify on a 20-node graph is ~2ms; for v1 this is acceptable. A
follow-up brief can swap to a hash if perf becomes a concern.)

Expose `dirty` via the hook return value. Wire to:
1. The Save button — disabled when `!dirty`.
2. A "You have unsaved changes" banner at top of the canvas (existing pattern
   from /workflows/<v>/<n> head/draft distinction).
3. `beforeunload` listener so navigating away with unsaved changes warns.

---

## may-touch

- `web/app/(app)/workflows/[verticalId]/[name]/build/page.tsx` — state lift, prop swap, hook integration, dirty-state wiring.
- `web/lib/hooks/use-workflow-builder-state.ts` (new) — the hook itself.
- `web/lib/hooks/use-workflow-builder-state.test.ts` (new) — 6 unit tests for the hook.
- `web/tests/builder-controlled-state.spec.ts` (new) — E2E smoke.

## must-NOT-touch

- Sibling brief surfaces: the connect handler (`onConnect`), the Add Step
  palette wiring, delete + duplicate, keyboard shortcuts. Those are
  separate briefs that DEPEND on this one landing first — leave clear
  TODO comments where the hook return value will be consumed.
- `nodeTypes` registry — node rendering is unchanged.
- Mini-map, controls, background — pass-through, no changes.
- ReactFlow version pin in `package.json` — controlled API is in v12 already.

---

## Acceptance criteria

- [ ] Hook `useWorkflowBuilderState` exported from `web/lib/hooks/use-workflow-builder-state.ts`.
- [ ] 6 unit tests pass for the hook (one per public method + dirty-state).
- [ ] `<ReactFlow>` in the builder page uses controlled props (`nodes` + `edges` + `onNodesChange` + `onEdgesChange`), NOT `defaultNodes` / `defaultEdges`.
- [ ] Dragging a node visibly updates and the new position is reflected in `builder.nodes` (verifiable via React DevTools).
- [ ] Save button reads `disabled={!builder.dirty}`.
- [ ] `beforeunload` listener fires only when `builder.dirty === true`.
- [ ] Navigating away after edit shows browser confirm dialog; after save, no dialog.
- [ ] E2E smoke `builder-controlled-state.spec.ts` passes: load `/workflows/pr_checker/standard_review/build`, drag the first node 100px right, verify React DevTools shows updated position OR pin a data-testid attribute the test can read.
- [ ] `pnpm typecheck && pnpm lint && pnpm test` clean.
- [ ] No `defaultNodes` / `defaultEdges` references remain anywhere in `web/app/`.

---

## Gotchas

1. **Initial node positions come from the seed**: `initial.nodes` is computed
   from `SEED_WORKFLOWS` at the top of the page. The hook seeds state with
   those exact positions on mount. Don't re-seed on every render.
2. **Edges referencing removed nodes**: `removeNode` cascades to remove
   edges where the removed node is source OR target — leaving orphan edges
   is what ReactFlow's invariant check catches with red console errors.
3. **`applyNodeChanges` mutates by id**: don't combine it with a manual filter
   in the same setter call. Use the hook's helper methods (`removeNode`,
   `addNode`) instead.
4. **Dirty-state JSON.stringify comparison**: for graphs > 100 nodes this
   becomes noticeable. The brief allows this for v1; a follow-up can swap
   to a content hash. Don't optimize prematurely.
5. **`useCallback` deps**: the change handlers have empty dep arrays
   because they only call `setNodes` / `setEdges` (stable refs from useState).
   Don't add `nodes` or `edges` to the deps — that causes unnecessary
   re-renders.

---

## References

- ReactFlow v12 controlled-mode docs: https://reactflow.dev/learn/advanced-use/uncontrolled-flow
- Audit: `docs/skycot/audit/user-journey/2026-05-20-wave-13-gui-canvas.md` (3.5/10 verdict)
- Builder page: `web/app/(app)/workflows/[verticalId]/[name]/build/page.tsx:782`
- Sibling briefs that depend on this: `web-builder-connect-and-add-step`, `web-builder-delete-and-duplicate`, `web-builder-keyboard-shortcuts-and-snap`
