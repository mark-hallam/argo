import type { Edge, Node, ReactFlowInstance } from "@xyflow/react";
import { useCallback } from "react";
import { toast } from "sonner";

import { findNearestPredecessor, newStepNode, newStepNodeAt } from "./builder-handlers";
import {
  TRUE_EDGE_STYLE,
  type PaletteItem,
  type SkyNodeData,
} from "./types";

export interface PaletteHandlerDeps {
  reactFlow: Pick<ReactFlowInstance, "screenToFlowPosition">;
  nodes: Node[];
  addNode: (node: Node) => void;
  addEdge: (edge: Edge) => void;
}

export interface PaletteHandlers {
  handlePaletteDrop: (item: PaletteItem, clientX: number, clientY: number) => void;
  handlePaletteSelect: (item: PaletteItem) => void;
}

/**
 * Drag-from-palette + click-to-add handlers for the left sidebar.
 *
 * Both paths spawn a node with sensible defaults; the drop path uses
 * the actual drop coordinate while the click path appends below the
 * last node. Non-trigger kinds auto-wire from the closest predecessor
 * (drop) or the last node (click). Triggers never receive an incoming
 * edge per `validateConnection`'s `trigger_incoming` rule. A node
 * dropped on a `decision` predecessor is left unwired so the user can
 * route it to the true/false branch explicitly.
 */
export function usePaletteHandlers(deps: PaletteHandlerDeps): PaletteHandlers {
  const { reactFlow, nodes, addNode, addEdge } = deps;

  const handlePaletteDrop = useCallback(
    (item: PaletteItem, clientX: number, clientY: number) => {
      const position = reactFlow.screenToFlowPosition({ x: clientX, y: clientY });
      const suffix = crypto.randomUUID().slice(0, 8);
      const node = newStepNodeAt(item.kind, suffix, position, {
        name: item.label,
        description: item.description,
        triggerKind: item.triggerKind,
      });
      addNode(node);
      if (item.kind !== "trigger") {
        const pred = findNearestPredecessor(nodes, position.y);
        if (pred && (pred.data as SkyNodeData).step.kind !== "decision") {
          addEdge({
            id: `e-${pred.id}-${node.id}`,
            source: pred.id,
            target: node.id,
            animated: true,
            style: TRUE_EDGE_STYLE,
          });
        }
      }
      toast.success(`${item.label} added`, { description: item.description });
    },
    [reactFlow, nodes, addNode, addEdge],
  );

  const handlePaletteSelect = useCallback(
    (item: PaletteItem) => {
      const suffix = crypto.randomUUID().slice(0, 8);
      const base = newStepNode(item.kind, nodes, suffix);
      const baseStep = (base.data as SkyNodeData).step;
      const node: Node = {
        ...base,
        data: {
          step: {
            ...baseStep,
            name: item.label,
            description: item.description,
            ...(item.triggerKind ? { triggerKind: item.triggerKind } : {}),
          },
        },
      };
      addNode(node);
      if (item.kind !== "trigger" && nodes.length > 0) {
        const pred = nodes[nodes.length - 1];
        if (pred && (pred.data as SkyNodeData).step.kind !== "decision") {
          addEdge({
            id: `e-${pred.id}-${node.id}`,
            source: pred.id,
            target: node.id,
            animated: true,
            style: TRUE_EDGE_STYLE,
          });
        }
      }
      toast.success(`${item.label} added`, { description: item.description });
    },
    [nodes, addNode, addEdge],
  );

  return { handlePaletteDrop, handlePaletteSelect };
}
