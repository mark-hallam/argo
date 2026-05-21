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
  type OnNodesChange,
} from "@xyflow/react";
import { Plus } from "lucide-react";
import { useMemo, useState } from "react";

import { SkycotStepNode } from "./BuilderCanvasNode";
import { AddStepPalette } from "./BuilderNodePalette";
import { STEP_KIND_META, type SkyNodeData, type StepKind } from "./types";

interface Props {
  nodes: Node[];
  edges: Edge[];
  onNodesChange: OnNodesChange;
  onEdgesChange: OnEdgesChange;
  onConnect: OnConnect;
  isValidConnection?: IsValidConnection<Edge>;
  onNodeClick: (event: React.MouseEvent, node: Node) => void;
  onSelectStepKind: (kind: StepKind) => void;
  onNodesDelete?: (deleted: Node[]) => void;
  onEdgesDelete?: (deleted: Edge[]) => void;
  onNodeContextMenu?: (event: React.MouseEvent, node: Node) => void;
  onEdgeContextMenu?: (event: React.MouseEvent, edge: Edge) => void;
  onPaneClick?: () => void;
}

export function BuilderCanvas({
  nodes,
  edges,
  onNodesChange,
  onEdgesChange,
  onConnect,
  isValidConnection,
  onNodeClick,
  onSelectStepKind,
  onNodesDelete,
  onEdgesDelete,
  onNodeContextMenu,
  onEdgeContextMenu,
  onPaneClick,
}: Props) {
  const [paletteOpen, setPaletteOpen] = useState(false);
  const nodeTypes = useMemo(() => ({ skycot: SkycotStepNode }), []);

  return (
    <div className="builder-canvas">
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
        deleteKeyCode={["Backspace", "Delete"]}
        fitView
        fitViewOptions={{ padding: 0.18 }}
        attributionPosition="bottom-right"
      >
        <Background gap={20} size={1.2} color="#d4d4d8" />
        <Controls position="bottom-left" />
        <MiniMap
          nodeColor={(n) => {
            const step = (n.data as SkyNodeData | undefined)?.step;
            return step ? STEP_KIND_META[step.kind].color : "#8b5cf6";
          }}
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
      {paletteOpen ? (
        <AddStepPalette
          onSelect={onSelectStepKind}
          onClose={() => setPaletteOpen(false)}
        />
      ) : null}
    </div>
  );
}
