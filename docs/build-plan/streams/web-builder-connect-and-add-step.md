# web-builder-connect-and-add-step — wire onConnect + Add Step palette

**Owner**: claude.
**Authored against**: origin/main `50e65e9f` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-builder-connect-and-add-step`.
**Priority**: P0 — wave-13 GUI canvas finding sub-point #2 ("Connect handles
do nothing, Add Step palette is dead theater"). Depends on
`web-builder-canvas-controlled-state` landing first.
**Sizing**: ~250-300 LOC across builder page + palette component + 4 unit
tests + 1 E2E.
**Blocked by**: `web-builder-canvas-controlled-state` must merge first
(supplies the `addNode` / `addEdge` helpers used here).

---

## Why this brief exists

Two related failures from wave-13:

1. **Connect handles do nothing**: Each custom node renders source/target
   `<Handle>` components, but the parent has no `onConnect` callback. The
   user drags from one node's source handle to another node's target
   handle, the connection line draws, releases, and **vanishes**.

2. **Add Step palette is dead theater**: At line ~890 of the builder page,
   the floating "+ Add step" button toggles a `<AddStepPalette>` panel that
   lists step kinds (skill_invoke, approval, tool_call, etc.). Clicking a
   kind … closes the palette. Nothing else. No new node lands on the canvas.

Both are 30-line fixes that were never written because uncontrolled mode
gave the parent no setter to push into. With controlled-state in place
(per the sibling brief), this brief delivers both.

---

## Pre-flight verification

```bash
# 1. Confirm the controlled-state brief has landed (this brief depends on it).
rg "useWorkflowBuilderState" web/lib/hooks/use-workflow-builder-state.ts
rg "useWorkflowBuilderState" web/app/\(app\)/workflows/\[verticalId\]/\[name\]/build/page.tsx

# 2. Locate the existing AddStepPalette component.
rg "AddStepPalette" web/app/\(app\)/workflows/\[verticalId\]/\[name\]/build/page.tsx -A 5

# 3. Locate the <Handle> components in the custom node JSX.
rg "<Handle " web/app/\(app\)/workflows/\[verticalId\]/\[name\]/build/page.tsx -A 2 | head -15
```

If `useWorkflowBuilderState` is not found, **STOP** — pick up the sibling
brief first and merge it before continuing.

---

## Phase 1 — Wire `onConnect`

### Add the handler

In `useWorkflowBuilderState` (the hook from the sibling brief), the `addEdge`
helper already exists. The builder page just needs an `onConnect` callback
that delegates to it:

```tsx
import type { Connection } from "@xyflow/react";

const onConnect = useCallback((connection: Connection) => {
  if (!connection.source || !connection.target) return;
  // Reject self-loops (source === target) at the UI layer.
  if (connection.source === connection.target) {
    toast.error("A step can't connect to itself.");
    return;
  }
  // Reject duplicate edges (already wired source→target).
  if (builder.edges.some((e) =>
    e.source === connection.source && e.target === connection.target
  )) {
    toast.error("These steps are already connected.");
    return;
  }
  builder.addEdge({
    id: `e-${connection.source}-${connection.target}`,
    source: connection.source,
    target: connection.target,
    type: "smoothstep",            // matches the existing seed-edge style
    animated: false,
  });
}, [builder]);
```

### Wire into ReactFlow

```tsx
<ReactFlow
  nodes={builder.nodes}
  edges={builder.edges}
  onNodesChange={builder.onNodesChange}
  onEdgesChange={builder.onEdgesChange}
  onConnect={onConnect}                  // ← NEW
  ...
