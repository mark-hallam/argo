import { useCallback, useRef } from "react";
import type { Edge, Node } from "@xyflow/react";

/**
 * Snapshot of the builder graph that an undo can restore. Captured
 * BEFORE a mutation; `label` is human-readable (toast surface).
 */
export interface BuilderSnapshot {
  nodes: Node[];
  edges: Edge[];
  label: string;
  takenAt: number;
}

const MAX_HISTORY = 50;

/**
 * Cmd+Z / Cmd+Shift+Z history for the visual workflow builder. Holds
 * at most {@link MAX_HISTORY} entries per session, not persisted (page
 * reload resets). Past = before current, future = after an undo;
 * pushing invalidates future. Refs (not state) so push/undo/redo do
 * not re-render every keystroke or drag-stop.
 *
 * Usage:
 *   const history = useBuilderHistory();
 *   history.push({ nodes, edges, label: "Add Approval gate" });
 *   const prev = history.undo();
 *   if (prev) { setNodes(prev.nodes); setEdges(prev.edges); }
 */
export function useBuilderHistory() {
  const past = useRef<BuilderSnapshot[]>([]);
  const future = useRef<BuilderSnapshot[]>([]);

  const push = useCallback((snapshot: BuilderSnapshot) => {
    past.current.push(snapshot);
    if (past.current.length > MAX_HISTORY) past.current.shift();
    future.current = [];
  }, []);

  const undo = useCallback((): BuilderSnapshot | null => {
    const prev = past.current.pop();
    if (!prev) return null;
    return prev;
  }, []);

  const redo = useCallback((): BuilderSnapshot | null => {
    const next = future.current.pop();
    if (!next) return null;
    return next;
  }, []);

  const pushFuture = useCallback((snapshot: BuilderSnapshot) => {
    future.current.push(snapshot);
    if (future.current.length > MAX_HISTORY) future.current.shift();
  }, []);

  const pushPast = useCallback((snapshot: BuilderSnapshot) => {
    past.current.push(snapshot);
    if (past.current.length > MAX_HISTORY) past.current.shift();
  }, []);

  const canUndo = useCallback(() => past.current.length > 0, []);
  const canRedo = useCallback(() => future.current.length > 0, []);

  const clear = useCallback(() => {
    past.current = [];
    future.current = [];
  }, []);

  return { push, undo, redo, pushFuture, pushPast, canUndo, canRedo, clear };
}

export const HISTORY_LIMIT = MAX_HISTORY;
