"use client";

import { useCallback, useMemo, useState } from "react";
import {
  Background,
  Controls,
  MiniMap,
  ReactFlow,
  type Edge as RfEdge,
  type Node as RfNode,
  type NodeMouseHandler,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import { cn } from "@/lib/utils";
import type {
  Edge as SkillEdge,
  Node as SkillNode,
  SkillGraph,
} from "@/lib/types/skill-ir";
import { AnnotationsPanel } from "./AnnotationsPanel";
import { EdgeRenderer, type SkillEdgeData } from "./EdgeRenderer";
import { NodeRenderer, type SkillNodeData } from "./NodeRenderer";

/**
 * Read-only Skill Canvas viewer (L4). Composes React Flow + per-kind
 * `NodeRenderer` + condition-aware `EdgeRenderer` + the `AnnotationsPanel`.
 * Layout: a deterministic top-down BFS from `graph.entry` so the same
 * IR draws identically on every load (React Flow's free-form positions
 * are unsuitable for an X-ray view — operators expect a stable layout).
 *
 * Editing is intentionally absent: `nodesDraggable`, `nodesConnectable`,
 * `elementsSelectable` (for edges) are all off. L7 unlocks them.
 */

export interface SkillCanvasProps {
  graph: SkillGraph;
  /**
   * Test hook — when set, skips ReactFlow rendering so unit tests can
   * assert on the underlying node/edge layout without depending on
   * jsdom-incompatible measurement APIs. Defaults to false.
   */
  skipFlowRender?: boolean;
  className?: string;
}

interface LayoutBox {
  x: number;
  y: number;
}

const ROW_SPACING = 110;
const COL_SPACING = 240;

export function computeLayout(graph: SkillGraph): Map<string, LayoutBox> {
  const adjacency = new Map<string, string[]>();
  for (const node of graph.nodes) adjacency.set(node.id, []);
  for (const edge of graph.edges) {
    const out = adjacency.get(edge.from);
    if (out && !out.includes(edge.to)) out.push(edge.to);
  }

  const depth = new Map<string, number>();
  const visited = new Set<string>();
  const queue: string[] = [];
  if (graph.nodes.some((n) => n.id === graph.entry)) {
    queue.push(graph.entry);
    depth.set(graph.entry, 0);
    visited.add(graph.entry);
  }

  while (queue.length > 0) {
    const current = queue.shift() as string;
    const currentDepth = depth.get(current) ?? 0;
    for (const next of adjacency.get(current) ?? []) {
      if (visited.has(next)) continue;
      visited.add(next);
      depth.set(next, currentDepth + 1);
      queue.push(next);
    }
  }

  // Any nodes not reachable from the entry get parked one row below the
  // deepest reachable node so they remain visible without overlapping
  // the main flow.
  const maxDepth = Math.max(0, ...Array.from(depth.values()));
  let orphanDepth = maxDepth + 1;
  for (const node of graph.nodes) {
    if (!depth.has(node.id)) {
      depth.set(node.id, orphanDepth);
      orphanDepth += 1;
    }
  }

  const rows = new Map<number, string[]>();
  for (const node of graph.nodes) {
    const d = depth.get(node.id) ?? 0;
    const list = rows.get(d) ?? [];
    list.push(node.id);
    rows.set(d, list);
  }

  const positions = new Map<string, LayoutBox>();
  for (const [d, ids] of rows.entries()) {
    const offset = -((ids.length - 1) * COL_SPACING) / 2;
    ids.forEach((id, idx) => {
      positions.set(id, { x: offset + idx * COL_SPACING, y: d * ROW_SPACING });
    });
  }
  return positions;
}

function toFlowNodes(
  graph: SkillGraph,
  layout: Map<string, LayoutBox>,
  selectedId: string | null,
): RfNode<SkillNodeData>[] {
  return graph.nodes.map<RfNode<SkillNodeData>>((irNode) => ({
    id: irNode.id,
    position: layout.get(irNode.id) ?? { x: 0, y: 0 },
    type: "skillNode",
    data: { irNode, isSelected: irNode.id === selectedId },
    draggable: false,
    selectable: true,
  }));
}

function toFlowEdges(graph: SkillGraph): RfEdge<SkillEdgeData>[] {
  return graph.edges.map<RfEdge<SkillEdgeData>>((irEdge: SkillEdge) => ({
    id: irEdge.id,
    source: irEdge.from,
    target: irEdge.to,
    type: "skillEdge",
    data: { cond: irEdge.cond },
  }));
}

const NODE_TYPES = { skillNode: NodeRenderer };
const EDGE_TYPES = { skillEdge: EdgeRenderer };

export function SkillCanvas({ graph, skipFlowRender, className }: SkillCanvasProps) {
  const [selectedId, setSelectedId] = useState<string | null>(null);

  const layout = useMemo(() => computeLayout(graph), [graph]);
  const flowNodes = useMemo(
    () => toFlowNodes(graph, layout, selectedId),
    [graph, layout, selectedId],
  );
  const flowEdges = useMemo(() => toFlowEdges(graph), [graph]);

  const selectedNode: SkillNode | null = useMemo(() => {
    if (!selectedId) return null;
    return graph.nodes.find((n) => n.id === selectedId) ?? null;
  }, [graph, selectedId]);

  const handleNodeClick: NodeMouseHandler = useCallback((_event, node) => {
    setSelectedId((prev) => (prev === node.id ? null : node.id));
  }, []);

  const handlePaneClick = useCallback(() => setSelectedId(null), []);

  return (
    <div
      data-testid="skill-canvas"
      data-node-count={graph.nodes.length}
      className={cn(
        "flex h-[640px] min-h-[480px] w-full overflow-hidden rounded-[var(--radius-lg)] border border-[var(--border)] bg-[var(--background)]",
        className,
      )}
    >
      <div className="relative flex-1 min-w-0">
        {skipFlowRender ? (
          <SkillCanvasDomFallback
            graph={graph}
            selectedId={selectedId}
            onSelect={(id) => setSelectedId((prev) => (prev === id ? null : id))}
          />
        ) : (
          <ReactFlow
            nodes={flowNodes}
            edges={flowEdges}
            nodeTypes={NODE_TYPES}
            edgeTypes={EDGE_TYPES}
            onNodeClick={handleNodeClick}
            onPaneClick={handlePaneClick}
            fitView
            fitViewOptions={{ padding: 0.2 }}
            proOptions={{ hideAttribution: true }}
            nodesDraggable={false}
            nodesConnectable={false}
            edgesFocusable={false}
            elementsSelectable
            zoomOnDoubleClick={false}
          >
            <Background gap={24} />
            <Controls showInteractive={false} />
            <MiniMap pannable zoomable />
          </ReactFlow>
        )}
      </div>
      <AnnotationsPanel node={selectedNode} />
    </div>
  );
}

/**
 * DOM-only fallback for tests + non-measurable environments. Lists
 * every node as a button so selection logic and per-kind data can be
 * asserted without ReactFlow's ResizeObserver dependency. Visual
 * differs from the real renderer — production always uses ReactFlow.
 */
interface FallbackProps {
  graph: SkillGraph;
  selectedId: string | null;
  onSelect: (id: string) => void;
}

function SkillCanvasDomFallback({ graph, selectedId, onSelect }: FallbackProps) {
  return (
    <ul
      data-testid="skill-canvas-fallback"
      role="list"
      className="flex flex-col gap-2 p-3 overflow-y-auto h-full"
    >
      {graph.nodes.map((node) => (
        <li key={node.id}>
          <button
            type="button"
            data-testid={`canvas-node-${node.id}`}
            data-kind={node.kind.type}
            data-selected={selectedId === node.id || undefined}
            aria-pressed={selectedId === node.id}
            onClick={() => onSelect(node.id)}
            className="block w-full rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--card)] px-3 py-2 text-left text-sm hover:bg-[var(--accent)]/30"
          >
            <span className="block text-[10px] uppercase tracking-wide text-[var(--muted-foreground)]">
              {node.kind.type}
            </span>
            <span className="block font-medium text-[var(--card-foreground)]">
              {node.label}
            </span>
          </button>
        </li>
      ))}
    </ul>
  );
}