/>
```

### Connection validation hook (optional but recommended)

To prevent obviously-wrong connections at drag time (e.g. connecting a
`trigger` to itself, or wiring an output handle to another output handle),
ReactFlow has a sibling prop `isValidConnection`:

```tsx
const isValidConnection = useCallback((connection: Connection) => {
  // Source must be a node id present in current state.
  const sourceNode = builder.nodes.find((n) => n.id === connection.source);
  const targetNode = builder.nodes.find((n) => n.id === connection.target);
  if (!sourceNode || !targetNode) return false;
  // Triggers cannot have incoming edges.
  if (targetNode.data?.kind === "trigger") return false;
  return true;
}, [builder.nodes]);
```

Pass via `isValidConnection={isValidConnection}` — when false, the dragged
connection line turns red and won't drop. **This is a UX polish** — v1 can
ship without it; the brief lists it as optional.

---

## Phase 2 — Wire the Add Step palette

### Locate the palette

In `web/app/(app)/workflows/[verticalId]/[name]/build/page.tsx` find the
`<AddStepPalette>` component (declared inline as a sub-component near the
bottom of the file, ~lines 890-920). The current onClick handlers are stubs:

```tsx
// CURRENT
<button onClick={() => setPaletteOpen(false)}>Skill invoke</button>
```

### New onClick: add a node to the controlled state

Refactor the palette to accept a callback prop:

```tsx
interface AddStepPaletteProps {
  onSelect: (kind: StepKind) => void;
  onClose: () => void;
}

