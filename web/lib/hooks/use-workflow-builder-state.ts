import { useCallback, useState } from "react";
import {
  applyEdgeChanges,
  applyNodeChanges,
  type Edge,
  type Node,
  type NodeChange,
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
 * `dirty` is a boolean that starts `false` and only flips `true` on
 * a user-initiated graph mutation (drag, add, remove, edge connect,
 * inline step edit). System-only ReactFlow callbacks — measurement
 * (`dimensions`) and selection (`select`) — leave it alone, so the
 * `beforeunload` listener in the page stays disarmed on initial
 * load. `reset()` and a fresh `initial*` snapshot both clear it.
 */

// ReactFlow fires `dimensions` after measuring each node on mount
// and `select` whenever a click toggles selection state. Neither
// changes the graph the user is editing, so neither should make the
// builder "dirty". Drag/move/add/remove/replace remain user signals.
const SYSTEM_NODE_CHANGE_TYPES: ReadonlySet<NodeChange["type"]> = new Set([
  "dimensions",
  "select",
]);
function hasUserNodeChange(changes: NodeChange[]): boolean {
  return changes.some((c) => !SYSTEM_NODE_CHANGE_TYPES.has(c.type));
}

export function useWorkflowBuilderState(
  initialNodes: Node[],
  initialEdges: Edge[],
) {
  const [nodes, setNodes] = useState<Node[]>(initialNodes);
  const [edges, setEdges] = useState<Edge[]>(initialEdges);
  const [dirty, setDirty] = useState(false);
  // Re-seed and clear dirty when the parent supplies a fresh
  // `initial` whose content differs from what we last seeded with
  // (e.g. async workflow fetch resolves after the hook first mounted
  // with empty placeholders). Without this, `nodes` stays as the old
  // empty seed while `initialNodes` is the loaded spec, and
  // `beforeunload` would arm on the first paint before the user has
  // interacted. Comparison is by JSON content (not identity) so a
  // parent that hands us a fresh literal each render — common in
  // tests, allowed for production callers — doesn't trigger an
  // infinite re-seed loop. See the React docs' "Storing information
  // from previous renders" pattern.
  const initialNodesJson = JSON.stringify(initialNodes);
  const initialEdgesJson = JSON.stringify(initialEdges);
  const [seededNodesJson, setSeededNodesJson] = useState(initialNodesJson);
  const [seededEdgesJson, setSeededEdgesJson] = useState(initialEdgesJson);
  if (
    seededNodesJson !== initialNodesJson
    || seededEdgesJson !== initialEdgesJson
  ) {
    setSeededNodesJson(initialNodesJson);
    setSeededEdgesJson(initialEdgesJson);
    setNodes(initialNodes);
    setEdges(initialEdges);
    setDirty(false);
  }

  const onNodesChange: OnNodesChange = useCallback((changes) => {
    setNodes((nds) => applyNodeChanges(changes, nds));
    if (hasUserNodeChange(changes)) setDirty(true);
  }, []);
  const onEdgesChange: OnEdgesChange = useCallback((changes) => {
    setEdges((eds) => applyEdgeChanges(changes, eds));
    // ReactFlow doesn't emit non-user edge changes during normal
    // initial render, so we don't need a system-vs-user filter here.
    if (changes.length > 0) setDirty(true);
  }, []);

  const addNode = useCallback((node: Node) => {
    setNodes((nds) => (nds.some((n) => n.id === node.id) ? nds : [...nds, node]));
    setDirty(true);
  }, []);

  const removeNode = useCallback((id: string) => {
    setNodes((nds) => nds.filter((n) => n.id !== id));
    setEdges((eds) => eds.filter((e) => e.source !== id && e.target !== id));
    setDirty(true);
  }, []);

  const addEdge = useCallback((edge: Edge) => {
    setEdges((eds) => (eds.some((e) => e.id === edge.id) ? eds : [...eds, edge]));
    setDirty(true);
  }, []);

  const removeEdge = useCallback((id: string) => {
    setEdges((eds) => eds.filter((e) => e.id !== id));
    setDirty(true);
  }, []);

  const reset = useCallback((nextNodes: Node[], nextEdges: Edge[]) => {
    setNodes(nextNodes);
    setEdges(nextEdges);
    setDirty(false);
  }, []);

  // Wrap raw setNodes/setEdges so that callers using the escape hatch
  // (e.g. inline step-config edits via `setNodes((nds) => ...)`) also
  // mark the canvas dirty. The hook's whole reason for existence is
  // to be the single source of truth for dirty tracking.
  const setNodesAndMark: typeof setNodes = useCallback((arg) => {
    setNodes(arg);
    setDirty(true);
  }, []);
  const setEdgesAndMark: typeof setEdges = useCallback((arg) => {
    setEdges(arg);
    setDirty(true);
  }, []);

  return {
    nodes,
    edges,
    setNodes: setNodesAndMark,
    setEdges: setEdgesAndMark,
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
