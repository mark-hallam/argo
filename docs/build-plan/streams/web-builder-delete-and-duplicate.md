# web-builder-delete-and-duplicate — delete key + context menu (duplicate / disconnect)

**Owner**: claude.
**Authored against**: origin/main `50e65e9f` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-builder-delete-and-duplicate`.
**Priority**: P1 — wave-13 GUI canvas finding sub-point #3 ("Pressing Delete
on a selected node does nothing; no way to duplicate a step; no way to
disconnect an edge without rebuilding"). Depends on
`web-builder-canvas-controlled-state` landing first.
**Sizing**: ~300-380 LOC across builder page + new context-menu component +
5 unit tests + 2 E2E.
**Blocked by**: `web-builder-canvas-controlled-state`.

---

## Why this brief exists

Three related missing affordances flagged in wave-13:

1. **Delete key does nothing** — ReactFlow's `deleteKeyCode` defaults to
   "Backspace", but the parent has no `onNodesDelete` / `onEdgesDelete`
   handlers and so the state never updates.
2. **No duplicate** — line-managers iterating on a workflow want to right-
   click a step and say "copy this approval gate, change the channel."
   That right-click does nothing today.
3. **No edge disconnect** — selecting an edge and pressing Delete should
   remove it without removing the connected nodes. Currently impossible.

This brief wires all three using the controlled-state foundation.

---

## Pre-flight verification

```bash
# 1. Sibling brief must have landed.
rg "useWorkflowBuilderState" web/lib/hooks/use-workflow-builder-state.ts
rg "removeNode|removeEdge" web/lib/hooks/use-workflow-builder-state.ts

# 2. Confirm no parallel context-menu component exists.
rg "WorkflowContextMenu|ContextMenu" web/components/skycot/workflows/ 2>/dev/null

# 3. Confirm ReactFlow exports for selection/keyboard events.
rg "OnSelectionChange\|deleteKeyCode" web/node_modules/@xyflow/react/dist/esm/index.d.ts 2>/dev/null | head -5
```

---

## Phase 1 — Wire delete keyboard handler

### ReactFlow already has the prop

```tsx
<ReactFlow
  ...
  deleteKeyCode={["Backspace", "Delete"]}   // ← NEW; supports both keys
  onNodesDelete={onNodesDelete}             // ← NEW
  onEdgesDelete={onEdgesDelete}             // ← NEW
>
```

### Handlers

```tsx
const onNodesDelete = useCallback((deleted: Node[]) => {
  // ReactFlow has already removed the nodes from its internal view by the
  // time this fires; our controlled state needs to follow. The applyNodeChanges
  // reducer in the sibling hook handles position-change events, but DELETE
  // events arrive here separately because we want a confirm prompt for
  // destructive deletions of trigger nodes.
  for (const node of deleted) {
    if (node.data?.kind === "trigger") {
      toast.error("Trigger steps can't be deleted — change the trigger kind from the side panel instead.");
      // Re-add the node — ReactFlow's optimistic removal must be undone.
      builder.addNode(node);
      continue;
    }
    builder.removeNode(node.id);  // cascades to edges touching this node
  }
}, [builder]);

const onEdgesDelete = useCallback((deleted: Edge[]) => {
  for (const edge of deleted) {
    builder.removeEdge(edge.id);
  }
}, [builder]);
```

### Confirm before delete (multi-step destructive deletions)

Bulk-delete (user selected 5 nodes via shift-click and hits Delete) is
destructive enough to warrant a confirm modal — but a single node delete
should be instant + undoable via toast action.

```tsx
const onNodesDelete = useCallback(async (deleted: Node[]) => {
  // Filter triggers first (block them entirely).
  const triggers = deleted.filter((n) => n.data?.kind === "trigger");
  const deletable = deleted.filter((n) => n.data?.kind !== "trigger");

  if (triggers.length > 0) {
    toast.error("Trigger steps can't be deleted.");
    for (const t of triggers) builder.addNode(t);  // undo ReactFlow's optimistic removal
  }
  if (deletable.length === 0) return;

  // Show a single toast with "Undo" action for small deletions.
  if (deletable.length <= 1) {
    for (const n of deletable) builder.removeNode(n.id);
    toast.success("Step deleted", {
      action: {
        label: "Undo",
        onClick: () => { for (const n of deletable) builder.addNode(n); },
      },
    });
    return;
  }

  // Bulk → confirm modal.
  const confirmed = await confirmDelete({
    title: `Delete ${deletable.length} steps?`,
    body: "This will remove the steps and any edges connecting them. You can undo for the next 5 seconds.",
  });
  if (!confirmed) {
    for (const n of deletable) builder.addNode(n);  // restore
    return;
  }
  for (const n of deletable) builder.removeNode(n.id);
  toast.success(`${deletable.length} steps deleted`);
}, [builder]);
```

(`confirmDelete` is a thin Promise wrapper around an existing
`<ConfirmDeleteModal>` if present, else a new component. `rg
"ConfirmDeleteModal\|confirmDelete" web/components/` to check.)

---

## Phase 2 — Right-click context menu

### New component

**File**: `web/components/skycot/workflows/WorkflowContextMenu.tsx` (new)

```tsx
"use client";

