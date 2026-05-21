import { useCallback, useMemo, useState } from "react";
import {
  applyEdgeChanges,
  applyNodeChanges,
  type Edge,
  type Node,
  type OnEdgesChange,
  type OnNodesChange,
} from "@xyflow/react";

/**
 * Single source of truth for the visual workflow builder.
 *
 * Wraps ReactFlow's controlled-mode change handlers and exposes typed
 * helpers (`addNode`, `removeNode`, `addEdge`, `removeEdge`) that
 * maintain graph invariants (unique ids, no orphan edges). Sibling
 * briefs — connect-and-add-step, delete-and-duplicate, keyboard
 * shortcuts — write through this surface.
 *
 * `dirty` becomes true when `nodes` or `edges` diverge from
 * `initialNodes` / `initialEdges`. The comparison is a JSON
 * round-trip; on graphs > 100 nodes a follow-up should swap to a
 * content hash.
 */
export function useWorkflowBuilderState(
  initialNodes: Node[],
  initialEdges: Edge[],
) {
  const [nodes, setNodes] = useState<Node[]>(initialNodes);
  const [edges, setEdges] = useState<Edge[]>(initialEdges);

  const onNodesChange: OnNodesChange = useCallback(
    (changes) => setNodes((nds) => applyNodeChanges(changes, nds)),
    [],
  );
  const onEdgesChange: OnEdgesChange = useCallback(
    (changes) => setEdges((eds) => applyEdgeChanges(changes, eds)),
    [],
  );

  const addNode = useCallback((node: Node) => {
    setNodes((nds) => (nds.some((n) => n.id === node.id) ? nds : [...nds, node]));
  }, []);

  const removeNode = useCallback((id: string) => {
    setNodes((nds) => nds.filter((n) => n.id !== id));
    setEdges((eds) => eds.filter((e) => e.source !== id && e.target !== id));
  }, []);

  const addEdge = useCallback((edge: Edge) => {
    setEdges((eds) => (eds.some((e) => e.id === edge.id) ? eds : [...eds, edge]));
  }, []);

  const removeEdge = useCallback((id: string) => {
    setEdges((eds) => eds.filter((e) => e.id !== id));
  }, []);

  const reset = useCallback((nextNodes: Node[], nextEdges: Edge[]) => {
    setNodes(nextNodes);
    setEdges(nextEdges);
  }, []);

  const dirty = useMemo(
    () =>
      nodes.length !== initialNodes.length
      || edges.length !== initialEdges.length
      || JSON.stringify(nodes) !== JSON.stringify(initialNodes)
      || JSON.stringify(edges) !== JSON.stringify(initialEdges),
    [nodes, edges, initialNodes, initialEdges],
  );

  return {
    nodes,
    edges,
    setNodes,
    setEdges,
    onNodesChange,
    onEdgesChange,
    addNode,
    removeNode,
    addEdge,
    removeEdge,
    reset,
    dirty,
  };
}
