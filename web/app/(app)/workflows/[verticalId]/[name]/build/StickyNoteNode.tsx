"use client";
import type { NodeProps } from "@xyflow/react";
import { X } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";

import type { SkyNoteData } from "./types";

/**
 * Yellow Post-it style annotation node. Double-click to enter edit
 * mode (auto-focused textarea), blur to commit. The × button deletes
 * the note. Drag handle is the body — React Flow handles the actual
 * drag, so no custom pointer handlers needed.
 *
 * Intentionally a distinct `nodeType` ("sticky") so all step-aware
 * selectors (currentSteps, validateConnection, TOML serializer) ignore
 * stickies — they're annotations, not part of the executable graph.
 */
export function StickyNoteNode({ data }: NodeProps) {
  const sticky = data as SkyNoteData;
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(sticky.text);
  const textareaRef = useRef<HTMLTextAreaElement | null>(null);

  // External edits (undo/redo, hook-level updates) keep the local draft
  // in sync when we're NOT mid-edit. Skip while editing so the user's
  // in-flight keystrokes are not clobbered.
  useEffect(() => {
    if (!editing) setDraft(sticky.text);
  }, [sticky.text, editing]);

  useEffect(() => {
    if (editing) textareaRef.current?.focus();
  }, [editing]);

  const handleDoubleClick = useCallback(() => {
    setEditing(true);
  }, []);

  const handleBlur = useCallback(() => {
    setEditing(false);
    if (draft !== sticky.text) {
      sticky.onTextChange(sticky.noteId, draft);
    }
  }, [draft, sticky]);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
      if (e.key === "Escape") {
        e.preventDefault();
        setDraft(sticky.text);
        setEditing(false);
      }
    },
    [sticky.text],
  );

  const handleDelete = useCallback(
    (e: React.MouseEvent) => {
      e.stopPropagation();
      sticky.onDelete(sticky.noteId);
    },
    [sticky],
  );

  return (
    <div
      data-testid={`sticky-note-${sticky.noteId}`}
      onDoubleClick={handleDoubleClick}
      style={{
        width: 200,
        minHeight: 140,
        background: "linear-gradient(135deg, #fef9c3, #fef3c7)",
        border: "1px solid rgba(202, 138, 4, 0.3)",
        borderRadius: 4,
        padding: "12px 14px 14px",
        boxShadow:
          "0 1px 2px rgba(0,0,0,0.08), 0 8px 18px -8px rgba(202, 138, 4, 0.35)",
        cursor: editing ? "text" : "grab",
        fontFamily: "var(--font-mono), monospace",
        fontSize: 13,
        color: "#713f12",
        lineHeight: 1.45,
        position: "relative",
        // Slight tilt sells the Post-it aesthetic and visually
        // separates stickies from rectangular step nodes.
        transform: "rotate(-0.6deg)",
        whiteSpace: "pre-wrap",
        wordBreak: "break-word",
      }}
    >
      <button
        type="button"
        aria-label="Delete sticky note"
        data-testid={`sticky-note-delete-${sticky.noteId}`}
        onClick={handleDelete}
        // nodrag stops React Flow from initiating a node drag when the
        // user clicks the close button.
        className="nodrag"
        style={{
          position: "absolute",
          top: 4,
          right: 4,
          width: 18,
          height: 18,
          padding: 0,
          border: "none",
          borderRadius: 9,
          background: "rgba(202, 138, 4, 0.15)",
          color: "#713f12",
          cursor: "pointer",
          display: "grid",
          placeItems: "center",
        }}
      >
        <X size={11} />
      </button>
      {editing ? (
        <textarea
          ref={textareaRef}
          data-testid={`sticky-note-input-${sticky.noteId}`}
          className="nodrag"
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          onBlur={handleBlur}
          onKeyDown={handleKeyDown}
          placeholder="Type a note…"
          style={{
            width: "100%",
            minHeight: 110,
            background: "transparent",
            border: "none",
            outline: "none",
            resize: "none",
            color: "inherit",
            fontFamily: "inherit",
            fontSize: "inherit",
            lineHeight: "inherit",
            padding: 0,
          }}
        />
      ) : (
        <div style={{ minHeight: 110 }}>
          {sticky.text || (
            <span style={{ opacity: 0.55, fontStyle: "italic" }}>
              Double-click to edit
            </span>
          )}
        </div>
      )}
    </div>
  );
}