import { useEffect, useRef } from "react";
import { Trash2, Copy, Unlink, Settings as SettingsIcon } from "lucide-react";

export interface ContextMenuItem {
  label: string;
  Icon: typeof Trash2;
  onClick: () => void;
  destructive?: boolean;
  disabled?: boolean;
}

interface Props {
  x: number;
  y: number;
  items: ContextMenuItem[];
  onClose: () => void;
}

export function WorkflowContextMenu({ x, y, items, onClose }: Props) {
  const ref = useRef<HTMLDivElement>(null);

  // Click outside or Escape closes.
  useEffect(() => {
    const handleClick = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) onClose();
    };
    const handleKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("mousedown", handleClick);
    window.addEventListener("keydown", handleKey);
    return () => {
      window.removeEventListener("mousedown", handleClick);
      window.removeEventListener("keydown", handleKey);
    };
  }, [onClose]);

  return (
    <div
      ref={ref}
      role="menu"
      style={{ position: "fixed", left: x, top: y, zIndex: 9999 }}
      className="builder-context-menu"
    >
      {items.map((item) => (
        <button
          key={item.label}
          type="button"
          role="menuitem"
          disabled={item.disabled}
          className={item.destructive ? "is-destructive" : undefined}
          onClick={() => { item.onClick(); onClose(); }}
        >
          <item.Icon size={14} /> {item.label}
        </button>
      ))}
    </div>
  );
}
```

### Parent state

```tsx
const [contextMenu, setContextMenu] = useState<{
  x: number; y: number; nodeId: string | null; edgeId: string | null;
} | null>(null);

const onNodeContextMenu = useCallback((event: React.MouseEvent, node: Node) => {
  event.preventDefault();
  setContextMenu({ x: event.clientX, y: event.clientY, nodeId: node.id, edgeId: null });
}, []);

const onEdgeContextMenu = useCallback((event: React.MouseEvent, edge: Edge) => {
  event.preventDefault();
  setContextMenu({ x: event.clientX, y: event.clientY, nodeId: null, edgeId: edge.id });
}, []);

const onPaneClick = useCallback(() => setContextMenu(null), []);
```

### Wire into ReactFlow

```tsx
<ReactFlow
  ...
  onNodeContextMenu={onNodeContextMenu}
  onEdgeContextMenu={onEdgeContextMenu}
  onPaneClick={onPaneClick}
