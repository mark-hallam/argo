import { useCallback, useMemo, useState } from "react";
import type { Node } from "@xyflow/react";

import type {
  SkyNodeData,
  SkyNoteData,
  StickyNote,
} from "@/app/(app)/workflows/[verticalId]/[name]/build/types";

export interface UseBuilderAnnotationsResult {
  notes: StickyNote[];
  pinnedData: Record<string, string>;
  addNote: (x: number, y: number) => string;
  updateNoteText: (id: string, text: string) => void;
  updateNotePosition: (id: string, x: number, y: number) => void;
  deleteNote: (id: string) => void;
  savePin: (stepId: string, jsonText: string) => void;
  clearPin: (stepId: string) => void;
  noteNodes: Node[];
  decorateWithPins: (nodes: Node[]) => Node[];
  isNoteId: (id: string) => boolean;
}

function genId(prefix: string): string {
  if (typeof crypto !== "undefined" && "randomUUID" in crypto) {
    return `${prefix}-${crypto.randomUUID().slice(0, 8)}`;
  }
  return `${prefix}-${Math.random().toString(36).slice(2, 10)}`;
}

/**
 * Owns the workflow-builder annotation layer: sticky notes + per-step
 * pinned input data. Lives in its own hook so `page.tsx` stays close
 * to orchestrator-shaped — every callback the canvas + inspector +
 * test-run banner need is returned here, and pin state is projected
 * into step-node `data.pinned` via `decorateWithPins` so the canvas
 * node renders the chip without prop-drilling.
 *
 * Pinned data is JSON text (not parsed objects) because the editor
 * round-trips the user's whitespace + formatting; parsing happens at
 * test-run time, not at edit time.
 */
export function useBuilderAnnotations(): UseBuilderAnnotationsResult {
  const [notes, setNotes] = useState<StickyNote[]>([]);
  const [pinnedData, setPinnedData] = useState<Record<string, string>>({});

  const addNote = useCallback((x: number, y: number): string => {
    const id = genId("note");
    setNotes((prev) => [
      ...prev,
      { id, text: "", x, y },
    ]);
    return id;
  }, []);

  const updateNoteText = useCallback((id: string, text: string) => {
    setNotes((prev) => prev.map((n) => (n.id === id ? { ...n, text } : n)));
  }, []);

  const updateNotePosition = useCallback((id: string, x: number, y: number) => {
    setNotes((prev) =>
      prev.map((n) => (n.id === id && (n.x !== x || n.y !== y) ? { ...n, x, y } : n)),
    );
  }, []);

  const deleteNote = useCallback((id: string) => {
    setNotes((prev) => prev.filter((n) => n.id !== id));
  }, []);

  const savePin = useCallback((stepId: string, jsonText: string) => {
    setPinnedData((prev) => ({ ...prev, [stepId]: jsonText }));
  }, []);

  const clearPin = useCallback((stepId: string) => {
    setPinnedData((prev) => {
      if (!(stepId in prev)) return prev;
      const next = { ...prev };
      delete next[stepId];
      return next;
    });
  }, []);

  // Build ReactFlow nodes for the current notes. Memoised on the notes
  // array + the two stable callbacks; React Flow re-renders only when
  // the array identity changes.
  const noteNodes = useMemo<Node[]>(() => {
    return notes.map((note) => {
      const data: SkyNoteData = {
        noteId: note.id,
        text: note.text,
        onTextChange: updateNoteText,
        onDelete: deleteNote,
      };
      return {
        id: note.id,
        type: "sticky",
        position: { x: note.x, y: note.y },
        data,
        // Stickies live behind step nodes so they read as annotations,
        // not as primary graph elements.
        zIndex: -1,
        // ReactFlow's default selection styling clashes with the
        // Post-it yellow; opt out and let the component own its look.
        selectable: true,
        draggable: true,
      };
    });
  }, [notes, updateNoteText, deleteNote]);

  // Projects current pin state into step-node `data.pinned`. Idempotent
  // when the pin set is empty so the common case (no pins) returns the
  // input array unchanged and React Flow's diff skips work.
  const decorateWithPins = useCallback(
    (nodes: Node[]): Node[] => {
      const pinnedIds = Object.keys(pinnedData);
      if (pinnedIds.length === 0) return nodes;
      const set = new Set(pinnedIds);
      return nodes.map((n) => {
        const sky = n.data as SkyNodeData | undefined;
        if (!sky?.step) return n;
        const shouldBePinned = set.has(sky.step.id);
        if (!!sky.pinned === shouldBePinned) return n;
        return { ...n, data: { ...sky, pinned: shouldBePinned } };
      });
    },
    [pinnedData],
  );

  const isNoteId = useCallback(
    (id: string) => notes.some((n) => n.id === id),
    [notes],
  );

  return {
    notes,
    pinnedData,
    addNote,
    updateNoteText,
    updateNotePosition,
    deleteNote,
    savePin,
    clearPin,
    noteNodes,
    decorateWithPins,
    isNoteId,
  };
}
