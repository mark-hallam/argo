"use client";
import {
  Background,
  Controls,
  MiniMap,
  ReactFlow,
  type Edge,
  type Node,
  type OnEdgesChange,
  type OnNodesChange,
} from "@xyflow/react";
import { Plus } from "lucide-react";
import { useMemo, useState } from "react";

import { SkycotStepNode } from "./BuilderCanvasNode";
import { AddStepPalette } from "./BuilderNodePalette";
import { STEP_KIND_META, type SkyNodeData } from "./types";

interface Props {
  nodes: Node[];
  edges: Edge[];
  onNodesChange: OnNodesChange;
  onEdgesChange: OnEdgesChange;
  onNodeClick: (event: React.MouseEvent, node: Node) => void;
}

export function BuilderCanvas({ nodes, edges, onNodesChange, onEdgesChange, onNodeClick }: Props) {
  const [paletteOpen, setPaletteOpen] = useState(false);
  const nodeTypes = useMemo(() => ({ skycot: SkycotStepNode }), []);

  return (
    <div className="builder-canvas">
      <ReactFlow
        nodes={nodes}
        edges={edges}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        nodeTypes={nodeTypes}
        onNodeClick={onNodeClick}
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
        onClick={() => setPaletteOpen((v) => !v)}
        aria-expanded={paletteOpen}
      >
        <Plus size={15} /> Add step
      </button>
      {paletteOpen ? <AddStepPalette onClose={() => setPaletteOpen(false)} /> : null}
    </div>
  );
}