>
```

### Duplicate node helper

```tsx
const duplicateNode = useCallback((nodeId: string) => {
  const original = builder.nodes.find((n) => n.id === nodeId);
  if (!original) return;
  if (original.data?.kind === "trigger") {
    toast.error("Trigger steps can't be duplicated — only one trigger per workflow.");
    return;
  }
  const newId = `step-${crypto.randomUUID().slice(0, 8)}`;
  const duplicated: Node = {
    ...original,
    id: newId,
    position: { x: original.position.x + 60, y: original.position.y + 60 },
    data: {
      ...original.data,
      name: `${original.data.name} (copy)`,
    },
    selected: false,
  };
  builder.addNode(duplicated);
  toast.success("Step duplicated");
}, [builder]);
```

### Items by context (node vs edge)

```tsx
const contextMenuItems = useMemo((): ContextMenuItem[] => {
  if (!contextMenu) return [];
  if (contextMenu.nodeId) {
    const node = builder.nodes.find((n) => n.id === contextMenu.nodeId);
    const isTrigger = node?.data?.kind === "trigger";
    return [
      { label: "Configure",  Icon: SettingsIcon, onClick: () => onNodeClick({} as any, node!) },
      { label: "Duplicate",  Icon: Copy,         onClick: () => duplicateNode(contextMenu.nodeId!), disabled: isTrigger },
      { label: "Delete",     Icon: Trash2,       onClick: () => onNodesDelete([node!]), destructive: true, disabled: isTrigger },
    ];
  }
  if (contextMenu.edgeId) {
    return [
      { label: "Disconnect", Icon: Unlink, onClick: () => builder.removeEdge(contextMenu.edgeId!), destructive: true },
    ];
  }
  return [];
}, [contextMenu, builder, duplicateNode, onNodesDelete, onNodeClick]);
```

### Render

```tsx
{contextMenu && contextMenuItems.length > 0 && (
  <WorkflowContextMenu
    x={contextMenu.x}
    y={contextMenu.y}
    items={contextMenuItems}
    onClose={() => setContextMenu(null)}
  />
)}
```

---

## may-touch

- `web/app/(app)/workflows/[verticalId]/[name]/build/page.tsx` — handlers + state + ReactFlow props + menu render.
- `web/components/skycot/workflows/WorkflowContextMenu.tsx` (new) — menu component.
- `web/tests/builder-delete.spec.ts` (new) — E2E delete.
- `web/tests/builder-duplicate.spec.ts` (new) — E2E duplicate + edge disconnect.
- 5 unit tests for context-menu component (render, click-outside, escape, item-click, destructive item styling).

## must-NOT-touch

- The `useWorkflowBuilderState` hook surface — use existing `removeNode` /
  `removeEdge` / `addNode`. Don't add a `duplicateNode` to the hook
  itself — that's UI-level logic with the position offset.
- Sibling features: connect, add step, keyboard shortcuts (separate briefs).
- Sonner toast import — already wired.

---

## Acceptance criteria

- [ ] Selecting a node and pressing `Delete` removes it from the canvas.
- [ ] Selecting a node and pressing `Backspace` also removes it (both key codes wired).
- [ ] Deleting a node cascades: any edges connecting it are removed too (no orphan edges).
- [ ] Trigger nodes cannot be deleted — attempt shows toast "Trigger steps can't be deleted" and the node remains.
- [ ] Single-node delete shows a toast with an "Undo" action.
- [ ] Multi-node delete (2+) shows a confirm modal before removing.
- [ ] Selecting an edge and pressing Delete removes only the edge, keeping both connected nodes.
- [ ] Right-click on a node opens a context menu at the cursor position with: Configure, Duplicate, Delete.
- [ ] Right-click on an edge opens a context menu with: Disconnect.
- [ ] Right-click on the canvas pane (empty area) closes any open menu.
- [ ] Clicking outside the open menu closes it.
- [ ] Pressing Escape closes the open menu.
- [ ] Duplicate adds a copy at +60px / +60px offset with name " (copy)" suffix.
- [ ] Duplicate of a trigger step is disabled (greyed out) in the menu.
- [ ] All menu items are reachable via Tab, activated via Enter / Space.
- [ ] `dirty` flag flips true after a delete, duplicate, or disconnect.
- [ ] E2E `builder-delete.spec.ts` passes: load builder, click node-2, press Delete, verify node + connected edge gone.
- [ ] E2E `builder-duplicate.spec.ts` passes: load builder, right-click node-2, click Duplicate, verify two copies present.
- [ ] `pnpm typecheck && pnpm lint && pnpm test` clean.

---

## Gotchas

1. **`onNodesDelete` is called AFTER ReactFlow's internal removal**: our
   `removeNode` would otherwise be a no-op because the change has already
   propagated. To support the "undo" toast action, we re-add the node via
   `addNode` — ReactFlow accepts this and re-renders correctly.
2. **`deleteKeyCode` accepts an array OR a single string**: passing
   `["Backspace", "Delete"]` covers macOS (Delete key sends Backspace) and
   Windows/Linux (Delete key sends Delete) without branching.
3. **The context menu MUST be position: fixed** (not absolute), because
   the canvas pans + zooms — absolute positioning would scroll away.
4. **`event.preventDefault()` in `onNodeContextMenu`**: required to
   suppress the native browser context menu. Forgetting this shows both
   menus simultaneously.
5. **Tab focus in the menu**: each `<button>` should have an explicit
   `tabIndex={0}` only if the parent isn't already focusable. Default
   `<button>` is tab-stoppable, so no extra attribute needed; just
   verify ordering with `role="menu"` + `role="menuitem"`.
6. **Trigger-protection is a UX rule, not a schema rule**: the underlying
   workflow data model allows zero or more triggers; this brief enforces
   "exactly one trigger" at the UI to prevent confused user states. A
   future brief may relax this when conditional triggers are added.

---

## References

- ReactFlow `deleteKeyCode`: https://reactflow.dev/api-reference/react-flow#deletekeycode
- ReactFlow `onNodesDelete` / `onEdgesDelete`: https://reactflow.dev/api-reference/react-flow#onnodesdelete
- ReactFlow `onNodeContextMenu` / `onEdgeContextMenu`: https://reactflow.dev/api-reference/react-flow#onnodecontextmenu
- Builder page: `web/app/(app)/workflows/[verticalId]/[name]/build/page.tsx`
- Sibling: `web-builder-canvas-controlled-state` (must merge first)
- Audit: `docs/skycot/audit/user-journey/2026-05-20-wave-13-gui-canvas.md`
