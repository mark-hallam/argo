# web-builder-keyboard-shortcuts-and-snap — keyboard zoom/fit + snap-to-grid + undo

**Owner**: claude.
**Authored against**: origin/main `50e65e9f` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-builder-keyboard-shortcuts-and-snap`.
**Priority**: P2 — wave-13 GUI canvas finding sub-point #4 ("No keyboard
shortcuts; nodes drag to pixel-imperfect positions, snap-to-grid disabled;
no undo/redo"). Polish brief — useful for power users + demo polish but not
launch-blocking. Depends on `web-builder-canvas-controlled-state`.
**Sizing**: ~350-450 LOC across builder page + new history hook + 6 unit
tests + 1 E2E.
**Blocked by**: `web-builder-canvas-controlled-state`.

---

## Why this brief exists

Wave-13 sub-finding: "Builder feels nothing like a power-user tool —
n8n / Figma / Linear all have Cmd+Z, snap-to-grid, Cmd+0 fit, Cmd+/Cmd-
zoom. Builder has none." Three rolled-up features:

1. **Keyboard shortcuts**: Cmd/Ctrl+Z = undo, Cmd/Ctrl+Shift+Z = redo,
   Cmd/Ctrl+0 = fit-to-view, Cmd/Ctrl+`=` zoom-in, Cmd/Ctrl+`-` zoom-out,
   Cmd/Ctrl+A = select all, Cmd/Ctrl+D = duplicate selected.
2. **Snap-to-grid**: dragged nodes snap to a 20px grid so workflows look
   visually aligned without manual fiddling.
3. **Undo/redo history**: every node-add, node-delete, edge-add, edge-
   delete, and significant position change pushes a history entry that
   Cmd+Z reverses.

---

## Pre-flight verification

```bash
# 1. Sibling foundations must be in place.
rg "useWorkflowBuilderState" web/lib/hooks/use-workflow-builder-state.ts
rg "onConnect|onNodesDelete" web/app/\(app\)/workflows/\[verticalId\]/\[name\]/build/page.tsx

# 2. Confirm no parallel undo/history utility exists.
rg "useUndo|useHistory|HistoryStack" web/lib/hooks/

# 3. Confirm ReactFlow `snapToGrid` + `snapGrid` props are available.
rg "snapToGrid|snapGrid" web/node_modules/@xyflow/react/dist/esm/index.d.ts 2>/dev/null | head -5
```

---

## Phase 1 — snap-to-grid

The simplest of the three. ReactFlow has built-in props:

```tsx
<ReactFlow
  ...
  snapToGrid                    // ← NEW
  snapGrid={[20, 20]}            // ← NEW — 20px grid matches the Background gap
>
```

That's it. Drags now snap to 20px increments. A future brief can make the
grid size operator-configurable; v1 hard-codes 20 to match the existing
`<Background gap={20}>` visual grid.

---

## Phase 2 — undo/redo history

### History hook

**File**: `web/lib/hooks/use-builder-history.ts` (new)

```ts
import { useCallback, useRef } from "react";
import type { Edge, Node } from "@xyflow/react";

export interface BuilderSnapshot {
  nodes: Node[];
  edges: Edge[];
  label: string;            // human-readable, e.g. "Add Approval gate"
  takenAt: number;           // Date.now()
}

/**
 * Cmd+Z / Cmd+Shift+Z history. Holds at most MAX_HISTORY snapshots
 * (default 50) per session. NOT persisted — page reload resets.
 *
 * Usage:
 *   const history = useBuilderHistory();
 *   // After every mutation:
 *   history.push({ nodes: builder.nodes, edges: builder.edges, label: "Add step" });
 *   // To undo:
 *   const prev = history.undo();
 *   if (prev) { builder.setNodes(prev.nodes); builder.setEdges(prev.edges); }
 */
const MAX_HISTORY = 50;

export function useBuilderHistory() {
  // Past = snapshots before current state. Future = snapshots after undo
  // (cleared by any new push).
  const past = useRef<BuilderSnapshot[]>([]);
  const future = useRef<BuilderSnapshot[]>([]);

  const push = useCallback((snapshot: BuilderSnapshot) => {
    past.current.push(snapshot);
    if (past.current.length > MAX_HISTORY) past.current.shift();
    future.current = [];                  // any new action invalidates future
  }, []);

  const undo = useCallback((): BuilderSnapshot | null => {
    const prev = past.current.pop();
    if (!prev) return null;
    return prev;
  }, []);

  const redo = useCallback((): BuilderSnapshot | null => {
    const next = future.current.pop();
    if (!next) return null;
    return next;
  }, []);

  const canUndo = useCallback(() => past.current.length > 0, []);
  const canRedo = useCallback(() => future.current.length > 0, []);

  return { push, undo, redo, canUndo, canRedo };
}
```

### Wire into builder

```tsx
const history = useBuilderHistory();

// Snapshot helper — captures current state.
const snapshot = useCallback((label: string): BuilderSnapshot => ({
  nodes: structuredClone(builder.nodes),
  edges: structuredClone(builder.edges),
  label,
  takenAt: Date.now(),
}), [builder.nodes, builder.edges]);

