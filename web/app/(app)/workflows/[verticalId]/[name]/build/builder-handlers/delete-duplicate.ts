import type { Edge, Node, ReactFlowInstance } from "@xyflow/react";
import { Copy, Settings as SettingsIcon, StickyNote as StickyNoteIcon, Trash2, Unlink } from "lucide-react";
import { useCallback, useMemo, useState, type Dispatch, type SetStateAction } from "react";
import { toast } from "sonner";

import type { useBuilderAnnotations } from "@/lib/hooks/use-builder-annotations";
import type { useWorkflowBuilderState } from "@/lib/hooks/use-workflow-builder-state";
import type { ContextMenuItem } from "@/components/skycot/workflows/WorkflowContextMenu";

import type { BuilderSnapshot } from "@/lib/hooks/use-builder-history";

import type { SkyNodeData } from "../types";

export interface ContextMenuState {
  x: number;
  y: number;
  nodeId: string | null;
  edgeId: string | null;
  // Flow-space coords captured at right-click on empty canvas, used as
  // the drop position for "Add sticky note". null when the menu was
  // opened on a node/edge.
  paneFlow: { x: number; y: number } | null;
}

function nodeStep(node: Node | undefined): SkyNodeData["step"] | undefined {
  return (node?.data as SkyNodeData | undefined)?.step;
}

type Builder = ReturnType<typeof useWorkflowBuilderState>;
type Annotations = ReturnType<typeof useBuilderAnnotations>;

export interface DeleteDuplicateDeps {
  builder: Builder;
  annotations: Annotations;
  reactFlow: Pick<ReactFlowInstance, "screenToFlowPosition">;
  selectedNode: Node | null;
  setSelectedNode: Dispatch<SetStateAction<Node | null>>;
  setTriggerConfigOpen: Dispatch<SetStateAction<boolean>>;
  captureSnapshot: (label: string) => BuilderSnapshot;
  pushHistory: (snapshot: BuilderSnapshot) => void;
}

export interface DeleteDuplicateHandlers {
  contextMenu: ContextMenuState | null;
  contextMenuItems: ContextMenuItem[];
  onNodesDelete: (deleted: Node[]) => void;
  onEdgesDelete: (deleted: Edge[]) => void;
  onNodesChange: Builder["onNodesChange"];
  duplicateNode: (nodeId: string) => void;
  onNodeContextMenu: (event: React.MouseEvent, node: Node) => void;
  onEdgeContextMenu: (event: React.MouseEvent, edge: Edge) => void;
  onPaneClick: () => void;
  onPaneContextMenu: (event: React.MouseEvent | MouseEvent) => void;
  closeContextMenu: () => void;
}

/**
 * Delete / duplicate / context-menu handlers for the visual workflow
 * builder.
 *
 * Owns the right-click context-menu state and the `onNodesChange`
 * wrapper that routes sticky-note position/remove changes to the
 * annotations hook (stickies live outside `builder.nodes`).
 *
 * Single delete is undoable via a toast action that calls
 * `builder.addNode`; bulk delete drops a `window.confirm` first.
 * Triggers can be neither deleted nor duplicated — the handler re-adds
 * the original node when a delete attempt selects one.
 */