function AddStepPalette({ onSelect, onClose }: AddStepPaletteProps) {
  const choices: { kind: StepKind; label: string; Icon: typeof Sparkles; tone: string }[] = [
    { kind: "skill_invoke", label: "Skill invoke",  Icon: Sparkles,    tone: "violet" },
    { kind: "approval",     label: "Approval gate", Icon: ShieldCheck, tone: "amber"  },
    { kind: "tool_call",    label: "Tool call",     Icon: Database,    tone: "cyan"   },
    { kind: "notify",       label: "Notify",        Icon: MessageSquare, tone: "sky"  },
    { kind: "decision",     label: "Decision",      Icon: GitBranch,   tone: "fuchsia" },
    { kind: "loop",         label: "Loop",          Icon: Repeat,      tone: "lime"   },
    { kind: "wait",         label: "Wait",          Icon: Clock,       tone: "slate"  },
  ];
  return (
    <div role="dialog" aria-label="Add step" className="builder-palette">
      {choices.map(({ kind, label, Icon, tone }) => (
        <button
          key={kind}
          type="button"
          onClick={() => { onSelect(kind); onClose(); }}
          className={`builder-palette-item builder-palette-item--${tone}`}
        >
          <Icon size={16} /> {label}
        </button>
      ))}
    </div>
  );
}
```

### `handleAddStep` in the parent

```tsx
const handleAddStep = useCallback((kind: StepKind) => {
  // Generate a fresh id that doesn't collide with existing nodes.
  const id = `step-${crypto.randomUUID().slice(0, 8)}`;

  // Default name + description per step kind.
  const defaults: Record<StepKind, { name: string; description: string }> = {
    skill_invoke: { name: "New skill",       description: "Run a learned skill" },
    approval:     { name: "Approval gate",   description: "Request human review" },
    tool_call:    { name: "Tool call",       description: "Invoke an external tool" },
    notify:       { name: "Notify",          description: "Send a notification" },
    decision:     { name: "Decision",        description: "Branch on a condition" },
    data:         { name: "Data step",       description: "Read or transform data" },
    loop:         { name: "Loop",            description: "Iterate over a list" },
    wait:         { name: "Wait",            description: "Pause for a duration" },
    trigger:      { name: "Trigger",         description: "Workflow entry point" },
  };

  // Place the new node visually offset from the last existing node so it doesn't
  // overlap. If the canvas is empty, place at (200, 200).
  const last = builder.nodes[builder.nodes.length - 1];
  const position = last
    ? { x: last.position.x + 220, y: last.position.y + 60 }
    : { x: 200, y: 200 };

  builder.addNode({
    id,
    type: "workflowStep",          // matches the custom nodeType key
    position,
    data: {
      kind,
      name: defaults[kind].name,
      description: defaults[kind].description,
    },
  });

  // Optional: scroll the new node into view + select it for immediate editing.
  // Implementation depends on whether the brief wants instant edit-mode entry;
  // v1 leaves it unselected and lets the user click.
  toast.success(`${defaults[kind].label} added — drag to position.`);
}, [builder]);
```

### Wire palette to parent

```tsx
{paletteOpen ? (
  <AddStepPalette
    onSelect={handleAddStep}
    onClose={() => setPaletteOpen(false)}
  />
) : null}
```

---

## may-touch

- `web/app/(app)/workflows/[verticalId]/[name]/build/page.tsx` — `onConnect` + `isValidConnection` + `handleAddStep` + palette refactor.
- `web/tests/builder-connect.spec.ts` (new) — E2E for the connect handler.
- `web/tests/builder-add-step.spec.ts` (new) — E2E for the Add Step palette.
- 4 inline unit tests for `onConnect` validation logic (self-loop, dup edge,
  trigger-incoming reject) — colocate in `builder-controlled-state.test.ts`
  from the sibling brief if it exists, else new file.

## must-NOT-touch

- The `useWorkflowBuilderState` hook (sibling brief's surface — extend by
  using the existing `addNode` / `addEdge` helpers, don't add new ones here).
- Step config drawer / side panel — that's separate brief
  (`web-builder-step-config-drawer`, not yet authored).
- Custom node rendering (`workflowStep` nodeType) — handles already exist.

---

## Acceptance criteria

- [ ] `<ReactFlow onConnect={onConnect}>` is wired and dragging from a source handle to a target handle creates a visible, persistent edge.
- [ ] Self-loop attempt (drag source-handle to same-node target-handle) shows toast "A step can't connect to itself." and does NOT create an edge.
- [ ] Duplicate-edge attempt shows toast "These steps are already connected." and does NOT create a second edge.
- [ ] `dirty` flag becomes true after one successful connect (sibling brief's wiring).
- [ ] Clicking the floating "+ Add step" button opens the palette.
- [ ] Clicking any of the 7 step-kind buttons in the palette adds a new node to the canvas at an offset position.
- [ ] The new node is editable / draggable like the seed nodes.
- [ ] Save button enables (sibling brief's `dirty` flag flips true).
- [ ] Pressing Escape with the palette open closes it without adding a node.
- [ ] Pressing Tab inside the palette moves focus through the 7 buttons; first/last cycle naturally.
- [ ] Palette button has `aria-expanded={paletteOpen}` (already correct in the current code).
- [ ] E2E `builder-connect.spec.ts` passes: load builder, drag node-1's source handle onto node-2's target handle, verify the edge persists across React state inspection.
- [ ] E2E `builder-add-step.spec.ts` passes: load builder, click "+ Add step", select "Skill invoke", verify a new node appears with name "New skill".
- [ ] `pnpm typecheck && pnpm lint && pnpm test` clean.

---

## Gotchas

1. **`onConnect` only fires on COMPLETED drag-drop** between handles —
   don't try to handle the in-flight drag state here. That's
   `onConnectStart` / `onConnectEnd` (not in scope for v1).
2. **Edge id uniqueness**: `id: \`e-${source}-${target}\`` is unique per
   pair. Don't include a timestamp — that breaks dedup because re-dragging
   the same source→target pair would produce a new id every time.
3. **`crypto.randomUUID()` is browser-only**: works in Next.js client
   components but NOT in SSR. Since the builder is a `"use client"`
   component, this is fine. Don't try to use it in any helper that might
   run server-side.
4. **The Add Step palette is positioned absolutely** at canvas bottom-
   right. Make sure the new node position offset doesn't push the node
   off-screen — the `fitView` prop will auto-recenter on the next render,
   but the initial position should already be in-viewport.
5. **`toast.success` import**: already imported at the top of the file from
   `sonner` — reuse the existing import, don't add a parallel one.

---

## References

- ReactFlow `onConnect` docs: https://reactflow.dev/api-reference/types/connection
- ReactFlow `isValidConnection`: https://reactflow.dev/api-reference/react-flow#isvalidconnection
- Builder page: `web/app/(app)/workflows/[verticalId]/[name]/build/page.tsx:782` (ReactFlow) + `~890` (AddStepPalette)
- Sibling: `web-builder-canvas-controlled-state` (must merge first)
- Audit: `docs/skycot/audit/user-journey/2026-05-20-wave-13-gui-canvas.md`