// Wrap the mutation helpers from sibling briefs so each mutation pushes
// a snapshot BEFORE the change.
const handleAddStep = useCallback((kind: StepKind) => {
  history.push(snapshot(`Add ${kind} step`));
  // ... rest of handleAddStep from connect-and-add-step brief
}, [history, snapshot, /* ... */]);

const onNodesDelete = useCallback((deleted: Node[]) => {
  history.push(snapshot(`Delete ${deleted.length} step${deleted.length === 1 ? "" : "s"}`));
  // ... rest of onNodesDelete from delete-and-duplicate brief
}, [history, snapshot, /* ... */]);

// Same wrap for onConnect, duplicateNode, removeEdge etc.
```

### Restore from snapshot

```tsx
const handleUndo = useCallback(() => {
  const prev = history.undo();
  if (!prev) return;
  // Push current state to "future" so Redo works.
  history.push(snapshot("(redo target)"));    // see gotcha #3
  builder.setNodes(prev.nodes);
  builder.setEdges(prev.edges);
  toast(`Undone: ${prev.label}`);
}, [builder, history, snapshot]);

const handleRedo = useCallback(() => {
  const next = history.redo();
  if (!next) return;
  builder.setNodes(next.nodes);
  builder.setEdges(next.edges);
  toast(`Redone: ${next.label}`);
}, [builder, history]);
```

---

## Phase 3 — Keyboard shortcuts

### Hook

**File**: `web/lib/hooks/use-builder-keyboard.ts` (new)

```ts
import { useEffect } from "react";

interface Bindings {
  onUndo: () => void;
  onRedo: () => void;
  onFitView: () => void;
  onZoomIn: () => void;
  onZoomOut: () => void;
  onSelectAll: () => void;
  onDuplicateSelected: () => void;
}

const isMac = typeof navigator !== "undefined" && /Mac|iPhone|iPod|iPad/.test(navigator.platform);

