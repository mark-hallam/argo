"use client";
import {
  Background,
  Controls,
  MiniMap,
  ReactFlow,
  type Edge,
  type IsValidConnection,
  type Node,
  type OnConnect,
  type OnEdgesChange,
  type OnNodeDrag,
  type OnNodesChange,
  type OnNodesDelete,
} from "@xyflow/react";
import { Keyboard, Plus } from "lucide-react";
import { useCallback, useMemo, useState } from "react";

import { SkycotStepNode } from "./BuilderCanvasNode";
import { AddStepPalette } from "./BuilderNodePalette";
import { SubWorkflowNode } from "./SubWorkflowNode";
import { SubWorkflowPicker } from "./SubWorkflowPicker";
import {
  PALETTE_DRAG_MIME,
  STEP_KIND_META,
  type PaletteItem,
  type SkyNodeData,
  type StepKind,
  type SubWorkflowRef,
} from "./types";

interface Props {
  nodes: Node[];
  edges: Edge[];
  onNodesChange: OnNodesChange;
  onEdgesChange: OnEdgesChange;
  onConnect: OnConnect;
  isValidConnection?: IsValidConnection<Edge>;
  onNodeClick: (event: React.MouseEvent, node: Node) => void;
  onSelectStepKind: (kind: StepKind, subWorkflow?: SubWorkflowRef) => void;
  onPaletteDrop?: (item: PaletteItem, clientX: number, clientY: number) => void;
  // Excluded from sub-workflow picker results so a workflow can't pick
  // itself as its own sub-workflow.
  currentVerticalId: string;
  currentName: string;
  onNodesDelete?: OnNodesDelete;
  onEdgesDelete?: (deleted: Edge[]) => void;
  onNodeContextMenu?: (event: React.MouseEvent, node: Node) => void;
  onEdgeContextMenu?: (event: React.MouseEvent, edge: Edge) => void;
  onPaneClick?: () => void;
  onNodeDragStart?: OnNodeDrag;
  onNodeDragStop?: OnNodeDrag;
  onShowShortcuts?: () => void;
}

export const SNAP_GRID_SIZE = 20;


export function BuilderCanvas({
  nodes,
  edges,
  onNodesChange,
  onEdgesChange,
  onConnect,
  isValidConnection,
  onNodeClick,
  onSelectStepKind,
  onPaletteDrop,
  currentVerticalId,
  currentName,
  onNodesDelete,
  onEdgesDelete,
  onNodeContextMenu,
  onEdgeContextMenu,
  onPaneClick,
  onNodeDragStart,
  onNodeDragStop,
  onShowShortcuts,
}: Props) {
  const [paletteOpen, setPaletteOpen] = useState(false);
  const [pickerOpen, setPickerOpen] = useState(false);
  const nodeTypes = useMemo(
    () => ({ skycot: SkycotStepNode, subWorkflow: SubWorkflowNode }),
    [],
  );

  // Accept palette drags. Filtering on the custom MIME means React
  // Flow's own pointer-based node drags (which never touch
  // dataTransfer) are ignored — no collision with existing handles.
  const handleDragOver = useCallback((e: React.DragEvent<HTMLDivElement>) => {
    if (!onPaletteDrop) return;
    if (!Array.from(e.dataTransfer.types).includes(PALETTE_DRAG_MIME)) return;
    e.preventDefault();
    e.dataTransfer.dropEffect = "copy";
  }, [onPaletteDrop]);

  const handleDrop = useCallback((e: React.DragEvent<HTMLDivElement>) => {
    if (!onPaletteDrop) return;
    const raw = e.dataTransfer.getData(PALETTE_DRAG_MIME);
    if (!raw) return;
    e.preventDefault();
    try {
      const item = JSON.parse(raw) as PaletteItem;
      onPaletteDrop(item, e.clientX, e.clientY);
    } catch {
      // Malformed payload — ignore.
    }
  }, [onPaletteDrop]);

  const handlePaletteSelect = useCallback(
    (kind: StepKind) => {
      if (kind === "sub_workflow") {
        setPickerOpen(true);
        setPaletteOpen(false);
        return;
      }
      onSelectStepKind(kind);
    },
    [onSelectStepKind],
  );

  const handlePickerPick = useCallback(
    (ref: SubWorkflowRef) => {
      setPickerOpen(false);
      onSelectStepKind("sub_workflow", ref);
    },
    [onSelectStepKind],
  );

  return (
    <div className="builder-canvas" onDragOver={handleDragOver} onDrop={handleDrop}>
      <ReactFlow
        nodes={nodes}
        edges={edges}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        onConnect={onConnect}
        isValidConnection={isValidConnection}
        nodeTypes={nodeTypes}
        onNodeClick={onNodeClick}
        onNodesDelete={onNodesDelete}
        onEdgesDelete={onEdgesDelete}
        onNodeContextMenu={onNodeContextMenu}
        onEdgeContextMenu={onEdgeContextMenu}
        onPaneClick={onPaneClick}
        onNodeDragStart={onNodeDragStart}
        onNodeDragStop={onNodeDragStop}
        deleteKeyCode={["Backspace", "Delete"]}
        snapToGrid
        snapGrid={[SNAP_GRID_SIZE, SNAP_GRID_SIZE]}
        fitView
        fitViewOptions={{ padding: 0.18 }}
        attributionPosition="bottom-right"
      >
        <Background gap={20} size={1.2} color="#d4d4d8" />
        <Controls position="bottom-left" />
        <MiniMap
          nodeColor={(n) => {
            const sky = n.data as SkyNodeData | undefined;
            const status = sky?.runState?.status;
            if (status === "running") return "#f59e0b";
            if (status === "success") return "#10b981";
            if (status === "error") return "#f43f5e";
            const step = sky?.step;
            return step ? STEP_KIND_META[step.kind].color : "#8b5cf6";
          }}
          nodeStrokeColor={(n) => {
            const step = (n.data as SkyNodeData | undefined)?.step;
            return step?.kind === "sub_workflow" ? "#1e1b4b" : "transparent";
          }}
          nodeStrokeWidth={3}
          maskColor="rgba(250,250,255,0.7)"
          pannable
          zoomable
          position="bottom-right"
        />
      </ReactFlow>
      <button
        type="button"
        className="builder-add-step"
        data-testid="builder-add-step"
        onClick={() => setPaletteOpen((v) => !v)}
        aria-expanded={paletteOpen}
      >
        <Plus size={15} /> Add step
      </button>
      {onShowShortcuts ? (
        <button
          type="button"
          className="builder-shortcuts-button"
          onClick={onShowShortcuts}
          aria-label="Show keyboard shortcuts"
          data-testid="builder-shortcuts-button"
          title="Keyboard shortcuts (?)"
        >
          <Keyboard size={14} /> Shortcuts
        </button>
      ) : null}
      {paletteOpen ? (
        <AddStepPalette
          onSelect={handlePaletteSelect}
          onClose={() => setPaletteOpen(false)}
        />
      ) : null}
      {pickerOpen ? (
        <SubWorkflowPicker
          currentVerticalId={currentVerticalId}
          currentName={currentName}
          onPick={handlePickerPick}
          onClose={() => setPickerOpen(false)}
        />
      ) : null}
    </div>
  );
}
