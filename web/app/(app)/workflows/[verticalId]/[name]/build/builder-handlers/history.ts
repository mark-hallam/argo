import type { Edge, Node, ReactFlowInstance } from "@xyflow/react";
import { useCallback, useRef, type Dispatch, type SetStateAction } from "react";
import { toast } from "sonner";

import {
  useBuilderHistory,
  type BuilderSnapshot,
} from "@/lib/hooks/use-builder-history";

import { SNAP_GRID_SIZE } from "../BuilderCanvas";

export interface HistoryHandlerDeps {
  nodes: Node[];
  edges: Edge[];
  setNodes: Dispatch<SetStateAction<Node[]>>;
  setEdges: Dispatch<SetStateAction<Edge[]>>;
}

export interface HistoryHandlers {
  history: ReturnType<typeof useBuilderHistory>;
  captureSnapshot: (label: string) => BuilderSnapshot;
  onNodeDragStart: () => void;
  onNodeDragStop: () => void;
  handleUndo: () => void;
  handleRedo: () => void;
  handleSelectAll: () => void;
  handleDuplicateSelected: () => void;
  onFitView: () => void;
  onZoomIn: () => void;
  onZoomOut: () => void;
}

/**
 * Cmd+Z/Cmd+Shift+Z + drag-history + select-all + duplicate-selected
 * handlers for the visual workflow builder.
 *
 * Keeps two refs internally:
 *
 *   - `stateRef` — latest nodes/edges so snapshot helpers always sample
 *     the current graph (the handlers below are stable across renders
 *     but read mutable state).
 *   - `preDragSnapshotRef` — the BEFORE-position graph captured at
 *     `onNodeDragStart`, popped on `onNodeDragStop` so each drag yields
 *     exactly one history entry (gotcha #4 from the builder brief).
 */
export function useHistoryHandlers(
  deps: HistoryHandlerDeps,
  reactFlow: Pick<ReactFlowInstance, "fitView" | "zoomIn" | "zoomOut">,
): HistoryHandlers {
  const { nodes, edges, setNodes, setEdges } = deps;
  const history = useBuilderHistory();

  const stateRef = useRef({ nodes, edges });
  stateRef.current = { nodes, edges };
  const preDragSnapshotRef = useRef<BuilderSnapshot | null>(null);

  const captureSnapshot = useCallback(
    (label: string): BuilderSnapshot => ({
      nodes: structuredClone(stateRef.current.nodes),
      edges: structuredClone(stateRef.current.edges),
      label,
      takenAt: Date.now(),
    }),
    [],
  );

  const onNodeDragStart = useCallback(() => {
    preDragSnapshotRef.current = captureSnapshot("Move step");
  }, [captureSnapshot]);

  const onNodeDragStop = useCallback(() => {
    const before = preDragSnapshotRef.current;
    preDragSnapshotRef.current = null;
    if (!before) return;
    const movedNodes = stateRef.current.nodes;
    const positionChanged =
      before.nodes.length !== movedNodes.length
      || before.nodes.some((bn, idx) => {
        const mn = movedNodes[idx];
        if (!mn || bn.id !== mn.id) return true;
        return bn.position.x !== mn.position.x || bn.position.y !== mn.position.y;
      });
    if (positionChanged) history.push(before);
  }, [history]);

  const handleUndo = useCallback(() => {
    const prev = history.undo();
    if (!prev) return;
    history.pushFuture({
      nodes: structuredClone(stateRef.current.nodes),
      edges: structuredClone(stateRef.current.edges),
      label: prev.label,
      takenAt: Date.now(),
    });
    setNodes(prev.nodes);
    setEdges(prev.edges);
    toast.message(`Undone: ${prev.label}`);
  }, [history, setNodes, setEdges]);

  const handleRedo = useCallback(() => {
    const next = history.redo();
    if (!next) return;
    history.pushPast({
      nodes: structuredClone(stateRef.current.nodes),
      edges: structuredClone(stateRef.current.edges),
      label: next.label,
      takenAt: Date.now(),
    });
    setNodes(next.nodes);
    setEdges(next.edges);
    toast.message(`Redone: ${next.label}`);
  }, [history, setNodes, setEdges]);

  const handleSelectAll = useCallback(() => {
    setNodes((nds) => nds.map((n) => ({ ...n, selected: true })));
  }, [setNodes]);

  const handleDuplicateSelected = useCallback(() => {
    const selected = stateRef.current.nodes.filter((n) => n.selected);
    if (selected.length === 0) return;
    history.push(
      captureSnapshot(
        `Duplicate ${selected.length} step${selected.length === 1 ? "" : "s"}`,
      ),
    );
    const clones: Node[] = selected.map((n) => {
      const id = (typeof crypto !== "undefined" && "randomUUID" in crypto)
        ? crypto.randomUUID()
        : `${n.id}-copy-${Date.now()}`;
      return {
        ...n,
        id,
        position: {
          x: n.position.x + SNAP_GRID_SIZE * 2,
          y: n.position.y + SNAP_GRID_SIZE * 2,
        },
        selected: false,
      };
    });
    setNodes((nds) => [
      ...nds.map((n) => ({ ...n, selected: false })),
      ...clones,
    ]);
  }, [captureSnapshot, history, setNodes]);

  const onFitView = useCallback(
    () => reactFlow.fitView({ padding: 0.18, duration: 250 }),
    [reactFlow],
  );
  const onZoomIn = useCallback(() => reactFlow.zoomIn({ duration: 150 }), [reactFlow]);
  const onZoomOut = useCallback(() => reactFlow.zoomOut({ duration: 150 }), [reactFlow]);

  return {
    history,
    captureSnapshot,
    onNodeDragStart,
    onNodeDragStop,
    handleUndo,
    handleRedo,
    handleSelectAll,
    handleDuplicateSelected,
    onFitView,
    onZoomIn,
    onZoomOut,
  };
}