export function useBuilderKeyboard(bindings: Bindings, enabled: boolean) {
  useEffect(() => {
    if (!enabled) return;
    const handler = (e: KeyboardEvent) => {
      // Don't intercept when typing in an input/textarea/contenteditable.
      const target = e.target as HTMLElement;
      if (
        target.tagName === "INPUT"
        || target.tagName === "TEXTAREA"
        || target.isContentEditable
      ) return;

      const mod = isMac ? e.metaKey : e.ctrlKey;
      if (!mod) return;

      switch (e.key.toLowerCase()) {
        case "z":
          e.preventDefault();
          if (e.shiftKey) bindings.onRedo();
          else bindings.onUndo();
          break;
        case "y":
          // Windows convention for redo.
          if (!isMac) {
            e.preventDefault();
            bindings.onRedo();
          }
          break;
        case "0":
          e.preventDefault();
          bindings.onFitView();
          break;
        case "=":
        case "+":
          e.preventDefault();
          bindings.onZoomIn();
          break;
        case "-":
          e.preventDefault();
          bindings.onZoomOut();
          break;
        case "a":
          e.preventDefault();
          bindings.onSelectAll();
          break;
        case "d":
          e.preventDefault();
          bindings.onDuplicateSelected();
          break;
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [enabled, bindings]);
}
```

### Wire in builder page

```tsx
const reactFlow = useReactFlow();    // from @xyflow/react — gives zoomIn/zoomOut/fitView

useBuilderKeyboard({
  onUndo: handleUndo,
  onRedo: handleRedo,
  onFitView: () => reactFlow.fitView({ padding: 0.18, duration: 250 }),
  onZoomIn: () => reactFlow.zoomIn({ duration: 150 }),
  onZoomOut: () => reactFlow.zoomOut({ duration: 150 }),
  onSelectAll: () => {
    builder.setNodes((nds) => nds.map((n) => ({ ...n, selected: true })));
  },
  onDuplicateSelected: () => {
    const selected = builder.nodes.filter((n) => n.selected);
    for (const n of selected) duplicateNode(n.id);
  },
}, /* enabled */ true);
```

**Wrap the page in `<ReactFlowProvider>`** if not already — `useReactFlow()`
requires it. Check:
```bash
rg "ReactFlowProvider" web/app/\(app\)/workflows/\[verticalId\]/\[name\]/build/page.tsx
```
If absent, wrap the builder JSX in `<ReactFlowProvider>`.

### Visual shortcut hints

Add a small floating "Keyboard shortcuts" affordance bottom-right (or in the
existing Controls area). Modal that opens on `?` key (no modifier) and lists:

```
Cmd+Z       Undo
Cmd+Shift+Z Redo
Cmd+0       Fit to view
Cmd+=       Zoom in
Cmd+-       Zoom out
Cmd+A       Select all
Cmd+D       Duplicate selected
Backspace   Delete selected
?           Show shortcuts (this menu)
```

A `<KeyboardShortcutsModal>` component sits in
`web/components/skycot/workflows/`. Trigger via:

```tsx
useEffect(() => {
  const handler = (e: KeyboardEvent) => {
    if (e.key === "?" && !e.metaKey && !e.ctrlKey) {
      const t = e.target as HTMLElement;
      if (t.tagName === "INPUT" || t.tagName === "TEXTAREA" || t.isContentEditable) return;
      setShowShortcuts(true);
    }
  };
  window.addEventListener("keydown", handler);
  return () => window.removeEventListener("keydown", handler);
}, []);
```

---

## may-touch

- `web/app/(app)/workflows/[verticalId]/[name]/build/page.tsx` — snap props + history wraps + keyboard hook + ReactFlowProvider wrap (if needed) + shortcuts modal trigger.
- `web/lib/hooks/use-builder-history.ts` (new).
- `web/lib/hooks/use-builder-keyboard.ts` (new).
- `web/components/skycot/workflows/KeyboardShortcutsModal.tsx` (new).
- `web/tests/builder-shortcuts.spec.ts` (new) — E2E for Cmd+Z + Cmd+0.
- 6 unit tests across the two new hooks.

## must-NOT-touch

- The `useWorkflowBuilderState` hook surface (sibling brief).
- ReactFlow `Background gap` — already 20px, matches the snap grid.
- Sonner toast — reuse.
- Any other page's keyboard handling — this hook scopes to the builder page only.

---

## Acceptance criteria

- [ ] Dragging a node snaps to a 20px grid (positions are multiples of 20).
- [ ] Cmd/Ctrl+Z undoes the most recent edit (add, delete, connect, disconnect, duplicate). Visible toast "Undone: ...".
- [ ] Cmd/Ctrl+Shift+Z redoes. Visible toast "Redone: ...".
- [ ] Cmd/Ctrl+Y also redoes on Windows/Linux.
- [ ] Cmd/Ctrl+0 fits the entire workflow to view with a smooth 250ms transition.
- [ ] Cmd/Ctrl+= / Cmd/Ctrl++ zoom in.
- [ ] Cmd/Ctrl+- zoom out.
- [ ] Cmd/Ctrl+A selects all nodes.
- [ ] Cmd/Ctrl+D duplicates selected nodes (uses the duplicate logic from sibling brief).
- [ ] None of the shortcuts trigger when typing in an `<input>`, `<textarea>`, or `contenteditable` element.
- [ ] Pressing `?` (no modifier) opens a modal listing the shortcuts.
- [ ] History buffer holds at most 50 entries.
- [ ] Page reload resets the history (no persistence in v1).
- [ ] E2E `builder-shortcuts.spec.ts` passes: add a step, press Cmd+Z, verify step removed; press Cmd+Shift+Z, verify step back.
- [ ] `pnpm typecheck && pnpm lint && pnpm test` clean.

---

## Gotchas

1. **`useReactFlow()` requires `<ReactFlowProvider>`**: if the builder
   page renders `<ReactFlow>` directly without a Provider, `useReactFlow`
   returns undefined and `zoomIn` is a no-op. Wrap the JSX subtree.
2. **`structuredClone` is browser-only but works fine in Next.js client
   components** (same as `crypto.randomUUID()`). Don't try to use it in
   SSR paths.
3. **Undo + Redo invariant**: when undoing, the CURRENT state is pushed
   to "future" before restoring "past". Without this, redo can't bring
   you forward. The implementation in `handleUndo` above does this with
   a synthetic "(redo target)" label — could be cleaner but is correct.
4. **Position-change snapshots**: every drag fires dozens of small
   position-change events. **Don't push a history snapshot per change** —
   that fills the buffer in 2 seconds. Only push on the FINAL position
   (mouse-up) — ReactFlow's `onNodeDragStop` is the right event for that.
5. **Mod key on Mac vs Windows**: `isMac` check at top of the keyboard
   hook switches between `metaKey` (Cmd) and `ctrlKey`. Don't use a
   library — the navigator.platform check is fine.
6. **`navigator.platform` is deprecated**: when the deprecation lands
   in browsers, swap to `navigator.userAgentData.platform` with a
   navigator-platform fallback for the v1 window. Out of scope for this
   brief — note as future cleanup.
7. **Cmd+R is reserved** for browser refresh — don't try to bind it.
8. **`e.preventDefault()` on each handled key is required** — otherwise
   Cmd+0 resets browser zoom, Cmd+= toggles browser zoom in, etc.

---

## References

- ReactFlow `snapToGrid`: https://reactflow.dev/api-reference/react-flow#snaptogrid
- ReactFlow `useReactFlow`: https://reactflow.dev/api-reference/hooks/use-react-flow
- `structuredClone` MDN: https://developer.mozilla.org/en-US/docs/Web/API/structuredClone
- Builder page: `web/app/(app)/workflows/[verticalId]/[name]/build/page.tsx`
- Sibling foundation: `web-builder-canvas-controlled-state`
- Audit: `docs/skycot/audit/user-journey/2026-05-20-wave-13-gui-canvas.md`