export function useDeleteDuplicateHandlers(
  deps: DeleteDuplicateDeps,
): DeleteDuplicateHandlers {
  const {
    builder,
    annotations,
    reactFlow,
    selectedNode,
    setSelectedNode,
    setTriggerConfigOpen,
    captureSnapshot,
    pushHistory,
  } = deps;

  const [contextMenu, setContextMenu] = useState<ContextMenuState | null>(null);

  // ReactFlow optimistically removes the node before this fires; for
  // single-node deletes we surface an "Undo" toast that re-adds via
  // builder.addNode (the controlled-state hook is idempotent on id).
  // Pushes a history snapshot for keyboard undo (Cmd+Z) too.
  const onNodesDelete = useCallback(
    (deleted: Node[]) => {
      // Stickies live in `annotations.notes`, not `builder.nodes`. Route
      // their deletes through the annotations hook so the source of
      // truth updates; otherwise the next render rebuilds the sticky.
      const stickies = deleted.filter((n) => n.type === "sticky");
      for (const s of stickies) annotations.deleteNote(s.id);

      const steps = deleted.filter((n) => n.type !== "sticky");
      if (steps.length === 0) return;
      const label = `Delete ${steps.length} step${steps.length === 1 ? "" : "s"}`;
      pushHistory(captureSnapshot(label));
      const triggers = steps.filter((n) => nodeStep(n)?.kind === "trigger");
      const deletable = steps.filter((n) => nodeStep(n)?.kind !== "trigger");

      if (triggers.length > 0) {
        toast.error("Trigger steps can't be deleted", {
          description: "Change the trigger kind from the side panel instead.",
        });
        for (const t of triggers) builder.addNode(t);
      }
      if (deletable.length === 0) return;

      // Single delete → instant + undoable.
      if (deletable.length === 1) {
        const only = deletable[0];
        builder.removeNode(only.id);
        if (selectedNode?.id === only.id) setSelectedNode(null);
        toast.success("Step deleted", {
          action: {
            label: "Undo",
            onClick: () => builder.addNode(only),
          },
        });
        return;
      }

      // Bulk delete → confirm via window.confirm (cheap, no modal dep).
      const confirmed = typeof window !== "undefined"
        ? window.confirm(
            `Delete ${deletable.length} steps? This also removes edges connecting them.`,
          )
        : true;
      if (!confirmed) {
        for (const n of deletable) builder.addNode(n);
        return;
      }
      for (const n of deletable) builder.removeNode(n.id);
      if (selectedNode && deletable.some((n) => n.id === selectedNode.id)) {
        setSelectedNode(null);
      }
      toast.success(`${deletable.length} steps deleted`);
    },
    [builder, selectedNode, setSelectedNode, captureSnapshot, pushHistory, annotations],
  );

  const onEdgesDelete = useCallback(
    (deleted: Edge[]) => {
      for (const edge of deleted) builder.removeEdge(edge.id);
    },
    [builder],
  );

  // Stickies live outside `builder.nodes` (their source is
  // `annotations.notes`). React Flow doesn't know that — it fires
  // position + remove changes for stickies alongside step nodes. Route
  // sticky-id changes to the annotations hook and pass step-node
  // changes through to the builder's controlled state.
  const onNodesChange = useCallback<Builder["onNodesChange"]>(
    (changes) => {
      const forBuilder: typeof changes = [];
      for (const change of changes) {
        if (change.type === "position" && annotations.isNoteId(change.id)) {
          if (change.position) {
            annotations.updateNotePosition(
              change.id,
              change.position.x,
              change.position.y,
            );
          }
          continue;
        }
        if (change.type === "remove" && annotations.isNoteId(change.id)) {
          annotations.deleteNote(change.id);
          continue;
        }
        forBuilder.push(change);
      }
      if (forBuilder.length > 0) builder.onNodesChange(forBuilder);
    },
    [builder, annotations],
  );

  const duplicateNode = useCallback(
    (nodeId: string) => {
      const original = builder.nodes.find((n) => n.id === nodeId);
      if (!original) return;
      const step = nodeStep(original);
      if (step?.kind === "trigger") {
        toast.error("Trigger steps can't be duplicated", {
          description: "Only one trigger per workflow.",
        });
        return;
      }
      if (!step) return;
      const suffix = typeof crypto !== "undefined" && "randomUUID" in crypto
        ? crypto.randomUUID().slice(0, 8)
        : Math.random().toString(36).slice(2, 10);
      const newId = `step-${suffix}`;
      const duplicated: Node = {
        ...original,
        id: newId,
        position: { x: original.position.x + 60, y: original.position.y + 60 },
        data: { step: { ...step, id: newId, name: `${step.name} (copy)` } },
        selected: false,
      };
      builder.addNode(duplicated);
      toast.success("Step duplicated");
    },
    [builder],
  );

  const onNodeContextMenu = useCallback((event: React.MouseEvent, node: Node) => {
    event.preventDefault();
    setContextMenu({ x: event.clientX, y: event.clientY, nodeId: node.id, edgeId: null, paneFlow: null });
  }, []);

  const onEdgeContextMenu = useCallback((event: React.MouseEvent, edge: Edge) => {
    event.preventDefault();
    setContextMenu({ x: event.clientX, y: event.clientY, nodeId: null, edgeId: edge.id, paneFlow: null });
  }, []);

  const onPaneClick = useCallback(() => setContextMenu(null), []);
  const closeContextMenu = useCallback(() => setContextMenu(null), []);

  const onPaneContextMenu = useCallback(
    (event: React.MouseEvent | MouseEvent) => {
      event.preventDefault();
      const flow = reactFlow.screenToFlowPosition({
        x: event.clientX,
        y: event.clientY,
      });
      setContextMenu({
        x: event.clientX,
        y: event.clientY,
        nodeId: null,
        edgeId: null,
        paneFlow: flow,
      });
    },
    [reactFlow],
  );

  const contextMenuItems = useMemo<ContextMenuItem[]>(() => {
    if (!contextMenu) return [];
    if (contextMenu.nodeId) {
      const node = builder.nodes.find((n) => n.id === contextMenu.nodeId);
      if (!node) return [];
      const isTrigger = nodeStep(node)?.kind === "trigger";
      return [
        {
          label: "Configure",
          Icon: SettingsIcon,
          onClick: () => {
            setSelectedNode(node);
            setTriggerConfigOpen(false);
          },
        },
        {
          label: "Duplicate",
          Icon: Copy,
          onClick: () => duplicateNode(node.id),
          disabled: isTrigger,
        },
        {
          label: "Delete",
          Icon: Trash2,
          onClick: () => onNodesDelete([node]),
          destructive: true,
          disabled: isTrigger,
        },
      ];
    }
    if (contextMenu.edgeId) {
      const edgeId = contextMenu.edgeId;
      return [
        {
          label: "Disconnect",
          Icon: Unlink,
          onClick: () => builder.removeEdge(edgeId),
          destructive: true,
        },
      ];
    }
    if (contextMenu.paneFlow) {
      const { x, y } = contextMenu.paneFlow;
      return [
        {
          label: "Add sticky note",
          Icon: StickyNoteIcon,
          onClick: () => {
            annotations.addNote(x, y);
            toast.success("Sticky note added", {
              description: "Double-click to edit, drag to reposition.",
            });
          },
        },
      ];
    }
    return [];
  }, [
    contextMenu,
    builder,
    duplicateNode,
    onNodesDelete,
    annotations,
    setSelectedNode,
    setTriggerConfigOpen,
  ]);

  return {
    contextMenu,
    contextMenuItems,
    onNodesDelete,
    onEdgesDelete,
    onNodesChange,
    duplicateNode,
    onNodeContextMenu,
    onEdgeContextMenu,
    onPaneClick,
    onPaneContextMenu,
    closeContextMenu,
  };
}
